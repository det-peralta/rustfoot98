//! The owner-drawn lists of the information dialogs (`TPrintListDlg`, `TClassificationDlg`, `TCoachManagerDlg`): every row is drawn by
//! the original's routine `1068:0ad6` from a string of *fields*, so rows are given here in the same encoding.
//!
//! A row is a sequence of fields, each `[modifier] width text TAB`:
//! * the optional **modifier** byte: `2`/`4` bold on/off, `3`/`5` italic on/off, `6` right-aligned field and ten more pixels after this
//!   and the fields that follow, `7` right-aligned field with no extra space from here on, `0xFC` blue text; one modifier per field;
//! * the **width** byte: the pixels the field advances (also the box a right-aligned text ends in);
//! * the text up to the TAB. A text starting with `0xFF` is the "starter" bullet, `0xFE` the dash of a substitute and `0xFD` a rule.
//!
//! The row starts at the left edge of the list plus two pixels; the text is drawn with its cell top at the top of the row. A row may
//! begin with `\u{1}fg;bg\u{1}` (each `r,g,b` or empty) for the colours the item gives itself (the club colours of a standings row,
//! the red and blue of the calendar); the selected row is drawn inverted (the original swaps the list's text and background colours).
//! Texts of the list in the dialog's `texts`: `<name>.top` (first visible row), `<name>.sel` (selected row), `<name>.focus` (the row
//! with the dotted focus rectangle). A list that does not fit gets a vertical scroll bar.
use super::draw::{focus_rect, text, Ctx, FontSpec, Hit};
use super::fb::{Fb, Rgb, BLACK, GRAY, SILVER, WHITE};
use super::form::Node;

/// What a field of a row shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Bullet,
    Dash,
    Rule,
}

/// A field placed in a row, the `x` offset counted from the row's left edge plus the margin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub x: i32,
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub blue: bool,
    pub kind: Kind,
}

/// The left margin of a row (`local_122` of `1068:0ad6`).
pub const MARGIN: i32 = 2;

/// The colours a row gives itself and the field part of the row.
pub fn split_colors(row: &str) -> (Option<Rgb>, Option<Rgb>, &str) {
    let Some(rest) = row.strip_prefix('\u{1}') else {
        return (None, None, row);
    };
    let Some((head, fields)) = rest.split_once('\u{1}') else {
        return (None, None, row);
    };
    let col = |t: &str| {
        let v: Vec<u8> = t.split(',').filter_map(|p| p.parse().ok()).collect();
        (v.len() == 3).then(|| Rgb(v[0], v[1], v[2]))
    };
    let (a, b) = head.split_once(';').unwrap_or((head, ""));
    (col(a), col(b), fields)
}

/// Encodes the colours of a row (the inverse of [`split_colors`]).
pub fn with_colors(fg: Option<Rgb>, bg: Option<Rgb>, fields: &str) -> String {
    if fg.is_none() && bg.is_none() {
        return fields.to_string();
    }
    let c = |c: Option<Rgb>| c.map_or(String::new(), |c| format!("{},{},{}", c.0, c.1, c.2));
    format!("\u{1}{};{}\u{1}{fields}", c(fg), c(bg))
}

/// `1068:08e0` and the placement of `1068:0ad6`: the fields of a row with their x offsets. `width_of(text, bold)` measures a text.
pub fn layout(fields: &str, width_of: &dyn Fn(&str, bool) -> i32) -> Vec<Placed> {
    let c: Vec<char> = fields.chars().collect();
    let (mut pos, mut x) = (0usize, 0i32);
    let (mut bold, mut italic, mut blue, mut spacing) = (false, false, false, 0i32);
    let mut out = Vec::new();
    while pos < c.len() {
        let mut right = false;
        let modifier = matches!(c[pos] as u32, 2..=7 | 0xFC);
        if modifier {
            match c[pos] as u32 {
                2 => bold = true,
                3 => italic = true,
                4 => bold = false,
                5 => italic = false,
                6 => {
                    right = true;
                    spacing = 10;
                }
                7 => {
                    right = true;
                    spacing = 0;
                }
                _ => blue = true,
            }
            pos += 1;
        }
        let Some(&w) = c.get(pos) else { break };
        let width = w as i32;
        pos += 1;
        let mut t = String::new();
        while pos < c.len() && c[pos] != '\t' {
            t.push(c[pos]);
            pos += 1;
        }
        pos += 1;
        let kind = match t.chars().next().map(|ch| ch as u32) {
            Some(0xFF) => Kind::Bullet,
            Some(0xFE) => Kind::Dash,
            Some(0xFD) => Kind::Rule,
            _ => Kind::Text,
        };
        let fx = if right && kind == Kind::Text {
            x + width - width_of(&t, bold)
        } else {
            x
        };
        out.push(Placed {
            x: fx,
            text: t,
            bold,
            italic,
            blue,
            kind,
        });
        x += width + spacing;
    }
    out
}

