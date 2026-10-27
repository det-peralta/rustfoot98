//! Drawing primitives and the controls, following what Windows 3.1 and Delphi 1 drew.
use super::fb::{Fb, Rgb, BLACK, GRAY, GREEN, NAVY, SILVER, WHITE};
use super::form::Node;
use super::json::Json;
use super::skin::{ansi, Font, Skin};
use super::widgets;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// The effective font of a control: Delphi inherits the parent's font unless the control sets its own.
#[derive(Clone, Debug)]
pub struct FontSpec {
    pub name: String,
    pub px: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Rgb,
}

impl Default for FontSpec {
    fn default() -> Self {
        FontSpec {
            name: "ms sans serif".into(),
            px: 11,
            bold: false,
            italic: false,
            underline: false,
            color: BLACK,
        }
    }
}

impl FontSpec {
    pub fn inherit(&self, n: &Node) -> FontSpec {
        let mut f = self.clone();
        if let Some(s) = n.str("Font.Name") {
            f.name = s.to_lowercase();
        }
        if n.props.contains_key("Font.Height") {
            f.px = -n.int("Font.Height", -11);
        }
        if n.props.contains_key("Font.Style") {
            let st = n.set("Font.Style");
            f.bold = st.contains(&"fsBold");
            f.italic = st.contains(&"fsItalic");
            f.underline = st.contains(&"fsUnderline");
        }
        if let Some(c) = n.color("Font.Color") {
            f.color = c;
        }
        f
    }
}

/// A control that was drawn: the screen rectangle the mouse can hit.
#[derive(Clone, Debug)]
pub struct Hit {
    pub name: String,
    pub class: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub enabled: bool,
}

impl Hit {
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Everything a control needs to draw itself.
pub struct Ctx<'a> {
    pub skin: &'a Skin,
    pub form: &'a str,
    /// Captions set by the program at run time, by control name.
    pub texts: &'a BTreeMap<String, String>,
    /// Origin of the form's client area on the screen.
    pub ox: i32,
    pub oy: i32,
    pub bg: Rgb,
    /// Name of the control that has the keyboard focus (the form's `ActiveControl`).
    pub focus: &'a str,
    /// Contents of list boxes set by the program, by control name.
    pub lists: &'a BTreeMap<String, Vec<String>>,
    /// Every control drawn is recorded here (in drawing order), for hit testing.
    pub hits: &'a RefCell<Vec<Hit>>,
}

impl<'a> Ctx<'a> {
    /// A copy of the context (for drawing the children of a container).
    pub fn sub(&self) -> Ctx<'a> {
        Ctx {
            skin: self.skin,
            form: self.form,
            texts: self.texts,
            ox: self.ox,
            oy: self.oy,
            bg: self.bg,
            focus: self.focus,
            lists: self.lists,
            hits: self.hits,
        }
    }
}

impl Ctx<'_> {
    /// A colour set by the program for a control: `<name>.<key>=r,g,b` in the texts.
    pub fn color_of(&self, n: &Node, key: &str) -> Option<Rgb> {
        let v: Vec<u8> = self
            .texts
            .get(&format!("{}.{key}", n.name))?
            .split(',')
            .filter_map(|p| p.parse().ok())
            .collect();
        (v.len() == 3).then(|| Rgb(v[0], v[1], v[2]))
    }

    pub fn font(&self, f: &FontSpec) -> Option<&Font> {
        self.skin.font(&f.name, f.px, f.bold, f.italic)
    }
}

/// Draws `s` with the top-left corner of its cell at (x, y); returns the advance.
pub fn text(fb: &mut Fb, font: &Font, x: i32, y: i32, s: &str, color: Rgb, underline: bool) -> i32 {
    let base = y + font.ascent;
    let mut pen = x;
    for c in s.chars() {
        let Some(g) = font.glyph(ansi(c)) else {
            continue;
        };
        for (r, bits) in g.rows.iter().enumerate() {
            for k in 0..g.w {
                if bits >> (g.w - 1 - k) & 1 == 1 {
                    fb.set(pen + g.left + k, base - g.top + r as i32, color);
                }
            }
        }
        pen += g.adv;
    }
    if underline {
        fb.fill(x, base + font.ul_pos, pen - x, font.ul_thick, color);
    }
    pen - x
}

/// Disabled text of a button-class control (`GrayString`): the glyphs masked with a checkerboard.
pub fn text_dithered(fb: &mut Fb, font: &Font, x: i32, y: i32, s: &str, color: Rgb, parity: i32) {
    let base = y + font.ascent;
    let mut pen = x;
    for c in s.chars() {
        let Some(g) = font.glyph(ansi(c)) else {
            continue;
        };
        for (r, bits) in g.rows.iter().enumerate() {
            for k in 0..g.w {
                let (px, py) = (pen + g.left + k, base - g.top + r as i32);
                if bits >> (g.w - 1 - k) & 1 == 1 && (px + py).rem_euclid(2) == parity {
                    fb.set(px, py, color);
                }
            }
        }
        pen += g.adv;
    }
}

/// Splits `s` into lines no wider than `width` (spaces break lines), like `DrawText` with `DT_WORDBREAK`.
pub fn wrap(font: &Font, s: &str, width: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split(' ') {
            let trial = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if font.width(&trial) <= width || line.is_empty() {
                line = trial;
            } else {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            }
        }
        lines.push(line);
    }
    lines
}

thread_local! {
    /// Top-left corner of the next dialog frame when the form is not centred (`Position` poDesigned: the form's own Left/Top).
    pub static FRAME_ORIGIN: std::cell::Cell<Option<(i32, i32)>> = const { std::cell::Cell::new(None) };
}

