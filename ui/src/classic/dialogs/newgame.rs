//! The start of the game: the choice between a new and a saved game (`TSELECTGAMEDLG`), the gauge that shows the game being built or
//! read (`TPROGRESSDLG`) and the players' names with the draw of their clubs (`TGETTEAMSDLG`).
//!
//! The order is the original's (`1020:24d6`): About, select game; a saved game is read (`1020:1d8d`) and played; for a new game the
//! countries are chosen (`TSelectCountriesDlg`, `1028:176b`; Cancel quits) and the currency (`TCurrencyDlg`, `1020:27de`), the game is
//! built under the gauge *A iniciar o jogo...* (`1028:05da`), then the names are typed and "Sortear equipas" gives each name a club (`1028:0b26`: a random one of the bottom six of
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
        } else if job.starts_with("new:") {
            "A iniciar o jogo..."
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
        Kind::SelectCountries => countries_bind(d),
        Kind::Currency => currency_bind(c, d),
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
        Kind::SelectCountries => countries_click(c, d, name),
        Kind::Currency => currency_click(c, d, name),
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
                if c.teams.is_empty() {
                    // invented clubs: nothing to choose
                    return Outcome::Replace(Box::new(open_progress(&format!("new:{name}"))));
                }
                return Outcome::Replace(Box::new(open_countries(c, &name)));
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

/// The choices of the new-game dialogs: the countries' list as `(code, chosen)` in the dialog's order and the currency (index into
/// [`CURRENCIES`]).
#[derive(Clone, Debug, Default)]
pub struct NewGame {
    pub countries: Vec<(String, bool)>,
    pub currency: Option<usize>,
}

/// The currencies of *Dinheiro* (the records at `DS:0062`, 101 bytes each).
pub const CURRENCIES: [&str; 5] = ["Escudos (contos)", "Euros", "Dólares", "Reais", "Libras"];

/// Fewest teams a game needs, and what *Selecção de Países* asks for (`1028:18d9` enables OK from 34).
const MIN_TEAMS: usize = rustfoot_core::world::MIN_TEAMS;

/// A sort key for the names of the sorted list (`Sorted`: the order of Windows 3.1's string comparison, where an accented letter comes
/// just before the same letter without accent: "África", "Áustria", "Alemanha").
fn collate(s: &str) -> Vec<(char, u8)> {
    s.chars()
        .map(|ch| {
            let low = ch.to_lowercase().next().unwrap_or(ch);
            let base = match low {
                'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ç' => 'c',
                other => other,
            };
            (base, (base == low) as u8)
        })
        .collect()
}

/// *Selecção de Países* (`1028:141b`): every country of the pack's teams with its number of teams, none chosen; rows
/// `code|name|count|chosen`.
pub fn open_countries(c: &Classic, game: &str) -> Dialog {
    let mut codes: Vec<String> = Vec::new();
    for t in &c.teams {
        if !codes.contains(&t.country) {
            codes.push(t.country.clone());
        }
    }
    let mut rows: Vec<(String, String, usize)> = codes
        .into_iter()
        .map(|code| {
            let n = c.teams.iter().filter(|t| t.country == code).count();
            let name = c
                .data
                .countries
                .get(&code)
                .cloned()
                .unwrap_or_else(|| code.clone());
            (code, name, n)
        })
        .collect();
    rows.sort_by_key(|r| collate(&r.1));
    let mut d = Dialog::new(Kind::SelectCountries, "TSELECTCOUNTRIESDLG");
    // the design size, not the design size plus the usual 4 (measured on the original: a 642x482 window)
    d.text("@client", "636x457");
    d.st.insert("game".into(), game.to_string());
    d.lists.insert(
        "ListBoxOfCountries".into(),
        rows.iter()
            .map(|(code, name, n)| format!("{code}|{name}|{n}|0"))
            .collect(),
    );
    d.set_num("sel", -1);
    d.set_num("top", 0);
    d.focus = "ListBoxOfCountries".into();
    d
}

/// `(code, teams, chosen)` of every row.
fn country_rows(d: &Dialog) -> Vec<(String, usize, bool)> {
    d.lists
        .get("ListBoxOfCountries")
        .map(|v| {
            v.iter()
                .map(|r| {
                    let c: Vec<&str> = r.split('|').collect();
                    (
                        c[0].to_string(),
                        c.get(2).and_then(|n| n.parse().ok()).unwrap_or(0),
                        c.get(3) == Some(&"1"),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn set_chosen(d: &mut Dialog, row: usize, on: bool) {
    if let Some(r) = d
        .lists
        .get_mut("ListBoxOfCountries")
        .and_then(|v| v.get_mut(row))
    {
        let keep = r
            .rsplit_once('|')
            .map_or(r.as_str(), |(a, _)| a)
            .to_string();
        *r = format!("{keep}|{}", on as u8);
    }
}

/// `1028:18d9`: the counts, OK from 34 teams on, the warning above 128.
fn countries_bind(d: &mut Dialog) {
    let rows = country_rows(d);
    let all: usize = rows.iter().map(|r| r.1).sum();
    let teams: usize = rows.iter().filter(|r| r.2).map(|r| r.1).sum();
    let countries = rows.iter().filter(|r| r.2).count();
    d.text("textAvailableTeams", all.to_string());
    d.text("textSelectedTeams", teams.to_string());
    d.text("textAvailableCountries", rows.len().to_string());
    d.text("textSelectedCountries", countries.to_string());
    d.text(
        "labelSelect",
        format!(
            "Seleccione os países com que pretende jogar, de modo a totalizar pelo menos {MIN_TEAMS} equipas."
        ),
    );
    d.text(
        "labelWarning",
        if teams > rustfoot_core::world::MAX_TEAMS {
            format!(
                "O Elifoot só comporta {} equipas.\nAlgumas das equipas dos países seleccionados não farão parte deste jogo.",
                rustfoot_core::world::MAX_TEAMS
            )
        } else {
            String::new()
        },
    );
    d.text("OKBtn.enabled", ((teams >= MIN_TEAMS) as u8).to_string());
    d.text("ListBoxOfCountries.sel", d.num("sel").to_string());
    d.text("ListBoxOfCountries.top", d.num("top").to_string());
}

/// Rows the list shows at once (405 pixels of 31-pixel rows).
const COUNTRY_ROWS: usize = 13;

fn countries_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let n = country_rows(d).len();
    let top = d.num("top").max(0) as usize;
    let max_top = n.saturating_sub(COUNTRY_ROWS);
    match name {
        // `ListBoxOfCountriesClick` (`1028:178c`): the row clicked is chosen or left out
        _ if name.starts_with("ListBoxOfCountries#") => {
            if let Some(i) = name
                .strip_prefix("ListBoxOfCountries#")
                .and_then(|i| i.parse::<usize>().ok())
            {
                let row = i;
                if let Some(r) = country_rows(d).get(row) {
                    set_chosen(d, row, !r.2);
                    d.set_num("sel", row as i64);
                }
            }
        }
        "ListBoxOfCountries.up" => d.set_num("top", top.saturating_sub(1) as i64),
        "ListBoxOfCountries.down" => d.set_num("top", (top + 1).min(max_top) as i64),
        "ListBoxOfCountries.pgup" => d.set_num("top", top.saturating_sub(COUNTRY_ROWS) as i64),
        "ListBoxOfCountries.pgdn" => d.set_num("top", (top + COUNTRY_ROWS).min(max_top) as i64),
        // `selectAllBtnClick` (`1028:1a7f`): every country
        "selectAllBtn" => {
            for row in 0..n {
                set_chosen(d, row, true);
            }
        }
        "OKBtn" => {
            let rows = country_rows(d);
            if rows.iter().filter(|r| r.2).map(|r| r.1).sum::<usize>() < MIN_TEAMS {
                return Outcome::Keep;
            }
            c.new_game.countries = rows.into_iter().map(|(code, _, on)| (code, on)).collect();
            let game = d.st.get("game").cloned().unwrap_or_default();
            return Outcome::Replace(Box::new(open_currency(&game)));
        }
        // the original leaves the program (`1020:24d6`)
        "CancelBtn" => {
            c.requests.push(Request::Quit);
            return Outcome::Close;
        }
        _ => {}
    }
    Outcome::Keep
}

/// *Dinheiro* (`1020:26b6`): the currency of the game, the last one chosen first (the original keeps it in `elif98.ini`).
pub fn open_currency(game: &str) -> Dialog {
    let mut d = Dialog::new(Kind::Currency, "TCURRENCYDLG");
    d.st.insert("game".into(), game.to_string());
    d.focus = "comboCurrency".into();
    d
}

fn currency_bind(c: &Classic, d: &mut Dialog) {
    if !d.st.contains_key("init") {
        d.st.insert("init".into(), "1".into());
        d.set_num("currency", c.new_game.currency.unwrap_or(3) as i64);
    }
    let sel = (d.num("currency").max(0) as usize).min(CURRENCIES.len() - 1);
    d.text("comboCurrency.items", CURRENCIES.join("\n"));
    d.text("comboCurrency.sel", sel.to_string());
    d.text("comboCurrency.text", CURRENCIES[sel]);
    let open = d.st.contains_key("open");
    d.text("comboCurrency.open", if open { "1" } else { "0" });
}

fn currency_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "comboCurrency" => {
            if d.st.remove("open").is_none() {
                d.st.insert("open".into(), "1".into());
            }
        }
        _ if name.starts_with("comboCurrency#") => {
            if let Some(i) = name
                .strip_prefix("comboCurrency#")
                .and_then(|i| i.parse::<usize>().ok())
                .filter(|&i| i < CURRENCIES.len())
            {
                d.set_num("currency", i as i64);
            }
            d.st.remove("open");
        }
        "OKBtn" => {
            c.new_game.currency = Some(d.num("currency").max(0) as usize);
            let game = d.st.get("game").cloned().unwrap_or_default();
            return Outcome::Replace(Box::new(open_progress(&format!("new:{game}"))));
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
        // the arrows move the current row (scrolling the list), the space bar chooses it or leaves it out
        (Kind::SelectCountries, Key::Up | Key::Down) => {
            let n = country_rows(d).len() as i64;
            if n == 0 {
                return None;
            }
            let cur = d.num("sel");
            let next = if key == Key::Down {
                (cur + 1).min(n - 1)
            } else {
                (cur - 1).max(0)
            };
            d.set_num("sel", next);
            let top = d.num("top");
            let rows = COUNTRY_ROWS as i64;
            d.set_num("top", top.min(next).max(next - rows + 1));
            Some(Outcome::Keep)
        }
        (Kind::SelectCountries, Key::Char(' ')) => {
            let row = d.num("sel");
            if let Some(r) = country_rows(d)
                .get(row.max(0) as usize)
                .filter(|_| row >= 0)
            {
                let on = !r.2;
                set_chosen(d, row as usize, on);
            }
            Some(Outcome::Keep)
        }
        (Kind::Currency, Key::Up | Key::Down) => {
            let cur = d.num("currency");
            let next = if key == Key::Down {
                (cur + 1).min(CURRENCIES.len() as i64 - 1)
            } else {
                (cur - 1).max(0)
            };
            d.set_num("currency", next);
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
    } else if !c.new_game.countries.is_empty() {
        World::from_countries(c.seed, &c.teams, &c.new_game.countries)
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

    fn team(i: usize, country: &str, level: u8) -> rustfoot_core::eft::EftTeam {
        use rustfoot_core::eft::{EftPlayer, EftTeam, Position};
        let mut players = Vec::new();
        for (pos, n) in [
            (Position::Goalkeeper, 2),
            (Position::Defender, 6),
            (Position::Midfielder, 6),
            (Position::Attacker, 5),
        ] {
            for k in 0..n {
                players.push(EftPlayer {
                    nationality: country.into(),
                    name: format!("J{i}-{k}-{}", pos as u8),
                    position: pos,
                });
            }
        }
        EftTeam {
            full_name: format!("Clube {i}"),
            short_name: format!("C{i}"),
            background: [255; 3],
            foreground: [0; 3],
            country: country.into(),
            level,
            players,
            coach: format!("T{i}"),
        }
    }

    /// A new game: *Selecção de Países* (OK from 34 teams, *Todas*, the names in the list's order), then *Dinheiro*, then the game
    /// is built from the chosen countries under *A iniciar o jogo...*.
    #[test]
    fn a_new_game_asks_for_the_countries_and_the_currency() {
        use crate::classic::Data;
        use crate::win31::{Forms, Skin};
        let mut c = Classic::new(
            rustfoot_core::world::World::synthetic(3),
            Skin::default(),
            Forms::default(),
            Data::default(),
        );
        c.data.countries = [
            ("BRA", "Brasil"),
            ("AUT", "Áustria"),
            ("ALE", "Alemanha"),
            ("AFS", "África do Sul"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        c.teams = (0..30).map(|i| team(i, "BRA", (i % 9 + 1) as u8)).collect();
        c.teams.extend((30..36).map(|i| team(i, "AUT", 3)));
        c.teams.extend((36..40).map(|i| team(i, "ALE", 5)));
        c.teams.extend((40..42).map(|i| team(i, "AFS", 2)));
        let mut d = open_countries(&c, "NOVO");
        let names: Vec<String> = d.lists["ListBoxOfCountries"]
            .iter()
            .map(|r| r.split('|').nth(1).unwrap().to_string())
            .collect();
        assert_eq!(names, ["África do Sul", "Áustria", "Alemanha", "Brasil"]);
        bind(&c, &mut d);
        assert_eq!(d.texts["textAvailableTeams"], "42");
        assert_eq!(d.texts["OKBtn.enabled"], "0");
        // Brasil (30) is not enough; with Áustria (36) OK is enabled
        countries_click(&mut c, &mut d, "ListBoxOfCountries#3");
        bind(&c, &mut d);
        assert_eq!(d.texts["textSelectedTeams"], "30");
        assert_eq!(d.texts["OKBtn.enabled"], "0");
        assert!(matches!(
            countries_click(&mut c, &mut d, "OKBtn"),
            Outcome::Keep
        ));
        countries_click(&mut c, &mut d, "ListBoxOfCountries#1");
        bind(&c, &mut d);
        assert_eq!(
            (
                d.texts["textSelectedTeams"].as_str(),
                d.texts["textSelectedCountries"].as_str()
            ),
            ("36", "2")
        );
        assert_eq!(d.texts["OKBtn.enabled"], "1");
        // clicking a chosen country again leaves it out; Todas takes them all
        countries_click(&mut c, &mut d, "ListBoxOfCountries#1");
        countries_click(&mut c, &mut d, "selectAllBtn");
        bind(&c, &mut d);
        assert_eq!(d.texts["textSelectedTeams"], "42");
        let Outcome::Replace(mut cur) = countries_click(&mut c, &mut d, "OKBtn") else {
            panic!("the currency follows");
        };
        assert_eq!(cur.kind, Kind::Currency);
        bind(&c, &mut cur);
        assert_eq!(cur.texts["comboCurrency.text"], "Reais", "Reais first");
        currency_click(&mut c, &mut cur, "comboCurrency#1");
        let Outcome::Replace(p) = currency_click(&mut c, &mut cur, "OKBtn") else {
            panic!("the game is built next");
        };
        assert_eq!(p.texts["@caption"], "A iniciar o jogo...");
        assert_eq!(c.new_game.currency, Some(1));
        let w = build_world(&c).unwrap();
        assert_eq!(
            w.divisions.iter().map(Vec::len).sum::<usize>() + w.pool.len(),
            42
        );
        // Cancel leaves the program
        let mut d = open_countries(&c, "NOVO");
        assert!(matches!(
            countries_click(&mut c, &mut d, "CancelBtn"),
            Outcome::Close
        ));
        assert_eq!(c.requests.last(), Some(&Request::Quit));
    }

    #[test]
    fn file_names_are_eight_upper_case_letters_or_digits() {
        assert_eq!(clean_name("abc-de.fg h1"), "ABCDEFGH");
        assert_eq!(clean_name("abcdefghij"), "ABCDEFGH");
        assert_eq!(clean_name("é!"), "");
        assert_eq!(clean_name("jogo_1"), "JOGO_1");
    }
}
