//! Immediate-mode UI of the Elifoot 98 port, independent of the window system (see `docs/ui-strategy.md`).
//!
//! [`App`] owns the game ([`elifoot_core::world::World`]), receives [`Event`]s and draws the current [`Screen`] through a
//! [`canvas::Canvas`]. Buttons are immediate: a screen calls [`App::button`] while drawing and gets `true` on the frame in which the
//! user releases the mouse over it. No retained widget tree, no callbacks: a screen is one function from state to pixels, which keeps
//! the front ends (SDL2, web canvas) trivial to maintain.
pub mod canvas;
mod dialogs;
mod screens;

use canvas::{Canvas, Rect, Rgb};
use elifoot_core::world::{Offer, Search, World};
use std::collections::HashMap;

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
    Backspace,
    Enter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    MouseMove(i32, i32),
    MouseDown(i32, i32),
    MouseUp(i32, i32),
    Wheel(i32),
    Key(Key),
    /// A typed character (text input).
    Text(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    League,
    Squad,
    Tactics,
    Market,
    Finance,
    Cup,
    Options,
}

pub const SCREENS: [(Screen, &str); 7] = [
    (Screen::League, "Liga"),
    (Screen::Squad, "Plantel"),
    (Screen::Tactics, "Táctica"),
    (Screen::Market, "Mercado"),
    (Screen::Finance, "Finanças"),
    (Screen::Cup, "Taça"),
    (Screen::Options, "Opções"),
];

/// What the screens ask the host (the window, the page) to do: only it can touch files or browser storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Write `App::save_text()` to the save slot, then report in `App::message`.
    Save,
    /// Read the save slot and call `App::load_text`.
    Load,
    /// Ask for an original `.e98` file and call `App::import_e98`.
    Import,
}

