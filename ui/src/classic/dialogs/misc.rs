//! Message boxes and the small dialogs without a group of their own: options, stadium, loan (to do, see `docs/classic-dialogs.md`).
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::Classic;

/// A message box with an OK button (the VCL's `ShowMessage`).
pub fn message(caption: &str, text: &str) -> Dialog {
    let mut d = Dialog::new(Kind::Message, "TCANTSAVETEAMDLG");
    d.text("@caption", caption);
    d.st.insert("text".into(), text.into());
    d
}

pub fn bind(_c: &Classic, _d: &mut Dialog) {}

pub fn click(_c: &mut Classic, _d: &mut Dialog, _name: &str) -> Outcome {
    Outcome::Close
}
