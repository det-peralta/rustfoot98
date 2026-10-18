//! The About box (`TABOUTDLG`): game name, authors, scrolling banner; OK closes it.
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::Classic;

pub fn open() -> Dialog {
    Dialog::new(Kind::About, "TABOUTDLG")
}

pub fn bind(_c: &Classic, _d: &mut Dialog) {}

pub fn click(_c: &mut Classic, _d: &mut Dialog, name: &str) -> Outcome {
    if name == "OKBtn" {
        Outcome::Close
    } else {
        Outcome::Keep
    }
}

pub fn tick(_c: &mut Classic, _d: &mut Dialog, _ms: u32) -> Outcome {
    Outcome::Keep
}