pub struct App {
    pub world: World,
    pub screen: Screen,
    pub division: usize,
    pub scroll: i32,
    /// Scroll of a second list on the same screen (the right-hand column of the market).
    pub scroll2: i32,
    pub message: String,
    /// Text fields by id; `focus` is the one receiving typed characters.
    pub fields: HashMap<&'static str, String>,
    pub focus: Option<&'static str>,
    /// A player picked in the line-up editor (index into the squad), waiting for the player to swap with.
    pub selected: Option<usize>,
    /// The market search form and its last result.
    pub query: Search,
    pub offers: Vec<Offer>,
    /// A pending request for the host; see [`App::take_request`].
    pub request: Option<Request>,
    field_hit: bool,
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
            scroll2: 0,
            message: String::new(),
            fields: HashMap::new(),
            focus: None,
            selected: None,
            query: Search::default(),
            offers: Vec::new(),
            request: None,
            field_hit: false,
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
            Event::Wheel(d) => {
                if self.screen == Screen::Market && self.mouse.0 < 398 {
                    self.scroll2 = (self.scroll2 - d * 28).max(0);
                } else {
                    self.scroll = (self.scroll - d * 24).max(0);
                }
            }
            Event::Text(ch) => {
                if let Some(id) = self.focus {
                    if !ch.is_control() {
                        self.fields.entry(id).or_default().push(ch);
                    }
                }
            }
            Event::Key(k) => {
                if let Some(id) = self.focus {
                    match k {
                        Key::Backspace => {
                            self.fields.entry(id).or_default().pop();
                        }
                        Key::Enter | Key::Escape | Key::Tab => self.focus = None,
                        _ => {}
                    }
                    return;
                }
                match k {
                    Key::Space => self.play_round(),
                    Key::S => self.play_season(),
                    Key::Tab => self.next_screen(),
                    Key::Num(n) if (1..=4).contains(&n) => self.division = n as usize - 1,
                    Key::Up => self.scroll = (self.scroll - 24).max(0),
                    Key::Down => self.scroll += 24,
                    _ => {}
                }
            }
        }
    }

    /// The pending host request, if any (the host calls this after every batch of events).
    pub fn take_request(&mut self) -> Option<Request> {
        self.request.take()
    }

    /// The game as text, for the host to store.
    pub fn save_text(&self) -> String {
        self.world.save()
    }

    fn after_load(&mut self, what: &str) {
        self.division = self.world.human_location().map_or(0, |l| l.0);
        self.scroll = 0;
        self.scroll2 = 0;
        self.selected = None;
        self.focus = None;
        self.offers.clear();
        self.message = what.to_string();
    }

    /// Replaces the game with one saved by [`App::save_text`].
    pub fn load_text(&mut self, text: &str) -> Result<(), String> {
        match elifoot_core::world::World::load(text) {
            Ok(w) => {
                self.world = w;
                self.after_load("Jogo carregado.");
                Ok(())
            }
            Err(e) => {
                self.message = format!("Não foi possível carregar: {e}");
                Err(e)
            }
        }
    }

    /// Starts from an original `.e98` save (`seed` drives the random numbers from here on).
    pub fn import_e98(&mut self, bytes: &[u8], seed: u32) -> Result<(), String> {
        match elifoot_core::e98::parse(bytes).and_then(|s| World::from_e98(seed, &s)) {
            Ok(w) => {
                self.world = w;
                self.after_load("Jogo original importado.");
                Ok(())
            }
            Err(e) => {
                self.message = format!("Não foi possível importar: {e}");
                Err(e)
            }
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
            Screen::Tactics => dialogs::tactics(self, c),
            Screen::Market => dialogs::market(self, c),
            Screen::Finance => screens::finance(self, c),
            Screen::Cup => screens::cup(self, c),
            Screen::Options => dialogs::options(self, c),
        }
        if self.released && !self.field_hit {
            self.focus = None;
        }
        self.field_hit = false;
        self.released = false;
    }

    fn header(&mut self, c: &mut dyn Canvas) {
        c.fill(Rect::new(0, 0, WIDTH, 44), theme::PANEL);
        c.text(12, 10, "Elifoot 98", theme::ACCENT, 20);
        let status = format!(
            "Época {} · J{}/14 · Eco {:.2}",
            self.world.year, self.world.round, self.world.economy
        );
        c.text(140, 14, &status, theme::MUTED, 13);
        let mut x = 338;
        for (s, label) in SCREENS {
            let r = Rect::new(x, 8, 64, 28);
            if self.tab(c, r, label, self.screen == s) {
                self.screen = s;
                self.scroll = 0;
                self.selected = None;
            }
            x += 66;
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

    /// A single-line text field: click to focus, type, Backspace/Enter/Esc. Returns the current text.
    pub fn text_field(
        &mut self,
        c: &mut dyn Canvas,
        id: &'static str,
        r: Rect,
        hint: &str,
    ) -> String {
        let focused = self.focus == Some(id);
        if self.hot(r) && self.released {
            self.focus = Some(id);
            self.field_hit = true;
        }
        c.fill(r, theme::BG);
        c.outline(r, if focused { theme::ACCENT } else { theme::LINE });
        let text = self.fields.get(id).cloned().unwrap_or_default();
        if text.is_empty() && !focused {
            c.text(r.x + 6, r.y + (r.h - 13) / 2 - 1, hint, theme::MUTED, 13);
        } else {
            let shown = if focused {
                format!("{text}|")
            } else {
                text.clone()
            };
            c.text(r.x + 6, r.y + (r.h - 13) / 2 - 1, &shown, theme::TEXT, 13);
        }
        text
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
        let w = c.text_width(label, 12);
        let col = if selected { theme::BG } else { theme::TEXT };
        c.text(r.x + (r.w - w) / 2, r.y + 7, label, col, 12);
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

    fn click(app: &mut App, r: Rect) {
        let (x, y) = (r.x + r.w / 2, r.y + r.h / 2);
        frame(app); // lays the widgets out
        app.event(Event::MouseMove(x, y));
        app.event(Event::MouseDown(x, y));
        app.event(Event::MouseUp(x, y));
        frame(app);
    }

    fn type_text(app: &mut App, s: &str) {
        for ch in s.chars() {
            app.event(Event::Text(ch));
        }
    }

    #[test]
    fn typing_goes_to_the_focused_field_and_not_to_the_shortcuts() {
        let mut app = App::new(5);
        app.screen = Screen::Market;
        click(&mut app, Rect::new(414, 86, 200, 26)); // the name field
        assert_eq!(app.focus, Some("name"));
        app.event(Event::Key(Key::Space)); // would play a round
        type_text(&mut app, "ab ");
        app.event(Event::Key(Key::Backspace));
        assert_eq!(app.fields["name"], "ab");
        assert_eq!(app.world.round, 0);
        app.event(Event::Key(Key::Enter));
        assert_eq!(app.focus, None);
    }

    #[test]
    fn the_market_searches_and_buys() {
        let mut app = App::new(11);
        app.screen = Screen::Market;
        click(&mut app, Rect::new(414, 152, 120, 28)); // Procurar
        assert!(!app.offers.is_empty(), "an empty search lists players");
        let before = app.world.pending.len();
        let cash = app.world.divisions[3]
            .iter()
            .map(|c| c.fin.cash)
            .max()
            .unwrap_or(0);
        assert!(cash > 0);
        // the first result's price button
        let price_btn = Rect::new(404 + 384 - 130, 54 + 140, 120, 28);
        click(&mut app, price_btn);
        assert!(
            app.world.pending.len() > before || !app.message.is_empty(),
            "the click either bought or explained"
        );
    }

    #[test]
    fn the_lineup_editor_swaps_a_starter_with_a_substitute() {
        let mut app = App::new(2);
        app.screen = Screen::Tactics;
        let before = app.world.human.as_ref().unwrap().plan.clone();
        let (d, i) = app.world.human_location().unwrap();
        let club = &app.world.divisions[d][i];
        let mut idx: Vec<usize> = (0..club.slots.len()).collect();
        idx.sort_by_key(|&k| {
            (
                club.slots[k].position,
                std::cmp::Reverse(club.slots[k].strength),
            )
        });
        let row_of = |k: usize| idx.iter().position(|&x| x == k).unwrap();
        let (a, b) = (before.starters[0], before.bench[0]);
        if row_of(a) < 17 && row_of(b) < 17 {
            let y = |k: usize| 54 + 34 + row_of(k) as i32 * 26;
            click(&mut app, Rect::new(30, y(a), 300, 20));
            assert_eq!(app.selected, Some(a));
            click(&mut app, Rect::new(30, y(b), 300, 20));
            let after = &app.world.human.as_ref().unwrap().plan;
            assert!(after.starters.contains(&b) && after.bench.contains(&a));
        }
    }

    #[test]
    fn swapping_roles_keeps_the_plan_consistent() {
        use elifoot_core::season::{Plan, PlannedSub};
        let mut p = Plan {
            starters: vec![0, 1, 2],
            bench: vec![3, 4],
            subs: vec![PlannedSub {
                minute: 60,
                out: 1,
                inn: 3,
            }],
        };
        dialogs::swap_roles(&mut p, 1, 3);
        assert_eq!(
            (p.starters.clone(), p.bench.clone()),
            (vec![0, 3, 2], vec![1, 4])
        );
        assert!(p.subs.is_empty(), "the planned sub no longer fits");
        dialogs::swap_roles(&mut p, 0, 9);
        assert_eq!(p.starters, vec![9, 3, 2]);
    }

    #[test]
    fn options_toggle_invitations() {
        let mut app = App::new(4);
        app.screen = Screen::Options;
        assert!(!app.world.accept_invitations);
        click(&mut app, Rect::new(26, 88, 330, 30));
        assert!(app.world.accept_invitations);
    }

    #[test]
    fn save_and_load_round_trip_through_the_host_requests() {
        let mut app = App::new(21);
        app.screen = Screen::Options;
        app.play_round();
        click(&mut app, Rect::new(26, 124, 150, 30)); // Gravar
        assert_eq!(app.take_request(), Some(Request::Save));
        let text = app.save_text();
        let (round, cash) = (app.world.round, app.world.divisions[0][0].fin.cash);
        app.play_season();
        assert_ne!(app.world.round, round);
        app.load_text(&text).unwrap();
        assert_eq!(
            (app.world.round, app.world.divisions[0][0].fin.cash),
            (round, cash)
        );
        assert!(app.load_text("not a save").is_err());
        assert!(app.message.contains("carregar"));
        click(&mut app, Rect::new(186, 124, 150, 30)); // Carregar
        assert_eq!(app.take_request(), Some(Request::Load));
        click(&mut app, Rect::new(346, 124, 190, 30)); // Importar
        assert_eq!(app.take_request(), Some(Request::Import));
        assert!(app.import_e98(&[0; 10], 1).is_err());
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
