//! The dialogs of the team editor, bound and answered as the original does (`EDITEQ.EXE`; captures in `analysis/captures/editor/`).
use super::{Editor, Request, MAX_PLAYERS};
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::dialogs::newgame::collate;
use rustfoot_core::eft::{EftPlayer, EftTeam, Position};
use rustfoot_core::market::code;

/// The 16 colours of the colour grid (`TColorGrid`), in its order (as `win31::widgets::GRID_COLORS`).
pub const GRID: [[u8; 3]; 16] = [
    [0, 0, 0],
    [128, 0, 0],
    [0, 128, 0],
    [128, 128, 0],
    [0, 0, 128],
    [128, 0, 128],
    [0, 128, 128],
    [192, 192, 192],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [0, 0, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

/// The grid cell nearest to a colour of a team file.
fn grid_index(c: [u8; 3]) -> usize {
    (0..16)
        .min_by_key(|&i| {
            (0..3)
                .map(|k| (GRID[i][k] as i32 - c[k] as i32).pow(2))
                .sum::<i32>()
        })
        .unwrap_or(0)
}

fn kind(name: &str) -> Kind {
    Kind::Other(name.to_string())
}

fn is(d: &Dialog, name: &str) -> bool {
    d.kind == kind(name)
}

/// What the editor was doing when the save prompt came up (`1000:10a7`): it goes on after a Yes or a No, not after Cancel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum After {
    Open,
    New,
    Quit,
}

/// The steps of a new team (`1000:16cf`): the name (Cancel drops the team), colours, country, level and coach, then players.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Colors,
    Country,
    Level,
    Coach,
    Players,
}

// ---- opening a dialog ------------------------------------------------------------------------------------------------------

/// *Abrir* (`TOpenTeamDlg`): the teams by short name, *Abrir* once one is chosen.
pub fn open_team(e: &Editor) -> Dialog {
    let mut d = Dialog::new(kind("ed_open"), "TOPENTEAMDLG");
    let mut teams: Vec<(String, String)> = e
        .files
        .iter()
        .map(|(f, t)| (t.short_name.trim().to_uppercase(), f.clone()))
        .collect();
    teams.sort_by_cached_key(|t| (collate(&t.0), t.1.clone()));
    d.lists.insert(
        "listBoxOfTeams".into(),
        teams.iter().map(|t| t.0.clone()).collect(),
    );
    d.lists
        .insert("@files".into(), teams.into_iter().map(|t| t.1).collect());
    // (no focus rectangle on the list until it is used: as the captures show it)
    d.focus = "-".into();
    d
}

/// *Nome* (`TSetTeamNameDlg`).
pub fn open_name(t: &EftTeam) -> Dialog {
    let mut d = Dialog::new(kind("ed_name"), "TSETTEAMNAMEDLG");
    d.text("TeamLongName", t.full_name.clone());
    d.text("TeamShortName", t.short_name.clone());
    d.focus = "TeamLongName".into();
    d
}

/// *Cores* (`TTeamColorDlg`): the left button picks the text's colour (T), the right one the background's (F).
pub fn open_colors(t: &EftTeam) -> Dialog {
    let mut d = Dialog::new(kind("ed_color"), "TTEAMCOLORDLG");
    // (the file's "background" is the ink, its "foreground" the club's colour)
    d.set_num("text", grid_index(t.background) as i64);
    d.set_num("fill", grid_index(t.foreground) as i64);
    d.focus = "ColorMatrix".into();
    d
}

/// *País* (`TSetCountryDlg`) for the team, or for a player (`player` is his name).
pub fn open_country(e: &Editor, code: &str, player: Option<&str>) -> Dialog {
    let mut d = Dialog::new(kind("ed_country"), "TSETCOUNTRYDLG");
    let mut list: Vec<(String, String)> = e.countries.clone();
    list.sort_by_cached_key(|c| collate(&c.1));
    // (a new team has no country yet: nothing is chosen and OK waits for a choice)
    let sel = list
        .iter()
        .position(|(c, _)| c == code)
        .map_or(-1, |i| i as i64);
    d.lists.insert(
        "countryList".into(),
        list.iter()
            .map(|(c, n)| format!("@FLAGS/{c}|{n}"))
            .collect(),
    );
    d.lists
        .insert("@codes".into(), list.into_iter().map(|(c, _)| c).collect());
    d.text("countryList.draw", "flag");
    d.set_num("sel", sel);
    let who = match player {
        Some(p) => format!("do {p}"),
        None => format!(
            "do {}",
            e.open
                .as_ref()
                .map_or(String::new(), |o| o.team.short_name.clone())
        ),
    };
    d.text(
        "Label1",
        format!("Escolha um país da lista, correspondente à nacionalidade {who}"),
    );
    if player.is_some() {
        d.st.insert("player".into(), "1".into());
    }
    d.focus = "countryList".into();
    d
}

