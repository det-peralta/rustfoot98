//! The controls of the Windows 3.1 / Delphi 1 look that go beyond the basic ones of `draw.rs`: check boxes, radio groups, combo boxes
//! (closed and with the drop-down list), spin edits, notebooks with tabs, colour grids, speed buttons, scroll bars and the lists with
//! columns. The text keys that drive them are documented in `docs/classic-dialogs.md`.
use super::draw::{
    bevel_miter, button_face, caption, control, focus_rect, is_enabled, sunken_box, text,
    text_dithered, Ctx, FontSpec, Hit, DITHER_PARITY,
};
use super::fb::{Fb, Rgb, BLACK, GRAY, NAVY, SILVER, WHITE};
use super::form::Node;
use super::skin::Font;

/// A value the program set for a control: `<name>.<key>` in the texts.
pub(super) fn get<'a>(cx: &'a Ctx, n: &Node, key: &str) -> Option<&'a str> {
    cx.texts
        .get(&format!("{}.{key}", n.name))
        .map(String::as_str)
}

pub(super) fn get_num(cx: &Ctx, n: &Node, key: &str) -> Option<i32> {
    get(cx, n, key).and_then(|v| v.trim().parse().ok())
}

fn flag(cx: &Ctx, n: &Node, key: &str) -> bool {
    get(cx, n, key).is_some_and(|v| v != "0" && !v.is_empty())
}

fn push_hit(cx: &Ctx, name: String, class: &str, x: i32, y: i32, w: i32, h: i32) {
    cx.hits.borrow_mut().push(Hit {
        name,
        class: class.into(),
        x,
        y,
        w,
        h,
        enabled: true,
    });
}

/// The colour of a pixel already in the frame buffer.
fn under(fb: &Fb, x: i32, y: i32) -> Rgb {
    let p = fb.get(x, y);
    Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8)
}

/// `DrawFocusRect` over arbitrary pixels: the lit dots (where the offsets from the window origin `(wx, wy)` add up to an odd number)
/// invert what is below them, the corners stay untouched. `solid` lights every pixel (the combo box does that).
#[allow(clippy::too_many_arguments)]
pub(super) fn focus_rect_not(
    fb: &mut Fb,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    wx: i32,
    wy: i32,
    solid: bool,
) {
    let dot = |fb: &mut Fb, px: i32, py: i32| {
        if solid || (px - wx + py - wy).rem_euclid(2) == 1 {
            let c = under(fb, px, py);
            fb.set(px, py, Rgb(!c.0, !c.1, !c.2));
        }
    };
    for i in 1..w - 1 {
        for j in [0, h - 1] {
            dot(fb, x + i, y + j);
        }
    }
    for j in 1..h - 1 {
        for i in [0, w - 1] {
            dot(fb, x + i, y + j);
        }
    }
}

/// `DrawFocusRect` on a row written in the colour `ink`: GDI's pattern brush XORs its lit dots with the background colour (white:
/// they invert) and the others with the text colour, so a list in red shows cyan between the dots (the team editor's, measured).
#[allow(clippy::too_many_arguments)]
pub(super) fn focus_rect_ink(
    fb: &mut Fb,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    wx: i32,
    wy: i32,
    ink: Rgb,
) {
    focus_rect_not(fb, x, y, w, h, wx, wy, false);
    let k = ink.q();
    let dot = |fb: &mut Fb, px: i32, py: i32| {
        if (px - wx + py - wy).rem_euclid(2) == 0 {
            let c = under(fb, px, py);
            fb.set(px, py, Rgb(c.0 ^ k.0, c.1 ^ k.1, c.2 ^ k.2));
        }
    };
    for i in 1..w - 1 {
        for j in [0, h - 1] {
            dot(fb, x + i, y + j);
        }
    }
    for j in 1..h - 1 {
        for i in [0, w - 1] {
            dot(fb, x + i, y + j);
        }
    }
}

/// The focus rectangle of a notebook tab: lit dots black, the others are XORed with the tab text colour.
#[allow(clippy::too_many_arguments)]
pub(super) fn focus_rect_tab(
    fb: &mut Fb,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    wx: i32,
    wy: i32,
    ink: Rgb,
) {
    let dot = |fb: &mut Fb, px: i32, py: i32| {
        if (px - wx + py - wy).rem_euclid(2) == 1 {
            fb.set(px, py, BLACK);
        } else {
            let c = under(fb, px, py);
            let k = ink.q();
            fb.set(px, py, Rgb(c.0 ^ k.0, c.1 ^ k.1, c.2 ^ k.2));
        }
    };
    for i in 1..w - 1 {
        for j in [0, h - 1] {
            dot(fb, x + i, y + j);
        }
    }
    for j in 1..h - 1 {
        for i in [0, w - 1] {
            dot(fb, x + i, y + j);
        }
    }
}

/// The face of the small square buttons of scroll bars, combo boxes and spin edits: a white line on the top and left, two grey lines
/// on the bottom and right.
pub(super) fn flat_button(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32) {
    fb.fill(x, y, w, h, SILVER);
    fb.fill(x, y, w - 1, 1, WHITE);
    fb.set(x + w - 1, y, GRAY);
    fb.fill(x, y, 1, h - 1, WHITE);
    fb.fill(x + w - 2, y + 1, 2, h - 1, GRAY);
    fb.fill(x + 1, y + h - 2, w - 1, 1, GRAY);
    fb.fill(x, y + h - 1, w, 1, GRAY);
}

/// A bitmap given as rows of `#` (ink) and anything else (transparent), drawn with its top-left corner at `(x, y)`.
fn glyph(fb: &mut Fb, x: i32, y: i32, rows: &[&str], color: Rgb) {
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            if ch == '#' {
                fb.set(x + c as i32, y + r as i32, color);
            }
        }
    }
}

const ARROW_UP: [&str; 7] = [
    "...#...", "..###..", ".#####.", "#######", "..###..", "..###..", "..###..",
];
const ARROW_DOWN: [&str; 7] = [
    "..###..", "..###..", "..###..", "#######", ".#####.", "..###..", "...#...",
];
/// The arrow of a combo box button: a down arrow with a bar below it.
const ARROW_COMBO: [&str; 9] = [
    "..###..", "..###..", "..###..", "#######", ".#####.", "..###..", "...#...", ".......",
    "#######",
];

/// The 13x13 square of a check box and the cross inside it.
fn check_square(fb: &mut Fb, x: i32, y: i32, checked: bool, color: Rgb) {
    fb.fill(x, y, 13, 13, BLACK);
    fb.fill(x + 1, y + 1, 11, 11, SILVER);
    if checked {
        for i in 1..=11 {
            fb.set(x + i, y + i, color);
            fb.set(x + 12 - i, y + i, color);
        }
    }
}

/// The text of a button-class control (check box, radio button): normal, or dithered when disabled. Returns its width.
fn button_text(
    fb: &mut Fb,
    font: &Font,
    x: i32,
    y: i32,
    s: &str,
    f: &FontSpec,
    enabled: bool,
) -> i32 {
    if enabled {
        text(fb, font, x, y, s, f.color, false)
    } else {
        text_dithered(fb, font, x, y, s, BLACK, DITHER_PARITY);
        font.width(s)
    }
}

/// A control of the button class with a mark (check box or radio button): the 13-pixel mark at the left, the caption 18 pixels
/// right of it, the focus rectangle around the caption clipped to the control. `(x, y, w, h)` is the control's rectangle.
#[allow(clippy::too_many_arguments)]
fn marked_button(
    fb: &mut Fb,
    cx: &Ctx,
    f: &FontSpec,
    (x, y, w, h): (i32, i32, i32, i32),
    cap: &str,
    mark: impl FnOnce(&mut Fb, i32, i32),
    enabled: bool,
    focused: bool,
) {
    mark(fb, x, y + (h - 13) / 2);
    let Some(font) = cx.font(f) else {
        return;
    };
    let cap = cap.replace('&', "");
    let ty = y + (h - font.ascent) / 2;
    let tw = button_text(fb, font, x + 18, ty, &cap, f, enabled);
    if focused {
        // the rectangle around the text is cut to the control's client area
        let (rx, ry, rw, rh) = (x + 16, ty - 1, tw + 4, font.height() + 3);
        let (x1, y1) = ((rx + rw).min(x + w), (ry + rh).min(y + h));
        let (rx, ry) = (rx.max(x), ry.max(y));
        focus_rect(fb, rx, ry, x1 - rx, y1 - ry, x, y);
    }
}

