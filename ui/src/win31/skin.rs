//! The skin pack: rasterised fonts (`fonts.bin`) and the buttons' bitmaps (`glyphs.bin`), both produced by
//! `tools/ui/skin.py` from the user's own game and Windows 3.1 install.
use std::path::Path;

pub struct Glyph {
    pub adv: i32,
    pub left: i32,
    pub top: i32,
    pub w: i32,
    pub rows: Vec<u64>,
}

pub struct Font {
    pub face: String,
    pub px: i32,
    pub bold: bool,
    pub italic: bool,
    pub ascent: i32,
    pub descent: i32,
    /// Underline: offset of its top below the baseline, and thickness.
    pub ul_pos: i32,
    pub ul_thick: i32,
    /// GDI's `tmHeight` (the cell a button caption is aligned with); `height()` is the line pitch of labels.
    pub tm_height: i32,
    first: u8,
    glyphs: Vec<Glyph>,
}

impl Font {
    pub fn height(&self) -> i32 {
        self.ascent + self.descent
    }
    pub fn glyph(&self, code: u8) -> Option<&Glyph> {
        self.glyphs.get(code.checked_sub(self.first)? as usize)
    }
    /// Width of `s` in pixels (Windows' ANSI code page for the accented letters; no kerning).
    pub fn width(&self, s: &str) -> i32 {
        s.chars()
            .map(|c| self.glyph(ansi(c)).map_or(0, |g| g.adv))
            .sum()
    }
}

/// The byte a character has in the Windows ANSI code page (Latin-1 range only; anything else becomes `?`).
pub fn ansi(c: char) -> u8 {
    let n = c as u32;
    if n < 256 {
        n as u8
    } else {
        b'?'
    }
}

pub struct Bitmap {
    pub w: i32,
    pub h: i32,
    /// 0x00RRGGBB, top row first.
    pub px: Vec<u32>,
}

pub struct Skin {
    fonts: Vec<Font>,
    bitmaps: Vec<(String, String, Bitmap)>,
}

struct Rd<'a> {
    d: &'a [u8],
    o: usize,
}

impl Rd<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let v = *self.d.get(self.o).ok_or("truncated skin")?;
        self.o += 1;
        Ok(v)
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(self.u8()? as u16 | (self.u8()? as u16) << 8)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(self.u16()? as u32 | (self.u16()? as u32) << 16)
    }
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self.d.get(self.o..self.o + n).ok_or("truncated skin")?;
        self.o += n;
        Ok(s)
    }
    fn pstr(&mut self) -> Result<String, String> {
        let n = self.u8()? as usize;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }
}

impl Skin {
    pub fn load(dir: &Path) -> Result<Skin, String> {
        let fonts = std::fs::read(dir.join("fonts.bin")).map_err(|e| format!("fonts.bin: {e}"))?;
        let glyphs =
            std::fs::read(dir.join("glyphs.bin")).map_err(|e| format!("glyphs.bin: {e}"))?;
        Skin::from_bytes(&fonts, &glyphs)
    }

