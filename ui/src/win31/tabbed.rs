//! `TTabbedNotebook` (the Perfil dialog of the coach) and `TRadioGroup`.
//!
//! The notebook draws a strip of tabs (the captions of its `TTabPage` children, in the notebook's `TabFont`) over a framed page and
//! then the controls of the selected page; `<name>.page` in the texts selects the page and the tabs are hit as `<name>#<page>`
//! (class `TabSetTab`). The tab geometry follows the look of Delphi 1's notebook (raised tabs, the selected one joined to the page)
//! but has not been measured against the original.
//!
//! A radio group lays its items out as Delphi 1's `TRadioGroup.Arrange` does; `<name>.index` is the selected item and the items are
//! hit as `<name>#<item>`.
use super::draw::{control, text, Ctx, FontSpec, Hit};
use super::fb::{Fb, BLACK, GRAY, WHITE};
use super::form::Node;
use super::json::Json;

/// Height of the tab strip above the page.
const TABS: i32 = 22;

fn page_of(cx: &Ctx, n: &Node) -> usize {
    cx.texts
        .get(&format!("{}.page", n.name))
        .and_then(|s| s.parse().ok())
        .unwrap_or(n.int("PageIndex", 0) as usize)
}

pub fn notebook(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let page = page_of(cx, n);
    let tab_px = -n.int("TabFont.Height", -11);
    let font = [tab_px, 11, 13]
        .iter()
        .find_map(|&px| cx.skin.font(n.str("TabFont.Name").unwrap_or("MS Sans Serif"), px, false, false));
    // the framed page
    let top = y + TABS - 1;
    fb.fill(x, top, w, 1, WHITE);
    fb.fill(x, top, 1, h - TABS + 1, WHITE);
    fb.fill(x + w - 2, top + 1, 1, h - TABS, GRAY);
    fb.fill(x + 1, y + h - 2, w - 2, 1, GRAY);
    fb.fill(x + w - 1, top, 1, h - TABS + 1, BLACK);
    fb.fill(x, y + h - 1, w, 1, BLACK);
    let mut pen = x + 4;
    for (i, p) in n.children.iter().enumerate() {
        let cap = p.str("Caption").unwrap_or("").replace('&', "");
        let tw = font.map_or(60, |f| f.width(&cap));
        let tab_w = tw + 16;
        let selected = i == page;
        let (ty, th) = if selected {
            (y, TABS + 1)
        } else {
            (y + 2, TABS - 3)
        };
        fb.fill(pen, ty + 1, tab_w, th - 1, cx.bg);
        fb.fill(pen, ty + 1, 1, th - 1, WHITE);
        fb.fill(pen + 1, ty, tab_w - 2, 1, WHITE);
        fb.fill(pen + tab_w - 1, ty + 1, 1, th - 1, BLACK);
        fb.fill(pen + tab_w - 2, ty + 1, 1, th - 1, GRAY);
        if selected {
            // the tab opens into the page
            fb.fill(pen + 1, top, tab_w - 3, 1, cx.bg);
        }
        if let Some(font) = font {
            text(fb, font, pen + 8, ty + 4, &cap, f.color, false);
        }
        cx.hits.borrow_mut().push(Hit {
            name: format!("{}#{i}", n.name),
            class: "TabSetTab".into(),
            x: pen,
            y: ty,
            w: tab_w,
            h: th,
            enabled: true,
        });
        pen += tab_w + 1;
    }
    if let Some(p) = n.children.get(page) {
        // the controls of the page are placed relative to the page, which sits at its own (Left, Top) in the notebook
        let sub = Ctx {
            ox: x + p.int("Left", 0),
            oy: y + p.int("Top", 0),
            ..cx.sub()
        };
        for c in &p.children {
            control(fb, &sub, c, f);
        }
    }
}

/// `TRadioGroup`: the group box with its caption and one radio button per item.
pub fn radio_group(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let Some(font) = cx.font(f) else { return };
    // the frame and caption: a group box without children
    let frame = {
        let mut m = n.clone();
        m.class = "TGroupBox".into();
        m.children.clear();
        m
    };
    control_box(fb, cx, &frame, f, x, y);
    let items = n.strings("Items.Strings");
    if items.is_empty() {
        return;
    }
    let th = font.height();
    let bh = (h - th - 5) / items.len() as i32;
    let margin = th + 1 + (h - th - 4 - bh * items.len() as i32) / 2;
    let sel: i32 = cx
        .texts
        .get(&format!("{}.index", n.name))
        .and_then(|v| v.parse().ok())
        .unwrap_or(n.int("ItemIndex", -1));
    for (i, it) in items.iter().enumerate() {
        let iy = y + margin + i as i32 * bh;
        let mut b = Node {
            class: "TRadioButton".into(),
            name: format!("{}#{i}", n.name),
            props: Default::default(),
            children: Vec::new(),
        };
        for (k, v) in [("Left", 8), ("Top", 0), ("Width", w - 16), ("Height", bh)] {
            b.props.insert(k.into(), Json::Num(v as f64));
        }
        b.props.insert("Caption".into(), Json::Str(it.clone()));
        let sub = Ctx {
            ox: x,
            oy: iy,
            ..cx.sub()
        };
        control(fb, &sub, &b, f);
        if sel == i as i32 {
            // the dot of the selected button, centred in the 13-pixel ring
            let (cx0, cy0) = (x + 8 + 6, iy + (bh - 13) / 2 + 6);
            for (dy, half) in [(-2, 1), (-1, 2), (0, 2), (1, 2), (2, 1)] {
                fb.fill(cx0 - half, cy0 + dy, 2 * half + 1, 1, f.color);
            }
        }
    }
}

fn control_box(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    // the group box frame and caption are drawn by the generic control routine at the control's own position
    let sub = Ctx {
        ox: x - n.int("Left", 0),
        oy: y - n.int("Top", 0),
        ..cx.sub()
    };
    control(fb, &sub, n, f);
}
