//! The menu bar of the main window and its drop-down menus.
use super::Classic;
use crate::win31::draw::{text, text_dithered};
use crate::win31::fb::{Rgb, BLACK, GRAY, NAVY, SILVER, WHITE};
use rustfoot_core::season::{plan_for_formation, Club, FORMATIONS};

pub struct Entry {
    pub name: String,
    pub label: String,
    pub shortcut: String,
    pub enabled: bool,
    pub separator: bool,
}

pub struct Top {
    pub name: String,
    pub label: String,
    pub entries: Vec<Entry>,
}

/// Frame position of the main window (client 632x434 with a menu bar).
pub const FRAME: (i32, i32) = (83, 63);

pub fn formation_of(label: &str) -> Option<usize> {
    let v: Vec<u8> = label.split('-').filter_map(|p| p.parse().ok()).collect();
    if v.len() != 3 {
        return None;
    }
    FORMATIONS.iter().position(|&f| f == (v[0], v[1], v[2]))
}

/// The menus as the form declares them, with the entries the state of the game hides or disables.
pub fn model(c: &Classic, club: &Club) -> Vec<Top> {
    let Some(form) = c.forms.get("TPREPAREGAMEDLG") else {
        return Vec::new();
    };
    let mut tops = Vec::new();
    let mut f_key = 0;
    for m in form.children.iter().filter(|n| n.class == "TMainMenu") {
        for t in &m.children {
            let mut entries = Vec::new();
            for e in &t.children {
                let label = e.str("Caption").unwrap_or("").replace('&', "");
                let mut en = Entry {
                    name: e.name.clone(),
                    shortcut: String::new(),
                    enabled: e.flag("Enabled", true),
                    separator: label == "-",
                    label,
                };
                if let Some(k) = formation_of(&en.label) {
                    f_key += 1;
                    en.shortcut = format!("F{f_key}");
                    en.enabled = plan_for_formation(&club.slots, FORMATIONS[k]).is_some();
                }
                // the shortcut the form declares ("Alt+F4" for *Sair*, "C" for *Classificação*)
                if let Some(s) = e.str("ShortCutText") {
                    en.shortcut = s.to_string();
                }
                match e.name.as_str() {
                    "Automtico1" => en.shortcut = "A".into(),
                    "Melhores1" => en.shortcut = "M".into(),
                    // printing is not available
                    "opcoesImprimir" => continue,
                    // the main window's refresh (`1018:11f3`) shows these only to a registered copy (features `0xe`, `0x13`, `0x46`,
                    // `0xc`, `0x10` of `1050:30bc`; checked on the original)
                    "TreinadorGestao"
                    | "Pediremprestimo1"
                    | "Sairsemgravar1"
                    | "Jogadoressobobservao1"
                    | "JogadorProcurar1"
                    | "EquipaAquisicoes1"
                        if !c.main.market.registered =>
                    {
                        continue
                    }
                    // borrowing is offered only to a club in the red or with a loan
                    "Pediremprestimo1" if club.fin.cash >= 0 && club.fin.loan == 0 => continue,
                    _ => {}
                }
                entries.push(en);
            }
            tops.push(Top {
                name: t.name.clone(),
                label: t.str("Caption").unwrap_or("").replace('&', ""),
                entries,
            });
        }
    }
    tops
}

/// The horizontal extents `(left, right)` of the top-level items in the bar.
pub fn bar_items(c: &Classic, tops: &[Top]) -> Vec<(i32, i32)> {
    let Some(f) = c.skin.font("system", 13, false, false) else {
        return Vec::new();
    };
    let mut pen = FRAME.0 + 1 + 8;
    tops.iter()
        .map(|t| {
            let w = f.width(&t.label);
            let r = (pen - 8, pen + w + 8);
            pen += w + 16;
            r
        })
        .collect()
}

const ITEM_H: i32 = 18;
const SEP_H: i32 = 7;