thread_local! {
    /// Set while a window below the active one is drawn: its caption bar and frame are silver with black lettering.
    pub static INACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `(caption bar and thick frame, caption lettering)` colours of the window being drawn.
fn caption_colors() -> (Rgb, Rgb) {
    if INACTIVE.with(|i| i.get()) {
        (SILVER, BLACK)
    } else {
        (NAVY, WHITE)
    }
}

fn frame_origin(centre: (i32, i32)) -> (i32, i32) {
    FRAME_ORIGIN.with(|o| o.get()).unwrap_or(centre)
}

/// A form with a caption bar (`bsSingle`): a black frame, a navy caption bar 18 pixels high and a black rule below it.
/// Returns the origin of the client area.
pub fn dialog_frame(
    fb: &mut Fb,
    skin: &Skin,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
) -> (i32, i32) {
    let (ow, oh) = (cw + 2, ch + 21);
    let (x, y) = frame_origin(((super::fb::W - ow) / 2, (super::fb::H - oh) / 2));
    let (bar, ink) = caption_colors();
    fb.fill(x, y, ow, oh, BLACK);
    fb.fill(x + 1, y + 1, cw, 18, bar);
    fb.fill(x + 1, y + 20, cw, ch, bg);
    if let Some(f) = skin.font("system", 13, false, false) {
        let w = f.width(caption);
        text(fb, f, x + 1 + (cw - w) / 2, y + 2, caption, ink, false);
    }
    (x + 1, y + 20)
}

/// A `bsSingle` form with a menu bar: the caption bar, then an 18-pixel white bar with the top-level items in the bold System font.
/// Returns the origin of the client area.
pub fn dialog_frame_menu(
    fb: &mut Fb,
    skin: &Skin,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
    items: &[String],
) -> (i32, i32) {
    let (ow, oh) = (cw + 2, ch + 40);
    let (x, y) = ((super::fb::W - ow) / 2, (super::fb::H - oh) / 2);
    let (bar, ink) = caption_colors();
    fb.fill(x, y, ow, oh, BLACK);
    fb.fill(x + 1, y + 1, cw, 18, bar);
    fb.fill(x + 1, y + 20, cw, 18, WHITE);
    fb.fill(x + 1, y + 39, cw, ch, bg);
    if let Some(f) = skin.font("system", 13, false, false) {
        let w = f.width(caption);
        text(fb, f, x + 1 + (cw - w) / 2, y + 2, caption, ink, false);
    }
    if let Some(f) = skin.font("system", 13, false, false) {
        let mut pen = x + 1 + 8;
        for it in items {
            let label = it.replace('&', "");
            let w = f.width(&label);
            text(fb, f, pen, y + 20, &label, BLACK, false);
            pen += w + 16;
        }
    }
    (x + 1, y + 39)
}

/// A form with `bsDialog` (thick frame): black outline, 4 navy pixels, a caption bar with a white line on three sides and a black rule
/// below it. The client area starts 5 pixels in and 24 down; the frame hides 4 pixels of the declared client size on each axis.
pub fn dialog_frame_thick(
    fb: &mut Fb,
    skin: &Skin,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
) -> (i32, i32) {
    dialog_frame_thick_sys(fb, skin, caption, cw, ch, bg, false)
}

/// `dialog_frame_thick`, optionally with the system menu box (the 18-pixel box with a dash that a form shows when its `BorderIcons`
/// contain `biSystemMenu`, which is also the default) at the left of the caption; the caption is centred in the rest of the bar.
pub fn dialog_frame_thick_sys(
    fb: &mut Fb,
    skin: &Skin,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
    sys: bool,
) -> (i32, i32) {
    let (w, h) = (cw + 6, ch + 25);
    let (x, y) = frame_origin(((super::fb::W - w) / 2, (super::fb::H - h) / 2));
    let (bar, ink) = caption_colors();
    fb.fill(x, y, w, h, BLACK);
    fb.fill(x + 1, y + 1, w - 2, h - 2, bar);
    fb.fill(x + 5, y + 24, w - 10, h - 29, bg);
    fb.fill(x + 5, y + 4, w - 10, 1, WHITE);
    fb.fill(x + 5, y + 4, 1, 20, WHITE);
    fb.fill(x + w - 6, y + 4, 1, 20, WHITE);
    fb.fill(x + 6, y + 23, w - 12, 1, BLACK);
    let mut left = x + 6;
    if sys {
        let (bx, by) = (x + 6, y + 5);
        fb.fill(bx, by, 18, 18, SILVER);
        fb.fill(bx + 18, by, 1, 18, BLACK);
        fb.fill(bx + 2, by + 7, 13, 3, BLACK);
        fb.fill(bx + 3, by + 8, 11, 1, WHITE);
        fb.fill(bx + 15, by + 8, 1, 2, GRAY);
        fb.fill(bx + 3, by + 10, 13, 1, GRAY);
        left = bx + 19;
    }
    if let Some(f) = skin.font("system", 13, false, false) {
        let tw = f.width(caption);
        let right = x + w - 6;
        text(
            fb,
            f,
            left + (right - left - tw) / 2,
            y + 6,
            caption,
            ink,
            false,
        );
    }
    (x + 5, y + 24)
}

/// The frame a form gets: thick for `bsDialog` (with the system menu box unless `BorderIcons` leaves it out), plain otherwise.
pub fn form_frame(
    fb: &mut Fb,
    skin: &Skin,
    form: &Node,
    caption: &str,
    cw: i32,
    ch: i32,
    bg: Rgb,
) -> (i32, i32) {
    if form.ident("BorderStyle") == Some("bsDialog") {
        let sys = !form.props.contains_key("BorderIcons")
            || form.set("BorderIcons").contains(&"biSystemMenu");
        dialog_frame_thick_sys(fb, skin, caption, cw, ch, bg, sys)
    } else {
        dialog_frame(fb, skin, caption, cw, ch, bg)
    }
}

/// The client size to draw a form with. The thick `bsDialog` frame (see `dialog_frame_thick`) surrounds a client area 4 pixels larger
/// than it was designed for in each direction (measured on the options, profile, stadium and progress dialogs, the calendar and the
/// scorers list), so these forms are drawn with `ClientWidth + 4` by `ClientHeight + 4`. The exception is the new-game dialog,
/// which the program shrinks itself. A program that resizes a form passes `@client=WxH` (in the same units) instead.
pub fn form_client(form: &Node) -> (i32, i32) {
    let (mut cw, mut ch) = (form.int("ClientWidth", 400), form.int("ClientHeight", 300));
    if form.ident("BorderStyle") == Some("bsDialog")
        && !form.class.eq_ignore_ascii_case("TSelectGameDlg")
    {
        cw += 4;
        ch += 4;
    }
    (cw, ch)
}

/// The face of a `TBitBtn`/`TButton` in its normal state: a black outline with cut corners (2 pixels for the default
/// button, 1 for the others), a white and a grey bevel of 2 pixels.
pub fn button_face(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, default: bool) {
    let i = if default { 2 } else { 1 };
    fb.fill(x, y, w, h, BLACK);
    fb.fill(x + i + 2, y + i + 2, w - 2 * i - 4, h - 2 * i - 4, SILVER);
    fb.fill(x + i, y + i, w - 2 * i, 2, WHITE);
    fb.fill(x + i, y + i, 2, h - 2 * i, WHITE);
    fb.fill(x + w - i - 2, y + i, 2, h - 2 * i, GRAY);
    fb.fill(x + i, y + h - i - 2, w - 2 * i, 2, GRAY);
    // mitred corners of the bevel and the cut outline corners
    fb.set(x + w - i - 2, y + i, WHITE);
    fb.set(x + w - i - 1, y + i, GRAY);
    fb.set(x + w - i - 2, y + i + 1, GRAY);
    fb.set(x + i - 1, y + h - i - 1, GRAY);
    for (cx, cy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        fb.set(x + cx, y + cy, SILVER);
    }
}

thread_local! {
    static FOCUS_ON_BUTTON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Draws the controls of `root` (the form) in order.
pub fn controls(fb: &mut Fb, cx: &Ctx, root: &Node) {
    let base = FontSpec::default().inherit(root);
    // the default button loses its thick frame to the button that has the focus
    FOCUS_ON_BUTTON.with(|c| {
        c.set(
            root.find(cx.focus)
                .is_some_and(|n| matches!(n.class.as_str(), "TBitBtn" | "TButton")),
        )
    });
    for n in &root.children {
        control(fb, cx, n, &base);
    }
    widgets::draw_overlays(fb, cx, root);
}

pub(super) fn control(fb: &mut Fb, cx: &Ctx, n: &Node, parent: &FontSpec) {
    // the program resized the control at run time: `<name>.size=WxH`
    if let Some((w, h)) = cx
        .texts
        .get(&format!("{}.size", n.name))
        .and_then(|v| v.split_once('x'))
        .and_then(|(a, b)| Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?)))
    {
        let mut m = n.clone();
        m.props.insert("Width".into(), Json::Num(w));
        m.props.insert("Height".into(), Json::Num(h));
        return control_node(fb, cx, &m, parent);
    }
    control_node(fb, cx, n, parent)
}