/// *Nível inicial* (`TTeamSkillDlg`): 1 to 20, shown as a gauge.
pub fn open_level(t: &EftTeam) -> Dialog {
    let mut d = Dialog::new(kind("ed_level"), "TTEAMSKILLDLG");
    d.set_num("level", t.level.clamp(1, 20) as i64);
    d.focus = "textSkill".into();
    d
}

/// *Treinador* (`TCoachNameDlg`).
pub fn open_coach(t: &EftTeam) -> Dialog {
    let mut d = Dialog::new(kind("ed_coach"), "TCOACHNAMEDLG");
    d.text("coachName", t.coach.clone());
    // (the focused edit has all its text selected)
    d.text("coachName.sellen", t.coach.chars().count().to_string());
    d.focus = "coachName".into();
    d
}

/// *Jogador* (`TEditPlayerDlg`): `index` is the player edited, `None` a new one (the position of the last new player, the team's
/// country).
pub fn open_player(e: &Editor, index: Option<usize>, position: Position) -> Dialog {
    let mut d = Dialog::new(kind("ed_player"), "TEDITPLAYERDLG");
    let team = e.open.as_ref().map(|o| &o.team);
    let p = index
        .and_then(|i| team.and_then(|t| t.players.get(i)))
        .cloned()
        .unwrap_or(EftPlayer {
            nationality: team.map_or(String::new(), |t| t.country.clone()),
            name: String::new(),
            position,
        });
    d.text("textName", p.name.clone());
    d.text("textName.sellen", p.name.chars().count().to_string());
    d.st.insert("nat".into(), p.nationality.clone());
    d.set_num("pos", p.position as i64);
    if let Some(i) = index {
        d.set_num("index", i as i64);
    } else {
        d.set_num("index", -1);
    }
    d.focus = "textName".into();
    d
}

/// *Transferir* (`TTransferPlayerDlg`): every other team by short name.
pub fn open_transfer(e: &Editor, index: usize) -> Dialog {
    let mut d = Dialog::new(kind("ed_transfer"), "TTRANSFERPLAYERDLG");
    let me = e.open.as_ref().map_or(String::new(), |o| o.file.clone());
    let mut teams: Vec<(String, String)> = e
        .files
        .iter()
        .filter(|(f, _)| !f.eq_ignore_ascii_case(&me))
        .map(|(f, t)| (t.short_name.trim().to_uppercase(), f.clone()))
        .collect();
    teams.sort_by_cached_key(|t| (collate(&t.0), t.1.clone()));
    d.lists.insert(
        "listBoxOfTeams".into(),
        teams.iter().map(|t| t.0.clone()).collect(),
    );
    d.lists
        .insert("@files".into(), teams.into_iter().map(|t| t.1).collect());
    let name = e
        .open
        .as_ref()
        .and_then(|o| o.team.players.get(index))
        .map_or(String::new(), |p| p.name.clone());
    d.text("textPlayerName", name);
    d.set_num("index", index as i64);
    // (no focus rectangle on the list until it is used: as the captures show it)
    d.focus = "-".into();
    d
}

/// *Gravar Equipa* (`TSaveTeamDlg`, `1000:0514`): the file name (the open file's, 8 letters) and *Gravar a equipa* ticked.
pub fn open_save(e: &Editor, after: After) -> Dialog {
    let mut d = Dialog::new(kind("ed_save"), "TSAVETEAMDLG");
    let base = e
        .open
        .as_ref()
        .map(|o| o.file.split('.').next().unwrap_or("").to_string())
        .unwrap_or_default();
    d.text("teamFilename.sellen", base.chars().count().to_string());
    d.text("teamFilename", base);
    d.text("saveTheTeam.checked", "1");
    d.st.insert(
        "after".into(),
        match after {
            After::Open => "open",
            After::New => "new",
            After::Quit => "quit",
        }
        .into(),
    );
    d.focus = "teamFilename".into();
    d
}

