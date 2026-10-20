//! The game with the original's own user interface: the Delphi forms rendered pixel for pixel by [`crate::win31`] and bound to the
//! rules of `rustfoot-core`. Needs the skin pack (`tools/ui/skin.py`); see `docs/pixel-exact-ui.md`.
pub mod dialog;
pub mod dialogs;
pub mod input;
pub mod main_win;
pub mod menu;
pub mod options;
pub mod round_view;
pub mod session;
pub mod text;
pub mod widget_input;

use crate::win31::draw::Hit;
use crate::win31::{Fb, Forms, Skin};
use rustfoot_core::world::World;
use std::collections::HashMap;

/// The files of the user's own game the interface needs besides the skin: country and referee names.
#[derive(Default)]
pub struct Data {
    /// `code -> name` from `COUNTRY.TXE`.
    pub countries: HashMap<String, String>,
    /// Referees as `(country code, name)` from `REFEREE.TXE`.
    pub referees: Vec<(String, String)>,
    /// Names of the games the player has saved (the host keeps their text).
    pub saves: Vec<String>,
    /// The description of each saved game (coach and date, lines joined by `\n`), by name; may lack entries.
    pub save_desc: HashMap<String, String>,
}

impl Data {
    pub fn from_txe(country: &[u8], referee: &[u8]) -> Data {
        let split = |s: &String| {
            s.split_once(' ')
                .map(|(a, b)| (a.to_string(), b.to_string()))
        };
        Data {
            countries: rustfoot_core::txe::parse(country)
                .iter()
                .skip(1)
                .filter_map(split)
                .collect(),
            referees: rustfoot_core::txe::parse(referee)
                .iter()
                .skip(1)
                .filter_map(split)
                .collect(),
            saves: Vec::new(),
            save_desc: HashMap::new(),
        }
    }

    /// Takes the host's list of saved games: one per line, `NAME` or `NAME<TAB>first line of the saved text` (whose header describes it).
    pub fn set_saves(&mut self, list: &str) {
        self.saves.clear();
        self.save_desc.clear();
        for l in list.lines().filter(|l| !l.is_empty()) {
            let (name, head) = l.split_once('\t').unwrap_or((l, ""));
            self.saves.push(name.to_string());
            let d = options::Header::split(head).0.description();
            if !d.is_empty() {
                self.save_desc.insert(name.to_string(), d);
            }
        }
    }
}

/// Keys the interface understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    BackTab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    /// Function key `F1..F12`.
    F(u8),
    Char(char),
}

/// Input events from the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Move(i32, i32),
    Down(i32, i32),
    Up(i32, i32),
    Key(Key),
}

/// What the host has to do for the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Leave the game.
    Quit,
    /// Store the game under this slot name (a saved game of the player).
    Save(String),
    /// Load the saved game of this name.
    Load(String),
}

/// The notebook pages of the main window, in the order of its tab set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Game,
    Player,
    Finances,
    Selection,
    Opponent,
    /// The pages below are reached from menus and buttons, not from the tab set.
    Sell,
    Contract,
    Message,
}

impl Page {
    pub const ALL: [Page; 5] = [
        Page::Game,
        Page::Player,
        Page::Finances,
        Page::Selection,
        Page::Opponent,
    ];
    /// Index of the page in the form's notebook.
    pub fn notebook_index(self) -> usize {
        match self {
            Page::Game => 0,
            Page::Player => 1,
            Page::Finances => 2,
            Page::Selection => 3,
            Page::Opponent => 4,
            Page::Sell => 5,
            Page::Contract => 6,
            Page::Message => 7,
        }
    }

    /// The tab that stays selected while this page shows.
    pub fn tab_index(self) -> usize {
        match self {
            Page::Game | Page::Message => 0,
            Page::Player | Page::Sell | Page::Contract => 1,
            Page::Finances => 2,
            Page::Selection => 3,
            Page::Opponent => 4,
        }
    }
}

/// What fills the screen below the dialogs.
pub enum Screen {
    /// The main window of the user's club.
    Main,
    /// The games of a round while the clock runs.
    Round(Box<round_view::RoundView>),
}