/// `TCheckBox`: a 13-pixel square (cross when `<name>.checked=1` or the form's `Checked`), 6 pixels of space, the caption; the focus
/// rectangle surrounds the caption.
pub(super) fn checkbox(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let enabled = is_enabled(cx, n);
    let checked = match get(cx, n, "checked") {
        Some(v) => v != "0",
        None => n.flag("Checked", false) || n.ident("State") == Some("cbChecked"),
    };
    let ink = if enabled { BLACK } else { GRAY };
    marked_button(
        fb,
        cx,
        f,
        (x, y, n.int("Width", 100), n.int("Height", 17)),
        caption(cx, n),
        |fb, mx, my| check_square(fb, mx, my, checked, ink),
        enabled,
        cx.focus == n.name,
    );
}

/// The ring of a radio button, with the dot inside when selected.
pub(super) fn radio_ring(fb: &mut Fb, x: i32, y: i32, selected: bool, color: Rgb) {
    const RING: [&str; 13] = [
        ".....###.....",
        "...##...##...",
        "..#.......#..",
        ".#.........#.",
        ".#.........#.",
        "#...........#",
        "#...........#",
        "#...........#",
        ".#.........#.",
        ".#.........#.",
        "..#.......#..",
        "...##...##...",
        ".....###.....",
    ];
    const DOT: [&str; 7] = [
        "..###..", ".#####.", "#######", "#######", "#######", ".#####.", "..###..",
    ];
    glyph(fb, x, y, &RING, color);
    if selected {
        glyph(fb, x + 3, y + 3, &DOT, color);
    }
}

/// `TRadioButton`: the ring (and dot when `<name>.checked=1`), the caption 18 pixels right of it.
pub(super) fn radio(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let checked = match get(cx, n, "checked") {
        Some(v) => v != "0",
        None => n.flag("Checked", false),
    };
    marked_button(
        fb,
        cx,
        f,
        (x, y, n.int("Width", 100), n.int("Height", 17)),
        caption(cx, n),
        |fb, mx, my| radio_ring(fb, mx, my, checked, f.color),
        is_enabled(cx, n),
        cx.focus == n.name,
    );
}

/// `TRadioGroup`: a group box whose items (`Items.Strings`) are radio buttons spread over its height; `<name>.index` selects one.
pub(super) fn radiogroup(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let Some(font) = cx.font(f) else {
        return;
    };
    let top = y + font.height() / 2 - 1;
    let (fw, fh) = (w - 1, h - (top - y) - 1);
    for (dx, c) in [(0, GRAY), (1, WHITE)] {
        let (l, t) = (x + dx, top + dx);
        fb.fill(l, t, fw, 1, c);
        fb.fill(l, t + fh - 1, fw, 1, c);
        fb.fill(l, t, 1, fh, c);
        fb.fill(l + fw - 1, t, 1, fh, c);
    }
    let cap = caption(cx, n);
    let tw = font.width(cap);
    fb.fill(x + 8, top, tw, 2, cx.bg);
    text(fb, font, x + 8, y, cap, f.color, false);
    let items: Vec<String> = match cx.lists.get(&n.name) {
        Some(v) => v.clone(),
        None => n.strings("Items.Strings"),
    };
    if items.is_empty() {
        return;
    }
    let index = get_num(cx, n, "index").unwrap_or(n.int("ItemIndex", -1));
    let bh = (h - font.tm_height - 5) / items.len() as i32;
    let enabled = is_enabled(cx, n);
    for (i, it) in items.iter().enumerate() {
        let by = y + font.tm_height + 2 + i as i32 * bh;
        marked_button(
            fb,
            cx,
            f,
            (x + 8, by, w - 16, bh),
            it,
            |fb, mx, my| radio_ring(fb, mx, my, index == i as i32, f.color),
            enabled,
            cx.focus == n.name && index == i as i32,
        );
        push_hit(
            cx,
            format!("{}#{i}", n.name),
            "RadioItem",
            x + 8,
            by,
            w - 16,
            bh,
        );
    }
}

/// The visible rows of a closed list and its scroll position: `(first row, rows that fit)`.
fn view(cx: &Ctx, n: &Node, count: usize, fit: usize) -> usize {
    let top = get_num(cx, n, "top").unwrap_or(0).max(0) as usize;
    top.min(count.saturating_sub(fit))
}

/// A Windows 3.1 vertical scroll bar from `y0` for `h` rows, its left black line at `x0` (the bar is 16 wide, the black line of the
/// frame to its right is not drawn). `top`, `count` and `fit` position the thumb; nothing is drawn by the caller when everything fits.
#[allow(clippy::too_many_arguments)]
pub(super) fn vscroll(
    fb: &mut Fb,
    cx: &Ctx,
    name: &str,
    x0: i32,
    y0: i32,
    h: i32,
    top: usize,
    count: usize,
    fit: usize,
) {
    fb.fill(x0, y0, 1, h, BLACK);
    fb.fill(x0 + 1, y0, 15, h, SILVER);
    // up and down buttons with a black line between them and the track
    flat_button(fb, x0 + 1, y0, 15, 15);
    glyph(fb, x0 + 5, y0 + 4, &ARROW_UP, BLACK);
    fb.fill(x0, y0 + 15, 17, 1, BLACK);
    flat_button(fb, x0 + 1, y0 + h - 15, 15, 15);
    glyph(fb, x0 + 5, y0 + h - 15 + 4, &ARROW_DOWN, BLACK);
    fb.fill(x0, y0 + h - 16, 17, 1, BLACK);
    // not a control: tells the program how many rows fit and how many there are (`w`, `h`)
    cx.hits.borrow_mut().push(Hit {
        name: format!("{name}#info"),
        class: "ScrollInfo".into(),
        x: 0,
        y: 0,
        w: fit as i32,
        h: count as i32,
        enabled: false,
    });
    let range = count.saturating_sub(fit);
    let first = y0 + 15;
    let last = y0 + h - 16 - 16;
    // Windows 3.1 places the thumb with (len - 2) rounded down, except at the end of the range (measured on a list of 54 rows)
    let ty = if range == 0 {
        first
    } else if top >= range {
        last
    } else {
        first + (last - first - 2) * top as i32 / range as i32
    };
    fb.fill(x0, ty, 17, 17, BLACK);
    flat_button(fb, x0 + 1, ty + 1, 15, 15);
    push_hit(cx, format!("{name}#up"), "ScrollUp", x0 + 1, y0, 15, 15);
    push_hit(
        cx,
        format!("{name}#down"),
        "ScrollDown",
        x0 + 1,
        y0 + h - 15,
        15,
        15,
    );
    push_hit(cx, format!("{name}#thumb"), "ScrollThumb", x0, ty, 17, 17);
    if ty > first {
        push_hit(
            cx,
            format!("{name}#pgup"),
            "ScrollPage",
            x0 + 1,
            first,
            15,
            ty - first,
        );
    }
    if ty < last {
        push_hit(
            cx,
            format!("{name}#pgdn"),
            "ScrollPage",
            x0 + 1,
            ty + 17,
            15,
            last - ty,
        );
    }
}

/// How a row of a list is painted: `<name>.style` rows in `lists` hold `fg=r,g,b bg=r,g,b bold` (all optional).
#[derive(Default, Clone)]
struct RowStyle {
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    bold: bool,
}

