//! Message boxes and the small dialogs without a group of their own: options (`TOPTIONSDLG`), stadium (`TSTADIUMDLG`) and loan
//! (`TLOANDLG`).
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::main_win::human_club;
use crate::classic::options::{self, Options};
use crate::classic::{text, Classic, Key};
use crate::win31::draw::wrap;
use crate::win31::fb::{Fb, Rgb, BLACK, GRAY, GREEN, WHITE, YELLOW};
use crate::win31::Skin;
use rustfoot_core::finance;

// ---------------------------------------------------------------------------------------------------------------------------------
// Message boxes

/// Delphi 1's message form (`Dialogs.pas`): margins and spacings of 8, buttons 77x27, the text wrapped at half the screen's width.
const MARGIN: i32 = 8;
const BUTTON: (i32, i32) = (77, 27);
const WRAP_AT: i32 = 400;

/// A message box with an OK button (the VCL's `ShowMessage`). It borrows the controls of `TCANTSAVETEAMDLG` (a label and the OK button)
/// and lays them out in [`bind`].
pub fn message(caption: &str, text: &str) -> Dialog {
    let mut d = Dialog::new(Kind::Message, "TCANTSAVETEAMDLG");
    d.text("@caption", caption);
    d.st.insert("text".into(), text.into());
    d.focus = "OKBtn".into();
    d
}

/// The box around `text` in a font of `width(s)` / `line_h`: the wrapped lines and the client size.
pub fn message_layout(
    text: &str,
    width: &dyn Fn(&str) -> i32,
    line_h: i32,
    wrap_at: i32,
    wrap_fn: &dyn Fn(&str, i32) -> Vec<String>,
) -> (Vec<String>, i32, i32) {
    let lines = wrap_fn(text, wrap_at);
    let tw = lines.iter().map(|l| width(l)).max().unwrap_or(0);
    let cw = (tw + 2 * MARGIN).max(BUTTON.0 + 2 * MARGIN);
    let ch = MARGIN + lines.len() as i32 * line_h + MARGIN + BUTTON.1 + MARGIN;
    (lines, cw, ch)
}