pub struct Classic {
    pub world: World,
    pub skin: Skin,
    pub forms: Forms,
    pub data: Data,
    pub fb: Fb,
    pub coach: String,
    pub main: main_win::MainState,
    pub screen: Screen,
    /// Modal dialogs, the last one on top.
    pub dialogs: Vec<dialog::Dialog>,
    /// Dialogs to open one after the other when the current ones are closed (end of round messages, auctions...).
    pub queue: std::collections::VecDeque<dialog::Dialog>,
    pub requests: Vec<Request>,
    /// The match day: options that act on it, the round being shown with what is needed to replay it, the dialogs still to come.
    pub md: session::State,
    /// The player's options (`TOptionsDlg`).
    pub opts: options::Options,
    /// False until a game was started or loaded: only the start dialogs show over the empty desktop.
    pub started: bool,
    /// The teams a new game is made of (empty: invented clubs) and the seed for it.
    pub teams: Vec<rustfoot_core::eft::EftTeam>,
    pub seed: u32,
    /// Name the current game is saved under (empty before the first game starts).
    pub game_name: String,
    /// The name of the game being loaded (waiting for the host's text).
    pub loading: Option<String>,
    /// The wall clock as `dd/mm/yy hh:mm`, set by the host (it goes into the description of saved games).
    pub clock: String,
    pub mouse: (i32, i32),
    pub hits: Vec<Hit>,
    /// Set whenever something changed that needs a new frame.
    pub dirty: bool,
}

impl Classic {
    pub fn new(world: World, skin: Skin, forms: Forms, data: Data) -> Classic {
        Classic {
            world,
            skin,
            forms,
            data,
            fb: Fb::new(),
            coach: "Treinador".into(),
            main: main_win::MainState::default(),
            screen: Screen::Main,
            dialogs: Vec::new(),
            queue: Default::default(),
            requests: Vec::new(),
            md: Default::default(),
            opts: options::Options::default(),
            started: true,
            teams: Vec::new(),
            seed: 1,
            game_name: String::new(),
            loading: None,
            clock: String::new(),
            mouse: (0, 0),
            hits: Vec::new(),
            dirty: true,
        }
    }

    /// The interface before any game exists: the About box opens first, then the choice of game (see `dialogs/newgame.rs`).
    /// `teams` are the `.EFT` files of the pack (none: invented clubs) and `seed` seeds the new game's world.
    pub fn start(
        skin: Skin,
        forms: Forms,
        data: Data,
        teams: Vec<rustfoot_core::eft::EftTeam>,
        seed: u32,
    ) -> Classic {
        let blank = World::empty(1, Default::default(), Vec::new(), 1998, 0);
        let mut c = Classic::new(blank, skin, forms, data);
        c.started = false;
        c.teams = teams;
        c.seed = seed.max(1);
        c.dialogs.push(dialogs::about::open_startup());
        c
    }

    /// The text of the game for the host to keep: a header line (coach, date, options) and the world.
    pub fn save_text(&self) -> String {
        let h = options::Header {
            coach: self.coach.clone(),
            stamp: self.clock.clone(),
            opts: None,
        };
        format!("{}\n{}", h.line(&self.opts), self.world.save())
    }

    /// Takes the text of the game the host was asked for with `Request::Load` and starts playing it. On a bad text a message box
    /// tells so and the choice of game opens again.
    pub fn load_text(&mut self, text: &str) -> Result<(), String> {
        let (h, body) = options::Header::split(text);
        match World::load(body) {
            Ok(w) => {
                self.world = w;
                if !h.coach.is_empty() {
                    self.coach = h.coach;
                }
                if let Some(o) = h.opts {
                    self.opts = o;
                }
                if let Some(n) = self.loading.take() {
                    self.game_name = n;
                }
                self.enter_game();
                Ok(())
            }
            Err(e) => {
                self.loading = None;
                self.dialogs.clear();
                self.dialogs.push(dialogs::newgame::open_select());
                self.message("Erro", "Não foi possível carregar o jogo.");
                self.dirty = true;
                Err(e)
            }
        }
    }

    /// Shows the main window of the game just created or loaded.
    pub fn enter_game(&mut self) {
        self.started = true;
        self.dialogs.clear();
        self.queue.clear();
        self.screen = Screen::Main;
        self.main = main_win::MainState::default();
        self.dirty = true;
    }

    /// Asks the host to store the game under its name (after a round, on exit).
    pub fn save_now(&mut self) {
        if self.started && !self.game_name.is_empty() {
            self.requests.push(Request::Save(self.game_name.clone()));
        }
    }

