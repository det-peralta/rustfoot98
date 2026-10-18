//! Drawing primitives and the controls, following what Windows 3.1 and Delphi 1 drew.
use super::fb::{Fb, Rgb, BLACK, GRAY, GREEN, NAVY, SILVER, WHITE};
use super::form::Node;
use super::skin::{ansi, Font, Skin};
use std::collections::BTreeMap;

/// The effective font of a control: Delphi inherits the parent's font unless the control sets its own.
#[derive(Clone, Debug)]
pub struct FontSpec {
    pub name: String,
    pub px: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Rgb,
}

impl Default for FontSpec {
    fn default() -> Self {
        FontSpec {
            name: "ms sans serif".into(),
            px: 11,
            bold: false,
            italic: false,
            underline: false,
            color: BLACK,
        }
    }
}

impl FontSpec {
    pub fn inherit(&self, n: &Node) -> FontSpec {
        let mut f = self.clone();
        if let Some(s) = n.str("Font.Name") {
            f.name = s.to_lowercase();
        }
        if n.props.contains_key("Font.Height") {
            f.px = -n.int("Font.Height", -11);
        }
        if n.props.contains_key("Font.Style") {
            let st = n.set("Font.Style");
            f.bold = st.contains(&"fsBold");
            f.italic = st.contains(&"fsItalic");
            f.underline = st.contains(&"fsUnderline");
        }
        if let Some(c) = n.color("Font.Color") {
            f.color = c;
        }
        f
    }
}

/// Everything a control needs to draw itself.
pub struct Ctx<'a> {
    pub skin: &'a Skin,
    pub form: &'a str,
    /// Captions set by the program at run time, by control name.
    pub texts: &'a BTreeMap<String, String>,
    /// Origin of the form's client area on the screen.
    pub ox: i32,
    pub oy: i32,
    pub bg: Rgb,
    /// Name of the control that has the keyboard focus (the form's `ActiveControl`).
    pub focus: &'a str,
}

impl Ctx<'_> {
    pub fn font(&self, f: &FontSpec) -> Option<&Font> {
        self.skin.font(&f.name, f.px, f.bold, f.italic)
    }
}

/// Draws `s` with the top-left corner of its cell at (x, y); returns the advance.
pub fn text(fb: &mut Fb, font: &Font, x: i32, y: i32, s: &str, color: Rgb, underline: bool) -> i32 {
    let base = y + font.ascent;
    let mut pen = x;
    for c in s.chars() {
        let Some(g) = font.glyph(ansi(c)) else {
            continue;
        };
        for (r, bits) in g.rows.iter().enumerate() {
            for k in 0..g.w {
                if bits >> (g.w - 1 - k) & 1 == 1 {
                    fb.set(pen + g.left + k, base - g.top + r as i32, color);
                }
            }
        }
        pen += g.adv;
    }
    if underline {
        fb.fill(x, base + font.ul_pos, pen - x, font.ul_thick, color);
    }
    pen - x
}

/// Splits `s` into lines no wider than `width` (spaces break lines), like `DrawText` with `DT_WORDBREAK`.
pub fn wrap(font: &Font, s: &str, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let trial = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.width(&trial) <= width || line.is_empty() {
                line = trial;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        lines.push(line);
    }
    lines
}

/// A form with a caption bar (`bsSingle`): a black frame, a navy caption bar 18 pixels high and a black rule below it.
/// Returns the origin of the client area.
pub fn dialog_frame(
    fb: &mut Fb,
    skin: &Skin,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
) -> (i32, i32) {
    let (ow, oh) = (cw + 2, ch + 21);
    let (x, y) = ((super::fb::W - ow) / 2, (super::fb::H - oh) / 2);
    fb.fill(x, y, ow, oh, BLACK);
    fb.fill(x + 1, y + 1, cw, 18, NAVY);
    fb.fill(x + 1, y + 20, cw, ch, bg);
    if let Some(f) = skin.font("system", 13, false, false) {
        let w = f.width(caption);
        text(fb, f, x + 1 + (cw - w) / 2, y + 2, caption, WHITE, false);
    }
    (x + 1, y + 20)
}

