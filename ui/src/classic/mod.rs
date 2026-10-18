//! The game with the original's own user interface: the Delphi forms rendered pixel for pixel by [`crate::win31`] and bound to the
//! rules of `elifoot-core`. Needs the skin pack (`tools/ui/skin.py`); see `docs/pixel-exact-ui.md`.
pub mod dialog;
pub mod dialogs;
pub mod input;
pub mod main_win;
pub mod menu;
pub mod round_view;
pub mod text;
pub mod widget_input;

use crate::win31::draw::Hit;
use crate::win31::{Fb, Forms, Skin};
use elifoot_core::world::World;
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
}

impl Data {
    pub fn from_txe(country: &[u8], referee: &[u8]) -> Data {
        let split = |s: &String| {
            s.split_once(' ')
                .map(|(a, b)| (a.to_string(), b.to_string()))
        };
        Data {
            countries: elifoot_core::txe::parse(country)
                .iter()
                .skip(1)
                .filter_map(split)
                .collect(),
            referees: elifoot_core::txe::parse(referee)
                .iter()
                .skip(1)
                .filter_map(split)
                .collect(),
            saves: Vec::new(),
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
            mouse: (0, 0),
            hits: Vec::new(),
            dirty: true,
        }
    }

    /// Draws the whole screen (800x600) into `self.fb`.
    pub fn draw(&mut self) {
        self.hits.clear();
        match &self.screen {
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
            Screen::Round(rv) => round_view::draw(&mut self.fb, &self.skin, rv),
        }
        let mut dialogs = std::mem::take(&mut self.dialogs);
        for d in dialogs.iter_mut() {
            d.draw(self);
        }
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
