//! The start of the game: the choice between a new and a saved game (`TSELECTGAMEDLG`), the gauge that shows the game being built or
//! read (`TPROGRESSDLG`) and the players' names with the draw of their clubs (`TGETTEAMSDLG`).
//!
//! The order is the original's (`1020:24d6`): About, select game; a saved game is read (`1020:1d8d`) and played; a new game is built
//! (`1028:05da`), then the names are typed and "Sortear equipas" gives each name a club (`1028:0b26`: a random one of the bottom six of
//! the 4th division, `World::pick_human`) and OK starts the game. Up to six names can be typed: every named coach gets a club
//! (`World::name_humans`) and the game is played in hot seat (the humans prepare the round in turn, see `docs/matchday.md`).
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::{Classic, Key, Request};
use crate::win31::fb::{Rgb, WHITE};
use rustfoot_core::world::World;

const NAVY: &str = "0,0,128";
const YELLOW: &str = "255,255,0";

/// How long the gauge takes to fill (the original shows it while the files are read).
const PROGRESS_MS: u32 = 1200;

pub fn open_select() -> Dialog {
    let mut d = Dialog::new(Kind::SelectGame, "TSELECTGAMEDLG");
    d.focus = "CancelBtn".into();
    d
}

/// The gauge dialog for `job`: `new:NAME` builds the world, `load:NAME` asks the host for the saved text.
pub fn open_progress(job: &str) -> Dialog {
    let mut d = Dialog::new(Kind::Progress, "TPROGRESSDLG");
    d.text(
        "@caption",
        if job == "save" {
            "A gravar o jogo..."
        } else {
            "A carregar o jogo..."
        },
    );
    // the window is a little larger than the form's client area (measured on the capture of the original)
    d.text("@client", "481x37");
    d.st.insert("job".into(), job.to_string());
    d
}

pub fn open_players() -> Dialog {
    let mut d = Dialog::new(Kind::CoachName, "TGETTEAMSDLG");
    d.focus = "playerName1".into();
    d
}

/// A file name of a saved game: 1 to 8 letters or digits (or `_`), upper case as the edit forces.
pub fn clean_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .map(|c| c.to_ascii_uppercase())
        .take(8)
        .collect()
}

/// The saved games as the list shows them (sorted).
pub fn sorted_saves(c: &Classic) -> Vec<String> {
    let mut v = c.data.saves.clone();
    v.sort();
    v
}

fn mode(d: &Dialog) -> &str {
    d.st.get("mode").map_or("", String::as_str)
}

fn select_bind(c: &Classic, d: &mut Dialog) {
    d.text("@caption", "Elifoot 98");
    // typing in the file name field chooses a new game (`textGameNameChange` / `textGameNameClick`)
    let typed = clean_name(d.texts.get("textGameName").map_or("", String::as_str));
    if d.texts.get("textGameName") != Some(&typed) {
        d.text("textGameName", typed.clone());
    }
    if d.st.get("last_name").map_or("", String::as_str) != typed {
        d.st.insert("last_name".into(), typed.clone());
        d.st.insert("mode".into(), "new".into());
        d.st.remove("warn");
    }
    let (new, cont) = (mode(d) == "new", mode(d) == "continue");
    let saves = sorted_saves(c);
    let sel =
        d.st.get("sel")
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|&i| i < saves.len());
    d.text("RadioNewGame.checked", if new { "1" } else { "0" });
    d.text("RadioContinueGame.checked", if cont { "1" } else { "0" });
    // the chosen radio button is yellow, the other navy (`1028:23ed`)
    d.text("RadioNewGame.color", if new { YELLOW } else { NAVY });
    d.text("RadioContinueGame.color", if cont { YELLOW } else { NAVY });
    match sel {
        Some(i) => d.text("ListBoxGames.sel", i.to_string()),
        None => {
            d.texts.remove("ListBoxGames.sel");
        }
    }
    let desc = match (cont, sel) {
        (true, Some(i)) => c.data.save_desc.get(&saves[i]).cloned().unwrap_or_default(),
        _ => String::new(),
    };
    d.lists.insert("ListBoxGames".into(), saves);
    d.text("labelGameDescription", desc);
    let ok = (new && !typed.is_empty()) || (cont && sel.is_some());
    d.text("OKBtn.enabled", if ok { "1" } else { "0" });
    let warn = d.st.contains_key("warn");
    d.text("labelWarning.visible", if warn { "1" } else { "0" });
    let Rgb(r, g, b) = WHITE;
    d.text("labelWarning.bgcolor", format!("{r},{g},{b}"));
}