fn parse_rgb(s: &str) -> Option<Rgb> {
    let v: Vec<u8> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() == 3).then(|| Rgb(v[0], v[1], v[2]))
}

fn row_style(cx: &Ctx, n: &Node, i: usize) -> RowStyle {
    let mut st = RowStyle::default();
    if let Some(row) = cx
        .lists
        .get(&format!("{}.style", n.name))
        .and_then(|v| v.get(i))
    {
        for tok in row.split_whitespace() {
            if let Some(v) = tok.strip_prefix("fg=") {
                st.fg = parse_rgb(v);
            } else if let Some(v) = tok.strip_prefix("bg=") {
                st.bg = parse_rgb(v);
            } else if tok == "bold" {
                st.bold = true;
            }
        }
    }
    st
}

/// The column layout of a list: x offsets from the list's left edge and an alignment per column.
fn columns(cx: &Ctx, n: &Node) -> Option<Vec<(i32, char)>> {
    let cols: Vec<i32> = get(cx, n, "cols")?
        .split(',')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    let align: Vec<char> = get(cx, n, "align").unwrap_or("").chars().collect();
    Some(
        cols.iter()
            .enumerate()
            .map(|(i, &c)| (c, align.get(i).copied().unwrap_or('l')))
            .collect(),
    )
}

/// One row of the team editor's country combo (`DrawFlagAndCountry`, `1020:0c4d`): the item rectangle filled white, the flag
/// (`@FORM/NAME|name`) a pixel down from its corner, the name in Arial 12 (16 pixels) at 44, 5.
fn flag_row(fb: &mut Fb, cx: &Ctx, (rx, ry, rw, rh): (i32, i32, i32, i32), s: &str) {
    fb.fill(rx, ry, rw, rh, WHITE);
    let (flag, name) = s.split_once('|').unwrap_or(("", s));
    if let Some((form, bmp)) = flag.strip_prefix('@').and_then(|f| f.split_once('/')) {
        if let Some(b) = cx.skin.bitmap(form, bmp) {
            for r in 0..b.h {
                for c in 0..b.w {
                    let p = b.px[(r * b.w + c) as usize];
                    fb.set(
                        rx + c,
                        ry + 1 + r,
                        Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    );
                }
            }
        }
    }
    let spec = FontSpec {
        name: "arial".into(),
        px: 16,
        bold: false,
        italic: false,
        underline: false,
        color: BLACK,
    };
    if let Some(font) = cx.font(&spec) {
        text(fb, font, rx + 44, ry + 5, name, BLACK, false);
    }
}

/// Draws one row of a list or combo box: the cells of `s` (`a|b|c`) at the column offsets of `<name>.cols` (from `x`), or the whole
/// text at `x + 3`. A cell that starts with `@` is a skin bitmap `FORM/NAME` (a flag), drawn at its offset and centred vertically.
#[allow(clippy::too_many_arguments)]
fn row_cells(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    x: i32,
    ry: i32,
    ih: i32,
    s: &str,
    fg: Rgb,
    bold_all: bool,
) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let bold = cx.font(&FontSpec {
        bold: true,
        ..f.clone()
    });
    let Some(cols) = columns(cx, n) else {
        let font = if bold_all { bold.unwrap_or(font) } else { font };
        text(fb, font, x + 3, ry, s, fg, false);
        return;
    };
    for (k, cell) in s.split('|').enumerate() {
        let Some(&(off, al)) = cols.get(k) else {
            break;
        };
        if let Some(spec) = cell.strip_prefix('@') {
            if let Some((form, name)) = spec.split_once('/') {
                if let Some(b) = cx.skin.bitmap(form, name) {
                    let (bx, by) = (x + off, ry + (ih - b.h) / 2);
                    for r in 0..b.h {
                        for c in 0..b.w {
                            let p = b.px[(r * b.w + c) as usize];
                            fb.set(
                                bx + c,
                                by + r,
                                Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                            );
                        }
                    }
                }
            }
            continue;
        }
        // an upper-case alignment letter (`L`, `R`, `C`) draws the column in bold
        let cf = if bold_all || al.is_ascii_uppercase() {
            bold.unwrap_or(font)
        } else {
            font
        };
        let tw = cf.width(cell);
        let tx = match al.to_ascii_lowercase() {
            'r' => x + off - tw,
            'c' => x + off - tw / 2,
            _ => x + off,
        };
        text(fb, cf, tx, ry, cell, fg, false);
    }
}

/// The standard list box (and the owner-drawn lists of the dialogs): rows from `lists[name]` or the form's `Items.Strings`, scroll
/// position `<name>.top`, selected row `<name>.sel`, columns `<name>.cols` / `<name>.align`, per-row style `lists[<name>.style]`, and a
/// scroll bar when the rows do not fit.
pub(super) fn listbox_rows(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let Some(font) = cx.font(f) else {
        return;
    };
    let items: Vec<String> = match cx.lists.get(&n.name) {
        Some(v) => v.clone(),
        None => n.strings("Items.Strings"),
    };
    let ih = get_num(cx, n, "itemheight")
        .or_else(|| n.props.get("ItemHeight").map(|_| n.int("ItemHeight", 0)))
        .filter(|&v| v > 0)
        .unwrap_or(font.height());
    // `<name>.inset=0`: the rows start at the top edge of the box (owner-drawn lists without a border)
    let inset = get_num(cx, n, "inset").unwrap_or(1);
    let inner_h = h - 2;
    let fit = (inner_h / ih).max(1) as usize;
    let cols = n.int("Columns", 0);
    if cols > 1 {
        // `Columns`: the rows run down a column and on to the next; no scroll bar while the columns fit
        let cw = (w - 2) / cols;
        let sel = get_num(cx, n, "sel")
            .filter(|&v| v >= 0)
            .map(|v| v as usize);
        let old = fb.clip;
        fb.clip = (
            (x + 1).max(old.0),
            (y + inset).max(old.1),
            (x + w - 1).min(old.2),
            (y + h - 1).min(old.3),
        );
        for (i, item) in items.iter().enumerate() {
            let (col, row) = ((i / fit) as i32, (i % fit) as i32);
            if col > cols {
                break;
            }
            let (cx0, ry) = (x + col * cw, y + inset + row * ih);
            let fg = if sel == Some(i) {
                fb.fill(cx0 + 1, ry, cw, ih, NAVY);
                WHITE
            } else {
                f.color
            };
            row_cells(fb, cx, n, f, cx0, ry, ih, item, fg, false);
            push_hit(
                cx,
                format!("{}#{i}", n.name),
                "ListRow",
                cx0 + 1,
                ry,
                cw,
                ih,
            );
        }
        fb.clip = old;
        return;
    }
    let scrolled = items.len() > fit;
    let right = if scrolled { x + w - 17 } else { x + w - 1 };
    let top = view(cx, n, items.len(), fit);
    let sel = get_num(cx, n, "sel")
        .filter(|&v| v >= 0)
        .map(|v| v as usize);
    let focused = cx.focus == n.name;
    let enabled = is_enabled(cx, n);
    let bold_all = flag(cx, n, "bold");
    let old = fb.clip;
    fb.clip = (
        (x + 1).max(old.0),
        (y + inset).max(old.1),
        right.min(old.2),
        (y + h - 1).min(old.3),
    );
    for (r, i) in (top..items.len().min(top + fit + 1)).enumerate() {
        let ry = y + inset + r as i32 * ih;
        let st = row_style(cx, n, i);
        let selected = sel == Some(i);
        let (fg, bg) = if selected {
            (WHITE, Some(NAVY))
        } else {
            (
                if enabled {
                    st.fg.unwrap_or(f.color)
                } else {
                    GRAY
                },
                st.bg,
            )
        };
        if let Some(b) = bg {
            fb.fill(x + 1, ry, right - x - 1, ih, b);
        }
        row_cells(fb, cx, n, f, x, ry, ih, &items[i], fg, bold_all || st.bold);
        // the focus rectangle sits on the selected row, or on the first one while nothing is selected (`caret`; none when negative)
        let caret = get_num(cx, n, "caret").unwrap_or(0);
        if focused && (selected || (sel.is_none() && caret >= 0 && i == caret as usize)) {
            if selected {
                focus_rect_not(fb, x + 1, ry, right - x - 1, ih, x, y, false);
            } else {
                focus_rect_ink(fb, x + 1, ry, right - x - 1, ih, x, y, fg);
            }
        }
        push_hit(
            cx,
            format!("{}#{i}", n.name),
            "ListRow",
            x + 1,
            ry.max(y + 1),
            right - x - 1,
            ih.min(y + h - 1 - ry),
        );
    }
    fb.clip = old;
    if scrolled {
        vscroll(
            fb,
            cx,
            &n.name,
            x + w - 17,
            y + 1,
            inner_h,
            top,
            items.len(),
            fit,
        );
    }
}

