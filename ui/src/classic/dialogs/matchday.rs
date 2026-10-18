//! Dialogs: matchday (to do, see `docs/classic-dialogs.md`).
use crate::classic::dialog::{Dialog, Kind, Outcome};
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

/// The break of the user's game: the dialog with the events so far, the squad and the substitutions (`TSUBSTITUTIONDLG`).
pub fn halftime(_c: &Classic) -> Dialog {
    Dialog::new(Kind::Halftime, "TSUBSTITUTIONDLG")
}

/// The round view is over: queue what the original shows next (coach changes, invitations, auctions, cup draw, end of season...).
pub fn end_of_round(_c: &mut Classic) {}
