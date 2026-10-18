//! Dialogs: market (to do, see `docs/classic-dialogs.md`).
use crate::classic::dialog::{Dialog, Outcome};
use crate::classic::{Classic, Key};

/// One auction dialog for every open auction of the board the user can still take part in (to do: the market agent's).
pub fn queue_auctions(_c: &mut Classic) {}

/// The squad viewer on `club` (to do: the market agent's).
pub fn open_plantel(_c: &Classic, club: &str, _player: Option<&str>) -> Dialog {
    let mut d = Dialog::new(crate::classic::dialog::Kind::Plantel, "TPLANTELDLG");
    d.st.insert("club".into(), club.into());
    d
}

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