/// The reasons a team cannot be saved (`1020:2720`), in the order of `TCantSaveTeamDlg`'s list.
pub fn reasons(e: &Editor, t: &EftTeam) -> Vec<&'static str> {
    let keepers = t
        .players
        .iter()
        .filter(|p| p.position == Position::Goalkeeper)
        .count();
    let field = t.players.len() - keepers;
    // (a foreigner is of a country outside the team's group, `1020:0b78` = 2: Bosman and PLOP players do not count)
    let class = |nat: &str| rustfoot_core::market::country_class(code(&t.country), code(nat));
    let foreign = t
        .players
        .iter()
        .filter(|p| class(&p.nationality) == 2)
        .count();
    let mut out = Vec::new();
    if t.full_name.trim().is_empty() || t.short_name.trim().is_empty() {
        out.push("Não está definido o nome da equipa");
    }
    if !e.countries.iter().any(|(c, _)| *c == t.country) {
        out.push("Não está definido o país da equipa");
    }
    if !(1..=20).contains(&t.level) {
        out.push("Não está definido o nível da equipa");
    }
    if t.players.len() < 14 {
        out.push("Não tem jogadores suficientes");
    }
    if t.players.len() > MAX_PLAYERS {
        out.push("Tem demasiados jogadores");
    }
    if keepers == 0 {
        out.push("Não tem guarda-redes");
    }
    if field < 10 {
        out.push("Não tem jogadores de campo suficientes");
    }
    if foreign > 5 {
        out.push("Tem demasiados jogadores estrangeiros");
    }
    if t.coach.trim().is_empty() {
        out.push("Não está definido o treinador");
    }
    out
}

/// *Acerca* (the game's box; the registration is not part of the port).
pub fn open_about() -> Dialog {
    let mut d = Dialog::new(kind("ed_about"), "TABOUTDLG");
    d.focus = "OKBtn".into();
    d
}

fn open_cant_save(why: Vec<&'static str>) -> Dialog {
    let mut d = Dialog::new(kind("ed_cantsave"), "TCANTSAVETEAMDLG");
    d.lists.insert(
        "listOfReasons".into(),
        why.into_iter().map(String::from).collect(),
    );
    d
}

// ---- binding -------------------------------------------------------------------------------------------------------------

