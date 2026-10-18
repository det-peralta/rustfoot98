//! The drawing interface every front end implements. A screen draws itself through [`Canvas`] only, so the same screens run on
//! SDL2 (desktop), on an HTML canvas (web) or on the recording [`DrawList`] used by the tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px < self.x + self.w && py < self.y + self.h
    }
    /// Shrinks the rectangle by `d` on every side.
    pub fn inset(&self, d: i32) -> Rect {
        Rect::new(
            self.x + d,
            self.y + d,
            (self.w - 2 * d).max(0),
            (self.h - 2 * d).max(0),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub trait Canvas {
    fn fill(&mut self, r: Rect, c: Rgb);
    fn outline(&mut self, r: Rect, c: Rgb);
    /// Draws `s` with its top-left corner at `(x, y)`; `size` is the font height in pixels.
    fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb, size: i32);
    fn text_width(&self, s: &str, size: i32) -> i32;
    fn line_height(&self, size: i32) -> i32 {
        size + size / 4 + 2
    }
}

/// One recorded drawing command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cmd {
    Fill(Rect, Rgb),
    Outline(Rect, Rgb),
    Text(i32, i32, String, Rgb, i32),
}

/// A canvas that only records what was drawn (approximate text metrics), for tests and golden-file checks.
#[derive(Default)]
pub struct DrawList {
    pub cmds: Vec<Cmd>,
}

impl Canvas for DrawList {
    fn fill(&mut self, r: Rect, c: Rgb) {
        self.cmds.push(Cmd::Fill(r, c));
    }
    fn outline(&mut self, r: Rect, c: Rgb) {
        self.cmds.push(Cmd::Outline(r, c));
    }
    fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb, size: i32) {
        self.cmds.push(Cmd::Text(x, y, s.to_string(), c, size));
    }
    fn text_width(&self, s: &str, size: i32) -> i32 {
        s.chars().count() as i32 * size * 11 / 20
    }
}

impl DrawList {
    pub fn texts(&self) -> Vec<&str> {
        self.cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Text(_, _, s, _, _) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }
}