/// `TComboBox`: the sunken box with the text part and the arrow button; `<name>.text` is the text shown (otherwise the selected item),
/// `<name>.items` / `lists[name]` / `Items.Strings` the items, `<name>.sel` the highlighted one, `<name>.open=1` shows the drop-down.
pub(super) fn combobox(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let w = n.int("Width", 100);
    let Some(font) = cx.font(f) else {
        return;
    };
    let items = combo_items(cx, n);
    let sel = get_num(cx, n, "sel")
        .filter(|&v| v >= 0)
        .map(|v| v as usize);
    let shown: String = match get(cx, n, "text") {
        Some(t) => t.to_string(),
        None => sel
            .and_then(|i| items.get(i).cloned())
            .or_else(|| cx.texts.get(&n.name).cloned())
            .unwrap_or_else(|| n.str("Text").unwrap_or("").to_string()),
    };
    // the closed combo box is a pixel shorter than the control's `Height`, but a drop-down list is at least as high as its font
    // needs (the line height plus 7: 32 pixels for the currency's 19-pixel Arial, measured)
    let mut h = n.int("Height", font.height() + 8) - 1;
    if n.ident("Style") == Some("csDropDownList") {
        h = h.max(font.tm_height + 7);
    }
    if get(cx, n, "draw") == Some("flag") {
        // an owner-drawn fixed combo is its item's height and 6
        h = n.int("ItemHeight", font.height()) + 6;
    }
    let enabled = is_enabled(cx, n);
    if n.flag("Ctl3D", true) {
        sunken_box(fb, x, y, w, h, WHITE);
    } else {
        fb.fill(x, y, w, h, BLACK);
        fb.fill(x + 1, y + 1, w - 2, h - 2, WHITE);
    }
    fb.fill(x + w - 17, y + 1, 1, h - 2, BLACK);
    flat_button(fb, x + w - 16, y + 1, 15, h - 2);
    glyph(
        fb,
        x + w - 12,
        y + 1 + (h - 2 - 9) / 2,
        &ARROW_COMBO,
        if enabled { BLACK } else { GRAY },
    );
    let ih_field = n.int("ItemHeight", font.height()).max(font.height());
    let ty = y + 1 + (h - 2 - ih_field) / 2;
    let focused = cx.focus == n.name && enabled && !flag(cx, n, "open");
    if get(cx, n, "draw") == Some("flag") {
        // the owner-drawn field: the item 3 pixels in, a navy frame and the focus rectangle around it while focused
        let (rx, ry, rw, rh) = (x + 3, y + 3, w - 3 - 19, h - 6);
        let old = fb.clip;
        fb.clip = (x + 1, y + 1, x + w - 17, y + h - 1);
        flag_row(fb, cx, (rx, ry, rw, rh), &shown);
        if focused {
            let (fx, fy, fw, fh) = (rx - 1, ry - 1, rw + 2, rh + 2);
            fb.fill(fx, fy, fw, 1, NAVY);
            fb.fill(fx, fy + fh - 1, fw, 1, NAVY);
            fb.fill(fx, fy, 1, fh, NAVY);
            fb.fill(fx + fw - 1, fy, 1, fh, NAVY);
            // (dotted around a country; with none chosen yet every pixel of it is inverted: both measured)
            if shown.is_empty() {
                focus_rect_not(fb, rx, ry, rw, rh, x, y, true);
            } else {
                focus_rect(fb, rx, ry, rw, rh, x, y);
            }
        }
        fb.clip = old;
        if flag(cx, n, "open") && !items.is_empty() {
            OVERLAYS.with(|o| o.borrow_mut().push((n.name.clone(), x, y, w, h, f.clone())));
        }
        return;
    }
    let list_style = n.ident("Style") != Some("csDropDown");
    let old = fb.clip;
    fb.clip = (x + 1, y + 1, x + w - 17, y + h - 1);
    if focused && list_style {
        let hw = w - 19 - 2;
        fb.fill(x + 2, y + 2, hw + 1, h - 4, NAVY);
        row_cells(fb, cx, n, f, x, ty, ih_field, &shown, WHITE, false);
        focus_rect_not(fb, x + 2, y + 2, hw + 1, h - 4, x, y, true);
    } else {
        let ink = if enabled { f.color } else { GRAY };
        row_cells(fb, cx, n, f, x, ty, ih_field, &shown, ink, false);
    }
    fb.clip = old;
    if focused && !list_style {
        let cw = font.width(&shown);
        fb.fill(x + 3 + cw, ty, 1, font.height(), BLACK);
    }
    if flag(cx, n, "open") && !items.is_empty() {
        // drawn after every other control so that it covers them
        OVERLAYS.with(|o| o.borrow_mut().push((n.name.clone(), x, y, w, h, f.clone())));
    }
}

/// An open combo box to draw on top of everything: name, position, size and font.
type Overlay = (String, i32, i32, i32, i32, FontSpec);

thread_local! {
    static OVERLAYS: std::cell::RefCell<Vec<Overlay>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Draws what has to be on top of all controls: the open drop-down lists.
pub(super) fn draw_overlays(fb: &mut Fb, cx: &Ctx, root: &Node) {
    let list = OVERLAYS.with(|o| std::mem::take(&mut *o.borrow_mut()));
    for (name, x, y, w, h, f) in list {
        let Some(n) = root.find(&name) else {
            continue;
        };
        let items = combo_items(cx, n);
        let sel = get_num(cx, n, "sel")
            .filter(|&v| v >= 0)
            .map(|v| v as usize);
        combo_dropdown(fb, cx, n, &f, x, y, w, h, &items, sel);
    }
}

fn combo_items(cx: &Ctx, n: &Node) -> Vec<String> {
    if let Some(v) = get(cx, n, "items") {
        return v.split('\n').map(str::to_string).collect();
    }
    match cx.lists.get(&n.name) {
        Some(v) => v.clone(),
        None => n.strings("Items.Strings"),
    }
}

/// The open drop-down list under a combo box: black frame (the combo's bottom line is its top), up to 8 rows, scroll bar beyond that.
#[allow(clippy::too_many_arguments)]
fn combo_dropdown(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    items: &[String],
    sel: Option<usize>,
) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let ih = n.int("ItemHeight", font.height()).max(font.height());
    let mut rows = items.len().min(n.int("DropDownCount", 8).max(1) as usize);
    if get(cx, n, "draw") == Some("flag") {
        // the list keeps the height of `DropDownCount` rows of the font: 3 of the 31-pixel flags
        rows = rows.min((n.int("DropDownCount", 8) * font.height() / ih).max(1) as usize);
    }
    let scrolled = items.len() > rows;
    let top = view(cx, n, items.len(), rows);
    let ly = y + h - 1;
    let lh = rows as i32 * ih + 2;
    fb.fill(x, ly, w, lh, BLACK);
    fb.fill(x + 1, ly + 1, w - 2, lh - 2, WHITE);
    let right = if scrolled { x + w - 17 } else { x + w - 1 };
    let old = fb.clip;
    fb.clip = (x + 1, ly + 1, right, ly + lh - 1);
    for r in 0..rows {
        let i = top + r;
        let ry = ly + 1 + r as i32 * ih;
        let hot = sel == Some(i);
        if get(cx, n, "draw") == Some("flag") {
            flag_row(fb, cx, (x + 1, ry, right - x - 1, ih), &items[i]);
            push_hit(
                cx,
                format!("{}#{i}", n.name),
                "ComboItem",
                x + 1,
                ry,
                right - x - 1,
                ih,
            );
            continue;
        }
        if hot {
            fb.fill(x + 1, ry, right - x - 1, ih, NAVY);
        }
        row_cells(
            fb,
            cx,
            n,
            f,
            x,
            ry,
            ih,
            &items[i],
            if hot { WHITE } else { f.color },
            false,
        );
        push_hit(
            cx,
            format!("{}#{i}", n.name),
            "ComboItem",
            x + 1,
            ry,
            right - x - 1,
            ih,
        );
    }
    fb.clip = old;
    if scrolled {
        vscroll(
            fb,
            cx,
            &n.name,
            x + w - 17,
            ly + 1,
            lh - 2,
            top,
            items.len(),
            rows,
        );
    }
}

