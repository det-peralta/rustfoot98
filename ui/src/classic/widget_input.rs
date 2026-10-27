//! The generic behaviour of the widgets the renderer draws (`win31/widgets.rs`): what a mouse click or a key does to a check box, radio
//! button, radio group, combo box, list box, scroll bar, spin edit or notebook tab, expressed as changes to the dialog's texts
//! (`<name>.checked`, `.sel`, `.top`, `.open`, `.page`, `.index`, `.value`, see `docs/classic-dialogs.md`). A dialog's `click` then only
//! has to react to the new state; it is called with the control's name (list rows and combo items arrive as `Name#row` after
//! `.sel` was set).
use super::dialog::{Dialog, KeyAct};
use super::Key;
use crate::win31::Forms;
use crate::win31::Node;

/// What the generic handling of a mouse press decided.
pub enum MouseAct {
    /// Tell the dialog's `click` that this control was pressed.
    Click(String),
    /// Handled, nothing to tell.
    Done,
    /// Not a widget of this module: the caller handles the press as before.
    Default,
}

fn get<'a>(d: &'a Dialog, name: &str, key: &str) -> Option<&'a str> {
    d.texts.get(&format!("{name}.{key}")).map(String::as_str)
}

fn get_i(d: &Dialog, name: &str, key: &str) -> Option<i32> {
    get(d, name, key).and_then(|v| v.trim().parse().ok())
}

fn set(d: &mut Dialog, name: &str, key: &str, v: impl ToString) {
    d.texts.insert(format!("{name}.{key}"), v.to_string());
}

fn node<'a>(c: &'a Forms, d: &Dialog, name: &str) -> Option<&'a Node> {
    c.get(&d.form)?.find(name)
}

fn parent_of<'a>(n: &'a Node, name: &str) -> Option<&'a Node> {
    n.children.iter().find_map(|c| {
        if c.name == name {
            Some(n)
        } else {
            parent_of(c, name)
        }
    })
}

/// The items of a combo box, list box or radio group.
fn items(c: &Forms, d: &Dialog, name: &str) -> Vec<String> {
    if let Some(v) = get(d, name, "items") {
        return v.split('\n').map(str::to_string).collect();
    }
    if let Some(v) = d.lists.get(name) {
        return v.clone();
    }
    node(c, d, name).map_or_else(Vec::new, |n| n.strings("Items.Strings"))
}

/// Rows that fit in a list (from the last frame's scroll bar record), or `default` when no scroll bar was drawn.
fn fit(d: &Dialog, name: &str, default: i32) -> i32 {
    d.hits
        .iter()
        .find(|h| h.class == "ScrollInfo" && h.name == format!("{name}#info"))
        .map_or(default, |h| h.w)
}

fn split(name: &str) -> (&str, Option<i32>) {
    match name.rsplit_once('#') {
        Some((a, b)) => (a, b.parse().ok()),
        None => (name, None),
    }
}

fn is_checked(c: &Forms, d: &Dialog, name: &str) -> bool {
    match get(d, name, "checked") {
        Some(v) => v != "0",
        None => node(c, d, name).is_some_and(|n| n.flag("Checked", false)),
    }
}

/// Scrolls a list so that row `row` is visible (`fit` rows are shown).
fn reveal(d: &mut Dialog, name: &str, row: i32, fit: i32) {
    let top = get_i(d, name, "top").unwrap_or(0);
    if row < top {
        set(d, name, "top", row.max(0));
    } else if row >= top + fit {
        set(d, name, "top", row - fit + 1);
    }
}