fn message_bind(c: &Classic, d: &mut Dialog) {
    let text = d.st.get("text").cloned().unwrap_or_default();
    let font = c.skin.font("ms sans serif", 11, true, false);
    let lh = font.map_or(13, |f| f.height());
    let (lines, cw, ch) = message_layout(
        &text,
        &|s| font.map_or(s.chars().count() as i32 * 6, |f| f.width(s)),
        lh,
        WRAP_AT,
        &|s, w| match font {
            Some(f) => wrap(f, s, w),
            None => s.split('\n').map(str::to_string).collect(),
        },
    );
    d.text("@client", format!("{cw}x{ch}"));
    d.text("Label1", lines.join("\n"));
    d.text("Label1.pos", format!("{MARGIN},{MARGIN}"));
    let by = MARGIN + lines.len() as i32 * lh + MARGIN;
    d.text("OKBtn.pos", format!("{},{by}", (cw - BUTTON.0) / 2));
    for hidden in ["Bevel1", "listOfReasons"] {
        d.text(format!("{hidden}.visible"), "0");
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Options

/// The combo boxes of the options dialog: control, working-value key, items.
fn combos() -> Vec<(&'static str, &'static str, Vec<String>)> {
    let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    vec![
        (
            "ComboViewPsichologicalLashes",
            "lashes",
            v(&options::LASHES),
        ),
        ("comboSaveGameWhen", "save", v(&options::SAVE_WHEN)),
        ("comboViewGetTeams", "cup", v(&options::CUP_DRAW)),
        ("ComboSound", "sound", v(&options::SOUND)),
        (
            "comboGameTimeProfile",
            "profile",
            options::PROFILES.iter().map(|p| p.0.to_string()).collect(),
        ),
        ("comboHalfTimeSubstitutions", "half", v(&options::NO_YES)),
        ("comboViewComputerPenalties", "pens", v(&options::NO_YES)),
    ]
}

/// The working values the dialog edits, copied from the options and written back on OK.
fn load_working(o: &Options, d: &mut Dialog) {
    d.set_num("lashes", o.lashes as i64);
    d.set_num("save", o.save_when as i64);
    d.set_num("cup", o.cup_draw as i64);
    d.set_num("sound", o.sound_off as i64);
    d.set_num("profile", o.profile as i64);
    d.set_num("half", o.half_time_subs as i64);
    d.set_num("pens", o.view_computer_penalties as i64);
}

/// The options the working values of the dialog stand for.
pub fn working_options(d: &Dialog) -> Options {
    let g = |k: &str, max: i64| d.num(k).clamp(0, max) as u8;
    Options {
        lashes: g("lashes", 2),
        save_when: g("save", 2),
        cup_draw: g("cup", 2),
        sound_off: d.num("sound") == 1,
        profile: g("profile", options::PROFILES.len() as i64 - 1),
        half_time_subs: d.num("half") == 1,
        view_computer_penalties: d.num("pens") == 1,
    }
}

fn options_bind(c: &Classic, d: &mut Dialog) {
    if !d.st.contains_key("init") {
        d.st.insert("init".into(), "1".into());
        load_working(&c.opts, d);
    }
    // the "Controle" page belongs to the registered versions (`1028:2e0c` removes it): only two tabs
    d.text("TabbedNotebook1.tabs", "Geral|Jogo");
    d.text("TabbedNotebook1.page", d.num("page").to_string());
    let open = d.st.get("open").cloned().unwrap_or_default();
    for (name, key, items) in combos() {
        let sel = (d.num(key).max(0) as usize).min(items.len() - 1);
        d.text(format!("{name}.text"), items[sel].clone());
        d.text(format!("{name}.sel"), sel.to_string());
        d.text(format!("{name}.items"), items.join("\n"));
        d.text(format!("{name}.open"), if open == name { "1" } else { "0" });
    }
}

fn options_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if let Some(i) = name
        .strip_prefix("TabbedNotebook1#")
        .or_else(|| name.strip_prefix("TabSet#"))
        .and_then(|i| i.parse::<i64>().ok())
    {
        d.set_num("page", i.clamp(0, 1));
        d.st.remove("open");
        return Outcome::Keep;
    }
    match name {
        "OKBtn" => {
            c.opts = working_options(d);
            return Outcome::Close;
        }
        "CancelBtn" => return Outcome::Close,
        _ => {}
    }
    for (combo, key, items) in combos() {
        if name == combo {
            let was = d.st.get("open").map(String::as_str) == Some(combo);
            d.st.remove("open");
            if !was {
                d.st.insert("open".into(), combo.into());
            }
            return Outcome::Keep;
        }
        if let Some(i) = name
            .strip_prefix(combo)
            .and_then(|r| r.strip_prefix('#'))
            .and_then(|i| i.parse::<usize>().ok())
        {
            if i < items.len() {
                d.set_num(key, i as i64);
            }
            d.st.remove("open");
            return Outcome::Keep;
        }
    }
    d.st.remove("open");
    Outcome::Keep
}

fn options_key(d: &mut Dialog, key: Key) -> Option<Outcome> {
    let combo = combos().into_iter().find(|(n, ..)| *n == d.focus)?;
    let (_, k, items) = combo;
    let cur = d.num(k).max(0) as usize;
    let next = match key {
        Key::Down | Key::Right => (cur + 1).min(items.len() - 1),
        Key::Up | Key::Left => cur.saturating_sub(1),
        _ => return None,
    };
    d.set_num(k, next as i64);
    Some(Outcome::Keep)
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Stadium

fn money(v: i64) -> String {
    text::money_words(v, "reais")
}

fn stadium_bind(c: &Classic, d: &mut Dialog) {
    let Some((_, club)) = human_club(c) else {
        return;
    };
    let f = &club.fin;
    d.set_num("level", f.stadium as i64);
    d.text("textCapacity", format!("{} lugares", f.capacity()));
    let cost = finance::expansion_cost(f, c.world.economy, c.world.scale);
    d.text(
        "labelBenchPrice",
        format!(
            "Preço de uma bancada com\n{} lugares: {}",
            finance::SEATS_PER_LEVEL,
            money(cost)
        ),
    );
    // the loan button belongs to the registered versions (`1030:28a3`)
    d.text("loanBtn.visible", "0");
    let can = f.stadium < finance::MAX_STADIUM_LEVEL && cost <= f.cash;
    d.text("constructBtn.enabled", if can { "1" } else { "0" });
    let built = d.st.contains_key("built");
    d.text("labelWarning.visible", if built { "1" } else { "0" });
    let Rgb(r, g, b) = WHITE;
    d.text("labelWarning.bgcolor", format!("{r},{g},{b}"));
}

fn stadium_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "BitBtn1" => Outcome::Close,
        "constructBtn" => match c.world.human_expand() {
            Ok(_) => {
                d.st.insert("built".into(), "1".into());
                Outcome::Keep
            }
            Err(e) => Outcome::Open(Box::new(message("Estádio", &e))),
        },
        _ => Outcome::Keep,
    }
}

