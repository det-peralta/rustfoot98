//! Modal dialogs: one of the original's forms drawn over the screen with texts, lists and states supplied by the program, and a
//! behaviour (`dialogs/`) that answers clicks and keys. See `docs/classic-dialogs.md` for the conventions.
use super::{dialogs, Classic, Key};
use crate::win31::draw::{
    controls, dialog_frame_thick, form_client, form_frame, Ctx, Hit, FRAME_ORIGIN,
};
use crate::win31::fb::{Rgb, SILVER};
use crate::win31::Node;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// Which behaviour a dialog has (one variant per dialog, in `dialogs/`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Message,
    About,
    SelectGame,
    /// *Selecção de Países* (`TSELECTCOUNTRIESDLG`) and *Dinheiro* (`TCURRENCYDLG`) of a new game.
    SelectCountries,
    Currency,
    CoachName,
    Progress,
    Options,
    Stadium,
    Loan,
    Standings,
    Scorers,
    Calendar,
    Winners,
    AllTimeScorers,
    Transactions,
    Acquisitions,
    Observed,
    Search,
    Sell,
    Contract,
    CoachHistory,
    CoachManager,
    CoachRanking,
    CoachProfile,
    TeamHistory,
    Halftime,
    ForcedSubstitution,
    Penalty,
    Shootout,
    CupDraw,
    EndOfSeason,
    Auction,
    Invitation,
    RefereeAid,
    /// "Chicotadas Psicológicas": the list of sacked coaches after a round.
    Lashes,
    Plantel,
    Other(String),
}

/// What the dialog on top of the stack does after handling an event.
pub enum Outcome {
    /// Stay open.
    Keep,
    /// Close this dialog.
    Close,
    /// Open another one on top of this.
    Open(Box<Dialog>),
    /// Close this dialog and open another in its place.
    Replace(Box<Dialog>),
}

pub struct Dialog {
    pub kind: Kind,
    /// Key in `Forms` (upper case form class name, e.g. `TABOUTDLG`).
    pub form: String,
    pub texts: BTreeMap<String, String>,
    pub lists: BTreeMap<String, Vec<String>>,
    /// Name of the control with the keyboard focus.
    pub focus: String,
    /// Colour of the form when the program changes it.
    pub bg: Option<Rgb>,
    /// Controls of the last frame, in drawing order.
    pub hits: Vec<Hit>,
    /// The behaviour's own state (numbers and strings by name).
    pub st: BTreeMap<String, String>,
}

impl Dialog {
    pub fn new(kind: Kind, form: &str) -> Dialog {
        Dialog {
            kind,
            form: form.to_uppercase(),
            texts: BTreeMap::new(),
            lists: BTreeMap::new(),
            focus: String::new(),
            bg: None,
            hits: Vec::new(),
            st: BTreeMap::new(),
        }
    }

    pub fn text(&mut self, k: impl AsRef<str>, v: impl Into<String>) {
        self.texts.insert(k.as_ref().to_string(), v.into());
    }