/// `TSpinEdit` (Delphi sample component): a sunken edit with the value (`<name>.value`, else the text) and, at its right, a pair of
/// small arrow buttons. `<name>.height` is the height the control really has when its `AutoSize` changed it; `<name>.style = editor`
/// draws the team editor's look (measured on its level dialog): the buttons behind a black rule, split by a black line, and the
/// value (selected) 4 pixels in.
pub(super) fn spinedit(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let w = n.int("Width", 60);
    let h = get_num(cx, n, "height").unwrap_or(n.int("Height", 24));
    let bg = n.color("Color").unwrap_or(WHITE);
    sunken_box(fb, x, y, w, h, bg);
    let value = match get(cx, n, "value") {
        Some(v) => v.to_string(),
        None => caption_text(cx, n),
    };
    if get(cx, n, "style") == Some("editor") {
        spinedit_editor(fb, cx, n, f, (x, y, w, h), &value);
        return;
    }
    // the two buttons (measured on the contract's new salary): a black rule on their left and between them, each face with a
    // white top-left and a grey bottom-right edge, 7-pixel arrows in the middle
    let bx = x + w - 15;
    let th = (h - 4) / 2;
    let (top, low) = (y + 1, y + 2 + th);
    let bh = h - 3 - th;
    fb.fill(bx, y + 1, 1, h - 2, BLACK);
    fb.fill(bx + 14, y + 1, 1, h - 2, BLACK);
    fb.fill(bx, top + th, 14, 1, BLACK);
    for (fy, fh) in [(top, th), (low, bh)] {
        fb.fill(bx + 1, fy, 12, fh, SILVER);
        fb.fill(bx + 1, fy, 12, 1, WHITE);
        fb.fill(bx + 1, fy, 1, fh, WHITE);
        fb.fill(bx + 13, fy, 1, fh, GRAY);
        fb.fill(bx, fy + fh - 1, 14, 1, GRAY);
    }
    glyph(
        fb,
        bx + 4,
        top + (th - 4) / 2,
        &["...#...", "..###..", ".#####.", "#######"],
        BLACK,
    );
    glyph(
        fb,
        bx + 4,
        low + (bh - 4) / 2,
        &["#######", ".#####.", "..###..", "...#..."],
        BLACK,
    );
    if let Some(font) = cx.font(f) {
        let old = fb.clip;
        fb.clip = (x + 1, y + 1, bx, y + h - 1);
        // the value 4 pixels in, and while focused the edit's caret: 2 pixels wide, as high as the font, before character
        // `<name>.caret` (measured on the contract's new salary)
        let ty = y + (h - font.height()) / 2;
        text(fb, font, x + 4, ty, &value, f.color, false);
        if cx.focus == n.name && !super::draw::INACTIVE.with(|a| a.get()) {
            if let Some(k) = get_num(cx, n, "caret") {
                let before: String = value.chars().take(k.max(0) as usize).collect();
                fb.fill(x + 5 + font.width(&before), ty, 2, font.height(), BLACK);
            }
        }
        fb.clip = old;
    }
    push_hit(cx, format!("{}#up", n.name), "SpinUp", bx, top, 14, th);
    push_hit(cx, format!("{}#down", n.name), "SpinDown", bx, low, 14, bh);
}

/// The team editor's spin edit: a black rule left of two 13-pixel buttons (white top and left, a 1-pixel grey shadow) split by a black
/// line, and the value selected, 4 pixels in.
fn spinedit_editor(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    (x, y, w, h): (i32, i32, i32, i32),
    value: &str,
) {
    let rule = x + w - 15;
    let (bx, bw) = (rule + 1, 13);
    let bh = (h - 2) / 2 - 1;
    fb.fill(rule, y + 1, 1, h - 2, BLACK);
    // a button: white top and left, a grey right column and a grey bottom row that also covers the rule
    let button = |fb: &mut Fb, by: i32, bh: i32| {
        fb.fill(bx, by, bw, bh, SILVER);
        fb.fill(bx, by, bw, 1, WHITE);
        fb.fill(bx, by, 1, bh - 1, WHITE);
        fb.fill(bx + bw - 1, by, 1, bh, GRAY);
        fb.fill(rule, by + bh - 1, bw + 1, 1, GRAY);
    };
    button(fb, y + 1, bh);
    fb.fill(rule, y + 1 + bh, bw + 1, 1, BLACK);
    let (y2, bh2) = (y + 2 + bh, h - 3 - bh);
    button(fb, y2, bh2);
    glyph(
        fb,
        bx + 2,
        y + 1 + (bh - 4) / 2,
        &["....#....", "...###...", "..#####..", ".#######."],
        BLACK,
    );
    glyph(
        fb,
        bx + 2,
        y2 + (bh2 - 4) / 2,
        &[".#######.", "..#####..", "...###...", "....#...."],
        BLACK,
    );
    if let Some(font) = cx.font(f) {
        let (tx, ty) = (x + 4, y + (h - font.height()) / 2 - 1);
        let old = fb.clip;
        fb.clip = (x + 1, y + 1, rule, y + h - 1);
        if cx.focus == n.name {
            // (the highlight stops 4 pixels short of the text's advance: measured with "12" in MS Sans Serif 32 bold)
            let hw = font.width(value) - 4;
            fb.fill(tx, ty, hw, font.height(), NAVY);
            text(fb, font, tx, ty, value, WHITE, false);
        } else {
            text(fb, font, tx, ty, value, f.color, false);
        }
        fb.clip = old;
    }
    push_hit(cx, format!("{}#up", n.name), "SpinUp", bx, y + 1, bw, bh);
    push_hit(cx, format!("{}#down", n.name), "SpinDown", bx, y2, bw, bh2);
}

fn caption_text(cx: &Ctx, n: &Node) -> String {
    cx.texts
        .get(&n.name)
        .cloned()
        .unwrap_or_else(|| n.int("Value", n.int("MinValue", 0)).to_string())
}