/// A mouse press on the control called `name` of class `class` (a `Hit` of the last frame).
pub fn mouse(c: &Forms, d: &mut Dialog, name: &str, class: &str) -> MouseAct {
    let (base, idx) = split(name);
    match class {
        "TCheckBox" => {
            let v = !is_checked(c, d, name);
            set(d, name, "checked", i32::from(v));
            d.focus = name.to_string();
            MouseAct::Click(name.to_string())
        }
        "TRadioButton" => {
            // the radio buttons of the same parent form a group
            let mates: Vec<String> = c
                .get(&d.form)
                .and_then(|f| parent_of(f, name).or(Some(f)))
                .map(|p| {
                    p.children
                        .iter()
                        .filter(|k| k.class == "TRadioButton")
                        .map(|k| k.name.clone())
                        .collect()
                })
                .unwrap_or_default();
            for m in mates {
                set(d, &m, "checked", i32::from(m == name));
            }
            d.focus = name.to_string();
            MouseAct::Click(name.to_string())
        }
        "RadioItem" => {
            if let Some(i) = idx {
                set(d, base, "index", i);
            }
            d.focus = base.to_string();
            MouseAct::Click(base.to_string())
        }
        "TComboBox" => {
            let open = get(d, name, "open").is_some_and(|v| v == "1");
            let others: Vec<String> = d
                .texts
                .iter()
                .filter(|(k, v)| k.ends_with(".open") && *v == "1")
                .map(|(k, _)| k.trim_end_matches(".open").to_string())
                .collect();
            for o in others {
                set(d, &o, "open", 0);
            }
            set(d, name, "open", i32::from(!open));
            d.focus = name.to_string();
            MouseAct::Done
        }
        "ComboItem" => {
            let Some(i) = idx else {
                return MouseAct::Done;
            };
            set(d, base, "sel", i);
            if d.texts.contains_key(&format!("{base}.text")) {
                if let Some(t) = items(c, d, base).get(i as usize).cloned() {
                    set(d, base, "text", t);
                }
            }
            set(d, base, "open", 0);
            d.focus = base.to_string();
            MouseAct::Click(base.to_string())
        }
        "NotebookTab" => {
            if let Some(i) = idx {
                set(d, base, "page", i);
            }
            d.focus = base.to_string();
            MouseAct::Click(base.to_string())
        }
        "ListRow" => {
            if let Some(i) = idx {
                set(d, base, "sel", i);
            }
            MouseAct::Click(name.to_string())
        }
        "ScrollUp" | "ScrollDown" | "ScrollPage" | "ScrollThumb" => {
            let n = items(c, d, base).len() as i32;
            let page = fit(d, base, 8);
            let top = get_i(d, base, "top").unwrap_or(0);
            let delta = match name.rsplit('#').next() {
                Some("up") => -1,
                Some("down") => 1,
                Some("pgup") => -page,
                Some("pgdn") => page,
                _ => 0,
            };
            let max = (n - page).max(0);
            set(d, base, "top", (top + delta).clamp(0, max));
            MouseAct::Done
        }
        "SpinUp" | "SpinDown" => {
            let step = if class == "SpinUp" { 1 } else { -1 };
            let (lo, hi) =
                node(c, d, base).map_or((0, 0), |n| (n.int("MinValue", 0), n.int("MaxValue", 0)));
            let cur = get_i(d, base, "value")
                .or_else(|| d.texts.get(base).and_then(|t| t.trim().parse().ok()))
                .unwrap_or(lo);
            let mut v = cur + step;
            if hi > lo {
                v = v.clamp(lo, hi);
            } else {
                v = v.max(lo);
            }
            set(d, base, "value", v);
            d.focus = base.to_string();
            MouseAct::Click(base.to_string())
        }
        _ => MouseAct::Default,
    }
}