/// Width of a vertical scroll bar.
pub const BAR: i32 = 16;

/// The scroll bar of a list: track, two arrow buttons and the thumb. Returns the hits `(name suffix, x, y, w, h)` of arrow up, arrow
/// down, page up and page down.
pub fn vscroll(
    fb: &mut Fb,
    x: i32,
    y: i32,
    h: i32,
    top: usize,
    count: usize,
    visible: usize,
) -> Vec<(&'static str, i32, i32, i32, i32)> {
    // the dithered track
    for yy in 0..h {
        for xx in 0..BAR {
            fb.set(
                x + xx,
                y + yy,
                if (xx + yy) % 2 == 0 { WHITE } else { SILVER },
            );
        }
    }
    let button = |fb: &mut Fb, bx: i32, by: i32| {
        fb.fill(bx, by, BAR, BAR, BLACK);
        fb.fill(bx + 1, by + 1, BAR - 2, BAR - 2, SILVER);
        fb.fill(bx + 1, by + 1, BAR - 3, 1, WHITE);
        fb.fill(bx + 1, by + 1, 1, BAR - 3, WHITE);
        fb.fill(bx + BAR - 3, by + 2, 1, BAR - 4, GRAY);
        fb.fill(bx + 2, by + BAR - 3, BAR - 4, 1, GRAY);
    };
    button(fb, x, y);
    button(fb, x, y + h - BAR);
    for k in 0..4 {
        // arrow up (4 rows) and down
        fb.fill(x + 7 - k, y + 5 + k, 2 * k + 2, 1, BLACK);
        fb.fill(x + 7 - k, y + h - BAR + 10 - k, 2 * k + 2, 1, BLACK);
    }
    let track = h - 2 * BAR;
    let thumb_h = if count == 0 {
        track
    } else {
        ((track as i64 * visible.min(count) as i64) / count as i64).max(BAR as i64) as i32
    };
    let range = count.saturating_sub(visible).max(1);
    let thumb_y =
        y + BAR + ((track - thumb_h) as i64 * top.min(range) as i64 / range as i64) as i32;
    button(fb, x, thumb_y);
    if thumb_h > BAR {
        fb.fill(x, thumb_y + BAR, BAR, thumb_h - BAR, BLACK);
        fb.fill(x + 1, thumb_y + BAR - 1, BAR - 2, thumb_h - BAR, SILVER);
        fb.fill(x + 1, thumb_y + 1, 1, thumb_h - 2, WHITE);
        fb.fill(x + BAR - 3, thumb_y + 2, 1, thumb_h - 3, GRAY);
        fb.fill(x + 2, thumb_y + thumb_h - 3, BAR - 4, 1, GRAY);
        fb.fill(x + 1, thumb_y + thumb_h - 1, BAR - 2, 1, BLACK);
    }
    vec![
        ("up", x, y, BAR, BAR),
        ("down", x, y + h - BAR, BAR, BAR),
        ("pgup", x, y + BAR, BAR, (thumb_y - y - BAR).max(0)),
        (
            "pgdn",
            x,
            thumb_y + thumb_h,
            BAR,
            (y + h - BAR - thumb_y - thumb_h).max(0),
        ),
    ]
}

/// Rows the list shows at once (a partly visible row counts).
pub fn visible_rows(client_h: i32, row_h: i32) -> usize {
    ((client_h + row_h - 1) / row_h).max(1) as usize
}