/// `TTabbedNotebook`: the tab strip (tabs of `TabsPerRow`, the selected one bold and joined to the page), the page frame, then the
/// controls of the page selected by `<name>.page`.
pub(super) fn tabbed_notebook(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let mut pages: Vec<&Node> = n
        .children
        .iter()
        .filter(|c| c.class == "TTabPage")
        .collect();
    if let Some(k) = get_num(cx, n, "tabcount") {
        pages.truncate(k.max(1) as usize);
    }
    if pages.is_empty() {
        return;
    }
    let page = (get_num(cx, n, "page")
        .unwrap_or(n.int("PageIndex", 0))
        .max(0) as usize)
        .min(pages.len() - 1);
    let cur = pages[page];
    // the strip as a whole takes the focus (Tab) and the arrow keys change the page
    push_hit(
        cx,
        n.name.clone(),
        "TTabbedNotebook",
        x,
        y,
        w,
        cur.int("Top", 29) - 3,
    );
    let frame_top = cur.int("Top", 29) - 3;
    // page frame: black outline, white and grey bevel
    let (fl, fr, fb_) = (x + 2, x + w - 3, y + h - 3);
    fb.fill(
        fl,
        y + frame_top,
        fr - fl + 1,
        fb_ - (y + frame_top) + 1,
        BLACK,
    );
    fb.fill(
        fl + 1,
        y + frame_top + 1,
        fr - fl - 1,
        fb_ - (y + frame_top) - 1,
        SILVER,
    );
    bevel_miter(
        fb,
        fl + 1,
        y + frame_top + 1,
        fr - 1,
        fb_ - 1,
        2,
        WHITE,
        GRAY,
    );
    // tabs
    let per_row = n.int("TabsPerRow", 3).max(1);
    let tw = (w - 4) / per_row;
    let tab_font = FontSpec {
        name: n
            .str("TabFont.Name")
            .map_or_else(|| f.name.clone(), str::to_lowercase),
        px: n.int("TabFont.Height", -11).abs(),
        bold: false,
        italic: false,
        underline: false,
        color: n.color("TabFont.Color").unwrap_or(BLACK),
    };
    let bold_font = FontSpec {
        bold: true,
        ..tab_font.clone()
    };
    for (i, p) in pages.iter().enumerate() {
        if i == page {
            continue;
        }
        let l = x + 2 + i as i32 * tw;
        tab_outline(fb, l, l + tw, y, y + frame_top, false);
        tab_label(fb, cx, &tab_font, p, l, tw, y);
        push_hit(
            cx,
            format!("{}#{i}", n.name),
            "NotebookTab",
            l,
            y + 4,
            tw,
            frame_top - 4,
        );
    }
    let l = x + 2 + page as i32 * tw;
    tab_outline(fb, l, l + tw, y, y + frame_top, true);
    let tab_text_w = tab_label(fb, cx, &bold_font, cur, l, tw, y);
    push_hit(
        cx,
        format!("{}#{page}", n.name),
        "NotebookTab",
        l,
        y + 4,
        tw,
        frame_top - 4,
    );
    if cx.focus == n.name {
        let th = cx.font(&bold_font).map_or(13, |t| t.height());
        focus_rect_tab(
            fb,
            l + (tw - tab_text_w + 1) / 2 - 2,
            y + TAB_TEXT_Y,
            tw - 2 * ((tw - tab_text_w + 1) / 2 - 2) + 1,
            th + 2,
            x,
            y,
            tab_font.color,
        );
    }
    let sub = Ctx {
        ox: x + cur.int("Left", 5),
        oy: y + cur.int("Top", 29),
        ..cx.sub()
    };
    for c in &cur.children {
        control(fb, &sub, c, f);
    }
}

/// Top of the text of a tab, from the top of the notebook.
const TAB_TEXT_Y: i32 = 7;

#[allow(clippy::too_many_arguments)]
fn tab_label(fb: &mut Fb, cx: &Ctx, f: &FontSpec, page: &Node, l: i32, tw: i32, y: i32) -> i32 {
    let Some(font) = cx.font(f) else {
        return 0;
    };
    let cap = page.str("Caption").unwrap_or("").replace('&', "");
    let w = font.width(&cap);
    text(
        fb,
        font,
        l + (tw - w + 1) / 2,
        y + TAB_TEXT_Y,
        &cap,
        f.color,
        false,
    );
    w
}

/// The outline of a tab at `[l, r]` (the black vertical lines are at `l` and `r`), from the notebook's top `y` down to the page frame
/// at `frame_y`. The selected tab is open towards the page.
fn tab_outline(fb: &mut Fb, l: i32, r: i32, y: i32, frame_y: i32, selected: bool) {
    // the top: black line, white line below
    fb.fill(l + 4, y + 4, r - l - 7, 1, BLACK);
    fb.fill(l + 4, y + 5, r - l - 6, 1, WHITE);
    // left corner: black diagonal
    for (dx, dy) in [(3, 5), (2, 6), (1, 7)] {
        fb.set(l + dx, y + dy, BLACK);
    }
    fb.fill(l, y + 8, 1, frame_y - y - 8, BLACK);
    // right corner
    for (dx, dy) in [(2, 6), (1, 7)] {
        fb.set(r - dx, y + dy, BLACK);
    }
    fb.fill(r, y + 8, 1, frame_y - y - 8, BLACK);
    if selected {
        fb.fill(l + 3, y + 6, r - l - 7, 1, WHITE);
        fb.fill(l + 2, y + 7, 2, 1, WHITE);
        fb.fill(l + 1, y + 8, 2, frame_y - y - 8, WHITE);
        fb.fill(r - 4, y + 6, 2, 1, GRAY);
        fb.fill(r - 3, y + 7, 2, 1, GRAY);
        fb.fill(r - 2, y + 8, 2, frame_y - y - 8, GRAY);
        // the opening in the page frame under the tab
        fb.fill(l + 1, frame_y, r - l - 1, 3, SILVER);
        fb.fill(l + 1, frame_y, 2, 3, WHITE);
        fb.fill(r - 2, frame_y, 2, 2, GRAY);
        fb.set(r - 2, frame_y + 2, GRAY);
        fb.set(r - 1, frame_y + 2, WHITE);
        fb.set(r, frame_y + 1, WHITE);
        fb.set(r, frame_y + 2, WHITE);
    } else {
        fb.set(l + 3, y + 6, WHITE);
        fb.set(l + 2, y + 7, WHITE);
        fb.fill(l + 1, y + 8, 1, frame_y - y - 8, WHITE);
        fb.set(r - 3, y + 6, GRAY);
        fb.set(r - 2, y + 7, GRAY);
        fb.fill(r - 1, y + 8, 1, frame_y - y - 8, GRAY);
    }
}

/// `TSpeedButton`: a raised button face (white and grey bevel, no outline) with its glyph and caption, drawn pressed when
/// `<name>.checked=1` (or the form's `Down`).
pub(super) fn speedbutton(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 25), n.int("Height", 25));
    let down = match get(cx, n, "checked") {
        Some(v) => v != "0",
        None => n.flag("Down", false),
    };
    let enabled = is_enabled(cx, n);
    if down {
        // pressed: the outline with cut corners, a shadow along the top and left, no bevel
        fb.fill(x, y, w, h, BLACK);
        fb.fill(x + 1, y + 1, w - 2, h - 2, SILVER);
        fb.fill(x + 1, y + 1, w - 2, 1, GRAY);
        fb.fill(x + 1, y + 1, 1, h - 2, GRAY);
        for (cx_, cy_) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            fb.set(x + cx_, y + cy_, SILVER);
        }
    } else {
        button_face(fb, x, y, w, h, false);
    }
    let shift = i32::from(down);
    let bmp = cx.skin.bitmap(cx.form, &n.name);
    let mut gh = 0;
    if let Some(b) = bmp {
        let glyphs = n.int("NumGlyphs", 1).max(1);
        let gw = b.w / glyphs;
        gh = b.h;
        let key = b.px[((b.h - 1) * b.w) as usize];
        let gi = if !enabled && glyphs > 1 { gw } else { 0 };
        let (gx, gy) = (x + (w - gw) / 2 + shift, y + 2 + n.int("Margin", 4) + shift);
        for r in 0..b.h {
            for c in 0..gw {
                let p = b.px[(r * b.w + c + gi) as usize];
                if p != key {
                    fb.set(
                        gx + c,
                        gy + r,
                        Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    );
                }
            }
        }
    }
    if let Some(font) = cx.font(f) {
        let cap = caption(cx, n).replace('&', "");
        let tw = font.width(&cap);
        let ty = y + 2 + n.int("Margin", 4) + gh + n.int("Spacing", 1).max(0) + shift;
        let tx = x + (w - tw) / 2 + shift;
        if enabled {
            text(fb, font, tx, ty, &cap, f.color, false);
        } else {
            text(fb, font, tx + 1, ty + 1, &cap, WHITE, false);
            text(fb, font, tx, ty, &cap, GRAY, false);
        }
    }
}