/// The face of a `TBitBtn`/`TButton` in its normal state: a black outline with cut corners (2 pixels for the default
/// button, 1 for the others), a white and a grey bevel of 2 pixels.
pub fn button_face(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, default: bool) {
    let i = if default { 2 } else { 1 };
    fb.fill(x, y, w, h, BLACK);
    fb.fill(x + i + 2, y + i + 2, w - 2 * i - 4, h - 2 * i - 4, SILVER);
    fb.fill(x + i, y + i, w - 2 * i, 2, WHITE);
    fb.fill(x + i, y + i, 2, h - 2 * i, WHITE);
    fb.fill(x + w - i - 2, y + i, 2, h - 2 * i, GRAY);
    fb.fill(x + i, y + h - i - 2, w - 2 * i, 2, GRAY);
    // mitred corners of the bevel and the cut outline corners
    fb.set(x + w - i - 2, y + i, WHITE);
    fb.set(x + w - i - 1, y + i, GRAY);
    fb.set(x + w - i - 2, y + i + 1, GRAY);
    fb.set(x + i - 1, y + h - i - 1, GRAY);
    for (cx, cy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        fb.set(x + cx, y + cy, SILVER);
    }
}

/// Draws the controls of `root` (the form) in order.
pub fn controls(fb: &mut Fb, cx: &Ctx, root: &Node) {
    let base = FontSpec::default().inherit(root);
    for n in &root.children {
        control(fb, cx, n, &base);
    }
}

fn control(fb: &mut Fb, cx: &Ctx, n: &Node, parent: &FontSpec) {
    if !n.flag("Visible", true) {
        return;
    }
    let f = parent.inherit(n);
    let (x, y) = (cx.ox + n.int("Left", 0), cx.oy + n.int("Top", 0));
    match n.class.as_str() {
        "TLabel" => label(fb, cx, n, &f, x, y),
        "TBitBtn" | "TButton" => bitbtn(fb, cx, n, &f, x, y),
        "TMemo" => memo(fb, cx, n, &f, x, y),
        "TBanner" => banner(fb, cx, n, &f, x, y),
        _ => {}
    }
}

fn caption<'a>(cx: &'a Ctx, n: &'a Node) -> &'a str {
    cx.texts
        .get(&n.name)
        .map(String::as_str)
        .or_else(|| n.str("Caption"))
        .unwrap_or("")
}

fn label(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let w = n.int("Width", 0);
    let s = caption(cx, n);
    let lines = if n.flag("WordWrap", false) {
        wrap(font, s, w)
    } else {
        vec![s.to_string()]
    };
    for (i, line) in lines.iter().enumerate() {
        let tw = font.width(line);
        let lx = match n.ident("Alignment") {
            Some("taCenter") => x + (w - tw) / 2,
            Some("taRightJustify") => x + w - tw,
            _ => x,
        };
        text(
            fb,
            font,
            lx,
            y + i as i32 * font.height(),
            line,
            f.color,
            f.underline,
        );
    }
}

fn memo(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let w = n.int("Width", 0);
    for (i, line) in n.strings("Lines.Strings").iter().enumerate() {
        let tw = font.width(line);
        let lx = match n.ident("Alignment") {
            Some("taCenter") => x + (w - tw) / 2,
            Some("taRightJustify") => x + w - tw,
            _ => x,
        };
        text(
            fb,
            font,
            lx,
            y + i as i32 * font.height(),
            line,
            f.color,
            false,
        );
    }
}

/// `TBanner`: the scrolling text; `texts["Banner1.pos"]`-style state: `<name>.scroll` is how many pixels the text has moved left.
fn banner(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let scroll: i32 = cx
        .texts
        .get(&format!("{}.scroll", n.name))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let old = fb.clip;
    fb.clip = (
        x.max(old.0),
        y.max(old.1),
        (x + w).min(old.2),
        (y + h).min(old.3),
    );
    text(
        fb,
        font,
        x + w - scroll,
        y,
        n.str("ScrollText").unwrap_or(""),
        f.color,
        false,
    );
    fb.clip = old;
}