fn control_node(fb: &mut Fb, cx: &Ctx, n: &Node, parent: &FontSpec) {
    let visible = match cx.texts.get(&format!("{}.visible", n.name)) {
        Some(v) => v != "0",
        None => n.flag("Visible", true),
    };
    if !visible {
        return;
    }
    let mut f = parent.inherit(n);
    if let Some(c) = cx.texts.get(&format!("{}.color", n.name)) {
        let v: Vec<u8> = c.split(',').filter_map(|p| p.parse().ok()).collect();
        if v.len() == 3 {
            f.color = Rgb(v[0], v[1], v[2]);
        }
    }
    let (mut left, mut top) = (n.int("Left", 0), n.int("Top", 0));
    if let Some(p) = cx.texts.get(&format!("{}.pos", n.name)) {
        // the program moved the control at run time
        let v: Vec<i32> = p.split(',').filter_map(|t| t.parse().ok()).collect();
        if v.len() == 2 {
            (left, top) = (v[0], v[1]);
        }
    }
    let (x, y) = (cx.ox + left, cx.oy + top);
    // the program resized the control at run time (`<name>.size=w,h`)
    let resized;
    let n = match cx.texts.get(&format!("{}.size", n.name)) {
        Some(sz) => {
            let v: Vec<f64> = sz.split(',').filter_map(|t| t.parse().ok()).collect();
            let mut m = n.clone();
            if v.len() == 2 {
                m.props.insert("Width".into(), super::json::Json::Num(v[0]));
                m.props
                    .insert("Height".into(), super::json::Json::Num(v[1]));
            }
            resized = m;
            &resized
        }
        None => n,
    };
    cx.hits.borrow_mut().push(Hit {
        name: n.name.clone(),
        class: n.class.clone(),
        x,
        y,
        w: n.int("Width", 0),
        h: n.int("Height", 0),
        enabled: is_enabled(cx, n),
    });
    match n.class.as_str() {
        "TLabel" => label(fb, cx, n, &f, x, y),
        "TBitBtn" | "TButton" => bitbtn(fb, cx, n, &f, x, y),
        "TMemo" => memo(fb, cx, n, &f, x, y),
        "TBanner" => banner(fb, cx, n, &f, x, y),
        "TBevel" => bevel(fb, n, x, y),
        "TPanel" => panel(fb, cx, n, &f, x, y),
        "TGroupBox" => groupbox(fb, cx, n, &f, x, y),
        "TEdit" => edit(fb, cx, n, &f, x, y),
        "TListBox" => listbox(fb, cx, n, &f, x, y),
        "TRadioButton" => widgets::radio(fb, cx, n, &f, x, y),
        "TCheckBox" => widgets::checkbox(fb, cx, n, &f, x, y),
        "TRadioGroup" => widgets::radiogroup(fb, cx, n, &f, x, y),
        "TComboBox" => widgets::combobox(fb, cx, n, &f, x, y),
        "TSpinEdit" => widgets::spinedit(fb, cx, n, &f, x, y),
        "TTabbedNotebook" => widgets::tabbed_notebook(fb, cx, n, &f, x, y),
        "TSpeedButton" => widgets::speedbutton(fb, cx, n, &f, x, y),
        "TColorGrid" => widgets::colorgrid(fb, cx, n, &f, x, y),
        "TPaintBox" => widgets::paintbox(fb, cx, n, x, y),
        "TGauge" => gauge(fb, cx, n, &f, x, y),
        "TImage" => image(fb, cx, n, x, y),
        "TNotebook" => notebook(fb, cx, n, &f, x, y),
        "TBiGauge" => bigauge(fb, cx, n, x, y),
        "TTabSet" => tabset(fb, cx, n, &f, x, y),
        _ => {}
    }
}

/// Whether a control is enabled: its `Enabled` property unless the program changed it (`<name>.enabled=0|1` in the texts).
pub(super) fn is_enabled(cx: &Ctx, n: &Node) -> bool {
    match cx.texts.get(&format!("{}.enabled", n.name)) {
        Some(v) => v != "0",
        None => n.flag("Enabled", true),
    }
}

fn caption_or<'a>(cx: &'a Ctx, n: &'a Node, prop: &str) -> &'a str {
    cx.texts
        .get(&n.name)
        .map(String::as_str)
        .or_else(|| n.str(prop))
        .unwrap_or("")
}

pub(super) fn caption<'a>(cx: &'a Ctx, n: &'a Node) -> &'a str {
    cx.texts
        .get(&n.name)
        .map(String::as_str)
        .or_else(|| n.str("Caption"))
        .unwrap_or("")
}

fn label(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let mut w = n.int("Width", 0);
    if cx
        .texts
        .get(&format!("{}.autosize", n.name))
        .is_some_and(|v| v == "1")
    {
        // an AutoSize label that got a new caption shrinks or grows to it
        w = caption(cx, n)
            .split('\n')
            .map(|l| font.width(l))
            .max()
            .unwrap_or(0);
    }
    if let Some(b) = cx.color_of(n, "bgcolor") {
        fb.fill(x, y, w, n.int("Height", 0), b);
    }
    let pad: i32 = cx
        .texts
        .get(&format!("{}.pad", n.name))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let (x, w) = (x + pad, w - pad);
    let s = caption(cx, n);
    let lines = if n.flag("WordWrap", false) {
        wrap(font, s, w)
    } else {
        s.split('\n').map(str::to_string).collect()
    };
    for (i, line) in lines.iter().enumerate() {
        let tw = font.width(line);
        let lx = match n.ident("Alignment") {
            Some("taCenter") => x + (w - tw) / 2,
            Some("taRightJustify") => x + w - tw,
            _ => x,
        };
        let ly = y + i as i32 * font.height();
        if is_enabled(cx, n) {
            text(fb, font, lx, ly, line, f.color, f.underline);
        } else {
            // a disabled label is embossed: white shifted by one pixel, grey on top
            text(fb, font, lx + 1, ly + 1, line, WHITE, f.underline);
            text(fb, font, lx, ly, line, GRAY, f.underline);
        }
    }
}

/// `TMemo`: the lines (`lists[name]`, else `texts[name]` split at newlines, else `Lines.Strings`) with the control's alignment. A bordered
/// memo is a sunken box on its `Color` and shows a vertical scroll bar (`ScrollBars`) when the lines do not fit; `<name>.top` is the
/// first visible line.
fn memo(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let lines: Vec<String> = match (cx.lists.get(&n.name), cx.texts.get(&n.name)) {
        (Some(v), _) => v.clone(),
        (None, Some(t)) => t.split('\n').map(str::to_string).collect(),
        _ => n.strings("Lines.Strings"),
    };
    let bordered = n.ident("BorderStyle") != Some("bsNone");
    let (mut x0, mut y0, mut right, mut bottom) = (x, y, x + w, y + h);
    if bordered {
        sunken_box(fb, x, y, w, h, n.color("Color").unwrap_or(WHITE));
        (x0, y0, right, bottom) = (x + 2, y + 1, x + w - 1, y + h - 1);
    } else if let Some(c) = n.color("Color").filter(|_| !n.flag("ParentColor", false)) {
        fb.fill(x, y, w, h, c);
    }
    let ih = font.height();
    let fit = ((bottom - y0) / ih).max(1) as usize;
    let vertical = matches!(n.ident("ScrollBars"), Some("ssVertical" | "ssBoth"));
    let scrolled = bordered && vertical && lines.len() > fit;
    if scrolled {
        right -= 16;
    }
    let top = widgets::get_num(cx, n, "top").unwrap_or(0).max(0) as usize;
    let top = top.min(lines.len().saturating_sub(fit));
    let old = fb.clip;
    fb.clip = (
        x0.max(old.0),
        y0.max(old.1),
        right.min(old.2),
        bottom.min(old.3),
    );
    for (r, line) in lines.iter().skip(top).enumerate() {
        let tw = font.width(line);
        let lx = match n.ident("Alignment") {
            Some("taCenter") => x0 + (right - x0 - tw) / 2,
            Some("taRightJustify") => x0 + right - x0 - tw,
            _ => x0,
        };
        text(fb, font, lx, y0 + r as i32 * ih, line, f.color, false);
    }
    fb.clip = old;
    if scrolled {
        widgets::vscroll(
            fb,
            cx,
            &n.name,
            x + w - 17,
            y + 1,
            h - 2,
            top,
            lines.len(),
            fit,
        );
    }
}

/// Delphi's `Frame3D`: a 1-pixel rectangle lit from the top left (`tl` there, `br` on the other two sides).
pub(super) fn frame3d(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, tl: Rgb, br: Rgb) {
    fb.fill(x, y, w - 1, 1, tl);
    fb.fill(x, y, 1, h - 1, tl);
    fb.fill(x, y + h - 1, w, 1, br);
    fb.fill(x + w - 1, y, 1, h, br);
}

/// `TBevel`: a lowered (default) or raised line, box or frame.
fn bevel(fb: &mut Fb, n: &Node, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let raised = n.ident("Style") == Some("bsRaised");
    let (dark, light) = if raised { (WHITE, GRAY) } else { (GRAY, WHITE) };
    match n.ident("Shape").unwrap_or("bsBox") {
        "bsFrame" => {
            frame3d(fb, x, y, w, h, dark, light);
            frame3d(fb, x + 1, y + 1, w - 2, h - 2, light, dark);
        }
        "bsTopLine" => {
            fb.fill(x, y, w, 1, dark);
            fb.fill(x, y + 1, w, 1, light);
        }
        "bsBottomLine" => {
            fb.fill(x, y + h - 2, w, 1, dark);
            fb.fill(x, y + h - 1, w, 1, light);
        }
        "bsLeftLine" => {
            fb.fill(x, y, 1, h, dark);
            fb.fill(x + 1, y, 1, h, light);
        }
        "bsRightLine" => {
            fb.fill(x + w - 2, y, 1, h, dark);
            fb.fill(x + w - 1, y, 1, h, light);
        }
        _ => frame3d(fb, x, y, w, h, dark, light),
    }
}