/// The 16 colours of a `TColorGrid`, in its order (as the team editor's grid shows them).
pub const GRID_COLORS: [Rgb; 16] = [
    Rgb(0, 0, 0),
    Rgb(128, 0, 0),
    Rgb(0, 128, 0),
    Rgb(128, 128, 0),
    Rgb(0, 0, 128),
    Rgb(128, 0, 128),
    Rgb(0, 128, 128),
    Rgb(192, 192, 192),
    Rgb(128, 128, 128),
    Rgb(255, 0, 0),
    Rgb(0, 255, 0),
    Rgb(255, 255, 0),
    Rgb(0, 0, 255),
    Rgb(255, 0, 255),
    Rgb(0, 255, 255),
    Rgb(255, 255, 255),
];

/// `TColorGrid`: 16 colour cells (4 x 4), each sunken in a 2-pixel frame with a 2-pixel gap, the foreground and background cells
/// marked (`<name>.fgmark` / `.bgmark`, default `FG` / `BG`) in the inverse of their colour; `<name>.index` is the foreground,
/// `<name>.bgindex` the background colour. Measured on the team editor's colour dialog.
pub(super) fn colorgrid(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 100), n.int("Height", 100));
    let (cw, ch) = (w / 4, h / 4);
    let fg = get_num(cx, n, "index").unwrap_or(0);
    let bg = get_num(cx, n, "bgindex").unwrap_or(15);
    let fgmark = get(cx, n, "fgmark").unwrap_or("FG");
    let bgmark = get(cx, n, "bgmark").unwrap_or("BG");
    for (i, c) in GRID_COLORS.iter().enumerate() {
        let (gx, gy) = (x + (i as i32 % 4) * cw, y + (i as i32 / 4) * ch);
        // the sunken frame: two dark lines top and left, two white lines bottom and right (the lower one a pixel longer to the left)
        fb.fill(gx + 1, gy + 1, cw - 3, 1, GRAY);
        fb.fill(gx + 1, gy + 2, cw - 4, 1, GRAY);
        fb.fill(gx + 1, gy + 3, 2, ch - 6, GRAY);
        fb.fill(gx + cw - 2, gy + 1, 1, ch - 3, WHITE);
        fb.fill(gx + cw - 3, gy + 2, 1, ch - 4, WHITE);
        fb.fill(gx + 1, gy + ch - 3, cw - 2, 1, WHITE);
        fb.fill(gx, gy + ch - 2, cw - 1, 1, WHITE);
        fb.fill(gx + 3, gy + 3, cw - 6, ch - 6, *c);
        let mark = if i as i32 == fg {
            Some(fgmark)
        } else if i as i32 == bg {
            Some(bgmark)
        } else {
            None
        };
        if let (Some(m), Some(font)) = (mark, cx.font(f)) {
            let ink = Rgb(255 - c.0, 255 - c.1, 255 - c.2);
            text(
                fb,
                font,
                gx + (cw - font.width(m)) / 2,
                gy + (ch - font.height()) / 2,
                m,
                ink,
                false,
            );
        }
        push_hit(cx, format!("{}#{i}", n.name), "ColorCell", gx, gy, cw, ch);
    }
    // (with the focus it shows nothing more: checked on the team editor's colour dialog)
}

/// `TPaintBox`: only the stadium's (`PaintBoxStadium`) is known; `<name>.level` is the stadium level (0..24, one stand of 5000 seats
/// per level). The picture is the original's `PaintBoxStadiumPaint`: up to three rings of stands (grey, 2-pixel light yellow
/// outline) around an ellipse with a green pitch. Stand fractions: level 1..6 fill the first ring (of 6 parts), 7..14 the second
/// (of 8), 15..24 the third (of 10). The ring and ellipse geometry were fitted to a capture of a level-2 stadium (approximate).
///
/// The original's geometry, decoded (`1030:2a7c`, `1030:2cee`; W, H the paint box, `Round` halves to even): ring `k` (0 the pitch's
/// ellipse) is an `Ellipse`/`Pie` in the centred box `round(W * (k/6 + 0.4))` x `round(H * (k/6 + 0.4))`, the stands a `Pie` from
/// the top to the angle `n * 3.14 / cap` (a whole ellipse from 3.14), grey with a 2-pixel `$80FFFF` pen, the pitch's ellipse green;
/// the pitch a `Rectangle` (0.37 W, 0.37 H)-(0.63 W, 0.63 H) with a 3-pixel white pen, then with 2 pixels the centre circle of
/// radius `round(0.02 W)` around (W div 2, H div 2), the halfway line from (0.5 W, 0.37 H) to (0.5 W, 0.63 H - 2) and the penalty
/// areas (0.37 W, 0.45 H)-(0.41 W, 0.55 H) and (0.59 W, 0.45 H)-(0.63 W, 0.55 H). Drawn with these exact boxes, the port's
/// outline rasteriser (not Windows 3.1's) is further from the capture than the fitted values below, so they stay.
pub(super) fn paintbox(fb: &mut Fb, cx: &Ctx, n: &Node, x: i32, y: i32) {
    if !n.name.to_lowercase().contains("stadium") {
        return;
    }
    let (w, h) = (n.int("Width", 332), n.int("Height", 241));
    let level = get_num(cx, n, "level").unwrap_or(0).clamp(0, 24);
    let stands = [(3, level - 14, 10), (2, level - 6, 8), (1, level, 6)];
    let (ccx, ccy) = (w as f64 / 2.0, h as f64 / 2.0);
    let (ox, oy) = (66.0, 47.8);
    let (dx_ring, dy_ring) = (27.7, 20.6);
    let grey = Rgb(128, 128, 128);
    let yellow = Rgb(255, 255, 128);
    for (k, n_, cap) in stands {
        if n_ <= 0 {
            continue;
        }
        let f = (n_.min(cap)) as f64 / cap as f64;
        let (rx, ry) = (ox + k as f64 * dx_ring, oy + k as f64 * dy_ring);
        ellipse_part(fb, x, y, w, h, (ccx, ccy), (rx, ry), f, Some(grey), yellow);
    }
    ellipse_part(
        fb,
        x,
        y,
        w,
        h,
        (ccx, ccy),
        (ox, oy),
        1.0,
        Some(Rgb(0, 128, 0)),
        yellow,
    );
    // the pitch: white lines, 3 pixels around, 2 pixels inside
    let (px, py) = (x + ccx as i32 - 44, y + ccy as i32 - 32);
    let line =
        |fb: &mut Fb, a: i32, b: i32, w_: i32, h_: i32| fb.fill(px + a, py + b, w_, h_, WHITE);
    line(fb, 0, 0, 88, 3);
    line(fb, 0, 62, 88, 3);
    line(fb, 0, 0, 3, 65);
    line(fb, 85, 0, 3, 65);
    line(fb, 43, 3, 2, 59);
    line(fb, 0, 19, 14, 2);
    line(fb, 0, 43, 14, 2);
    line(fb, 12, 19, 2, 26);
    line(fb, 73, 19, 15, 2);
    line(fb, 73, 43, 15, 2);
    line(fb, 73, 19, 2, 26);
    // centre circle (15 pixels across, 2-pixel line)
    const CIRCLE: [&str; 15] = [
        "....#####....",
        "..###...###..",
        ".##.......##.",
        ".#.........#.",
        "##.........##",
        "#...........#",
        "#...........#",
        "#...........#",
        "#...........#",
        "#...........#",
        "##.........##",
        ".#.........#.",
        ".##.......##.",
        "..###...###..",
        "....#####....",
    ];
    glyph(fb, px + 37, py + 24, &CIRCLE, WHITE);
}