/// Keys for the focused widget (before the generic Enter / Escape / Tab handling). `None` when the key is not for a widget.
pub fn key(c: &Forms, d: &mut Dialog, key: Key) -> Option<KeyAct> {
    let name = d.focus.clone();
    let class = d.hits.iter().find(|h| h.name == name)?.class.clone();
    let open = get(d, &name, "open").is_some_and(|v| v == "1");
    match (class.as_str(), key) {
        ("TCheckBox", Key::Char(' ')) => {
            let v = !is_checked(c, d, &name);
            set(d, &name, "checked", i32::from(v));
            Some(KeyAct::Press(name))
        }
        ("TRadioButton", Key::Char(' ')) => match mouse(c, d, &name, "TRadioButton") {
            MouseAct::Click(n) => Some(KeyAct::Press(n)),
            _ => Some(KeyAct::Done),
        },
        ("TComboBox", Key::F(4)) => {
            set(d, &name, "open", i32::from(!open));
            Some(KeyAct::Done)
        }
        ("TComboBox", Key::Escape) if open => {
            set(d, &name, "open", 0);
            Some(KeyAct::Done)
        }
        ("TComboBox", Key::Enter) if open => {
            set(d, &name, "open", 0);
            if d.texts.contains_key(&format!("{name}.text")) {
                let sel = get_i(d, &name, "sel").unwrap_or(0).max(0) as usize;
                if let Some(t) = items(c, d, &name).get(sel).cloned() {
                    set(d, &name, "text", t);
                }
            }
            Some(KeyAct::Press(name))
        }
        ("TComboBox", Key::Up | Key::Down) => {
            let n = items(c, d, &name).len() as i32;
            if n == 0 {
                return Some(KeyAct::Done);
            }
            let cur = get_i(d, &name, "sel").unwrap_or(-1);
            let new = (cur + if key == Key::Down { 1 } else { -1 }).clamp(0, n - 1);
            set(d, &name, "sel", new);
            let rows = node(c, d, &name)
                .map_or(8, |nd| nd.int("DropDownCount", 8))
                .min(n);
            reveal(d, &name, new, rows);
            if open {
                Some(KeyAct::Done)
            } else {
                Some(KeyAct::Press(name))
            }
        }
        ("TListBox", Key::Up | Key::Down) => {
            let n = items(c, d, &name).len() as i32;
            if n == 0 {
                return Some(KeyAct::Done);
            }
            let cur = get_i(d, &name, "sel").unwrap_or(-1);
            let new = (cur + if key == Key::Down { 1 } else { -1 }).clamp(0, n - 1);
            set(d, &name, "sel", new);
            let rows = fit(d, &name, n);
            reveal(d, &name, new, rows);
            Some(KeyAct::Press(format!("{name}#{new}")))
        }
        // type-ahead: a letter selects the next row that starts with it (the Windows list box does this by itself)
        ("TListBox", Key::Char(ch)) if ch.is_alphanumeric() => {
            let list = items(c, d, &name);
            let n = list.len();
            let cur = get_i(d, &name, "sel").unwrap_or(-1);
            let lower = |s: &str| {
                s.chars()
                    .next()
                    .is_some_and(|f| f.to_lowercase().eq(ch.to_lowercase()))
            };
            let found = (1..=n)
                .map(|k| ((cur + k as i32).rem_euclid(n.max(1) as i32)) as usize)
                .find(|&i| list.get(i).is_some_and(|s| lower(s)));
            match found {
                Some(new) => {
                    set(d, &name, "sel", new as i32);
                    let rows = fit(d, &name, n as i32);
                    reveal(d, &name, new as i32, rows);
                    Some(KeyAct::Press(format!("{name}#{new}")))
                }
                None => Some(KeyAct::Done),
            }
        }
        ("TRadioGroup", Key::Up | Key::Down | Key::Left | Key::Right) => {
            let n = items(c, d, &name).len() as i32;
            let cur = get_i(d, &name, "index").unwrap_or(0);
            let step = if matches!(key, Key::Down | Key::Right) {
                1
            } else {
                -1
            };
            set(d, &name, "index", (cur + step).clamp(0, (n - 1).max(0)));
            Some(KeyAct::Press(name))
        }
        ("TTabbedNotebook", Key::Left | Key::Right) => {
            let pages = node(c, d, &name).map_or(1, |n| {
                n.children.iter().filter(|k| k.class == "TTabPage").count() as i32
            });
            let pages = get_i(d, &name, "tabcount").unwrap_or(pages).max(1);
            let cur = get_i(d, &name, "page").unwrap_or(0);
            let step = if key == Key::Right { 1 } else { -1 };
            set(d, &name, "page", (cur + step).rem_euclid(pages));
            Some(KeyAct::Press(name))
        }
        ("TSpinEdit", Key::Up | Key::Down) => {
            let cls = if key == Key::Up { "SpinUp" } else { "SpinDown" };
            match mouse(c, d, &format!("{name}#x"), cls) {
                MouseAct::Click(n) => Some(KeyAct::Press(n)),
                _ => Some(KeyAct::Done),
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classic::dialog::Kind;

    #[test]
    fn names_split_at_the_row() {
        assert_eq!(split("List#12"), ("List", Some(12)));
        assert_eq!(split("List"), ("List", None));
        assert_eq!(split("List#up"), ("List", None));
    }

    #[test]
    fn selecting_below_the_view_scrolls_down() {
        let mut d = Dialog::new(Kind::Other("t".into()), "TX");
        reveal(&mut d, "L", 12, 10);
        assert_eq!(d.texts["L.top"], "3");
        reveal(&mut d, "L", 1, 10);
        assert_eq!(d.texts["L.top"], "1");
        reveal(&mut d, "L", 5, 10);
        assert_eq!(d.texts["L.top"], "1");
    }

    #[test]
    fn the_fit_comes_from_the_scroll_bar_record() {
        let mut d = Dialog::new(Kind::Other("t".into()), "TX");
        assert_eq!(fit(&d, "L", 7), 7);
        d.hits.push(crate::win31::draw::Hit {
            name: "L#info".into(),
            class: "ScrollInfo".into(),
            x: 0,
            y: 0,
            w: 31,
            h: 54,
            enabled: false,
        });
        assert_eq!(fit(&d, "L", 7), 31);
    }
}