/// The box of a drop-down menu: `(x, y, w, h)`.
pub fn dropdown_rect(c: &Classic, tops: &[Top], open: usize) -> (i32, i32, i32, i32) {
    let bar = bar_items(c, tops);
    let (x, _) = bar.get(open).copied().unwrap_or((FRAME.0, 0));
    let y = FRAME.1 + 38;
    let Some(f) = c.skin.font("system", 13, false, false) else {
        return (x, y, 0, 0);
    };
    let t = &tops[open];
    let label_w = t
        .entries
        .iter()
        .map(|e| f.width(&e.label))
        .max()
        .unwrap_or(0);
    let short_w = t
        .entries
        .iter()
        .map(|e| f.width(&e.shortcut))
        .max()
        .unwrap_or(0);
    let w = if short_w > 0 {
        16 + label_w + 8 + short_w + 15
    } else {
        16 + label_w + 14
    };
    let h: i32 = 2 + t
        .entries
        .iter()
        .map(|e| if e.separator { SEP_H } else { ITEM_H })
        .sum::<i32>();
    (x, y, w, h)
}

/// The entry under `(px, py)` of the open drop-down menu.
pub fn entry_at(c: &Classic, tops: &[Top], open: usize, px: i32, py: i32) -> Option<usize> {
    let (x, y, w, _) = dropdown_rect(c, tops, open);
    if px <= x || px >= x + w - 1 {
        return None;
    }
    let mut top = y + 1;
    for (i, e) in tops[open].entries.iter().enumerate() {
        let h = if e.separator { SEP_H } else { ITEM_H };
        if (top..top + h).contains(&py) {
            return (!e.separator).then_some(i);
        }
        top += h;
    }
    None
}

/// Draws the open menu: the title in the bar highlighted and the drop-down box under it.
pub fn draw_open(c: &mut Classic, tops: &[Top], open: usize, hl: Option<usize>) {
    let Some(f) = c.skin.font("system", 13, false, false) else {
        return;
    };
    let bar = bar_items(c, tops);
    if let Some(&(l, r)) = bar.get(open) {
        c.fb.fill(l, FRAME.1 + 20, r - l, 18, NAVY);
        text(
            &mut c.fb,
            f,
            l + 8,
            FRAME.1 + 20,
            &tops[open].label,
            WHITE,
            false,
        );
    }
    let (x, y, w, h) = dropdown_rect(c, tops, open);
    c.fb.fill(x + 1, y + 1, w, h, SILVER); // the shadow
    c.fb.fill(x, y, w, h, BLACK);
    c.fb.fill(x + 1, y + 1, w - 2, h - 2, WHITE);
    let t = &tops[open];
    let sc_x = x
        + 16
        + t.entries
            .iter()
            .map(|e| f.width(&e.label))
            .max()
            .unwrap_or(0)
        + 8;
    let mut top = y + 1;
    for (i, e) in t.entries.iter().enumerate() {
        if e.separator {
            c.fb.fill(x + 1, top + 3, w - 2, 1, BLACK);
            top += SEP_H;
            continue;
        }
        let on = hl == Some(i);
        if on {
            c.fb.fill(x + 1, top, w - 2, ITEM_H.min(y + h - 1 - top), NAVY);
        }
        let color: Rgb = match (on, e.enabled) {
            (true, _) => WHITE,
            (false, true) => BLACK,
            (false, false) => SILVER,
        };
        if on && !e.enabled {
            text_dithered(&mut c.fb, f, x + 16, top, &e.label, color, 0);
            text_dithered(&mut c.fb, f, sc_x, top, &e.shortcut, color, 0);
        } else {
            text(&mut c.fb, f, x + 16, top, &e.label, color, false);
            text(&mut c.fb, f, sc_x, top, &e.shortcut, color, false);
        }
        top += ITEM_H;
    }
    let _ = GRAY;
}