/// The left cut line of the stands ends this many pixels further out than the right one (measured on the original's picture).
const LEFT_TIP: f64 = 1.7;

/// Reach of the 2-pixel outline of the stadium picture from the geometric outline.
const PEN: f64 = 1.1;

/// A filled ellipse (centre and half axes in the paint box's coordinates) or the part of it above the radial cut lines that span the
/// fraction `f` of the circle around the top, with a 2-pixel outline: GDI's `Ellipse` and `Pie` with a wide pen.
#[allow(clippy::too_many_arguments)]
fn ellipse_part(
    fb: &mut Fb,
    ox: i32,
    oy: i32,
    w: i32,
    h: i32,
    (cx, cy): (f64, f64),
    (rx, ry): (f64, f64),
    f: f64,
    fill: Option<Rgb>,
    pen: Rgb,
) {
    let half = std::f64::consts::PI * f.min(1.0);
    // the cut lines run from the centre to the ellipse points at +-`half` from the top (parametric angle)
    let (ex, ey) = (rx * half.sin(), -ry * half.cos());
    let phi = ex.atan2(-ey);
    let full = f >= 1.0;
    for py in 0..h {
        for px in 0..w {
            let (dx, dy) = (px as f64 + 0.5 - cx, py as f64 + 0.5 - cy);
            let rho2 = (dx / rx).powi(2) + (dy / ry).powi(2);
            let grad = 2.0 * ((dx / (rx * rx)).powi(2) + (dy / (ry * ry)).powi(2)).sqrt();
            let d_arc = if grad > 0.0 {
                (rho2 - 1.0).abs() / grad
            } else {
                f64::MAX
            };
            let theta = dx.abs().atan2(-dy);
            let in_sector = full || theta <= phi;
            let mut d = f64::MAX;
            if in_sector || (full && d_arc < 1.0) {
                d = d_arc;
            }
            if !full {
                // distance to the two rays (segments from the centre to the ellipse point)
                let seg = |sx: f64| {
                    let (vx, vy) = (sx, ey);
                    let len2 = vx * vx + vy * vy;
                    let t = ((dx * vx + dy * vy) / len2).clamp(0.0, 1.0);
                    ((dx - t * vx).powi(2) + (dy - t * vy).powi(2)).sqrt()
                };
                d = d.min(seg(ex)).min(seg(-ex - LEFT_TIP));
                // the arc counts only where it is part of the outline
                if !in_sector && d_arc < 1.0 && theta - phi > 0.0 && d > d_arc {
                    d = f64::MAX.min(d);
                }
            }
            let inside = rho2 <= 1.0 && in_sector;
            if d < PEN && (rho2 <= 1.0 || d_arc < PEN) {
                fb.set(ox + px, oy + py, pen);
            } else if inside {
                if let Some(c) = fill {
                    fb.set(ox + px, oy + py, c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::win31::skin::Skin;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// A skin with one tiny font, enough to build a drawing context.
    fn skin() -> Skin {
        let mut b = b"EFSK".to_vec();
        b.extend_from_slice(&1u16.to_le_bytes());
        b.push(1);
        b.push(b'x');
        b.extend_from_slice(&[10, 0, 0, 8, 2, 32, 1, 1, 10]);
        for _ in 32..256 {
            b.extend_from_slice(&[3, 0, 8, 0, 0]);
        }
        let mut g = b"EFGL".to_vec();
        g.extend_from_slice(&0u16.to_le_bytes());
        Skin::from_bytes(&b, &g).unwrap()
    }

    fn with_ctx<R>(f: impl FnOnce(&Ctx) -> R) -> R {
        let skin = skin();
        let (texts, lists) = (BTreeMap::new(), BTreeMap::new());
        let hits = RefCell::new(Vec::new());
        let cx = Ctx {
            skin: &skin,
            form: "T",
            texts: &texts,
            ox: 0,
            oy: 0,
            bg: SILVER,
            focus: "",
            lists: &lists,
            hits: &hits,
        };
        f(&cx)
    }

    #[test]
    fn a_checked_box_has_a_cross() {
        let mut fb = Fb::new();
        check_square(&mut fb, 10, 10, true, BLACK);
        assert_eq!(fb.get(10, 10), 0x000000); // frame
        assert_eq!(fb.get(11, 11), 0x000000); // cross starts in the corner of the inside
        assert_eq!(fb.get(16, 16), 0x000000); // centre
        assert_eq!(fb.get(12, 11), 0xc0c0c0);
        check_square(&mut fb, 40, 10, false, BLACK);
        assert_eq!(fb.get(46, 16), 0xc0c0c0);
    }

    #[test]
    fn the_radio_dot_is_drawn_only_when_selected() {
        let mut fb = Fb::new();
        radio_ring(&mut fb, 0, 0, false, BLACK);
        assert_eq!(fb.get(6, 6), 0);
        radio_ring(&mut fb, 20, 0, true, BLACK);
        assert_eq!(fb.get(26, 6), 0x000000);
        assert_eq!(fb.get(26, 0), 0x000000); // ring top
    }

    #[test]
    fn the_scroll_thumb_follows_the_position() {
        // 497 rows high like the ranking list: thumb box top between 15 and h - 32 below the bar's first row
        let tops = |top: usize| {
            with_ctx(|cx| {
                let mut fb = Fb::new();
                vscroll(&mut fb, cx, "L", 100, 41, 497, top, 54, 31);
                (0..497)
                    .map(|r| 41 + r)
                    .filter(|&y| fb.get(102, y) == 0 && fb.get(114, y) == 0)
                    .collect::<Vec<_>>()
            })
        };
        // black lines across: below the up button, thumb top and bottom, above the down button
        assert_eq!(tops(0), vec![56, 72, 522]);
        assert_eq!(tops(23), vec![56, 506, 522]);
        assert_eq!(tops(10), vec![56, 250, 266, 522]);
    }

    #[test]
    fn scroll_bars_tell_how_many_rows_fit() {
        with_ctx(|cx| {
            let mut fb = Fb::new();
            vscroll(&mut fb, cx, "L", 0, 0, 100, 0, 40, 6);
            let hits = cx.hits.borrow();
            let info = hits.iter().find(|h| h.class == "ScrollInfo").unwrap();
            assert_eq!((info.name.as_str(), info.w, info.h), ("L#info", 6, 40));
            assert!(hits
                .iter()
                .any(|h| h.class == "ScrollUp" && h.name == "L#up"));
            assert!(hits
                .iter()
                .any(|h| h.class == "ScrollPage" && h.name == "L#pgdn"));
        });
    }

    #[test]
    fn the_selected_tab_is_open_to_the_page() {
        let mut fb = Fb::new();
        fb.fill(0, 0, 400, 100, SILVER);
        // page frame top at y = 30; tabs [2, 128] and [128, 254]; the first is selected
        fb.fill(2, 30, 300, 1, BLACK);
        tab_outline(&mut fb, 2, 128, 0, 30, true);
        tab_outline(&mut fb, 128, 254, 0, 30, false);
        assert_eq!(fb.get(60, 4), 0x000000); // top line
        assert_eq!(fb.get(60, 30), 0xc0c0c0); // open under the selected tab
        assert_eq!(fb.get(200, 30), 0x000000); // closed under the other
        assert_eq!(fb.get(128, 20), 0x000000); // shared black edge
        assert_eq!(fb.get(126, 20), 0x808080); // grey bevel of the selected tab
    }

    #[test]
    fn the_focus_rectangle_inverts_what_is_below() {
        let mut fb = Fb::new();
        fb.fill(0, 0, 40, 40, NAVY);
        focus_rect_not(&mut fb, 5, 5, 20, 10, 0, 0, true);
        assert_eq!(fb.get(10, 5), 0xf8fc78);
        assert_eq!(fb.get(5, 5), 0x000080); // corner untouched
    }
}