pub fn bind(e: &Editor, d: &mut Dialog) {
    // the editor's edits keep their text a pixel further from the edge than the game's
    for n in [
        "TeamLongName",
        "TeamShortName",
        "coachName",
        "textName",
        "teamFilename",
    ] {
        d.text(format!("{n}.indent"), "3");
        // the caret after the text of the focused edit while nothing is selected
        let selected = d
            .texts
            .get(&format!("{n}.sellen"))
            .is_some_and(|v| v != "0");
        if d.focus == n && !selected {
            let len = d.texts.get(n).map_or(0, |t| t.chars().count());
            d.text(format!("{n}.caret"), len.to_string());
        } else {
            d.texts.remove(&format!("{n}.caret"));
        }
    }
    // and their long labels wrap (on the captures, although the forms do not ask for it)
    let wrap: &[&str] = match d.form.as_str() {
        "TTEAMCOLORDLG" => &["Example"],
        "TTEAMSKILLDLG" => &["Label1"],
        "TTRANSFERPLAYERDLG" => &["Label2", "Label3"],
        "TSETCOUNTRYDLG" => &["Label1"],
        _ => &[],
    };
    for n in wrap {
        d.text(format!("{n}.wrap"), "1");
    }
    if is(d, "ed_about") {
        // the game's box with the editor's version (`98.002` on the capture), its banner and blinking notice
        d.text("textGameName", "Elifoot 98");
        d.text("textSubVersion", "98.002");
        d.text(
            "labelNotRegistered",
            crate::classic::dialogs::about::NOT_REGISTERED,
        );
        let shown = d.st.get("blink").map(String::as_str) != Some("off");
        d.text("labelNotRegistered.visible", if shown { "1" } else { "0" });
        d.text("labelWarning.visible", "0");
        d.text("Banner1.scroll", d.num("scroll").to_string());
        return;
    }
    if is(d, "ed_open") || is(d, "ed_transfer") {
        let sel = d.texts.get("listBoxOfTeams.sel").cloned();
        d.text("OKBtn.enabled", if sel.is_some() { "1" } else { "0" });
        if is(d, "ed_open") {
            d.text("listBoxOfFormats.items", "Elifoot 98");
            d.text("listBoxOfFormats.sel", "0");
            d.text("listBoxOfFormats.text", "Elifoot 98");
            d.text("textTeamName", "");
        }
    } else if is(d, "ed_color") {
        let (t, f) = (d.num("text") as usize, d.num("fill") as usize);
        d.text("ColorMatrix.index", t.to_string());
        d.text("ColorMatrix.bgindex", f.to_string());
        d.text("ColorMatrix.fgmark", "T");
        d.text("ColorMatrix.bgmark", "F");
        let rgb = |c: [u8; 3]| format!("{},{},{}", c[0], c[1], c[2]);
        d.text("Example.color", rgb(GRID[t % 16]));
        d.text("Example.bgcolor", rgb(GRID[f % 16]));
    } else if is(d, "ed_country") {
        if !d.texts.contains_key("countryList.sel") && d.num("sel") >= 0 {
            let sel = d.num("sel") as usize;
            let list = d.lists.get("countryList").cloned().unwrap_or_default();
            d.text("countryList.sel", sel.to_string());
            d.text(
                "countryList.text",
                list.get(sel).cloned().unwrap_or_default(),
            );
        }
        // the list opens with the chosen country on its first row
        let open = d.texts.get("countryList.open").is_some_and(|v| v == "1");
        if open && !d.st.contains_key("shown") {
            let sel = d.texts.get("countryList.sel").cloned().unwrap_or_default();
            d.text("countryList.top", sel);
            d.st.insert("shown".into(), "1".into());
        } else if !open {
            d.st.remove("shown");
        }
        d.text(
            "OKBtn.enabled",
            flag(d.texts.contains_key("countryList.sel")),
        );
    } else if is(d, "ed_save") {
        // the file name only while *Gravar a equipa* is ticked, and then OK wants one (`1000:063e`)
        let on = d.texts.get("saveTheTeam.checked").is_some_and(|v| v == "1");
        d.text("teamFilename.enabled", flag(on));
        d.text("OKBtn.enabled", flag(!on || filled(d, "teamFilename")));
    } else if is(d, "ed_name") {
        // both names (`1008:1a3a`)
        d.text(
            "OKBtn.enabled",
            flag(filled(d, "TeamLongName") && filled(d, "TeamShortName")),
        );
    } else if is(d, "ed_coach") {
        d.text("OKBtn.enabled", flag(filled(d, "coachName")));
    } else if is(d, "ed_level") {
        let v = d.num("level").clamp(1, 20);
        d.text("textSkill.value", v.to_string());
        // (`AutoSize` makes it 47 pixels high, and the value is selected)
        d.text("textSkill.height", "47");
        d.text("textSkill.style", "editor");
        d.text("gaugeSkill.progress", (v * 5).to_string());
    } else if is(d, "ed_player") {
        let pos = d.num("pos");
        for (i, r) in [
            "RadioKeeper",
            "RadioDefender",
            "RadioMidfield",
            "RadioForwarder",
        ]
        .iter()
        .enumerate()
        {
            d.text(
                format!("{r}.checked"),
                if i as i64 == pos { "1" } else { "0" },
            );
        }
        let nat = d.st.get("nat").cloned().unwrap_or_default();
        d.text("textCountry", e.country_name(&nat));
        d.text("bitmapCountry.image", format!("FLAGS/{nat}"));
        // a position (always one here) and a name (`1008:1fe5`)
        d.text("OKBtn.enabled", flag(filled(d, "textName")));
    }
    // the steps of a new team between its name and its players cannot be cancelled
    let step = ["ed_color", "ed_country", "ed_level", "ed_coach"]
        .iter()
        .any(|k| is(d, k));
    if step && e.new_steps.is_some() && !d.st.contains_key("player") {
        d.text("CancelBtn.visible", "0");
    }
}

fn flag(on: bool) -> &'static str {
    if on {
        "1"
    } else {
        "0"
    }
}

fn filled(d: &Dialog, edit: &str) -> bool {
    d.texts.get(edit).is_some_and(|t| !t.trim().is_empty())
}

// ---- answering -----------------------------------------------------------------------------------------------------------

fn list_sel(d: &Dialog, list: &str) -> Option<usize> {
    d.texts
        .get(&format!("{list}.sel"))
        .and_then(|v| v.parse().ok())
}

