//! The About box (`TABOUTDLG`): game name and version, authors, the scrolling banner of thanks and the blinking "unregistered" notice.
//!
//! Behaviour from the decompiled code: `FormShow` (`1048:3c41`) fills the labels; `Timer1` (500 ms, `1048:3e53`) toggles the visibility of
//! the registration notice while the copy is unregistered; the banner scrolls by `ScrollPixels` every `ScrollInterval` ms by itself. The
//! registration (`TCodeDlg`) is not part of the port: the "Registar" button is drawn but does nothing.
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::Classic;
use crate::win31::{Forms, Skin};

/// The version label of the capture of the original (`98.003`).
pub const VERSION: &str = "98.003";
/// The registration notice of a copy that is not registered (`10c8:12b2`).
pub const NOT_REGISTERED: &str = "Cópia não registada";

/// The box from the menu "Acerca...".
pub fn open() -> Dialog {
    let mut d = Dialog::new(Kind::About, "TABOUTDLG");
    d.focus = "OKBtn".into();
    d
}

/// The box shown when the program starts: OK leads on to the choice of game.
pub fn open_startup() -> Dialog {
    let mut d = open();
    d.st.insert("startup".into(), "1".into());
    d
}

pub fn bind(_c: &Classic, d: &mut Dialog) {
    d.text("textGameName", "Elifoot 98");
    d.text("textSubVersion", VERSION);
    d.text("labelNotRegistered", NOT_REGISTERED);
    let shown = d.st.get("blink").map(String::as_str) != Some("off");
    d.text("labelNotRegistered.visible", if shown { "1" } else { "0" });
    d.text("labelWarning.visible", "0");
    d.text("Banner1.scroll", d.num("scroll").to_string());
}

pub fn click(_c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "OKBtn" if d.st.contains_key("startup") => {
            Outcome::Replace(Box::new(super::newgame::open_select()))
        }
        "OKBtn" => Outcome::Close,
        // the registration is not part of the port
        _ => Outcome::Keep,
    }
}

/// Width in pixels of the whole banner text (its font is the form's: Arial 13 bold).
fn banner_width(skin: &Skin, text: &str) -> i32 {
    skin.font("arial", 13, true, false)
        .map_or(0, |f| f.width(text))
}

pub fn tick(c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    tick_box(&c.skin, &c.forms, d, ms);
    c.dirty = true;
    Outcome::Keep
}

/// The banner and the blinking notice after `ms` more milliseconds (the game's box and the team editor's).
pub fn tick_box(skin: &Skin, forms: &Forms, d: &mut Dialog, ms: u32) {
    let (interval, pixels, width, text) = {
        let b = forms.get(&d.form).and_then(|f| f.find("Banner1"));
        (
            b.map_or(30, |b| b.int("ScrollInterval", 30)).max(1) as u32,
            b.map_or(1, |b| b.int("ScrollPixels", 1)),
            b.map_or(513, |b| b.int("Width", 513)),
            b.and_then(|b| b.str("ScrollText"))
                .unwrap_or("")
                .to_string(),
        )
    };
    let total = width + banner_width(skin, &text);
    let (scroll, rest) = advance_banner(
        d.num("scroll") as i32,
        d.num("scroll_ms") as u32 + ms,
        interval,
        pixels,
        total,
    );
    d.set_num("scroll", scroll as i64);
    d.set_num("scroll_ms", rest as i64);
    // Timer1: every 500 ms the notice of an unregistered copy appears or disappears
    let t = d.num("blink_ms") + ms as i64;
    d.set_num("blink_ms", t % 500);
    if t >= 500 {
        let was_on = d.st.get("blink").map(String::as_str) != Some("off");
        d.st.insert(
            "blink".into(),
            if was_on { "off" } else { "on" }.to_string(),
        );
    }
}

/// The banner's scroll position after `ms` more milliseconds (including what was left over): one step of `pixels` per `interval`;
/// when the text has gone by completely (`total` pixels) it starts again from the right edge. Returns the position and the leftover.
pub fn advance_banner(scroll: i32, ms: u32, interval: u32, pixels: i32, total: i32) -> (i32, u32) {
    let steps = (ms / interval) as i32;
    let mut s = scroll + steps * pixels;
    if total > 0 && s > total {
        s %= total;
    }
    (s, ms % interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_scrolls_and_wraps() {
        assert_eq!(advance_banner(0, 95, 30, 1, 1000), (3, 5));
        assert_eq!(advance_banner(998, 90, 30, 1, 1000), (1, 0));
        assert_eq!(advance_banner(5, 29, 30, 1, 1000), (5, 29));
    }
}