    pub fn num(&self, k: &str) -> i64 {
        self.st.get(k).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    pub fn set_num(&mut self, k: &str, v: i64) {
        self.st.insert(k.to_string(), v.to_string());
    }

    /// Draws the dialog centred on the screen over what is already in `c.fb`.
    pub fn draw(&mut self, c: &mut Classic) {
        dialogs::bind(c, self);
        let Classic {
            skin, forms, fb, ..
        } = c;
        self.render(fb, skin, forms);
    }

    /// Draws the dialog as its texts and lists stand (bound by its owner: the game or the team editor).
    pub fn render(
        &mut self,
        fb: &mut crate::win31::fb::Fb,
        skin: &crate::win31::Skin,
        forms: &crate::win31::Forms,
    ) {
        let Some(form) = forms.get(&self.form) else {
            return;
        };
        let (mut cw, mut ch) = form_client(form);
        if let Some((a, b)) = self.texts.get("@client").and_then(|v| v.split_once('x')) {
            cw = a.parse().unwrap_or(cw);
            ch = b.parse().unwrap_or(ch);
        }
        let bg = self.bg.or_else(|| form.color("Color")).unwrap_or(SILVER);
        let caption = self
            .texts
            .get("@caption")
            .map(String::as_str)
            .or(form.str("Caption"))
            .unwrap_or("");
        // `@frame=thick` draws the sizeable form's thick frame; `@origin=x,y` puts the frame's corner there (a form that is not centred)
        let thick = form.ident("BorderStyle") == Some("bsDialog")
            || self.texts.get("@frame").is_some_and(|f| f == "thick");
        let origin = self.texts.get("@origin").and_then(|v| {
            let (a, b) = v.split_once(',')?;
            Some((a.parse().ok()?, b.parse().ok()?))
        });
        FRAME_ORIGIN.with(|o| o.set(origin));
        let (ox, oy) = if thick && form.ident("BorderStyle") != Some("bsDialog") {
            dialog_frame_thick(fb, skin, caption, cw, ch, bg)
        } else {
            form_frame(fb, skin, form, caption, cw, ch, bg)
        };
        FRAME_ORIGIN.with(|o| o.set(None));
        let (fx, fy) = (ox, oy);
        let (ox, oy) = (ox + self.num_text("@dx"), oy + self.num_text("@dy"));
        let hits = RefCell::new(Vec::new());
        let cx = Ctx {
            skin,
            form: &self.form,
            texts: &self.texts,
            ox,
            oy,
            bg,
            focus: if self.focus.is_empty() {
                form.ident("ActiveControl").unwrap_or("")
            } else {
                &self.focus
            },
            lists: &self.lists,
            hits: &hits,
        };
        // the controls are clipped to the window's client area (the thick frame draws 4 pixels less than `cw` x `ch`)
        let old_clip = fb.clip;
        let (iw, ih) = if thick { (cw - 4, ch - 4) } else { (cw, ch) };
        fb.clip = (fx, fy, fx + iw, fy + ih);
        controls(fb, &cx, form);
        fb.clip = old_clip;
        self.hits = hits.into_inner();
        // a form without `ActiveControl` gives the focus to its first tab stop when it shows (`TCustomForm.SetActive`): drawn again
        // with it
        if self.focus.is_empty() && form.ident("ActiveControl").is_none() {
            let first = self
                .hits
                .iter()
                .filter(|h| tab_stop(h, &self.texts))
                .min_by_key(|h| tab_path(form, &h.name))
                .map(|h| h.name.clone());
            if let Some(name) = first {
                self.focus = name;
                self.render(fb, skin, forms);
            }
        }
    }

    fn num_text(&self, k: &str) -> i32 {
        self.texts.get(k).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    /// The control under a screen position (topmost first) that reacts to clicks.
    pub fn hit_at(&self, x: i32, y: i32) -> Option<&Hit> {
        self.hits.iter().rev().find(|h| {
            h.contains(x, y)
                && matches!(
                    h.class.as_str(),
                    "TBitBtn"
                        | "TButton"
                        | "TEdit"
                        | "TRadioButton"
                        | "TCheckBox"
                        | "TListBox"
                        | "ListRow"
                        | "TComboBox"
                        | "TSpinEdit"
                        | "TSpeedButton"
                        | "TImage"
                        | "TPanel"
                        | "TGauge"
                        | "TabSetTab"
                        | "TColorGrid"
                        | "TPaintBox"
                        | "ComboItem"
                        | "ScrollUp"
                        | "ScrollDown"
                        | "ScrollThumb"
                        | "ScrollPage"
                        | "NotebookTab"
                        | "RadioItem"
                        | "SpinUp"
                        | "SpinDown"
                        | "ColorCell"
                )
        })
    }
}

fn find_button(n: &Node, want_default: bool) -> Option<String> {
    for c in &n.children {
        let hit = if want_default {
            c.flag("Default", false) || c.ident("Kind") == Some("bkOK")
        } else {
            c.flag("Cancel", false) || c.ident("Kind") == Some("bkCancel")
        };
        if hit && matches!(c.class.as_str(), "TBitBtn" | "TButton") {
            return Some(c.name.clone());
        }
        if let Some(r) = find_button(c, want_default) {
            return Some(r);
        }
    }
    None
}

/// The default button of a form (answers Enter) and its cancel button (answers Escape).
pub fn default_button(c: &Classic, d: &Dialog) -> Option<String> {
    default_button_in(&c.forms, d)
}

/// Whether the control with the focus in `d` takes typed text (an edit or a spin edit).
pub fn focus_takes_text(forms: &crate::win31::Forms, d: &Dialog) -> bool {
    forms
        .get(&d.form)
        .and_then(|f| f.find(&d.focus))
        .is_some_and(|n| matches!(n.class.as_str(), "TEdit" | "TSpinEdit"))
}

pub fn default_button_in(forms: &crate::win31::Forms, d: &Dialog) -> Option<String> {
    forms.get(&d.form).and_then(|f| find_button(f, true))
}

pub fn cancel_button(c: &Classic, d: &Dialog) -> Option<String> {
    cancel_button_in(&c.forms, d)
}

pub fn cancel_button_in(forms: &crate::win31::Forms, d: &Dialog) -> Option<String> {
    forms.get(&d.form).and_then(|f| find_button(f, false))
}

/// Whether the control can take the focus: an enabled button, edit, list... (a radio button only while it is the chosen one: Delphi
/// sets `TabStop` with `Checked`).
fn tab_stop(h: &Hit, texts: &BTreeMap<String, String>) -> bool {
    h.enabled
        && (h.class != "TRadioButton"
            || texts
                .get(&format!("{}.checked", h.name))
                .is_some_and(|v| v == "1"))
        && matches!(
            h.class.as_str(),
            "TBitBtn"
                | "TButton"
                | "TEdit"
                | "TRadioButton"
                | "TCheckBox"
                | "TListBox"
                | "TComboBox"
                | "TSpinEdit"
                | "TTabbedNotebook"
                | "TRadioGroup"
        )
}

/// The `TabOrder` of a control and of its containers, from the form down (controls the form does not know sort last).
fn tab_path(form: &Node, name: &str) -> Vec<i32> {
    fn walk(n: &Node, name: &str, path: &mut Vec<i32>) -> bool {
        for (i, c) in n.children.iter().enumerate() {
            path.push(c.int("TabOrder", 1000 + i as i32));
            if c.name == name || walk(c, name, path) {
                return true;
            }
            path.pop();
        }
        false
    }
    let mut path = Vec::new();
    if walk(form, name, &mut path) {
        path
    } else {
        vec![i32::MAX]
    }
}

/// Deletes the selected characters of the focused edit (`<name>.selstart`, `.sellen`): what typing or Backspace does first; true when
/// there was a selection.
fn take_selection(d: &mut Dialog) -> bool {
    let f = d.focus.clone();
    let num = |d: &Dialog, k: &str| {
        d.texts
            .get(&format!("{f}.{k}"))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0)
    };
    let (a, len) = (num(d, "selstart"), num(d, "sellen"));
    d.texts.remove(&format!("{f}.sellen"));
    d.texts.remove(&format!("{f}.selstart"));
    if len == 0 {
        return false;
    }
    let t = d.texts.entry(f).or_default();
    let chars: Vec<char> = t.chars().collect();
    let a = a.min(chars.len());
    let b = (a + len).min(chars.len());
    *t = chars[..a].iter().chain(&chars[b..]).collect();
    true
}

/// What the generic key handling of a dialog turned a key into.
pub enum KeyAct {
    /// Press the named control.
    Press(String),
    /// Nothing more to do (focus moved, text edited).
    Done,
    /// Not handled.
    None,
}

/// Enter presses the default button, Escape the cancel button, Tab moves the focus and typing edits the focused `TEdit` (its text is
/// `texts[name]`, at most `MaxLength` characters).
pub fn generic_key(c: &Classic, d: &mut Dialog, key: Key) -> KeyAct {
    generic_key_in(&c.forms, d, key)
}

/// `generic_key` for a dialog of any program (the game or the team editor), with its forms.
pub fn generic_key_in(forms: &crate::win31::Forms, d: &mut Dialog, key: Key) -> KeyAct {
    let focusable = tab_stop;
    let is_edit = d
        .hits
        .iter()
        .any(|h| h.name == d.focus && h.class == "TEdit");
    if let Some(act) = super::widget_input::key(forms, d, key) {
        return act;
    }
    match key {
        Key::Enter => {
            // a button that has the focus takes the key (Windows: the default button only when the focus is elsewhere)
            let focused = d
                .hits
                .iter()
                .find(|h| h.name == d.focus && matches!(h.class.as_str(), "TBitBtn" | "TButton"));
            match focused {
                Some(h) if h.enabled => KeyAct::Press(h.name.clone()),
                Some(_) => KeyAct::Done,
                None => match default_button_in(forms, d) {
                    Some(b) => KeyAct::Press(b),
                    None => KeyAct::None,
                },
            }
        }
        Key::Escape => match cancel_button_in(forms, d) {
            Some(b) => KeyAct::Press(b),
            None => KeyAct::None,
        },
        Key::Tab | Key::BackTab => {
            let mut names: Vec<&Hit> = d.hits.iter().filter(|h| focusable(h, &d.texts)).collect();
            // Tab follows the `TabOrder` of the form (among the children of each container), then the drawing order
            if let Some(form) = forms.get(&d.form) {
                names.sort_by_cached_key(|h| tab_path(form, &h.name));
            }
            names.dedup_by(|a, b| a.name == b.name);
            if names.is_empty() {
                return KeyAct::Done;
            }
            let cur = names.iter().position(|h| h.name == d.focus);
            let n = names.len();
            let next = match (cur, key) {
                (Some(i), Key::Tab) => (i + 1) % n,
                (Some(i), _) => (i + n - 1) % n,
                (None, _) => 0,
            };
            d.focus = names[next].name.clone();
            KeyAct::Done
        }
        Key::Char(ch) if is_edit && !ch.is_control() => {
            let node = forms.get(&d.form).and_then(|f| f.find(&d.focus));
            let max = node.map_or(255, |n| n.int("MaxLength", 0));
            // (`CharCase`: the edit changes the letters as they are typed)
            let ch = match node.and_then(|n| n.ident("CharCase")) {
                Some("ecUpperCase") => ch.to_uppercase().next().unwrap_or(ch),
                Some("ecLowerCase") => ch.to_lowercase().next().unwrap_or(ch),
                _ => ch,
            };
            take_selection(d);
            let t = d.texts.entry(d.focus.clone()).or_default();
            if max == 0 || (t.chars().count() as i32) < max {
                t.push(ch);
            }
            KeyAct::Done
        }
        Key::Backspace if is_edit => {
            if !take_selection(d) {
                d.texts.entry(d.focus.clone()).or_default().pop();
            }
            KeyAct::Done
        }
        _ => KeyAct::None,
    }
}