/// The names typed in the dialog, in order, without the empty ones.
fn typed_names(d: &Dialog) -> Vec<String> {
    (1..=6)
        .map(|i| {
            d.texts
                .get(&format!("playerName{i}"))
                .map_or("", |s| s.trim())
                .to_string()
        })
        .filter(|n| !n.is_empty())
        .collect()
}

fn players_bind(c: &Classic, d: &mut Dialog) {
    let drawn = d.st.contains_key("drawn");
    for i in 1..=6 {
        d.text(
            format!("playerName{i}.enabled"),
            if drawn { "0" } else { "1" },
        );
    }
    if drawn {
        d.text("Button1", "OK");
        let col = |v: [u8; 3]| format!("{},{},{}", v[0], v[1], v[2]);
        // the seats in the order the names were typed
        for (k, h) in c.world.humans().enumerate() {
            let club = c
                .world
                .divisions
                .iter()
                .flatten()
                .chain(c.world.pool.iter())
                .find(|x| x.name == h.club);
            if let Some(club) = club {
                let n = k + 1;
                d.text(format!("playerTeam{n}"), club.full_name.clone());
                d.text(format!("playerTeam{n}.color"), col(club.colors.0));
                d.text(format!("playerTeam{n}.bgcolor"), col(club.colors.1));
            }
        }
    }
}

pub fn bind(c: &Classic, d: &mut Dialog) {
    match d.kind {
        Kind::SelectGame => select_bind(c, d),
        Kind::Progress => {
            d.text("gaugeProgress.progress", progress(d).to_string());
        }
        _ => players_bind(c, d),
    }
}

fn progress(d: &Dialog) -> i64 {
    (d.num("ms") * 100 / PROGRESS_MS as i64).clamp(0, 100)
}

/// The row of the list of games a click chose: from `ListBoxGames#row`, or from the mouse when the list box itself was hit.
fn list_row(c: &Classic, d: &Dialog, name: &str) -> Option<usize> {
    if let Some(i) = name
        .strip_prefix("ListBoxGames#")
        .and_then(|i| i.parse().ok())
    {
        return Some(i);
    }
    let h = d.hits.iter().find(|h| h.name == "ListBoxGames")?;
    let ih = c
        .forms
        .get(&d.form)
        .and_then(|f| f.find("ListBoxGames"))
        .map_or(16, |n| n.int("ItemHeight", 16))
        .max(1);
    let row = (c.mouse.1 - h.y - 1) / ih;
    (row >= 0).then_some(row as usize)
}

pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match d.kind {
        Kind::SelectGame => select_click(c, d, name),
        Kind::CoachName => players_click(c, d, name),
        _ => Outcome::Keep,
    }
}

fn select_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let saves = sorted_saves(c);
    match name {
        "RadioNewGame" => {
            d.st.insert("mode".into(), "new".into());
            d.focus = "textGameName".into();
        }
        "RadioContinueGame" => {
            d.st.insert("mode".into(), "continue".into());
            d.focus = "ListBoxGames".into();
        }
        // a click in the file name field chooses a new game (`textGameNameClick`); getting there with Tab does not
        "textGameName" => {
            d.st.insert("mode".into(), "new".into());
            d.st.remove("warn");
        }
        n if n == "ListBoxGames" || n.starts_with("ListBoxGames#") => {
            if let Some(i) = list_row(c, d, n).filter(|&i| i < saves.len()) {
                d.st.insert("sel".into(), i.to_string());
                d.st.insert("mode".into(), "continue".into());
                d.st.remove("warn");
            }
        }
        "CancelBtn" => {
            c.requests.push(Request::Quit);
            return Outcome::Close;
        }
        "OKBtn" => {
            if mode(d) == "new" {
                let name = clean_name(d.texts.get("textGameName").map_or("", String::as_str));
                if name.is_empty() {
                    d.st.insert("warn".into(), "1".into());
                    return Outcome::Keep;
                }
                return Outcome::Replace(Box::new(open_progress(&format!("new:{name}"))));
            }
            let sel = d.st.get("sel").and_then(|s| s.parse::<usize>().ok());
            if mode(d) == "continue" {
                if let Some(n) = sel.and_then(|i| saves.get(i)) {
                    return Outcome::Replace(Box::new(open_progress(&format!("load:{n}"))));
                }
            }
        }
        _ => {}
    }
    Outcome::Keep
}

