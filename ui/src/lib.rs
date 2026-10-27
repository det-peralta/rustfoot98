//! Immediate-mode UI of the Elifoot 98 port, independent of the window system (see `docs/ui-strategy.md`).
//!
//! [`App`] owns the game ([`rustfoot_core::world::World`]), receives [`Event`]s and draws the current [`Screen`] through a
//! [`canvas::Canvas`]. Buttons are immediate: a screen calls [`App::button`] while drawing and gets `true` on the frame in which the
//! user releases the mouse over it. No retained widget tree, no callbacks: a screen is one function from state to pixels, which keeps
//! the front ends (SDL2, web canvas) trivial to maintain.
pub mod canvas;
pub mod classic;
mod dialogs;
pub mod editor;
mod screens;
pub mod win31;

use canvas::{Canvas, Rect, Rgb};
use rustfoot_core::world::{Offer, Search, World};
use std::collections::HashMap;

pub const WIDTH: i32 = 800;
pub const HEIGHT: i32 = 600;

/// The two looks. `Modern` is the dark default; `Classic` imitates the original's Windows 3.1 look (green desktop, silver
/// bevelled buttons, navy selections, yellow captions) with our own drawing: no bitmaps or fonts of the game are used.
pub mod theme {
    use super::Rgb;
    use std::cell::Cell;

    thread_local! {
        static CLASSIC: Cell<bool> = const { Cell::new(false) };
    }

    pub fn set_classic(on: bool) {
        CLASSIC.with(|c| c.set(on));
    }

    pub fn classic() -> bool {
        CLASSIC.with(Cell::get)
    }

    fn pick(modern: Rgb, classic_: Rgb) -> Rgb {
        if classic() {
            classic_
        } else {
            modern
        }
    }

    pub fn bg() -> Rgb {
        pick(Rgb(18, 22, 30), Rgb(0, 128, 0))
    }
    pub fn panel() -> Rgb {
        pick(Rgb(28, 34, 46), Rgb(0, 128, 0))
    }
    pub fn line() -> Rgb {
        pick(Rgb(52, 62, 80), Rgb(192, 192, 192))
    }
    pub fn text() -> Rgb {
        pick(Rgb(226, 232, 240), Rgb(255, 255, 255))
    }
    pub fn muted() -> Rgb {
        pick(Rgb(138, 150, 168), Rgb(200, 224, 200))
    }
    pub fn accent() -> Rgb {
        pick(Rgb(56, 189, 248), Rgb(255, 255, 0))
    }
    pub fn good() -> Rgb {
        pick(Rgb(74, 222, 128), Rgb(170, 255, 170))
    }
    pub fn bad() -> Rgb {
        pick(Rgb(248, 113, 113), Rgb(255, 150, 150))
    }
    pub fn button() -> Rgb {
        pick(Rgb(40, 52, 72), Rgb(192, 192, 192))
    }
    pub fn button_hot() -> Rgb {
        pick(Rgb(58, 76, 106), Rgb(214, 214, 214))
    }
    /// Label colour on a button face.
    pub fn button_text() -> Rgb {
        pick(Rgb(226, 232, 240), Rgb(0, 0, 0))
    }
    /// Highlighted rows (starters, the user's club).
    pub fn sel() -> Rgb {
        pick(Rgb(40, 52, 72), Rgb(0, 0, 128))
    }
    pub fn header_bg() -> Rgb {
        pick(Rgb(28, 34, 46), Rgb(192, 192, 192))
    }
    pub fn header_text() -> Rgb {
        pick(Rgb(138, 150, 168), Rgb(0, 0, 0))
    }
    pub fn title() -> Rgb {
        pick(Rgb(56, 189, 248), Rgb(0, 0, 128))
    }
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
    /// Write `App::save_text()` to slot `n`, then call `App::set_slot(n, Some(label))` and report in `App::message`.
    Save(usize),
    /// Read slot `n` and call `App::load_text`.
    Load(usize),
    /// Erase slot `n`, then `App::set_slot(n, None)`.
    Delete(usize),
    /// Ask for an original `.e98` file and call `App::import_e98`.
    Import,
}