/// A filled rectangle with an outline of `w` pixels.
fn rect(fb: &mut Fb, (x, y, wd, ht): (i32, i32, i32, i32), fill: Rgb, line: Rgb, w: i32) {
    fb.fill(x, y, wd, ht, line);
    fb.fill(x + w, y + w, wd - 2 * w, ht - 2 * w, fill);
}

fn circle(fb: &mut Fb, cx: i32, cy: i32, r: i32, c: Rgb) {
    let (mut x, mut y, mut e) = (r, 0, 1 - r);
    while x >= y {
        for (dx, dy) in [
            (x, y),
            (y, x),
            (-y, x),
            (-x, y),
            (-x, -y),
            (-y, -x),
            (y, -x),
            (x, -y),
        ] {
            fb.set(cx + dx, cy + dy, c);
        }
        y += 1;
        if e < 0 {
            e += 2 * y + 1;
        } else {
            x -= 1;
            e += 2 * (y - x) + 1;
        }
    }
}

/// The rings of stands around the pitch for a stadium of `level` (1..=24): three tiers of 6, 8 and 10 steps (`PaintBoxStadiumPaint`
/// draws `level`, `level - 6` and `level - 14` steps of a ring, the outer ones only once the inner are complete). Returns the number
/// of steps of each tier, inner first.
pub fn tiers(level: i64) -> [i64; 3] {
    [
        level.clamp(0, 6),
        (level - 6).clamp(0, 8),
        (level - 14).clamp(0, 10),
    ]
}

