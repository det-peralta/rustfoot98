//! The game with the original's own user interface: the Delphi forms rendered pixel for pixel by [`crate::win31`] and bound to the
//! rules of `elifoot-core`. Needs the skin pack (`tools/ui/skin.py`); see `docs/pixel-exact-ui.md`.
pub mod main_win;
pub mod text;

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
        }
    }
}

/// Keys the interface understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Tab,
    Up,
    Down,
    Left,
    Right,
    Char(char),
}

/// The notebook pages of the main window, in the order of its tab set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Game,
    Player,
    Finances,
    Selection,
    Opponent,
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
        }
    }
}

pub struct Classic {
    pub world: World,
    pub skin: Skin,
    pub forms: Forms,
    pub data: Data,
    pub fb: Fb,
    pub coach: String,
    pub main: main_win::MainState,
    pub mouse: (i32, i32),
    pub hits: Vec<Hit>,
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
            mouse: (0, 0),
            hits: Vec::new(),
        }
    }

    /// Draws the whole screen (800x600) into `self.fb`.
    pub fn draw(&mut self) {
        self.hits.clear();
        main_win::draw(self);
    }
}