/// Number of save slots offered by the options screen.
pub const SLOTS: usize = 8;

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
    /// What each save slot holds (a one-line label), as reported by the host.
    pub slots: [Option<String>; SLOTS],
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
            slots: Default::default(),
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
    pub fn classic(&self) -> bool {
        theme::classic()
    }

    pub fn take_request(&mut self) -> Option<Request> {
        self.request.take()
    }

    /// One line describing the game, stored next to a save: season, round and the user's club.
    pub fn slot_label(&self) -> String {
        let club = self.world.human.as_ref().map_or("-", |h| h.club.as_str());
        format!(
            "Época {} · J{}/14 · {}",
            self.world.year, self.world.round, club
        )
    }

    /// The host reports what slot `n` holds now.
    pub fn set_slot(&mut self, n: usize, label: Option<String>) {
        if let Some(s) = self.slots.get_mut(n) {
            *s = label;
        }
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
        match rustfoot_core::world::World::load(text) {
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
        match rustfoot_core::e98::parse(bytes).and_then(|s| World::from_e98(seed, &s)) {
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
        c.fill(Rect::new(0, 0, WIDTH, HEIGHT), theme::bg());
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
        c.fill(Rect::new(0, 0, WIDTH, 44), theme::header_bg());
        if theme::classic() {
            c.fill(Rect::new(0, 44, WIDTH, 2), Rgb(0, 0, 0));
        }
        c.text(
            12,
            10,
            "Elifoot 98",
            if theme::classic() {
                theme::header_text()
            } else {
                theme::title()
            },
            20,
        );
        let status = format!(
            "Época {} · J{}/14 · Eco {:.2}",
            self.world.year, self.world.round, self.world.economy
        );
        c.text(
            140,
            14,
            &status,
            if theme::classic() {
                theme::header_text()
            } else {
                theme::muted()
            },
            13,
        );
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
        let pressed = hot && self.down;
        if theme::classic() {
            bevel(
                c,
                r,
                pressed,
                if hot {
                    theme::button_hot()
                } else {
                    theme::button()
                },
            );
        } else {
            c.fill(
                r,
                if pressed {
                    theme::accent()
                } else if hot {
                    theme::button_hot()
                } else {
                    theme::button()
                },
            );
            c.outline(r, theme::line());
        }
        let w = c.text_width(label, 14);
        let nudge = (theme::classic() && pressed) as i32;
        c.text(
            r.x + (r.w - w) / 2 + nudge,
            r.y + (r.h - 14) / 2 - 1 + nudge,
            label,
            theme::button_text(),
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
        c.fill(r, theme::bg());
        c.outline(
            r,
            if focused {
                theme::accent()
            } else {
                theme::line()
            },
        );
        let text = self.fields.get(id).cloned().unwrap_or_default();
        if text.is_empty() && !focused {
            c.text(r.x + 6, r.y + (r.h - 13) / 2 - 1, hint, theme::muted(), 13);
        } else {
            let shown = if focused {
                format!("{text}|")
            } else {
                text.clone()
            };
            c.text(r.x + 6, r.y + (r.h - 13) / 2 - 1, &shown, theme::text(), 13);
        }
        text
    }

    fn tab(&mut self, c: &mut dyn Canvas, r: Rect, label: &str, selected: bool) -> bool {
        let hot = self.hot(r);
        if theme::classic() {
            if selected {
                bevel(c, r, true, theme::sel());
            } else {
                bevel(
                    c,
                    r,
                    false,
                    if hot {
                        theme::button_hot()
                    } else {
                        theme::button()
                    },
                );
            }
        } else {
            c.fill(
                r,
                if selected {
                    theme::accent()
                } else if hot {
                    theme::button_hot()
                } else {
                    theme::button()
                },
            );
        }
        let w = c.text_width(label, 12);
        let col = match (selected, theme::classic()) {
            (true, true) => Rgb(255, 255, 255),
            (true, false) => theme::bg(),
            (false, _) => theme::button_text(),
        };
        c.text(
            r.x + (r.w - w) / 2,
            r.y + (r.h - 12) / 2 - 1,
            label,
            col,
            12,
        );
        hot && self.released
    }
}

/// A Windows 3.1 style button face: black frame, white top-left edge, grey shade bottom-right (inverted when pressed).
fn bevel(c: &mut dyn Canvas, r: Rect, pressed: bool, face: Rgb) {
    c.fill(r, Rgb(0, 0, 0));
    let inner = Rect::new(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    let (light, shade) = if pressed {
        (Rgb(128, 128, 128), Rgb(255, 255, 255))
    } else {
        (Rgb(255, 255, 255), Rgb(128, 128, 128))
    };
    c.fill(inner, shade);
    c.fill(Rect::new(inner.x, inner.y, inner.w - 1, inner.h - 1), light);
    c.fill(
        Rect::new(inner.x + 2, inner.y + 2, inner.w - 4, inner.h - 4),
        face,
    );
}

/// Chooses the look; the choice is process-wide.
pub fn set_classic(on: bool) {
    theme::set_classic(on);
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
        use rustfoot_core::season::{Plan, PlannedSub};
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
    fn save_slots_round_trip_through_the_host_requests() {
        let mut app = App::new(21);
        app.screen = Screen::Options;
        app.play_round();
        // slot rows start at y 112 and are 54 apart; buttons Gravar (x 416), Carregar (x 546), Apagar (x 676)
        let row = |n: i32, b: i32| {
            Rect::new(
                [416, 546, 676][b as usize],
                112 + 54 * n,
                if b == 2 { 80 } else { 120 },
                26,
            )
        };
        click(&mut app, row(2, 0));
        assert_eq!(app.take_request(), Some(Request::Save(2)));
        let text = app.save_text();
        let label = app.slot_label();
        assert!(label.contains("J1/14"));
        app.set_slot(2, Some(label.clone()));
        let (round, cash) = (app.world.round, app.world.divisions[0][0].fin.cash);
        app.play_season();
        app.load_text(&text).unwrap();
        assert_eq!(
            (app.world.round, app.world.divisions[0][0].fin.cash),
            (round, cash)
        );
        assert!(app.load_text("not a save").is_err());
        assert!(app.message.contains("carregar"));
        click(&mut app, row(2, 1));
        assert_eq!(app.take_request(), Some(Request::Load(2)));
        click(&mut app, row(2, 2));
        assert_eq!(app.take_request(), Some(Request::Delete(2)));
        click(&mut app, Rect::new(26, 124, 230, 30));
        assert_eq!(app.take_request(), Some(Request::Import));
        assert!(app.import_e98(&[0; 10], 1).is_err());
    }

    #[test]
    fn the_balance_and_the_referee_aid_are_shown() {
        let mut app = App::new(8);
        let dl = frame(&mut app);
        assert!(dl.texts().contains(&"Ajuda do árbitro"));
        app.screen = Screen::Finance;
        let dl = frame(&mut app);
        assert!(dl.texts().contains(&"Balanço da época"));
        assert!(dl.texts().contains(&"Resultado"));
    }

    #[test]
    fn the_classic_look_is_a_green_desktop() {
        let mut app = App::new(9);
        app.screen = Screen::Options;
        let first_fill = |dl: &DrawList| match dl.cmds[0] {
            canvas::Cmd::Fill(_, c) => c,
            _ => panic!("the frame starts with the background"),
        };
        assert_ne!(first_fill(&frame(&mut app)), Rgb(0, 128, 0));
        click(&mut app, Rect::new(266, 124, 120, 30)); // Visual
        assert!(app.classic());
        assert_eq!(first_fill(&frame(&mut app)), Rgb(0, 128, 0));
        set_classic(false);
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