/// Paints the `TPaintBox` of the stadium form: yellow-outlined grey stands growing with the level around the green pitch with its white
/// markings.
pub fn paint_stadium(d: &Dialog, fb: &mut Fb, _skin: &Skin, ox: i32, oy: i32) {
    let (bx, by, bw, bh) = (ox + 71, oy + 8, 332, 241);
    let level = d.num("level");
    let (px, py, pw, ph) = (bx + bw / 2 - 70, by + bh / 2 - 40, 140, 80);
    let light = Rgb(255, 255, 128);
    let t = tiers(level);
    // each step adds 3 pixels at the sides and 2 above and below; the tiers stack outwards
    let grow = |steps: i64| ((steps * 3) as i32, (steps * 2) as i32);
    let tier_steps = [t[0], 6 + t[1], 14 + t[2]];
    for k in (0..3).rev() {
        if t[k] == 0 {
            continue;
        }
        let (gx, gy) = grow(tier_steps[k]);
        rect(
            fb,
            (px - gx, py - gy, pw + 2 * gx, ph + 2 * gy),
            GRAY,
            light,
            2,
        );
    }
    rect(fb, (px, py, pw, ph), GREEN, light, 2);
    // markings
    let (lx, ly) = (px + 6, py + 6);
    rect(fb, (lx, ly, pw - 12, ph - 12), GREEN, WHITE, 1);
    fb.fill(px + pw / 2, ly, 1, ph - 12, WHITE);
    circle(fb, px + pw / 2, py + ph / 2, 10, WHITE);
    for side in [0, 1] {
        let x = if side == 0 { lx } else { px + pw - 6 - 16 };
        rect(fb, (x, py + ph / 2 - 18, 17, 36), GREEN, WHITE, 1);
    }
    let _ = (BLACK, YELLOW);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Loan

fn loan_bind(c: &Classic, d: &mut Dialog) {
    let Some((_, club)) = human_club(c) else {
        return;
    };
    let f = &club.fin;
    let (e, g) = (c.world.economy, c.world.scale);
    let step = finance::loan_step(g);
    let max = finance::max_loan(finance::rating(&club.slots), club.k, e, g).max(f.loan);
    d.text("textLoan", money(f.loan));
    d.text("textInterest", money(finance::loan_interest(f.loan)));
    d.text("textMoney", money(f.cash));
    d.text("borrowBtn", money(step));
    d.text("payBtn", money(step));
    d.text("borrowBtn.enabled", if f.loan < max { "1" } else { "0" });
    d.text(
        "payBtn.enabled",
        if f.loan > 0 && step < f.cash {
            "1"
        } else {
            "0"
        },
    );
}

fn loan_click(c: &mut Classic, _d: &mut Dialog, name: &str) -> Outcome {
    let res = match name {
        "closeBtn" => return Outcome::Close,
        "borrowBtn" => c.world.human_borrow(),
        "payBtn" => c.world.human_repay(),
        _ => return Outcome::Keep,
    };
    match res {
        Err(e) => Outcome::Open(Box::new(message("Empréstimos", &e))),
        Ok(_) => Outcome::Keep,
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------

pub fn bind(c: &Classic, d: &mut Dialog) {
    match d.kind {
        Kind::Message => message_bind(c, d),
        Kind::Options => options_bind(c, d),
        Kind::Stadium => stadium_bind(c, d),
        Kind::Loan => loan_bind(c, d),
        _ => {}
    }
}

pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match d.kind {
        Kind::Options => options_click(c, d, name),
        Kind::Stadium => stadium_click(c, d, name),
        Kind::Loan => loan_click(c, d, name),
        _ => Outcome::Close,
    }
}

pub fn key(_c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    match d.kind {
        Kind::Options => options_key(d, key),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_boxes_wrap_and_size_like_delphi() {
        // every glyph 6 wide, line 13 high
        let width = |s: &str| s.chars().count() as i32 * 6;
        let wrap_fn = |s: &str, w: i32| {
            let mut lines = vec![String::new()];
            for word in s.split(' ') {
                let l = lines.last_mut().unwrap();
                let trial = if l.is_empty() {
                    word.to_string()
                } else {
                    format!("{l} {word}")
                };
                if width(&trial) <= w || l.is_empty() {
                    *l = trial;
                } else {
                    lines.push(word.to_string());
                }
            }
            lines
        };
        let (lines, cw, ch) = message_layout("ola", &width, 13, 400, &wrap_fn);
        assert_eq!(lines, vec!["ola"]);
        assert_eq!(cw, 77 + 16); // the button is wider than the text
        assert_eq!(ch, 8 + 13 + 8 + 27 + 8);
        let long = "palavra ".repeat(30);
        let (lines, cw, _) = message_layout(long.trim(), &width, 13, 400, &wrap_fn);
        assert!(lines.len() > 1);
        assert!(cw <= 400 + 16);
    }

    #[test]
    fn stadium_tiers_fill_inner_first() {
        assert_eq!(tiers(1), [1, 0, 0]);
        assert_eq!(tiers(6), [6, 0, 0]);
        assert_eq!(tiers(7), [6, 1, 0]);
        assert_eq!(tiers(15), [6, 8, 1]);
        assert_eq!(tiers(24), [6, 8, 10]);
    }

    #[test]
    fn working_options_round_trip() {
        let o = Options {
            lashes: 0,
            save_when: 2,
            cup_draw: 2,
            sound_off: false,
            profile: 0,
            half_time_subs: false,
            view_computer_penalties: true,
        };
        let mut d = Dialog::new(Kind::Options, "TOPTIONSDLG");
        load_working(&o, &mut d);
        assert_eq!(working_options(&d), o);
    }
}