/// Draws the list `n` at `(x, y)` (the outer rectangle of the box, 1 pixel frame) with `rows` in the field encoding.
#[allow(clippy::too_many_arguments)]
pub fn draw_list(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    list_bg: Rgb,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rows: &[String],
) {
    let Some(font) = cx.font(f) else { return };
    let bold_font = cx.font(&FontSpec {
        bold: true,
        ..f.clone()
    });
    let italic_font = cx.font(&FontSpec {
        italic: true,
        ..f.clone()
    });
    let rh = n.int("ItemHeight", font.height());
    let (cx0, cy0, cw, ch) = (x + 1, y + 1, w - 2, h - 2);
    let get = |k: &str| {
        cx.texts
            .get(&format!("{}.{k}", n.name))
            .and_then(|v| v.parse::<usize>().ok())
    };
    let visible = visible_rows(ch, rh);
    let bar = rows.len() as i32 * rh > ch;
    let rw = if bar { cw - BAR } else { cw };
    let top = get("top")
        .unwrap_or(0)
        .min(rows.len().saturating_sub(ch.max(1) as usize / rh as usize));
    let (sel, focus) = (get("sel"), get("focus"));
    let old = fb.clip;
    fb.clip = (cx0, cy0, cx0 + rw, cy0 + ch);
    for (k, row) in rows.iter().enumerate().skip(top).take(visible + 1) {
        let ty = cy0 + (k - top) as i32 * rh;
        let (fg, bg, fields) = split_colors(row);
        let (mut ink, mut paper) = (f.color, list_bg);
        if sel == Some(k) {
            (ink, paper) = (list_bg, f.color);
        }
        if let Some(c) = fg {
            ink = c;
        }
        if let Some(c) = bg {
            paper = c;
        }
        fb.fill(cx0, ty, rw, rh, paper);
        let width_of = |t: &str, bold: bool| match (bold, bold_font) {
            (true, Some(b)) => b.width(t),
            _ => font.width(t),
        };
        for p in layout(fields, &width_of) {
            let fnt = match (p.bold, p.italic) {
                (true, _) => bold_font.unwrap_or(font),
                (false, true) => italic_font.unwrap_or(font),
                _ => font,
            };
            let color = if p.blue { Rgb(0, 0, 255) } else { ink };
            let px = cx0 + MARGIN + p.x;
            match p.kind {
                Kind::Text => {
                    text(fb, fnt, px, ty, &p.text, color, false);
                }
                Kind::Bullet => fb.fill(px + 4, ty + 6, 3, 3, color),
                Kind::Dash => fb.fill(px + 3, ty + 7, 5, 1, color),
                Kind::Rule => fb.fill(cx0, ty, rw, 1, color),
            }
        }
        if focus == Some(k) {
            focus_rect(fb, cx0, ty, rw, rh, cx0, cy0);
        }
        cx.hits.borrow_mut().push(Hit {
            name: format!("{}#{k}", n.name),
            class: "ListRow".into(),
            x: cx0,
            y: ty,
            w: rw,
            h: rh.min(cy0 + ch - ty),
            enabled: true,
        });
    }
    fb.clip = old;
    if bar {
        let hits = vscroll(fb, cx0 + cw - BAR, cy0, ch, top, rows.len(), visible);
        for (k, bx, by, bw, bh) in hits {
            if bw > 0 && bh > 0 {
                cx.hits.borrow_mut().push(Hit {
                    name: format!("{}.{k}", n.name),
                    class: "TSpeedButton".into(),
                    x: bx,
                    y: by,
                    w: bw,
                    h: bh,
                    enabled: true,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono(t: &str, _bold: bool) -> i32 {
        6 * t.chars().count() as i32
    }

    #[test]
    fn calendar_style_row_with_right_aligned_numbers() {
        // round (right, +10 spacing from now on), opponent, a goal count (right, no spacing), colon
        let row = "\u{6}\u{14}3\t\u{78}Santos\t\u{7}\u{f}12\t\u{7}\u{a}:\t\u{7}\u{f}0\t";
        let p = layout(row, &mono);
        assert_eq!(p.len(), 5);
        // right aligned: x = 0 + 20 - width("3")
        assert_eq!((p[0].x, p[0].text.as_str()), (14, "3"));
        // 20 + 10 spacing, then the opponent left aligned
        assert_eq!((p[1].x, p[1].text.as_str()), (30, "Santos"));
        // the spacing continues after the left field (30 + 120 + 10), goal count right aligned in 15
        assert_eq!(p[2].x, 160 + 15 - 12);
        // mode 7 stops the extra spacing
        assert_eq!(p[3].x, 160 + 15 + 10 - 6);
        assert_eq!(p[4].x, 160 + 15 + 10 + 15 - 6);
    }

    #[test]
    fn bold_and_blue_persist_until_changed() {
        let row = "\u{2}\u{14}a\t\u{14}b\t\u{4}\u{14}c\t\u{fc}\u{14}d\t";
        let p = layout(row, &mono);
        assert_eq!(
            p.iter().map(|f| (f.bold, f.blue)).collect::<Vec<_>>(),
            [(true, false), (true, false), (false, false), (false, true)]
        );
    }

    #[test]
    fn special_texts_and_colour_prefix() {
        let p = layout("\u{14}\u{ff}\t\u{14}\u{fe}\t\u{0}\u{fd}\t", &mono);
        assert_eq!(
            p.iter().map(|f| f.kind.clone()).collect::<Vec<_>>(),
            [Kind::Bullet, Kind::Dash, Kind::Rule]
        );
        let s = with_colors(Some(Rgb(1, 2, 3)), None, "x");
        assert_eq!(split_colors(&s), (Some(Rgb(1, 2, 3)), None, "x"));
        assert_eq!(split_colors("plain"), (None, None, "plain"));
    }

    #[test]
    fn the_rows_that_fit() {
        assert_eq!(visible_rows(225, 16), 15);
        assert_eq!(visible_rows(224, 16), 14);
    }
}