/// `TPanel`: face colour, an optional bevel and a centred caption.
fn panel(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let color = cx.color_of(n, "bgcolor").unwrap_or_else(|| {
        if n.flag("ParentColor", false) {
            cx.bg
        } else {
            n.color("Color").unwrap_or(SILVER)
        }
    });
    fb.fill(x, y, w, h, color);
    let bw = n.int("BevelWidth", 1);
    let (outer, inner) = (
        n.ident("BevelOuter").unwrap_or("bvRaised"),
        n.ident("BevelInner").unwrap_or("bvNone"),
    );
    let mut inset = 0;
    for (kind, _) in [(outer, 0), (inner, 1)] {
        let (tl, br) = match kind {
            "bvRaised" => (WHITE, GRAY),
            "bvLowered" => (GRAY, WHITE),
            _ => continue,
        };
        for k in 0..bw {
            frame3d(
                fb,
                x + inset + k,
                y + inset + k,
                w - 2 * (inset + k),
                h - 2 * (inset + k),
                tl,
                br,
            );
        }
        inset += bw;
    }
    if let Some(font) = cx.font(f) {
        let cap = caption(cx, n);
        if !cap.is_empty() && h >= font.height() {
            let tw = font.width(cap);
            text(
                fb,
                font,
                x + (w - tw) / 2,
                y + (h - font.height()) / 2,
                cap,
                f.color,
                false,
            );
        }
    }
    let child_font = f.clone();
    let (cx2, cy2) = (x, y);
    let sub = Ctx {
        ox: cx2,
        oy: cy2,
        ..cx.sub()
    };
    for c in &n.children {
        control(fb, &sub, c, &child_font);
    }
}

/// `TGroupBox`: an etched frame (grey line, white line one pixel further in) whose top edge is interrupted by the caption.
fn groupbox(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let Some(font) = cx.font(f) else {
        return;
    };
    let own_bg = cx.color_of(n, "bgcolor");
    if let Some(b) = own_bg {
        // the program painted the group box (the squads of the match day take the colours of the club)
        fb.fill(x, y, w, h, b);
    }
    let bg = own_bg.unwrap_or(cx.bg);
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
    fb.fill(x + 8, top, tw, 2, bg);
    text(fb, font, x + 8, y, cap, f.color, false);
    let sub = Ctx {
        ox: x,
        oy: y,
        bg,
        ..cx.sub()
    };
    for c in &n.children {
        control(fb, &sub, c, f);
    }
}

/// The 3-D frame Ctl3D draws around edits and list boxes: grey outside top/left, white outside bottom/right, a black border and a
/// white interior.
pub(super) fn sunken_box(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, interior: Rgb) {
    fb.fill(x, y, w, h, BLACK);
    fb.fill(x + 1, y + 1, w - 2, h - 2, interior);
    fb.fill(x - 1, y - 1, w + 1, 1, GRAY);
    fb.fill(x - 1, y - 1, 1, h + 1, GRAY);
    fb.fill(x, y + h, w + 1, 1, WHITE);
    fb.fill(x + w, y, 1, h + 1, WHITE);
}

fn edit(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let enabled = is_enabled(cx, n);
    sunken_box(fb, x, y, w, h, cx.color_of(n, "bgcolor").unwrap_or(WHITE));
    let Some(font) = cx.font(f) else {
        return;
    };
    let s = caption_or(cx, n, "Text");
    let (tx, ty) = (x + 2, y + (h - font.height()) / 2);
    let old = fb.clip;
    fb.clip = (x + 1, y + 1, x + w - 1, y + h - 1);
    // selected characters: `<name>.selstart` and `<name>.sellen` are character positions
    let num = |k: &str| {
        cx.texts
            .get(&format!("{}.{k}", n.name))
            .and_then(|v| v.parse::<usize>().ok())
    };
    let chars: Vec<char> = s.chars().collect();
    let (a, len) = (
        num("selstart").unwrap_or(0).min(chars.len()),
        num("sellen").unwrap_or(0),
    );
    let b = (a + len).min(chars.len());
    let pre: String = chars[..a].iter().collect();
    let mid: String = chars[a..b].iter().collect();
    let post: String = chars[b..].iter().collect();
    let ink = if enabled { f.color } else { GRAY };
    let mut pen = tx;
    pen += text(fb, font, pen, ty, &pre, ink, false);
    if !mid.is_empty() {
        let mw = font.width(&mid);
        fb.fill(pen + 1, ty, mw, font.height(), NAVY);
        text(fb, font, pen, ty, &mid, WHITE, false);
        pen += mw;
    }
    text(fb, font, pen, ty, &post, ink, false);
    // the caret: a black line before character `<name>.caret`, a pixel taller than the font's line
    if cx.focus == n.name && enabled {
        if let Some(c) = num("caret") {
            let before: String = chars[..c.min(chars.len())].iter().collect();
            fb.fill(
                tx + 1 + font.width(&before),
                ty,
                1,
                font.height() + 1,
                BLACK,
            );
        }
    }
    fb.clip = old;
}

/// The owner-drawn squad list of the main screen: rows `pos|name|strength|wage|mark`, 16 pixels high, grouped by position with a black
/// rule and 4 extra pixels above the first row of every group but the first.
#[allow(clippy::too_many_arguments)]
fn roster(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    font: &Font,
    color: Rgb,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rows: &[String],
) {
    let old = fb.clip;
    fb.clip = (x + 1, y + 1, x + w - 1, y + h - 1);
    let mut top = y + 1;
    let mut last = String::new();
    let focus: Option<usize> = cx
        .texts
        .get(&format!("{}.focus", n.name))
        .and_then(|s| s.parse().ok());
    for (i, row) in rows.iter().enumerate() {
        let c: Vec<&str> = row.split('|').collect();
        let pos = c.first().copied().unwrap_or("");
        let first_of_group = i > 0 && pos != last;
        if first_of_group {
            fb.fill(x + 1, top, w - 2, 1, color);
            top += 4;
        }
        last = pos.to_string();
        let ty = top;
        let selected = cx
            .texts
            .get(&format!("{}.sel", n.name))
            .and_then(|v| v.parse::<usize>().ok())
            == Some(i);
        let (color, back) = if selected {
            (cx.color_of(n, "bg").unwrap_or(WHITE), Some(color))
        } else {
            (color, None)
        };
        if let Some(b) = back {
            fb.fill(x + 1, ty, w - 2, 16, b);
        }
        match c.get(5).copied().unwrap_or("") {
            "+" => {
                for (dy, (a, b)) in [(0, (5, 8)), (1, (4, 9)), (2, (4, 9)), (3, (5, 8))] {
                    fb.fill(x + a, ty + 5 + dy, b - a, 1, color);
                }
            }
            "-" => fb.fill(x + 4, ty + 6, 6, 1, color),
            _ => {}
        }
        text(fb, font, x + 23, ty, pos, color, false);
        text(
            fb,
            font,
            x + 38,
            ty,
            c.get(1).copied().unwrap_or(""),
            color,
            false,
        );
        text(
            fb,
            font,
            x + 172,
            ty,
            c.get(2).copied().unwrap_or(""),
            color,
            false,
        );
        let wage = c.get(3).copied().unwrap_or("");
        text(fb, font, x + 318 - font.width(wage), ty, wage, color, false);
        text(
            fb,
            font,
            x + 330,
            ty,
            c.get(4).copied().unwrap_or(""),
            color,
            false,
        );
        if focus == Some(i) {
            focus_rect(fb, x + 1, ty, w - 2, 16, x + 1, y + 1);
        }
        cx.hits.borrow_mut().push(Hit {
            name: format!("{}#{i}", n.name),
            class: "ListRow".into(),
            x: x + 1,
            y: ty,
            w: w - 2,
            h: 16,
            enabled: true,
        });
        top = ty + 16;
    }
    fb.clip = old;
}