pub fn click(e: &mut Editor, d: &mut Dialog, name: &str) -> Outcome {
    // a disabled or hidden button does nothing (Enter on a default button that waits for input)
    let off = |k: &str| {
        d.texts
            .get(&format!("{name}.{k}"))
            .is_some_and(|v| v == "0")
    };
    if off("enabled") || off("visible") {
        return Outcome::Keep;
    }
    let cancel = name == "CancelBtn";
    if is(d, "ed_open") {
        if let Some(i) = name
            .strip_prefix("listBoxOfTeams#")
            .and_then(|i| i.parse::<usize>().ok())
        {
            d.text("listBoxOfTeams.sel", i.to_string());
            d.focus = "listBoxOfTeams".into();
            return Outcome::Keep;
        }
        if name == "OKBtn" {
            if let Some(f) =
                list_sel(d, "listBoxOfTeams").and_then(|i| d.lists["@files"].get(i).cloned())
            {
                e.open_file(&f);
            }
            return Outcome::Close;
        }
        return if cancel {
            Outcome::Close
        } else {
            Outcome::Keep
        };
    }
    if is(d, "ed_cantsave") || is(d, "ed_about") {
        return if name == "OKBtn" {
            Outcome::Close
        } else {
            Outcome::Keep
        };
    }
    if is(d, "ed_save") {
        return save_click(e, d, name);
    }
    if is(d, "ed_transfer") {
        if let Some(i) = name
            .strip_prefix("listBoxOfTeams#")
            .and_then(|i| i.parse::<usize>().ok())
        {
            d.text("listBoxOfTeams.sel", i.to_string());
            d.focus = "listBoxOfTeams".into();
            return Outcome::Keep;
        }
        if name == "OKBtn" {
            let to = list_sel(d, "listBoxOfTeams").and_then(|i| d.lists["@files"].get(i).cloned());
            let idx = d.num("index") as usize;
            if let (Some(to), Some(o)) = (to, e.open.as_mut()) {
                if idx < o.team.players.len() {
                    let p = o.team.players.remove(idx);
                    e.transfers.push((to, p));
                    e.modified = true;
                    e.selected = None;
                }
            }
            return Outcome::Close;
        }
        return if cancel {
            Outcome::Close
        } else {
            Outcome::Keep
        };
    }
    // the dialogs of the team (and of a new team's steps)
    if cancel {
        if is(d, "ed_name") && e.new_steps.is_some() {
            // a new team without a name is dropped (`1000:16cf`)
            e.open = None;
            e.new_steps = None;
            e.modified = false;
            return Outcome::Close;
        }
        if is(d, "ed_player") && d.num("index") < 0 {
            // the end of *Adicionar*
            e.new_steps = None;
        }
        return next_step(e, Outcome::Close);
    }
    if is(d, "ed_color") {
        // a cell: the left button picks the text's colour, the right one the background's (`name#i!r`)
        if let Some(cell) = name.strip_prefix("ColorMatrix#") {
            let (i, right) = match cell.strip_suffix("!r") {
                Some(i) => (i, true),
                None => (cell, false),
            };
            if let Ok(i) = i.parse::<i64>() {
                d.set_num(if right { "fill" } else { "text" }, i);
            }
            return Outcome::Keep;
        }
        match name {
            "BitBtn1" => {
                // *Trocar*: text and background change places
                let (t, f) = (d.num("text"), d.num("fill"));
                d.set_num("text", f);
                d.set_num("fill", t);
                return Outcome::Keep;
            }
            "OKBtn" => {
                if let Some(o) = e.open.as_mut() {
                    o.team.background = GRID[d.num("text") as usize % 16];
                    o.team.foreground = GRID[d.num("fill") as usize % 16];
                    e.modified = true;
                }
                return next_step(e, Outcome::Close);
            }
            _ => return Outcome::Keep,
        }
    }
    if is(d, "ed_country") {
        if name == "OKBtn" {
            let sel = list_sel(d, "countryList").unwrap_or(0);
            let code = d.lists["@codes"].get(sel).cloned().unwrap_or_default();
            if d.st.contains_key("player") {
                // back to the player's dialog
                if let Some(p) = e.dialogs.last_mut().filter(|p| is(p, "ed_player")) {
                    p.st.insert("nat".into(), code);
                }
                return Outcome::Close;
            }
            if let Some(o) = e.open.as_mut() {
                o.team.country = code;
                e.modified = true;
            }
            return next_step(e, Outcome::Close);
        }
        return Outcome::Keep;
    }
    if name != "OKBtn" {
        if is(d, "ed_player") {
            if let Some(i) = [
                "RadioKeeper",
                "RadioDefender",
                "RadioMidfield",
                "RadioForwarder",
            ]
            .iter()
            .position(|r| *r == name)
            {
                d.set_num("pos", i as i64);
            }
            if name == "changeBtn" {
                let nat = d.st.get("nat").cloned().unwrap_or_default();
                let who = d.texts.get("textName").cloned().unwrap_or_default();
                return Outcome::Open(Box::new(open_country(e, &nat, Some(&who))));
            }
        }
        if is(d, "ed_level") && name == "textSkill" {
            let v = d
                .texts
                .get("textSkill.value")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            d.set_num("level", i64::clamp(v, 1, 20));
        }
        return Outcome::Keep;
    }
    let Some(o) = e.open.as_mut() else {
        return Outcome::Close;
    };
    let text = |k: &str| d.texts.get(k).cloned().unwrap_or_default();
    if is(d, "ed_name") {
        o.team.full_name = text("TeamLongName");
        o.team.short_name = text("TeamShortName").to_uppercase();
    } else if is(d, "ed_level") {
        o.team.level = d.num("level").clamp(1, 20) as u8;
    } else if is(d, "ed_coach") {
        o.team.coach = text("coachName");
    } else if is(d, "ed_player") {
        let pos = match d.num("pos") {
            0 => Position::Goalkeeper,
            1 => Position::Defender,
            2 => Position::Midfielder,
            _ => Position::Attacker,
        };
        let p = EftPlayer {
            nationality: d.st.get("nat").cloned().unwrap_or_default(),
            name: text("textName"),
            position: pos,
        };
        let index = d.num("index");
        if index >= 0 {
            if let Some(slot) = o.team.players.get_mut(index as usize) {
                *slot = p;
            }
        } else {
            o.team.players.push(p);
            e.last_position = pos;
            e.modified = true;
            // *Adicionar* goes on with another empty player while there is room
            if e.open
                .as_ref()
                .is_some_and(|o| o.team.players.len() < MAX_PLAYERS)
            {
                return Outcome::Replace(Box::new(open_player(e, None, pos)));
            }
            e.new_steps = None;
            return Outcome::Close;
        }
    }
    e.modified = true;
    next_step(e, Outcome::Close)
}

