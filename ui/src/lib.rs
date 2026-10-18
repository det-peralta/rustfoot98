//! Immediate-mode UI of the Elifoot 98 port, independent of the window system (see `docs/ui-strategy.md`).
//!
//! [`App`] owns the game ([`elifoot_core::world::World`]), receives [`Event`]s and draws the current [`Screen`] through a
//! [`canvas::Canvas`]. Buttons are immediate: a screen calls [`App::button`] while drawing and gets `true` on the frame in which the
//! user releases the mouse over it. No retained widget tree, no callbacks: a screen is one function from state to pixels, which keeps
//! the front ends (SDL2, web canvas) trivial to maintain.
pub mod canvas;
mod screens;

use canvas::{Canvas, Rect, Rgb};
use elifoot_core::world::World;

pub const WIDTH: i32 = 800;
pub const HEIGHT: i32 = 600;

pub mod theme {
    use super::Rgb;
    pub const BG: Rgb = Rgb(18, 22, 30);
    pub const PANEL: Rgb = Rgb(28, 34, 46);
    pub const LINE: Rgb = Rgb(52, 62, 80);
    pub const TEXT: Rgb = Rgb(226, 232, 240);
    pub const MUTED: Rgb = Rgb(138, 150, 168);
    pub const ACCENT: Rgb = Rgb(56, 189, 248);
    pub const GOOD: Rgb = Rgb(74, 222, 128);
    pub const BAD: Rgb = Rgb(248, 113, 113);
    pub const BUTTON: Rgb = Rgb(40, 52, 72);
    pub const BUTTON_HOT: Rgb = Rgb(58, 76, 106);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Space,
    S,
    Tab,
    Num(u8),
    Left,
    Right,
    Up,
    Down,
    Escape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    MouseMove(i32, i32),
    MouseDown(i32, i32),
    MouseUp(i32, i32),
    Wheel(i32),
    Key(Key),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    League,
    Squad,
    Finance,
    Cup,
}

pub const SCREENS: [(Screen, &str); 4] = [
    (Screen::League, "Liga"),
    (Screen::Squad, "Plantel"),
    (Screen::Finance, "Finanças"),
    (Screen::Cup, "Taça"),
];

pub struct App {
    pub world: World,
    pub screen: Screen,
    pub division: usize,
    pub scroll: i32,
    pub message: String,
    mouse: (i32, i32),
    down: bool,
    released: bool,
}

impl App {
    pub fn new(seed: u32) -> App {
        let world = World::synthetic(seed);
        let division = world.human_location().map_or(0, |l| l.0);
        App {
            world,
            screen: Screen::League,
            division,
            scroll: 0,
            message: String::new(),
            mouse: (0, 0),
            down: false,
            released: false,
        }
    }

    pub fn event(&mut self, e: Event) {
        match e {
            Event::MouseMove(x, y) => self.mouse = (x, y),
            Event::MouseDown(x, y) => {
                self.mouse = (x, y);
                self.down = true;
            }
            Event::MouseUp(x, y) => {
                self.mouse = (x, y);
                self.down = false;
                self.released = true;
            }
            Event::Wheel(d) => self.scroll = (self.scroll - d * 24).max(0),
            Event::Key(k) => match k {
                Key::Space => self.play_round(),
                Key::S => self.play_season(),
                Key::Tab => self.next_screen(),
                Key::Num(n) if (1..=4).contains(&n) => self.division = n as usize - 1,
                Key::Up => self.scroll = (self.scroll - 24).max(0),
                Key::Down => self.scroll += 24,
                _ => {}
            },
        }
    }

    pub fn play_round(&mut self) {
        self.world.play_round();
        self.message = format!("Jornada {} jogada.", self.world.round);
        self.scroll = 0;
    }

    pub fn play_season(&mut self) {
        self.world.play_season();
        self.message = format!("Época {} terminada.", self.world.year - 1);
        self.scroll = 0;
    }

    fn next_screen(&mut self) {
        let i = SCREENS
            .iter()
            .position(|(s, _)| *s == self.screen)
            .unwrap_or(0);
        self.screen = SCREENS[(i + 1) % SCREENS.len()].0;
        self.scroll = 0;
    }