fn players_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if name != "Button1" {
        return Outcome::Keep;
    }
    let names = typed_names(d);
    if d.st.contains_key("drawn") {
        // OK: the game starts with the first player's main window
        c.coach = names.first().cloned().unwrap_or_default();
        c.enter_game();
        return Outcome::Close;
    }
    // "Sortear equipas": nothing happens until a name is typed
    if !names.is_empty() {
        c.world.name_humans(&names);
        d.st.insert("drawn".into(), "1".into());
        d.focus = "Button1".into();
    }
    Outcome::Keep
}

pub fn key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    match (&d.kind, key) {
        (Kind::SelectGame, Key::Up | Key::Down) => {
            let n = sorted_saves(c).len();
            if n == 0 || d.focus != "ListBoxGames" {
                return None;
            }
            let cur = d.st.get("sel").and_then(|s| s.parse::<usize>().ok());
            let next = match (cur, key) {
                (None, _) => 0,
                (Some(i), Key::Down) => (i + 1).min(n - 1),
                (Some(i), _) => i.saturating_sub(1),
            };
            d.st.insert("sel".into(), next.to_string());
            d.st.insert("mode".into(), "continue".into());
            Some(Outcome::Keep)
        }
        (Kind::CoachName, Key::Enter) if d.focus.starts_with("playerName") => {
            d.focus = "Button1".into();
            Some(Outcome::Keep)
        }
        _ => None,
    }
}

/// Builds the world of a new game from the pack's teams (invented clubs without any).
pub fn build_world(c: &Classic) -> Result<World, String> {
    if c.teams.is_empty() {
        Ok(World::synthetic(c.seed))
    } else {
        World::from_teams(c.seed, &c.teams)
    }
}

pub fn tick(c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    if d.kind != Kind::Progress {
        return Outcome::Keep;
    }
    d.set_num("ms", d.num("ms") + ms as i64);
    c.dirty = true;
    if progress(d) < 100 {
        return Outcome::Keep;
    }
    let job = d.st.get("job").cloned().unwrap_or_default();
    if job == "save" {
        // the gauge is full: the host stores the game and the day is played
        c.save_now();
        c.play_round();
        return Outcome::Close;
    }
    if let Some(name) = job.strip_prefix("new:") {
        return match build_world(c) {
            Ok(w) => {
                c.world = w;
                c.game_name = name.to_string();
                c.opts = Default::default();
                Outcome::Replace(Box::new(open_players()))
            }
            Err(e) => {
                // back to the choice, with the error over it
                c.dialogs.push(open_select());
                Outcome::Replace(Box::new(super::message("Erro", &e)))
            }
        };
    }
    // a saved game: the host answers the request with the text (`Classic::load_text`), which closes this dialog
    if let Some(name) = job.strip_prefix("load:") {
        if !d.st.contains_key("waiting") {
            d.st.insert("waiting".into(), "1".into());
            c.loading = Some(name.to_string());
            c.requests.push(Request::Load(name.to_string()));
        }
    }
    Outcome::Keep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_coach_gets_a_club_and_the_game_starts_in_hot_seat() {
        use crate::classic::Data;
        use crate::win31::{Forms, Skin};
        let w = rustfoot_core::world::World::synthetic(3);
        let mut c = Classic::new(w, Skin::default(), Forms::default(), Data::default());
        let mut d = open_players();
        for (i, n) in [(1, "Ana"), (2, ""), (3, " Rui "), (4, "Zé")] {
            d.texts.insert(format!("playerName{i}"), n.into());
        }
        assert!(matches!(
            players_click(&mut c, &mut d, "Button1"),
            Outcome::Keep
        ));
        assert_eq!(c.world.human_count(), 3, "the empty name gets no club");
        let mut bound = open_players();
        bound.st.insert("drawn".into(), "1".into());
        bind(&c, &mut bound);
        assert!((1..=3).all(|k| bound.texts.contains_key(&format!("playerTeam{k}"))));
        assert!(!bound.texts.contains_key("playerTeam4"));
        assert!(matches!(
            players_click(&mut c, &mut d, "Button1"),
            Outcome::Close
        ));
        assert_eq!((c.coach.as_str(), c.world.active_seat()), ("Ana", 0));
        assert_eq!(
            c.world
                .humans()
                .map(|h| h.coach.clone())
                .collect::<Vec<_>>(),
            ["Ana", "Rui", "Zé"]
        );
    }

    #[test]
    fn file_names_are_eight_upper_case_letters_or_digits() {
        assert_eq!(clean_name("abc-de.fg h1"), "ABCDEFGH");
        assert_eq!(clean_name("abcdefghij"), "ABCDEFGH");
        assert_eq!(clean_name("é!"), "");
        assert_eq!(clean_name("jogo_1"), "JOGO_1");
    }
}
