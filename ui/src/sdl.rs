//! SDL2 front end: implements [`Canvas`] on an SDL2 renderer (window or offscreen surface) with SDL2_ttf text, and maps SDL events.
use crate::canvas::{Canvas, Rect, Rgb};
use crate::{Event, Key};
use sdl2::pixels::Color;
use sdl2::rect::Rect as SRect;
use sdl2::render::{Canvas as SdlCanvas, RenderTarget, TextureCreator};
use sdl2::ttf::{Font, Sdl2TtfContext};
use std::collections::HashMap;

/// A font file found on the system (the original used the Windows 3.1 bitmap fonts, which are not redistributable).
pub fn find_font() -> Option<String> {
    if let Ok(p) = std::env::var("ELIFOOT_FONT") {
        return Some(p);
    }
    [
        "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ]
    .iter()
    .find(|p| std::path::Path::new(p).exists())
    .map(|s| s.to_string())
}

pub struct Backend<'a, 'ttf, T: RenderTarget, C> {
    pub canvas: &'a mut SdlCanvas<T>,
    creator: &'a TextureCreator<C>,
    ttf: &'ttf Sdl2TtfContext,
    path: String,
    fonts: HashMap<i32, Font<'ttf, 'static>>,
}

impl<'a, 'ttf, T: RenderTarget, C> Backend<'a, 'ttf, T, C> {
    pub fn new(
        canvas: &'a mut SdlCanvas<T>,
        creator: &'a TextureCreator<C>,
        ttf: &'ttf Sdl2TtfContext,
        path: String,
    ) -> Self {
        Backend {
            canvas,
            creator,
            ttf,
            path,
            fonts: HashMap::new(),
        }
    }

    fn font(&mut self, size: i32) -> &Font<'ttf, 'static> {
        let (ttf, path) = (self.ttf, self.path.clone());
        self.fonts.entry(size).or_insert_with(|| {
            ttf.load_font(path, size.max(6) as u16)
                .expect("cannot load the font")
        })
    }
}

fn col(c: Rgb) -> Color {
    Color::RGB(c.0, c.1, c.2)
}

fn srect(r: Rect) -> SRect {
    SRect::new(r.x, r.y, r.w.max(0) as u32, r.h.max(0) as u32)
}

impl<T: RenderTarget, C> Canvas for Backend<'_, '_, T, C> {
    fn fill(&mut self, r: Rect, c: Rgb) {
        self.canvas.set_draw_color(col(c));
        let _ = self.canvas.fill_rect(srect(r));
    }
    fn outline(&mut self, r: Rect, c: Rgb) {
        self.canvas.set_draw_color(col(c));
        let _ = self.canvas.draw_rect(srect(r));
    }
    fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb, size: i32) {
        if s.is_empty() {
            return;
        }
        let surface = match self.font(size).render(s).blended(col(c)) {
            Ok(s) => s,
            Err(_) => return,
        };
        if let Ok(tex) = self.creator.create_texture_from_surface(&surface) {
            let _ = self.canvas.copy(
                &tex,
                None,
                SRect::new(x, y, surface.width(), surface.height()),
            );
        }
    }
    fn text_width(&self, s: &str, size: i32) -> i32 {
        match self.fonts.get(&size) {
            Some(f) => f.size_of(s).map(|(w, _)| w as i32).unwrap_or(0),
            None => s.chars().count() as i32 * size * 11 / 20,
        }
    }
}

/// Maps an SDL event to a UI event (`None` for the ones the UI ignores).
pub fn map_event(e: &sdl2::event::Event) -> Option<Event> {
    use sdl2::event::Event as E;
    use sdl2::keyboard::Keycode as K;
    Some(match e {
        E::MouseMotion { x, y, .. } => Event::MouseMove(*x, *y),
        E::MouseButtonDown { x, y, .. } => Event::MouseDown(*x, *y),
        E::MouseButtonUp { x, y, .. } => Event::MouseUp(*x, *y),
        E::MouseWheel { y, .. } => Event::Wheel(*y),
        E::TextInput { text, .. } => Event::Text(text.chars().next()?),
        E::KeyDown {
            keycode: Some(k), ..
        } => Event::Key(match *k {
            K::Space => Key::Space,
            K::S => Key::S,
            K::Tab => Key::Tab,
            K::Left => Key::Left,
            K::Right => Key::Right,
            K::Up => Key::Up,
            K::Down => Key::Down,
            K::Escape => Key::Escape,
            K::Backspace => Key::Backspace,
            K::Return | K::KpEnter => Key::Enter,
            K::Num1 | K::Kp1 => Key::Num(1),
            K::Num2 | K::Kp2 => Key::Num(2),
            K::Num3 | K::Kp3 => Key::Num(3),
            K::Num4 | K::Kp4 => Key::Num(4),
            _ => return None,
        }),
        _ => return None,
    })
}
