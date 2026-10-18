//! Dialogs: newgame (to do, see `docs/classic-dialogs.md`).
use crate::classic::dialog::{Dialog, Outcome};
use crate::classic::{Classic, Key};

pub fn bind(_c: &Classic, _d: &mut Dialog) {}

pub fn click(_c: &mut Classic, _d: &mut Dialog, _name: &str) -> Outcome {
    Outcome::Close
}

pub fn key(_c: &mut Classic, _d: &mut Dialog, _key: Key) -> Option<Outcome> {
    None
}

pub fn tick(_c: &mut Classic, _d: &mut Dialog, _ms: u32) -> Outcome {
    Outcome::Keep
}