/// After a dialog of a new team, the next one of its steps (`1000:16cf` then *Adicionar*).
fn next_step(e: &mut Editor, done: Outcome) -> Outcome {
    let Some(steps) = e.new_steps.as_mut() else {
        return done;
    };
    if steps.is_empty() {
        e.new_steps = None;
        return done;
    }
    let step = steps.remove(0);
    let Some(t) = e.open.as_ref().map(|o| o.team.clone()) else {
        e.new_steps = None;
        return done;
    };
    let d = match step {
        Step::Colors => open_colors(&t),
        Step::Country => open_country(e, &t.country, None),
        Step::Level => open_level(&t),
        Step::Coach => open_coach(&t),
        Step::Players => open_player(e, None, e.last_position),
    };
    Outcome::Replace(Box::new(d))
}

/// The answer to *Gravar Equipa*: OK with the box ticked checks and stores the team, OK without it drops the changes; Cancel stops what
/// was being done.
fn save_click(e: &mut Editor, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "OKBtn" => {
            let save = d.texts.get("saveTheTeam.checked").is_some_and(|v| v == "1");
            if save {
                let Some(o) = e.open.clone() else {
                    return Outcome::Close;
                };
                let why = reasons(e, &o.team);
                if !why.is_empty() {
                    return Outcome::Replace(Box::new(open_cant_save(why)));
                }
                let base = crate::classic::dialogs::newgame::clean_name(
                    d.texts.get("teamFilename").map_or("", String::as_str),
                );
                if base.is_empty() {
                    return Outcome::Keep;
                }
                let file = format!("{base}.EFT");
                let bytes = rustfoot_core::eft::write(&o.team);
                e.requests.push(Request::Save(file.clone(), bytes));
                match e
                    .files
                    .iter_mut()
                    .find(|(f, _)| f.eq_ignore_ascii_case(&file))
                {
                    Some(slot) => slot.1 = o.team.clone(),
                    None => e.files.push((file.clone(), o.team.clone())),
                }
            }
            // saved or dropped, the team is closed (`CloseTeam`, `1000:10a7`): cancelling *Abrir* then leaves the window empty
            e.open = None;
            e.selected = None;
            e.modified = false;
            let after = d.st.get("after").cloned().unwrap_or_default();
            e.go_on(&after)
        }
        "CancelBtn" => Outcome::Close,
        _ => Outcome::Keep,
    }
}