/// The two-row score table of the main screen's first notebook page: `name|w|d|l|gf|ga|pts|r,g,b` per row, 16 pixels high.
fn classification(fb: &mut Fb, font: &Font, x: i32, y: i32, w: i32, rows: &[String]) {
    // glyph start of each number relative to the list's left edge: wins, draws, losses, goals for, colon, goals against, points
    const COLS: [i32; 7] = [137, 157, 177, 207, 220, 232, 262];
    for (i, row) in rows.iter().enumerate() {
        let c: Vec<&str> = row.split('|').collect();
        let color = c
            .get(7)
            .map(|t| {
                t.split(',')
                    .filter_map(|v| v.parse::<u8>().ok())
                    .collect::<Vec<_>>()
            })
            .filter(|v| v.len() == 3)
            .map_or(WHITE, |v| Rgb(v[0], v[1], v[2]));
        let ty = y + 1 + i as i32 * 16;
        if let Some(b) = c
            .get(8)
            .map(|t| {
                t.split(',')
                    .filter_map(|v| v.parse::<u8>().ok())
                    .collect::<Vec<_>>()
            })
            .filter(|v| v.len() == 3)
        {
            fb.fill(x, ty, w, 16, Rgb(b[0], b[1], b[2]));
        }
        text(
            fb,
            font,
            x + 3,
            ty,
            c.first().copied().unwrap_or(""),
            color,
            false,
        );
        let toks = [
            c.get(1),
            c.get(2),
            c.get(3),
            c.get(4),
            Some(&":"),
            c.get(5),
            c.get(6),
        ];
        for (k, t) in toks.iter().enumerate() {
            let t = t.copied().unwrap_or("");
            let bearing = t
                .chars()
                .next()
                .and_then(|ch| font.glyph(ansi(ch)))
                .map_or(0, |g| g.left);
            text(fb, font, x + COLS[k] - bearing, ty, t, color, false);
        }
    }
}

/// The balance list of the finances page: rows `label|value|b` (`b` for a bold row), 16 pixels high, values right-aligned.
fn balance_list(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32, rows: &[String]) {
    let (Some(reg), Some(bold)) = (
        cx.font(f),
        cx.font(&FontSpec {
            bold: true,
            ..f.clone()
        }),
    ) else {
        return;
    };
    let _ = n;
    for (i, row) in rows.iter().enumerate() {
        let c: Vec<&str> = row.split('|').collect();
        let font = if c.get(2) == Some(&"b") { bold } else { reg };
        let ty = y + i as i32 * 16;
        text(
            fb,
            font,
            x + 2,
            ty,
            c.first().copied().unwrap_or(""),
            f.color,
            false,
        );
        let v = c.get(1).copied().unwrap_or("");
        text(fb, font, x + 212 - font.width(v), ty, v, f.color, false);
    }
}

/// The owner-drawn list of *Selecção de Países* (`DrawCountry` `1028:171d`): rows of 31 pixels with the flag (`FLAGS/<code>`) at the
/// left, the name 43 pixels in and the number of teams right-aligned 20 pixels before the scroll bar; a chosen country is yellow and
/// bold, the current row has the focus rectangle. Rows are `code|name|count|chosen` (`1` or `0`); `<name>.top`, `<name>.sel`.
#[allow(clippy::too_many_arguments)]
fn countries(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    rows: &[String],
) {
    const ROW: i32 = 31;
    // the rows are written larger than the form's font (Arial 16 pixels, measured)
    let f = &FontSpec {
        px: 16,
        ..f.clone()
    };
    let (Some(reg), Some(bold)) = (
        cx.font(f),
        cx.font(&FontSpec {
            bold: true,
            ..f.clone()
        }),
    ) else {
        return;
    };
    let num = |k: &str| {
        cx.texts
            .get(&format!("{}.{k}", n.name))
            .and_then(|v| v.parse::<i64>().ok())
    };
    let fit = ((h - 4) / ROW) as usize;
    let top = (num("top").unwrap_or(0).max(0) as usize).min(rows.len().saturating_sub(fit));
    let sel = num("sel").unwrap_or(-1);
    let scroll = rows.len() > fit;
    let (rx, rw) = (x + 2, w - 4 - if scroll { 16 } else { 0 });
    for (k, row) in rows.iter().enumerate().skip(top).take(fit) {
        let c: Vec<&str> = row.split('|').collect();
        // (the rows start one pixel above the list's client area: measured)
        let ry = y + 1 + (k - top) as i32 * ROW;
        let on = c.get(3) == Some(&"1");
        if on {
            fb.fill(rx, ry, rw, ROW, Rgb(248, 252, 0));
        }
        if let Some(b) = cx.skin.bitmap("FLAGS", c.first().copied().unwrap_or("")) {
            for r in 0..b.h {
                for col in 0..b.w {
                    let p = b.px[(r * b.w + col) as usize];
                    fb.set(
                        rx - 1 + col,
                        ry + 1 + r,
                        Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    );
                }
            }
        }
        let font = if on { bold } else { reg };
        let ty = ry + (ROW - font.height()) / 2 - 1;
        text(
            fb,
            font,
            rx + 43,
            ty,
            c.get(1).copied().unwrap_or(""),
            BLACK,
            false,
        );
        let count = c.get(2).copied().unwrap_or("0");
        let label = if count == "1" {
            "1 equipa".to_string()
        } else {
            format!("{count} equipas")
        };
        // placed as if it were always plural: "1 equipa" ends one letter short of the others
        text(
            fb,
            font,
            rx + rw - 20 - font.width(&format!("{count} equipas")),
            ty,
            &label,
            BLACK,
            false,
        );
        if k as i64 == sel {
            focus_rect(fb, rx, ry, rw, ROW, rx, ry);
        }
        cx.hits.borrow_mut().push(Hit {
            name: format!("{}#{k}", n.name),
            class: "ListRow".into(),
            x: rx,
            y: ry,
            w: rw,
            h: ROW,
            enabled: true,
        });
    }
    if scroll {
        widgets::vscroll(
            fb,
            cx,
            &n.name,
            x + w - 16,
            y + 2,
            h - 4,
            top,
            rows.len(),
            fit,
        );
    }
}

fn listbox(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let flat = (!n.flag("Ctl3D", true) && n.flag("ParentColor", false))
        || n.ident("BorderStyle") == Some("bsNone")
        || cx
            .texts
            .get(&format!("{}.flat", n.name))
            .is_some_and(|v| v == "1");
    if flat {
        fb.fill(x, y, w, h, cx.color_of(n, "bgcolor").unwrap_or(cx.bg));
    } else {
        sunken_box(fb, x, y, w, h, cx.color_of(n, "bg").unwrap_or(WHITE));
    }
    let Some(font) = cx.font(f) else {
        return;
    };
    if n.name == "ListBoxBalance" {
        if let Some(rows) = cx.lists.get(&n.name) {
            balance_list(fb, cx, n, f, x, y, rows);
        }
        return;
    }
    if n.name == "ListBoxOfCountries" {
        if let Some(rows) = cx.lists.get(&n.name) {
            countries(fb, cx, n, f, x, y, w, h, rows);
        }
        return;
    }
    if n.name == "ListBoxClassification" {
        if let Some(rows) = cx.lists.get(&n.name) {
            classification(fb, font, x, y, w, rows);
        }
        return;
    }
    if n.ident("Style") == Some("lbOwnerDrawFixed")
        && matches!(
            n.name.as_str(),
            "listOfItems"
                | "ListBoxDivision1"
                | "ListBoxDivision2"
                | "ListBoxDivision3"
                | "ListBoxDivision4"
                | "ListBoxOfCoaches"
        )
    {
        // rows in the field encoding of the original's `DrawItem` routines (`printlist.rs`)
        if let Some(rows) = cx.lists.get(&n.name) {
            let bg = if flat {
                cx.bg
            } else {
                cx.color_of(n, "bg").unwrap_or(WHITE)
            };
            super::printlist::draw_list(fb, cx, n, f, bg, x, y, w, h, rows);
            if n.name.starts_with("ListBoxDivision") {
                // a flat list has a plain black outline over the rows' edges
                for (rx, ry, rw, rh) in [
                    (x, y, w, 1),
                    (x, y + h - 1, w, 1),
                    (x, y, 1, h),
                    (x + w - 1, y, 1, h),
                ] {
                    fb.fill(rx, ry, rw, rh, BLACK);
                }
            }
        }
        return;
    }
    if n.ident("Style") == Some("lbOwnerDrawVariable") {
        if let Some(rows) = cx.lists.get(&n.name) {
            roster(fb, cx, n, font, f.color, x, y, w, h, rows);
        }
        return;
    }
    widgets::listbox_rows(fb, cx, n, f, x, y);
    // `<name>.flat=1`: a list painted in the colours of a club has a plain black outline (no 3-D frame)
    if cx
        .texts
        .get(&format!("{}.flat", n.name))
        .is_some_and(|v| v == "1")
    {
        fb.fill(x, y, w, 1, BLACK);
        fb.fill(x, y + h - 1, w, 1, BLACK);
        fb.fill(x, y, 1, h, BLACK);
        fb.fill(x + w - 1, y, 1, h, BLACK);
    }
}

