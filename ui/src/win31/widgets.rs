//! The controls of the Windows 3.1 / Delphi 1 look that go beyond the basic ones of `draw.rs`: check boxes, radio groups, combo boxes
//! (closed and with the drop-down list), spin edits, notebooks with tabs, colour grids, speed buttons, scroll bars and the lists with
//! columns. The text keys that drive them are documented in `docs/classic-dialogs.md`.
use super::draw::{
    bevel_miter, caption, control, focus_rect, is_enabled, sunken_box, text, text_dithered, Ctx,
    FontSpec, Hit, DITHER_PARITY,
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
    fb.fill(x, y, 13, 13, color);
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
        None => n.flag("Checked", false),
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
    let range = count.saturating_sub(fit);
    let first = y0 + 15;
    let last = y0 + h - 16 - 16;
    let ty = if range == 0 {
        first
    } else {
        first
            + ((last - first) as i64 * top.min(range) as i64 * 2 + range as i64) as i32
                / (2 * range as i32)
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
    let inner_h = h - 2;
    let fit = (inner_h / ih).max(1) as usize;
    let scrolled = items.len() > fit;
    let right = if scrolled { x + w - 17 } else { x + w - 1 };
    let top = view(cx, n, items.len(), fit);
    let sel = get_num(cx, n, "sel")
        .filter(|&v| v >= 0)
        .map(|v| v as usize);
    let focused = cx.focus == n.name;
    let cols = columns(cx, n);
    let enabled = is_enabled(cx, n);
    let old = fb.clip;
    fb.clip = (
        (x + 1).max(old.0),
        (y + 1).max(old.1),
        right.min(old.2),
        (y + h - 1).min(old.3),
    );
    for (r, i) in (top..items.len().min(top + fit + 1)).enumerate() {
        let ry = y + 1 + r as i32 * ih;
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
        let rf = if st.bold {
            cx.font(&FontSpec {
                bold: true,
                ..f.clone()
            })
            .unwrap_or(font)
        } else {
            font
        };
        let ty = ry + (ih - rf.height()).max(0) / 2;
        match &cols {
            Some(cols) => {
                for (k, cell) in items[i].split('|').enumerate() {
                    let Some(&(off, al)) = cols.get(k) else {
                        break;
                    };
                    let tw = rf.width(cell);
                    let tx = match al {
                        'r' => x + off - tw,
                        'c' => x + off - tw / 2,
                        _ => x + off,
                    };
                    text(fb, rf, tx, ty, cell, fg, false);
                }
            }
            None => {
                text(fb, rf, x + 2, ty, &items[i], fg, false);
            }
        }
        if selected && focused {
            focus_rect_not(fb, x + 1, ry, right - x - 1, ih, x, y, false);
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
    // Windows 3.1 closes a combo box to the height of its text field plus 7 pixels, whatever the form says
    let h = font.height().max(n.int("ItemHeight", font.height())) + 7;
    let enabled = is_enabled(cx, n);
    sunken_box(fb, x, y, w, h, WHITE);
    fb.fill(x + w - 17, y + 1, 1, h - 2, BLACK);
    flat_button(fb, x + w - 16, y + 1, 15, h - 2);
    glyph(
        fb,
        x + w - 12,
        y + 6,
        &ARROW_COMBO,
        if enabled { BLACK } else { GRAY },
    );
    let ty = y + 1 + (h - 2 - font.height()) / 2;
    let focused = cx.focus == n.name && enabled && !flag(cx, n, "open");
    let list_style = n.ident("Style") != Some("csDropDown");
    let old = fb.clip;
    fb.clip = (x + 1, y + 1, x + w - 17, y + h - 1);
    if focused && list_style {
        let hw = w - 19 - 2;
        fb.fill(x + 2, y + 2, hw + 1, h - 4, NAVY);
        text(fb, font, x + 3, ty, &shown, WHITE, false);
        focus_rect_not(fb, x + 2, y + 2, hw + 1, h - 4, x, y, true);
    } else if enabled {
        text(fb, font, x + 3, ty, &shown, f.color, false);
    } else {
        text(fb, font, x + 3, ty, &shown, GRAY, false);
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

thread_local! {
    /// Open combo boxes of the frame being drawn: name, position, size and font.
    static OVERLAYS: std::cell::RefCell<Vec<(String, i32, i32, i32, i32, FontSpec)>> =
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
    let rows = items.len().min(n.int("DropDownCount", 8).max(1) as usize);
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
        if hot {
            fb.fill(x + 1, ry, right - x - 1, ih, NAVY);
        }
        text(
            fb,
            font,
            x + 3,
            ry + (ih - font.height()) / 2,
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
/// small arrow buttons.
pub(super) fn spinedit(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 60), n.int("Height", 24));
    let bg = n.color("Color").unwrap_or(WHITE);
    sunken_box(fb, x, y, w, h, bg);
    let bh = (h - 2) / 2;
    let bx = x + w - 17;
    flat_button(fb, bx, y + 1, 16, bh);
    flat_button(fb, bx, y + 1 + bh, 16, h - 2 - bh);
    glyph(
        fb,
        bx + 5,
        y + 1 + (bh - 4) / 2,
        &["...#...", "..###..", ".#####."],
        BLACK,
    );
    glyph(
        fb,
        bx + 5,
        y + 1 + bh + (h - 2 - bh - 3) / 2,
        &[".#####.", "..###..", "...#..."],
        BLACK,
    );
    if let Some(font) = cx.font(f) {
        let s = match get(cx, n, "value") {
            Some(v) => v.to_string(),
            None => caption_text(cx, n),
        };
        let old = fb.clip;
        fb.clip = (x + 1, y + 1, bx, y + h - 1);
        text(
            fb,
            font,
            x + 2,
            y + (h - font.height()) / 2,
            &s,
            f.color,
            false,
        );
        fb.clip = old;
    }
    push_hit(cx, format!("{}#up", n.name), "SpinUp", bx, y + 1, 16, bh);
    push_hit(
        cx,
        format!("{}#down", n.name),
        "SpinDown",
        bx,
        y + 1 + bh,
        16,
        h - 2 - bh,
    );
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
    fb.fill(x, y, w, h, SILVER);
    if down {
        fb.fill(x, y, w, h, BLACK);
        fb.fill(x + 1, y + 1, w - 1, h - 1, GRAY);
        fb.fill(x + 2, y + 2, w - 3, h - 3, SILVER);
    } else {
        fb.fill(x, y, w, h, BLACK);
        fb.fill(x + 1, y + 1, w - 2, h - 2, SILVER);
        bevel_miter(fb, x + 1, y + 1, x + w - 2, y + h - 2, 2, WHITE, GRAY);
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
        text(
            fb,
            font,
            x + (w - tw) / 2 + shift,
            ty,
            &cap,
            if enabled { f.color } else { GRAY },
            false,
        );
    }
}

/// `TColorGrid`: 16 colour cells (4 x 4) with the foreground (`F`) and background (`B`) marks; `<name>.index` is the foreground,
/// `<name>.bgindex` the background colour.
pub(super) fn colorgrid(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    const COLORS: [Rgb; 16] = [
        Rgb(0, 0, 0),
        Rgb(128, 0, 0),
        Rgb(0, 128, 0),
        Rgb(128, 128, 0),
        Rgb(0, 0, 128),
        Rgb(128, 0, 128),
        Rgb(0, 128, 128),
        Rgb(128, 128, 128),
        Rgb(192, 192, 192),
        Rgb(255, 0, 0),
        Rgb(0, 255, 0),
        Rgb(255, 255, 0),
        Rgb(0, 0, 255),
        Rgb(255, 0, 255),
        Rgb(0, 255, 255),
        Rgb(255, 255, 255),
    ];
    let (w, h) = (n.int("Width", 100), n.int("Height", 100));
    let (cw, ch) = (w / 4, h / 4);
    let fg = get_num(cx, n, "index").unwrap_or(0);
    let bg = get_num(cx, n, "bgindex").unwrap_or(15);
    fb.fill(x, y, w, h, BLACK);
    for (i, c) in COLORS.iter().enumerate() {
        let (gx, gy) = (x + (i as i32 % 4) * cw, y + (i as i32 / 4) * ch);
        fb.fill(gx + 1, gy + 1, cw - 1, ch - 1, *c);
        let mark = if i as i32 == fg {
            Some("F")
        } else if i as i32 == bg {
            Some("B")
        } else {
            None
        };
        if let (Some(m), Some(font)) = (mark, cx.font(f)) {
            let ink = if c.0 as u32 + c.1 as u32 + c.2 as u32 > 300 {
                BLACK
            } else {
                WHITE
            };
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
    if cx.focus == n.name {
        let (gx, gy) = (x + (fg % 4) * cw, y + (fg / 4) * ch);
        focus_rect(fb, gx + 2, gy + 2, cw - 3, ch - 3, x, y);
    }
}