    pub fn from_bytes(fonts: &[u8], glyphs: &[u8]) -> Result<Skin, String> {
        let mut r = Rd { d: fonts, o: 0 };
        if r.take(4)? != b"EFSK" {
            return Err("not a font pack".into());
        }
        let mut list = Vec::new();
        for _ in 0..r.u16()? {
            let face = r.pstr()?;
            let (px, bold, italic, ascent, descent, first, ul_pos, ul_thick, tm_height) = (
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
                r.u8()?,
            );
            let mut gl = Vec::new();
            for _ in first as usize..256 {
                let (adv, left, top, w, h) =
                    (r.u8()?, r.u8()? as i8, r.u8()? as i8, r.u8()?, r.u8()?);
                let nb = (w as usize).div_ceil(8);
                let mut rows = Vec::new();
                for _ in 0..h {
                    let mut v = 0u64;
                    for _ in 0..nb {
                        v = v << 8 | r.u8()? as u64;
                    }
                    rows.push(v >> (nb * 8 - w as usize));
                }
                gl.push(Glyph {
                    adv: adv as i32,
                    left: left as i32,
                    top: top as i32,
                    w: w as i32,
                    rows,
                });
            }
            list.push(Font {
                face,
                px: px as i32,
                bold: bold != 0,
                italic: italic != 0,
                ascent: ascent as i32,
                descent: descent as i32,
                ul_pos: ul_pos as i32,
                ul_thick: ul_thick as i32,
                tm_height: tm_height as i32,
                first,
                glyphs: gl,
            });
        }
        let mut r = Rd { d: glyphs, o: 0 };
        if r.take(4)? != b"EFGL" {
            return Err("not a glyph pack".into());
        }
        let mut bitmaps = Vec::new();
        for _ in 0..r.u16()? {
            let form = r.pstr()?;
            let name = r.pstr()?;
            let n = r.u32()? as usize;
            let bmp = r.take(n)?.to_vec();
            if let Some(b) = decode_bmp(&bmp) {
                bitmaps.push((form, name, b));
            }
        }
        Ok(Skin {
            fonts: list,
            bitmaps,
        })
    }

    pub fn font(&self, face: &str, px: i32, bold: bool, italic: bool) -> Option<&Font> {
        let face = face.to_lowercase();
        self.fonts
            .iter()
            .find(|f| f.face == face && f.px == px && f.bold == bold && f.italic == italic)
            .or_else(|| {
                self.fonts
                    .iter()
                    .find(|f| f.face == face && f.px == px && f.bold == bold)
            })
    }

    pub fn bitmap(&self, form: &str, name: &str) -> Option<&Bitmap> {
        self.bitmaps
            .iter()
            .find(|(f, n, _)| f.eq_ignore_ascii_case(form) && n.eq_ignore_ascii_case(name))
            .map(|(_, _, b)| b)
    }
}

/// Decodes a 1, 4 or 8 bits-per-pixel (or 24) Windows BMP file.
pub fn decode_bmp(d: &[u8]) -> Option<Bitmap> {
    if d.get(..2)? != b"BM" {
        return None;
    }
    let u32at =
        |o: usize| u32::from_le_bytes(d.get(o..o + 4).unwrap_or(&[0; 4]).try_into().unwrap());
    let off = u32at(10) as usize;
    let hsize = u32at(14) as usize;
    let w = u32at(18) as i32;
    let h = u32at(22) as i32;
    let bpp = u16::from_le_bytes([*d.get(28)?, *d.get(29)?]) as usize;
    let ncol = match u32at(46) {
        0 if bpp <= 8 => 1 << bpp,
        n => n as usize,
    };
    let pal: Vec<u32> = (0..ncol)
        .map(|i| {
            let p = 14 + hsize + i * 4;
            (*d.get(p + 2).unwrap_or(&0) as u32) << 16
                | (*d.get(p + 1).unwrap_or(&0) as u32) << 8
                | *d.get(p).unwrap_or(&0) as u32
        })
        .collect();
    let row = (w as usize * bpp).div_ceil(32) * 4;
    let mut px = vec![0u32; (w * h) as usize];
    for y in 0..h {
        let src = off + (h - 1 - y) as usize * row;
        for x in 0..w as usize {
            let v = match bpp {
                1 => pal[(*d.get(src + x / 8)? >> (7 - x % 8) & 1) as usize],
                4 => pal[(*d.get(src + x / 2)? >> (if x % 2 == 0 { 4 } else { 0 }) & 15) as usize],
                8 => pal[*d.get(src + x)? as usize],
                24 => {
                    (*d.get(src + x * 3 + 2)? as u32) << 16
                        | (*d.get(src + x * 3 + 1)? as u32) << 8
                        | *d.get(src + x * 3)? as u32
                }
                _ => return None,
            };
            px[y as usize * w as usize + x] = v;
        }
    }
    Some(Bitmap { w, h, px })
}