fn bitbtn(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 64), n.int("Height", 64));
    let default = cx.focus == n.name || n.flag("Default", false);
    // content of the thin-framed button sits one pixel up and to the right of the default button's
    let (ix, iy) = if default { (0, 2) } else { (1, 1) };
    button_face(fb, x, y, w, h, default);
    let kind = n.ident("Kind").unwrap_or("bkCustom");
    let std = match kind {
        "bkOK" => Some(("BBOK", "OK")),
        "bkCancel" => Some(("BBCANCEL", "Cancelar")),
        "bkYes" => Some(("BBYES", "&Sim")),
        "bkNo" => Some(("BBNO", "&Não")),
        "bkHelp" => Some(("BBHELP", "&Ajuda")),
        "bkClose" => Some(("BBCLOSE", "&Fechar")),
        "bkAbort" => Some(("BBABORT", "&Abortar")),
        "bkRetry" => Some(("BBRETRY", "&Repetir")),
        "bkIgnore" => Some(("BBIGNORE", "&Ignorar")),
        "bkAll" => Some(("BBALL", "&Todos")),
        _ => None,
    };
    let bmp = cx
        .skin
        .bitmap(cx.form, &n.name)
        .or_else(|| std.and_then(|(b, _)| cx.skin.bitmap("*", b)));
    let cap = match caption(cx, n) {
        "" => std.map_or("", |(_, c)| c),
        c => c,
    };
    let gh = bmp.map_or(0, |b| b.h);
    if let Some(b) = bmp {
        let glyphs = if b.w >= 2 * b.h {
            2.max(n.int("NumGlyphs", 1))
        } else {
            n.int("NumGlyphs", 1).max(1)
        };
        let gw = b.w / glyphs;
        let key = b.px[((b.h - 1) * b.w) as usize];
        let (gx, gy) = (x + (w - gw) / 2, y + 2 + n.int("Margin", 4));
        for r in 0..b.h {
            for c in 0..gw {
                let p = b.px[(r * b.w + c) as usize];
                if p != key {
                    fb.set(
                        gx + c,
                        gy + r,
                        Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    );
                }
            }
        }
    }
    if let Some(font) = cx.font(f) {
        let label: String = cap.replace('&', "");
        let tw = font.width(&label);
        let ty = y + h - 2 - n.int("Margin", 4) - font.height() - (2 - iy);
        let _ = gh;
        text(fb, font, x + ix + (w - tw) / 2, ty, &label, f.color, false);
        if cx.focus == n.name {
            focus_rect(fb, x + (w - tw) / 2 - 1, ty - 1, tw + 2, font.height() + 2);
        }
    }
}

/// `DrawFocusRect`: a dotted rectangle (every other pixel, `(x + y)` even), corners left out.
pub fn focus_rect(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32) {
    for i in 0..w {
        for yy in [y, y + h - 1] {
            if (x + i + yy) % 2 == 0 && i != 0 && i != w - 1 {
                fb.set(x + i, yy, BLACK);
            }
        }
    }
    for j in 1..h - 1 {
        for xx in [x, x + w - 1] {
            if (xx + y + j) % 2 == 0 {
                fb.set(xx, y + j, BLACK);
            }
        }
    }
}

/// The desktop colour of the original install.
pub const DESKTOP: Rgb = GREEN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_button_has_a_thick_frame() {
        let mut fb = Fb::new();
        button_face(&mut fb, 10, 10, 64, 64, true);
        assert_eq!(fb.get(10, 40), 0x000000); // frame column 0
        assert_eq!(fb.get(11, 40), 0x000000); // frame column 1
        assert_eq!(fb.get(12, 40), 0xf8fcf8); // white bevel
        button_face(&mut fb, 100, 10, 64, 64, false);
        assert_eq!(fb.get(100, 40), 0x000000);
        assert_eq!(fb.get(101, 40), 0xf8fcf8);
        assert_eq!(fb.get(100, 10), 0xc0c0c0); // cut corner
    }

    #[test]
    fn the_focus_rectangle_is_dotted_without_corners() {
        let mut fb = Fb::new();
        fb.fill(0, 0, 50, 50, SILVER);
        focus_rect(&mut fb, 10, 10, 16, 13);
        assert_eq!(fb.get(10, 10), 0xc0c0c0);
        assert_eq!(fb.get(11, 10), 0xc0c0c0); // (x + y) odd: off
        assert_eq!(fb.get(12, 10), 0x000000); // even: on
        assert_eq!(fb.get(10, 12), 0x000000);
    }

    #[test]
    fn labels_wrap_at_spaces() {
        // a tiny hand-made font: every glyph 3 wide
        let skin_bytes = {
            let mut b = b"EFSK".to_vec();
            b.extend_from_slice(&1u16.to_le_bytes());
            b.push(1);
            b.push(b'x');
            b.extend_from_slice(&[10, 0, 0, 8, 2, 32, 1, 1]);
            for _ in 32..256 {
                b.extend_from_slice(&[3, 0, 8, 0, 0]);
            }
            b
        };
        let mut g = b"EFGL".to_vec();
        g.extend_from_slice(&0u16.to_le_bytes());
        let skin = Skin::from_bytes(&skin_bytes, &g).unwrap();
        let f = skin.font("x", 10, false, false).unwrap();
        assert_eq!(wrap(f, "ab cd ef", 9), vec!["ab", "cd", "ef"]);
        assert_eq!(wrap(f, "ab cd ef", 20), vec!["ab cd", "ef"]);
    }
}