    /// Draws the whole screen (800x600) into `self.fb`.
    pub fn draw(&mut self) {
        self.hits.clear();
        if !self.started {
            main_win::desktop(&mut self.fb, &self.skin);
        }
        // the windows below the topmost dialog have the inactive caption
        let inactive = |on: bool| crate::win31::draw::INACTIVE.with(|i| i.set(on));
        inactive(!self.dialogs.is_empty());
        match &self.screen {
            Screen::Main if !self.started => {}
            Screen::Main => {
                main_win::draw(self);
                if let (Some((open, hl)), Some((_, club))) =
                    (self.main.menu, main_win::human_club(self))
                {
                    if self.dialogs.is_empty() {
                        let tops = menu::model(self, &club);
                        if open < tops.len() {
                            menu::draw_open(self, &tops, open, hl);
                        }
                    }
                }
            }
            Screen::Round(rv) => {
                round_view::draw(&mut self.fb, &self.skin, rv, self.dialogs.is_empty())
            }
        }
        let mut dialogs = std::mem::take(&mut self.dialogs);
        let n = dialogs.len();
        for (i, d) in dialogs.iter_mut().enumerate() {
            inactive(i + 1 < n);
            d.draw(self);
        }
        inactive(false);
        self.dialogs = dialogs;
        self.dirty = false;
    }

    /// Draws a frame when something changed; true when `fb` holds a new picture.
    pub fn frame(&mut self) -> bool {
        if !self.dirty {
            return false;
        }
        self.draw();
        true
    }

    /// Opens a dialog on top of the others.
    pub fn open(&mut self, d: dialog::Dialog) {
        self.dialogs.push(d);
        self.dirty = true;
    }

    /// Shows a message box with an OK button.
    pub fn message(&mut self, caption: &str, text: &str) {
        self.open(dialogs::message(caption, text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare() -> Classic {
        let mut sk = b"EFSK".to_vec();
        sk.extend_from_slice(&0u16.to_le_bytes());
        let mut gl = b"EFGL".to_vec();
        gl.extend_from_slice(&0u16.to_le_bytes());
        let skin = Skin::from_bytes(&sk, &gl).unwrap();
        let forms = Forms::from_json("{}").unwrap();
        Classic::start(skin, forms, Data::default(), Vec::new(), 7)
    }

    #[test]
    fn the_game_starts_with_the_about_box_and_no_game() {
        let c = bare();
        assert!(!c.started);
        assert_eq!(c.dialogs.len(), 1);
        assert_eq!(c.dialogs[0].kind, dialog::Kind::About);
    }

    #[test]
    fn a_saved_game_keeps_the_coach_and_options() {
        let mut c = bare();
        c.world = World::synthetic(3);
        c.coach = "Zeca".into();
        c.clock = "01/02/98 10:30".into();
        c.opts.profile = 2;
        c.opts.half_time_subs = false;
        let text = c.save_text();
        let mut d = Data::default();
        d.set_saves(&format!("JOGO\t{}", text.lines().next().unwrap()));
        assert_eq!(d.save_desc["JOGO"], "Zeca\n01/02/98 10:30");
        let mut c2 = bare();
        c2.loading = Some("JOGO".into());
        c2.load_text(&text).unwrap();
        assert!(c2.started && c2.dialogs.is_empty());
        assert_eq!(c2.coach, "Zeca");
        assert_eq!(c2.game_name, "JOGO");
        assert_eq!(c2.opts, c.opts);
        // an old save without the header still loads
        let old = c.world.save();
        let mut c3 = bare();
        assert!(c3.load_text(&old).is_ok());
        assert_eq!(c3.coach, "Treinador");
    }

    #[test]
    fn a_bad_saved_game_returns_to_the_choice_with_a_message() {
        let mut c = bare();
        assert!(c.load_text("garbage").is_err());
        assert!(!c.started);
        assert_eq!(c.dialogs.len(), 2);
        assert_eq!(c.dialogs[1].kind, dialog::Kind::Message);
    }

    #[test]
    fn leaving_saves_only_with_a_game_name() {
        let mut c = bare();
        c.save_now();
        assert!(c.requests.is_empty());
        c.world = World::synthetic(3);
        c.enter_game();
        c.game_name = "X".into();
        c.command("jogoSair");
        assert_eq!(c.requests, vec![Request::Save("X".into()), Request::Quit]);
        c.requests.clear();
        c.command("Sairsemgravar1");
        assert_eq!(c.requests, vec![Request::Quit]);
    }
}