    /// Draws one frame. Call after feeding the events of the frame.
    pub fn draw(&mut self, c: &mut dyn Canvas) {
        c.fill(Rect::new(0, 0, WIDTH, HEIGHT), theme::BG);
        self.header(c);
        match self.screen {
            Screen::League => screens::league(self, c),
            Screen::Squad => screens::squad(self, c),
            Screen::Finance => screens::finance(self, c),
            Screen::Cup => screens::cup(self, c),
        }
        self.released = false;
    }

    fn header(&mut self, c: &mut dyn Canvas) {
        c.fill(Rect::new(0, 0, WIDTH, 44), theme::PANEL);
        c.text(12, 10, "Elifoot 98", theme::ACCENT, 20);
        let status = format!(
            "Época {}   Jornada {}/14   Economia {:.2}",
            self.world.year, self.world.round, self.world.economy
        );
        c.text(150, 14, &status, theme::MUTED, 14);
        let mut x = 470;
        for (s, label) in SCREENS {
            let r = Rect::new(x, 8, 76, 28);
            if self.tab(c, r, label, self.screen == s) {
                self.screen = s;
                self.scroll = 0;
            }
            x += 80;
        }
    }

    /// Whether the mouse is over `r`.
    pub fn hot(&self, r: Rect) -> bool {
        r.contains(self.mouse.0, self.mouse.1)
    }

    /// True on the frame in which the mouse was released (the click of immediate-mode widgets).
    pub fn released_now(&self) -> bool {
        self.released
    }

    /// Draws a button and returns true when it was clicked this frame.
    pub fn button(&mut self, c: &mut dyn Canvas, r: Rect, label: &str) -> bool {
        let hot = self.hot(r);
        c.fill(
            r,
            if hot && self.down {
                theme::ACCENT
            } else if hot {
                theme::BUTTON_HOT
            } else {
                theme::BUTTON
            },
        );
        c.outline(r, theme::LINE);
        let w = c.text_width(label, 14);
        c.text(
            r.x + (r.w - w) / 2,
            r.y + (r.h - 14) / 2 - 1,
            label,
            theme::TEXT,
            14,
        );
        hot && self.released
    }

    fn tab(&mut self, c: &mut dyn Canvas, r: Rect, label: &str, selected: bool) -> bool {
        let hot = self.hot(r);
        c.fill(
            r,
            if selected {
                theme::ACCENT
            } else if hot {
                theme::BUTTON_HOT
            } else {
                theme::BUTTON
            },
        );
        let w = c.text_width(label, 13);
        let col = if selected { theme::BG } else { theme::TEXT };
        c.text(r.x + (r.w - w) / 2, r.y + 7, label, col, 13);
        hot && self.released
    }
}

#[cfg(feature = "sdl")]
pub mod sdl;

#[cfg(test)]
mod tests {
    use super::*;
    use canvas::DrawList;

    fn frame(app: &mut App) -> DrawList {
        let mut dl = DrawList::default();
        app.draw(&mut dl);
        dl
    }

    #[test]
    fn every_screen_draws_the_game_state() {
        let mut app = App::new(1998);
        let dl = frame(&mut app);
        assert!(dl.texts().iter().any(|t| t.contains("Época 1998")));
        assert!(dl.texts().contains(&"Jogar jornada"));
        for (s, _) in SCREENS {
            app.screen = s;
            let dl = frame(&mut app);
            assert!(dl.cmds.len() > 20, "{s:?} draws something");
        }
    }

    #[test]
    fn clicking_a_button_plays_a_round() {
        let mut app = App::new(7);
        frame(&mut app); // lays the buttons out
        let r = screens::PLAY_ROUND;
        let (x, y) = (r.x + 5, r.y + 5);
        app.event(Event::MouseMove(x, y));
        app.event(Event::MouseDown(x, y));
        app.event(Event::MouseUp(x, y));
        frame(&mut app);
        assert_eq!(app.world.round, 1);
        assert!(app.message.contains("Jornada 1"));
    }

    #[test]
    fn keys_switch_screens_and_divisions_and_play() {
        let mut app = App::new(3);
        app.event(Event::Key(Key::Tab));
        assert_eq!(app.screen, Screen::Squad);
        app.event(Event::Key(Key::Num(2)));
        assert_eq!(app.division, 1);
        app.event(Event::Key(Key::Space));
        assert_eq!(app.world.round, 1);
        app.event(Event::Key(Key::S));
        assert_eq!(app.world.year, 1999);
    }
}
