//! The frame buffer: 800x600, one `u32` (0x00RRGGBB) per pixel, colours quantised like the original's 16-bit display
//! (5-6-5 bits, no replication: white is 248,252,248).
pub const W: i32 = 800;
pub const H: i32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// The colour as the 16-bit display shows it.
    pub fn q(self) -> Rgb {
        Rgb(self.0 & 0xf8, self.1 & 0xfc, self.2 & 0xf8)
    }
    pub fn pack(self) -> u32 {
        (self.0 as u32) << 16 | (self.1 as u32) << 8 | self.2 as u32
    }
}

pub const BLACK: Rgb = Rgb(0, 0, 0);
pub const WHITE: Rgb = Rgb(255, 255, 255);
pub const SILVER: Rgb = Rgb(192, 192, 192);
pub const GRAY: Rgb = Rgb(128, 128, 128);
pub const GREEN: Rgb = Rgb(0, 128, 0);
pub const NAVY: Rgb = Rgb(0, 0, 128);
pub const YELLOW: Rgb = Rgb(255, 255, 0);

pub struct Fb {
    pub px: Vec<u32>,
    /// Drawing is clipped to this rectangle (x0, y0, x1, y1; the end is exclusive).
    pub clip: (i32, i32, i32, i32),
}

impl Default for Fb {
    fn default() -> Self {
        Fb::new()
    }
}

impl Fb {
    pub fn new() -> Fb {
        Fb {
            px: vec![0; (W * H) as usize],
            clip: (0, 0, W, H),
        }
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgb) {
        let (x0, y0, x1, y1) = self.clip;
        if x >= x0 && x < x1 && y >= y0 && y < y1 {
            self.px[(y * W + x) as usize] = c.q().pack();
        }
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        if (0..W).contains(&x) && (0..H).contains(&y) {
            self.px[(y * W + x) as usize]
        } else {
            0
        }
    }

    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
        let (x0, y0, x1, y1) = self.clip;
        for yy in y.max(y0)..(y + h).min(y1) {
            for xx in x.max(x0)..(x + w).min(x1) {
                self.px[(yy * W + xx) as usize] = c.q().pack();
            }
        }
    }

    /// A 24-bit BMP file of the whole buffer (for the screenshot tool and the pixel comparisons).
    pub fn bmp(&self) -> Vec<u8> {
        let row = (W * 3 + 3) & !3;
        let size = 54 + row * H;
        let mut out = Vec::with_capacity(size as usize);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0, 54, 0, 0, 0, 40, 0, 0, 0]);
        out.extend_from_slice(&(W as u32).to_le_bytes());
        out.extend_from_slice(&(H as u32).to_le_bytes());
        out.extend_from_slice(&[1, 0, 24, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&((row * H) as u32).to_le_bytes());
        out.extend_from_slice(&[0; 16]);
        for y in (0..H).rev() {
            for x in 0..W {
                let p = self.px[(y * W + x) as usize];
                out.extend_from_slice(&[p as u8, (p >> 8) as u8, (p >> 16) as u8]);
            }
            out.resize(out.len() + (row - W * 3) as usize, 0);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_quantised_like_the_16_bit_display() {
        let mut fb = Fb::new();
        fb.set(1, 1, WHITE);
        fb.set(2, 1, YELLOW);
        fb.set(3, 1, GREEN);
        assert_eq!(fb.get(1, 1), 0xf8fcf8);
        assert_eq!(fb.get(2, 1), 0xf8fc00);
        assert_eq!(fb.get(3, 1), 0x008000);
    }

    #[test]
    fn drawing_is_clipped() {
        let mut fb = Fb::new();
        fb.clip = (10, 10, 20, 20);
        fb.fill(0, 0, 100, 100, WHITE);
        assert_eq!(fb.get(9, 15), 0);
        assert_eq!(fb.get(10, 10), 0xf8fcf8);
        assert_eq!(fb.get(20, 15), 0);
    }

    #[test]
    fn bmp_has_the_right_size() {
        assert_eq!(Fb::new().bmp().len(), 54 + 800 * 3 * 600);
    }
}