pub(super) const DITHER_PARITY: i32 = 0;

/// A bevel of `bs` pixels inside the inclusive rectangle `(x1, y1)..(x2, y2)` with 45-degree joints: `tl` on the top and left,
/// `br` on the bottom and right.
#[allow(clippy::too_many_arguments)]
pub(super) fn bevel_miter(
    fb: &mut Fb,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    bs: i32,
    tl: Rgb,
    br: Rgb,
) {
    for k in 0..bs {
        fb.fill(x1 + k, y1 + k, x2 - x1 - 2 * k + 1, 1, tl);
        fb.fill(x1 + k, y1 + k, 1, y2 - y1 - 2 * k + 1, tl);
        fb.fill(x1 + k + 1, y2 - k, x2 - x1 - 2 * k, 1, br);
        fb.fill(x2 - k, y1 + k + 1, 1, y2 - y1 - 2 * k, br);
    }
}

/// `TBiGauge` (a VBX control): black frame, grey/white bevel, silver face and a bar of `value / max` of the inner width.
/// `<name>.value` and `<name>.barcolor` (r,g,b) in the texts set the run-time state.
fn bigauge(fb: &mut Fb, cx: &Ctx, n: &Node, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let get = |k: &str| cx.texts.get(&format!("{}.{k}", n.name));
    let value: i32 = get("value")
        .and_then(|v| v.parse().ok())
        .unwrap_or(n.int("Value", 0));
    let (min, max) = (n.int("Min", 0), n.int("Max", 100));
    let bar = get("barcolor")
        .map(|c| {
            c.split(',')
                .filter_map(|v| v.parse::<u8>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() == 3)
        .map(|v| Rgb(v[0], v[1], v[2]))
        .or_else(|| n.color("BarColor"))
        .unwrap_or(Rgb(0, 255, 0));
    let bs = n.int("BevelSize", 3);
    let margin = n.int("Margin", 3);
    if n.int("BorderStyle", 0) == 1 {
        fb.fill(x, y, w, h, BLACK);
    }
    let (x1, y1, x2, y2) = (x + 1, y + 1, x + w - 2, y + h - 2);
    fb.fill(x1, y1, x2 - x1 + 1, y2 - y1 + 1, SILVER);
    bevel_miter(fb, x1, y1, x2, y2, bs, GRAY, WHITE);
    let (bx, by) = (x1 + bs + margin, y1 + bs + margin);
    let (bw, bh) = (
        x2 - x1 + 1 - 2 * (bs + margin),
        y2 - y1 + 1 - 2 * (bs + margin),
    );
    let filled = if max > min {
        bw * (value - min).clamp(0, max - min) / (max - min)
    } else {
        0
    };
    fb.fill(bx, by, filled, bh, bar);
}

/// `TTabSet` (bottom tabs): the labels in MS Sans Serif; the selected tab is outlined by slanted edges and a base line, a grey line
/// runs along the top everywhere else. `<name>.tabs=a|b|c`, `<name>.index=n`, `<name>.color=r,g,b` in the texts.
fn tabset(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let w = n.int("Width", 0);
    let tabs: Vec<String> = match cx.texts.get(&format!("{}.tabs", n.name)) {
        Some(t) => t.split('|').map(str::to_string).collect(),
        None => n.strings("Tabs.Strings"),
    };
    let sel: usize = cx
        .texts
        .get(&format!("{}.index", n.name))
        .and_then(|v| v.parse().ok())
        .unwrap_or(n.int("TabIndex", 0) as usize);
    let Some(font) = cx.font(f) else {
        return;
    };
    let widths: Vec<i32> = tabs.iter().map(|t| font.width(t)).collect();
    // the text positions: every tab is 9 pixels wider than its caption (the selected one is padded by its slanted edges)
    let mut pens = Vec::new();
    let mut pen = x + 9;
    for wd in &widths {
        pens.push(pen);
        pen += wd + 9;
    }
    // the strip: a grey line over a black one, open above the selected tab
    let (sl, sr) = if sel < tabs.len() {
        (pens[sel] - 5, pens[sel] + widths[sel] + 6)
    } else {
        (x + w, x + w)
    };
    let seg = |fb: &mut Fb, a: i32, b: i32, row: i32, c: Rgb| {
        if b > a {
            fb.fill(a, y + row, b - a, 1, c);
        }
    };
    seg(fb, x, sl, 0, GRAY);
    seg(fb, sr, x + w, 0, GRAY);
    seg(fb, x, sl, 1, BLACK);
    seg(fb, sr + 1, x + w, 1, BLACK);
    // slanted edges of the outlines: `/` leans left going down, `\` leans right
    let back = |fb: &mut Fb, bx: i32, row0: i32| {
        for row in row0..=15 {
            fb.set(bx - row / 3, y + row, BLACK);
        }
    };
    let fwd = |fb: &mut Fb, bx: i32, row0: i32, off: i32| {
        for row in row0..=15 {
            fb.set(
                bx + off + (row - row0) / 3 * (row0 != 2) as i32 + (row / 3) * (row0 == 2) as i32,
                y + row,
                BLACK,
            );
        }
    };
    for (i, t) in tabs.iter().enumerate() {
        let pen = pens[i];
        cx.hits.borrow_mut().push(Hit {
            name: format!("{}#{i}", n.name),
            class: "TabSetTab".into(),
            x: pen - 6,
            y,
            w: widths[i] + 12,
            h: n.int("Height", 25),
            enabled: true,
        });
        text(fb, font, pen, y + 2, t, f.color, false);
        if i == sel {
            let left = pen - 5;
            let right = sr;
            fb.fill(left, y, right - left, 2, cx.bg);
            for (row, dx) in [
                (1, 0),
                (2, 0),
                (3, 1),
                (4, 1),
                (5, 1),
                (6, 2),
                (7, 2),
                (8, 2),
                (10, 3),
                (11, 3),
                (13, 4),
                (14, 4),
            ] {
                fb.set(left + dx, y + row, WHITE);
            }
            for row in 1..=14 {
                fb.set(right - ((row + 1) / 3), y + row, GRAY);
                fb.set(right + 1 - ((row + 1) / 3), y + row, BLACK);
            }
            for row in 1..=15 {
                fb.set(left - 1 + row / 3, y + row, BLACK);
            }
            fb.fill(left + 5, y + 15, right - left - 9, 1, GRAY);
            seg(fb, left + 4, right - 3, 16, BLACK);
        } else {
            // left edge: the first tab owns a full `\`, the others a short one under the neighbour's `/`
            let prev_sel = i > 0 && i - 1 == sel;
            let bl = if i == 0 { pen - 6 } else { pen - 2 };
            let _ = prev_sel;
            let mut xl = bl + 1;
            if i == 0 {
                fwd(fb, bl, 2, 0);
                xl = bl + 5;
            } else {
                for row in 9..=15 {
                    fb.set(bl - 1 + (row - 9) / 3, y + row, BLACK);
                }
            }
            // right edge: `/` from the boundary to the next tab
            let next_sel = i + 1 == sel;
            let br = pen + widths[i] + if next_sel { 3 } else { 7 };
            let xr = if next_sel {
                for row in 9..=15 {
                    fb.set(br + 1 - (row - 9) / 3, y + row, BLACK);
                }
                br - 1
            } else {
                back(fb, br, 2);
                br - 5
            };
            seg(fb, xl, xr + 1, 16, BLACK);
        }
    }
}

/// `TNotebook`: only the current page is drawn (`<name>.page=<index>` in the texts; the form's `PageIndex` otherwise); the pages
/// start at the notebook's corner.
fn notebook(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let page: usize = cx
        .texts
        .get(&format!("{}.page", n.name))
        .and_then(|s| s.parse().ok())
        .unwrap_or(n.int("PageIndex", 0) as usize);
    if let Some(p) = n.children.get(page) {
        let sub = Ctx {
            ox: x,
            oy: y,
            ..cx.sub()
        };
        for c in &p.children {
            control(fb, &sub, c, f);
        }
    }
}

/// `TImage`: a bitmap of the skin, named by `<name>.image=FORM/NAME` (e.g. `FLAGS/BRA`) in the texts.
fn image(fb: &mut Fb, cx: &Ctx, n: &Node, x: i32, y: i32) {
    let Some(spec) = cx.texts.get(&format!("{}.image", n.name)) else {
        return;
    };
    let Some((form, name)) = spec.split_once('/') else {
        return;
    };
    if let Some(b) = cx.skin.bitmap(form, name) {
        // `Stretch`: the bitmap is scaled to the control's size (nearest neighbour, as StretchBlt does for whole factors)
        let (w, h) = if n.flag("Stretch", false) {
            (n.int("Width", b.w), n.int("Height", b.h))
        } else {
            (b.w, b.h)
        };
        for r in 0..h {
            for c in 0..w {
                let p = b.px[((r * b.h / h) * b.w + c * b.w / w) as usize];
                fb.set(x + c, y + r, Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8));
            }
        }
    }
}

/// `TGauge` (Delphi's sample component): a black bar that fills from the left with `Progress` percent, white behind it, a black frame
/// on the right, and the percentage written over both with inverted colours. `<name>.progress` in the texts sets the value.
fn gauge(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let p: i32 = cx
        .texts
        .get(&format!("{}.progress", n.name))
        .and_then(|s| s.parse().ok())
        .unwrap_or(n.int("Progress", 0));
    let bw = w - 10;
    let filled = bw * p.clamp(0, 100) / 100;
    fb.fill(x, y, w, h, SILVER);
    fb.fill(x + 5, y, bw, h, WHITE);
    fb.fill(x + 5, y, filled, h, BLACK);
    fb.fill(x + 5 + bw - 1, y, 1, h, BLACK);
    fb.fill(x + 5, y, bw, 1, BLACK);
    fb.fill(x + 5, y + h - 1, bw, 1, BLACK);
    if let Some(font) = cx.font(f) {
        let s = format!("{p}%");
        let tw = font.width(&s);
        let (tx, ty) = (x + 5 + (bw - tw) / 2 - 1, y + (h - font.height()) / 2);
        // inverted colours: draw the glyphs, then flip them
        let mut mask = Fb::new();
        text(&mut mask, font, tx, ty, &s, WHITE, false);
        for yy in ty..ty + font.height() {
            for xx in tx..tx + tw + 2 {
                if mask.get(xx, yy) != 0 {
                    let under = fb.get(xx, yy);
                    let inv = Rgb(!(under >> 16) as u8, !(under >> 8) as u8, !under as u8);
                    fb.set(xx, yy, inv);
                }
            }
        }
    }
}

/// `TBanner`: the scrolling text; `texts["Banner1.pos"]`-style state: `<name>.scroll` is how many pixels the text has moved left.
fn banner(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let Some(font) = cx.font(f) else {
        return;
    };
    let (w, h) = (n.int("Width", 0), n.int("Height", 0));
    let scroll: i32 = cx
        .texts
        .get(&format!("{}.scroll", n.name))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let old = fb.clip;
    fb.clip = (
        x.max(old.0),
        y.max(old.1),
        (x + w).min(old.2),
        (y + h).min(old.3),
    );
    text(
        fb,
        font,
        x + w - scroll,
        y,
        n.str("ScrollText").unwrap_or(""),
        f.color,
        false,
    );
    fb.clip = old;
}

/// The distance of a glyph on top of a caption from the button's client top: the `Margin` property, or (default `-1`) the margin
/// that centres glyph, spacing and caption as `TBitBtn.CalcButtonLayout` does.
fn glyph_margin(cx: &Ctx, n: &Node, h: i32, gh: i32, f: &FontSpec) -> i32 {
    let m = n.int("Margin", -1);
    if m >= 0 {
        return m;
    }
    let th = cx.font(f).map_or(0, |t| t.tm_height);
    (h - 4 - (gh + n.int("Spacing", 4) + th) + 1) / 2
}

/// A `TBitBtn` with `Layout = blGlyphLeft` (the default): glyph and caption side by side, placed as `TBitBtn.CalcButtonLayout` does inside
/// the client rectangle `(x + 2, y + 2, w - 5, h - 5)` (measured on the OK button of the list dialogs).
#[allow(clippy::too_many_arguments)]
fn bitbtn_glyph_left(
    fb: &mut Fb,
    cx: &Ctx,
    n: &Node,
    f: &FontSpec,
    x: i32,
    y: i32,
    (w, h): (i32, i32),
    bmp: Option<&super::skin::Bitmap>,
    cap: &str,
    enabled: bool,
) {
    let (cw, ch) = (w - 5, h - 5);
    let font = cx.font(f);
    let label: String = cap.replace('&', "");
    let tw = font.map_or(0, |t| t.width(&label));
    let th = font.map_or(0, |t| t.tm_height);
    let (glyphs, gw, gh) = match bmp {
        Some(b) => {
            let glyphs = if b.w >= 2 * b.h {
                2.max(n.int("NumGlyphs", 1))
            } else {
                n.int("NumGlyphs", 1).max(1)
            };
            (glyphs, b.w / glyphs, b.h)
        }
        None => (1, 0, 0),
    };
    let (mut margin, mut spacing) = (n.int("Margin", -1), n.int("Spacing", 4));
    if tw == 0 || gw == 0 {
        spacing = 0;
    }
    if margin == -1 {
        if spacing == -1 {
            margin = (cw - (gw + tw)) / 3;
            spacing = margin;
        } else {
            margin = (cw - (gw + spacing + tw) + 1) / 2;
        }
    } else if spacing == -1 {
        spacing = (cw - (margin + gw) - tw) / 2;
    }
    let gx = x + 2 + margin;
    if let Some(b) = bmp {
        let key = b.px[((b.h - 1) * b.w) as usize];
        let gy = y + 2 + (ch - gh + 1) / 2;
        glyph_image(fb, b, gx, gy, gw, glyphs, key, enabled);
    }
    if let Some(font) = font {
        let (tx, ty) = (gx + gw + spacing, y + 2 + (ch - th + 1) / 2);
        if enabled {
            text(fb, font, tx, ty, &label, f.color, false);
        } else {
            text(fb, font, tx + 1, ty + 1, &label, WHITE, false);
            text(fb, font, tx, ty, &label, GRAY, false);
        }
        if cx.focus == n.name {
            focus_rect(fb, tx - 1, ty - 1, tw + 2, th + 2, x, y);
        }
    }
}

fn bitbtn(fb: &mut Fb, cx: &Ctx, n: &Node, f: &FontSpec, x: i32, y: i32) {
    let (w, h) = (n.int("Width", 64), n.int("Height", 64));
    let enabled = is_enabled(cx, n);
    let default = (enabled && cx.focus == n.name)
        || ((n.flag("Default", false) || n.ident("Kind") == Some("bkOK"))
            && !FOCUS_ON_BUTTON.with(|c| c.get()));
    button_face(fb, x, y, w, h, default);
    let kind = n.ident("Kind").unwrap_or("bkCustom");
    let std = match kind {
        "bkOK" => Some(("BBOK", "OK")),
        "bkCancel" => Some(("BBCANCEL", "Cancelar")),
        "bkYes" => Some(("BBYES", "&Sim")),
        "bkNo" => Some(("BBNO", "&Não")),
        "bkHelp" => Some(("BBHELP", "&Ajuda")),
        "bkClose" => Some(("BBCLOSE", "&Fechar")),
        "bkAbort" => Some(("BBABORT", "&Abortar")),
        "bkRetry" => Some(("BBRETRY", "&Repetir")),
        "bkIgnore" => Some(("BBIGNORE", "&Ignorar")),
        "bkAll" => Some(("BBALL", "&Todos")),
        _ => None,
    };
    let bmp = cx
        .skin
        .bitmap(cx.form, &n.name)
        .or_else(|| std.and_then(|(b, _)| cx.skin.bitmap("*", b)));
    let cap = match caption(cx, n) {
        "" => std.map_or("", |(_, c)| c),
        c => c,
    };
    let gh = bmp.map_or(0, |b| b.h);
    if n.ident("Layout").is_none_or(|l| l == "blGlyphLeft") {
        bitbtn_glyph_left(fb, cx, n, f, x, y, (w, h), bmp, cap, enabled);
        return;
    }
    if let Some(b) = bmp {
        let glyphs = if b.w >= 2 * b.h {
            2.max(n.int("NumGlyphs", 1))
        } else {
            n.int("NumGlyphs", 1).max(1)
        };
        let gw = b.w / glyphs;
        let key = b.px[((b.h - 1) * b.w) as usize];
        let margin = glyph_margin(cx, n, h, gh, f);
        let (gx, gy) = (x + (w - gw) / 2, y + 2 + margin);
        glyph_image(fb, b, gx, gy, gw, glyphs, key, enabled);
    }
    if let Some(font) = cx.font(f) {
        let label: String = cap.replace('&', "");
        let tw = font.width(&label);
        // the caption is centred with the odd pixel on the left (the raster MS Sans Serif puts it on the right)
        let round_up = i32::from(font.face != "ms sans serif");
        let spaced = n.int("Margin", -1) < 0 && n.int("Spacing", 4) >= 0;
        let ty = if spaced {
            y + 2 + glyph_margin(cx, n, h, gh, f) + gh + n.int("Spacing", 4)
        } else if n.int("Margin", -1) >= 0
            && n.int("Spacing", 4) == -1
            && gh > 0
            && font.height() != font.tm_height
        {
            // `CalcButtonLayout` with a margin and `Spacing = -1`: the caption is centred in what the glyph leaves, so its place
            // depends on the glyph's height (client area `h - 5` high, from `y + 2`)
            let m = n.int("Margin", 4);
            y + 2 + m + gh + (h - 5 - m - gh - font.tm_height) / 2
        } else {
            y + h - 2 - n.int("Margin", 4) - font.tm_height
                + if font.height() != font.tm_height {
                    0
                } else if font.face == "ms sans serif" {
                    1
                } else if !default && !font.bold {
                    // a thin-framed button raises the caption of a regular raster font by a pixel
                    -1
                } else {
                    0
                }
        };
        let _ = gh;
        let (tx, ty) = if n.class == "TButton" {
            (x + (w - tw) / 2 - 1, y + (h - font.height()) / 2 + 1)
        } else {
            (x + (w - tw + round_up) / 2, ty)
        };
        if enabled {
            text(fb, font, tx, ty, &label, f.color, false);
        } else {
            text(fb, font, tx + 1, ty + 1, &label, WHITE, false);
            text(fb, font, tx, ty, &label, GRAY, false);
        }
        if cx.focus == n.name {
            focus_rect(
                fb,
                x + (w - tw + round_up) / 2 - 1,
                ty - 1,
                tw + 2,
                font.height() + 2,
                x,
                y,
            );
        }
    }
}

/// The glyph of a `TBitBtn` at `(gx, gy)`: the first image (`gw` wide) without its transparent colour `key`; a disabled button shows
/// the bitmap's second image, or, with only one, the one Windows makes: the dark pixels in grey with a white copy one pixel down and
/// to the right (the yellow of the stadium's bulldozer disappears, measured).
#[allow(clippy::too_many_arguments)]
fn glyph_image(
    fb: &mut Fb,
    b: &super::skin::Bitmap,
    gx: i32,
    gy: i32,
    gw: i32,
    glyphs: i32,
    key: u32,
    enabled: bool,
) {
    let px = |r: i32, c: i32, at: i32| b.px[(r * b.w + c + at) as usize];
    if enabled || glyphs > 1 {
        let at = if enabled { 0 } else { gw };
        for r in 0..b.h {
            for c in 0..gw {
                let p = px(r, c, at);
                if p != key {
                    fb.set(
                        gx + c,
                        gy + r,
                        Rgb((p >> 16) as u8, (p >> 8) as u8, p as u8),
                    );
                }
            }
        }
        return;
    }
    let dark = |p: u32| {
        let (r, g, bl) = ((p >> 16) & 255, (p >> 8) & 255, p & 255);
        p != key && (r * 30 + g * 59 + bl * 11) < 128 * 100
    };
    for (dx, colour) in [(1, WHITE), (0, GRAY)] {
        for r in 0..b.h {
            for c in 0..gw {
                if dark(px(r, c, 0)) {
                    fb.set(gx + c + dx, gy + r + dx, colour);
                }
            }
        }
    }
}

/// `DrawFocusRect`: a dotted rectangle;/// `DrawFocusRect`: a dotted rectangle; the dots follow a checkerboard anchored at the origin `(wx, wy)` of the window it is drawn in
/// (lit where the offsets from that origin add up to an odd number); the corners stay dark.
pub fn focus_rect(fb: &mut Fb, x: i32, y: i32, w: i32, h: i32, wx: i32, wy: i32) {
    // the rectangle XORs the pixels: the lit ones go black; over the 16-colour green the others turn olive (index 2 ^ 1 = 3)
    let dot = |fb: &mut Fb, px: i32, py: i32| {
        if (px - wx + py - wy).rem_euclid(2) == 1 {
            fb.set(px, py, BLACK);
        } else if fb.get(px, py) == GREEN.pack() {
            fb.set(px, py, Rgb(128, 128, 0));
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

/// The desktop colour of the original install.
pub const DESKTOP: Rgb = GREEN;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_button_has_a_thick_frame() {
        let mut fb = Fb::new();
        button_face(&mut fb, 10, 10, 64, 64, true);
        assert_eq!(fb.get(10, 40), 0x000000); // frame column 0
        assert_eq!(fb.get(11, 40), 0x000000); // frame column 1
        assert_eq!(fb.get(12, 40), 0xf8fcf8); // white bevel
        button_face(&mut fb, 100, 10, 64, 64, false);
        assert_eq!(fb.get(100, 40), 0x000000);
        assert_eq!(fb.get(101, 40), 0xf8fcf8);
        assert_eq!(fb.get(100, 10), 0xc0c0c0); // cut corner
    }

    #[test]
    fn the_focus_rectangle_is_dotted_without_corners() {
        let mut fb = Fb::new();
        fb.fill(0, 0, 50, 50, SILVER);
        focus_rect(&mut fb, 10, 10, 16, 13, 10, 10);
        assert_eq!(fb.get(10, 10), 0xc0c0c0);
        assert_eq!(fb.get(11, 10), 0x000000); // offsets add up to an odd number: lit
        assert_eq!(fb.get(12, 10), 0xc0c0c0);
        assert_eq!(fb.get(10, 11), 0x000000);
    }

    #[test]
    fn labels_wrap_at_spaces() {
        // a tiny hand-made font: every glyph 3 wide
        let skin_bytes = {
            let mut b = b"EFSK".to_vec();
            b.extend_from_slice(&1u16.to_le_bytes());
            b.push(1);
            b.push(b'x');
            b.extend_from_slice(&[10, 0, 0, 8, 2, 32, 1, 1, 10]);
            for _ in 32..256 {
                b.extend_from_slice(&[3, 0, 8, 0, 0]);
            }
            b
        };
        let mut g = b"EFGL".to_vec();
        g.extend_from_slice(&0u16.to_le_bytes());
        let skin = Skin::from_bytes(&skin_bytes, &g).unwrap();
        let f = skin.font("x", 10, false, false).unwrap();
        assert_eq!(wrap(f, "ab cd ef", 9), vec!["ab", "cd", "ef"]);
        assert_eq!(wrap(f, "ab cd ef", 20), vec!["ab cd", "ef"]);
    }
}
