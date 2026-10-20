//! The information dialogs built on the original's generic list form `TPrintListDlg` (top scorers, calendar, champions, transfers,
//! acquisitions; the coach lists in `coach.rs` use the same form) and the four standings of `TClassificationDlg`.
//!
//! Every list row is the string the original's item class builds (`TCalendarEntry` `1010:217b`, `TLastWinners` `1010:cbd3`,
//! `TPlayer` `1010:76a3`, `TLastTransfer` `1010:d0e0`, `TTeam` `1010:3277`) in the field encoding described in
//! `win31/printlist.rs`; `1050:3938` (`TPrintListDlg.Execute`) lays the form out from the flags it is given.
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::text;
use crate::classic::{main_win, Classic, Key};
use crate::win31::fb::Rgb;
use crate::win31::printlist::{visible_rows, with_colors};
use rustfoot_core::history::Fixture;
use rustfoot_core::season::{standings, Club};
use rustfoot_core::world::World;

/// The flags of `TPrintListDlg.Execute`.
pub const AUTOCLOSE: u32 = 1;
pub const NUMBERED: u32 = 2;
pub const WIDE: u32 = 8;
pub const SELECT_LAST: u32 = 0x10;

/// What a list dialog shows: its caption, the rows in field encoding and the flags.
pub struct Spec {
    pub caption: String,
    pub rows: Vec<String>,
    pub flags: u32,
}

/// One field of a row: `[modifier] width text TAB`.
pub fn field(modifier: &str, width: u32, text: &str) -> String {
    format!("{modifier}{}{text}\t", char::from(width as u8))
}

/// The prefix the numbered lists put in front of every row (`1068:0ad6`: `#6#20 n TAB`).
pub fn numbered(i: usize, row: String) -> String {
    format!("{}{row}", field("\u{6}", 0x14, &(i + 1).to_string()))
}

/// The position letter of a player (`DS:18fc` is `GDMA`).
pub fn pos_letter(position: u8) -> char {
    char::from(b"GDMA"[(position & 3) as usize])
}

/// Name of a player as `1058:13fe` writes it: a star ("craque") gets a `*`.
pub fn player_name(club: &Club, i: usize) -> String {
    let star = if club.slots[i].star() { "*" } else { "" };
    format!("{}{star}", club.names[i])
}

/// The players of the world with the club they play for.
pub fn all_players(w: &World) -> Vec<(&Club, usize)> {
    let mut clubs: Vec<&Club> = w.divisions.iter().flatten().chain(w.pool.iter()).collect();
    clubs.sort_by_key(|c| c.master); // the master list of all teams
    clubs
        .into_iter()
        .flat_map(|c| (0..c.slots.len()).map(move |i| (c, i)))
        .collect()
}

type Scorer<'a> = (&'a Club, usize, u32);

/// The players with exactly `g` goals for `g` from `from` down to `to`, in the order of the world.
fn take<'a>(
    players: &[(&'a Club, usize)],
    goals: &dyn Fn(&Club, usize) -> u32,
    from: i64,
    to: i64,
    picked: &mut Vec<Scorer<'a>>,
) {
    let mut g = from;
    while g >= to {
        picked.extend(
            players
                .iter()
                .filter(|&&(c, i)| goals(c, i) as i64 == g)
                .map(|&(c, i)| (c, i, g as u32)),
        );
        g -= 1;
    }
}

/// `1010:d83a`: the best scorers (this season or of all time), grouped by goals from the top and sorted by `CompareGoals`
/// (`1050:18d7`; the career variant `19ca`): goals, then strength, ties keeping the order of the world.
pub fn top_scorers(w: &World, all_time: bool) -> Vec<Scorer<'_>> {
    let goals = |c: &Club, i: usize| -> u32 {
        if all_time {
            c.slots[i].history[1]
        } else {
            c.slots[i].goals as u32
        }
    };
    let players = all_players(w);
    let max = players.iter().map(|&(c, i)| goals(c, i)).max().unwrap_or(0) as i64;
    if max <= 0 {
        return Vec::new();
    }
    let rounded = |k: f64| ((max as f64 * k).round_ties_even() as i64).max(1);
    let low = rounded(if all_time { 0.85 } else { 0.7 });
    let mut picked: Vec<Scorer<'_>> = Vec::new();
    take(&players, &goals, max, low, &mut picked);
    if picked.len() < 6 {
        let (hi, lo) = (low - 1, rounded(0.51));
        if lo <= hi {
            take(&players, &goals, hi, lo, &mut picked);
        }
    }
    // players equal in goals and strength come out in the reverse of the world's order (measured on the original's list)
    picked.reverse();
    picked.sort_by(|a, b| {
        b.2.cmp(&a.2)
            .then(b.0.slots[b.1].strength.cmp(&a.0.slots[a.1].strength))
    });
    picked
}

pub fn scorers(w: &World, all_time: bool, bare_flags: u32) -> Spec {
    let rows = top_scorers(w, all_time)
        .into_iter()
        .enumerate()
        .map(|(k, (c, i, g))| {
            let mut r = String::from("\u{0} \t");
            r += &field("\u{2}", 0x78, &player_name(c, i));
            r += &field("\u{6}", 0x14, &g.to_string());
            r += &field("\u{4}", 0x8c, &c.name);
            numbered(k, r)
        })
        .collect();
    Spec {
        caption: if all_time {
            "Melhores marcadores de sempre"
        } else {
            "Melhores marcadores desta época"
        }
        .into(),
        rows,
        flags: NUMBERED | bare_flags,
    }
}

/// `1010:217b`, the colours of `1010:2597` and `1010:254c`: a home game is white on blue (league) or red (cup), the others the
/// reverse.
pub fn calendar_row(f: &Fixture) -> String {
    let mut r = if f.round < 1 {
        field("\u{6}", 0x14, " ")
    } else {
        field("\u{6}", 0x14, &f.round.to_string())
    };
    r += &field("", 0x78, &f.opponent);
    r += &field(
        "",
        0x0f,
        &char::from(b"CFN"[f.venue.min(2) as usize]).to_string(),
    );
    if f.played() {
        r += &field("\u{7}", 0x0f, &f.goals.0.to_string());
        r += &field("\u{7}", 0x0a, ":");
        r += &field("\u{7}", 0x0f, &f.goals.1.to_string());
        let note = match f.result {
            1 => "  a. p.",
            2 => "  v. pen.",
            3 => "  d. pen.",
            _ => " ",
        };
        r += &field("", 0x1e, note);
        r += &field("\u{6}", 0x46, &f.gate.to_string());
    }
    let key = if f.cup {
        Rgb(255, 0, 0)
    } else {
        Rgb(0, 0, 255)
    };
    let white = Rgb(255, 255, 255);
    if f.venue == 0 {
        with_colors(Some(white), Some(key), &r)
    } else {
        with_colors(Some(key), Some(white), &r)
    }
}

pub fn calendar(w: &World, club: &str) -> Spec {
    Spec {
        caption: "Calendário".into(),
        rows: w.calendar(club).iter().map(calendar_row).collect(),
        flags: 0,
    }
}

/// `1010:cbd3`: the champions of past seasons, the coach of the user in bold.
pub fn winners(w: &World) -> Spec {
    let coach = |name: &str| {
        let m = if w.is_human_coach(name) {
            "\u{2}"
        } else {
            ""
        };
        field(m, 0x82, if name.is_empty() { " " } else { name })
    };
    let team = |name: &str| field("\u{4}", 0x78, if name.is_empty() { " " } else { name });
    let rows = w
        .history
        .winners
        .iter()
        .map(|x| {
            let mut r = field("", 0x28, &x.year.to_string());
            r += &team(&x.champion);
            r += &coach(&x.champion_coach);
            r += &team(&x.cup_winner);
            r += &coach(&x.cup_coach);
            r
        })
        .collect();
    Spec {
        caption: "Últimos vencedores do campeonato e da taça".into(),
        rows,
        flags: WIDE,
    }
}

/// The name of the coach of the user's club.
pub fn human_coach(w: &World) -> String {
    w.human
        .as_ref()
        .and_then(|h| {
            w.divisions
                .iter()
                .flatten()
                .chain(w.pool.iter())
                .find(|c| c.name == h.club)
        })
        .map(|c| c.coach_name.clone())
        .unwrap_or_default()
}

/// `1010:d0e0`: a transfer of the list of the last ones (a transfer of the user's club is drawn in blue).
pub fn transfer_row(w: &World, t: &rustfoot_core::world::Transfer) -> String {
    let mut r = String::new();
    if w.is_human_club(&t.from) || w.is_human_club(&t.to) {
        r.push('\u{fc}');
    }
    // the player is looked up in the squad of the club that bought him
    let found = all_players(w)
        .into_iter()
        .find(|&(c, i)| c.name == t.to && c.names[i] == t.player);
    match found {
        Some((c, i)) => {
            r += &field("", 0x0f, &pos_letter(c.slots[i].position).to_string());
            r += &field("", 0x78, &player_name(c, i));
            r += &field("", 0x19, &String::from_utf8_lossy(&c.slots[i].nat));
        }
        None => {
            r += &field("", 0x0f, " ");
            r += &field("", 0x78, " ");
            r += &field("", 0x19, " ");
        }
    }
    r += &field("\u{6}", 0x14, &t.strength.to_string());
    r += &field("\u{6}", 0x50, &text::int(t.fee));
    r += &field("", 0xb4, if t.to.is_empty() { " " } else { &t.to });
    r
}

pub fn transactions(w: &World) -> Spec {
    Spec {
        caption: "Últimas transferências".into(),
        rows: w.transfers.iter().map(|t| transfer_row(w, t)).collect(),
        flags: WIDE | SELECT_LAST,
    }
}

/// `EquipaAquisicoes1Click`: the players the user has bought and that arrive with the end of the round.
pub fn acquisitions(w: &World) -> Spec {
    let rows = w
        .pending
        .iter()
        .filter_map(|p| {
            let (c, i) = all_players(w)
                .into_iter()
                .find(|&(c, i)| c.name == p.seller && c.names[i] == p.player)?;
            let s = &c.slots[i];
            let mut r = String::from("\u{0} \t");
            r += &field("", 0x0f, &pos_letter(s.position).to_string());
            r += &field("\u{2}", 0x78, &player_name(c, i));
            r += &field("\u{4}", 0x8c, &c.name);
            r += &field("\u{6}", 0x14, &s.strength.to_string());
            r += &field("", 0x19, &String::from_utf8_lossy(&s.nat));
            Some(r)
        })
        .collect();
    Spec {
        caption: "Jogadores adquiridos".into(),
        rows,
        flags: 0,
    }
}

/// `1010:3277` with flags 8: a row of the standings, in the colours of the club.
pub fn standing_row(c: &Club) -> String {
    let mut r = field("", 0x78, &c.name);
    r += &field("\u{7}", 0x14, &c.wins.to_string());
    r += &field("\u{7}", 0x14, &c.draws.to_string());
    r += &field("\u{7}", 0x14, &c.losses.to_string());
    r += &field("\u{7}", 0x1e, &c.goals_for.to_string());
    r += &field("\u{7}", 0x0a, ":");
    r += &field("\u{7}", 0x0f, &c.goals_against.to_string());
    r += &field("\u{7}", 0x1e, &c.points.to_string());
    let rgb = |c: [u8; 3]| Rgb(c[0], c[1], c[2]);
    with_colors(Some(rgb(c.colors.0)), Some(rgb(c.colors.1)), &r)
}

/// `1050:1ab6` (`SortDivisionLists`): points, goal difference, goals scored; ties keep the order of the division.
pub fn table_order(clubs: &[Club]) -> Vec<usize> {
    let mut idx = standings(clubs);
    idx.sort_by(|&a, &b| {
        let key = |i: usize| {
            let c = &clubs[i];
            (c.points, c.goals_for - c.goals_against, c.goals_for)
        };
        key(b).cmp(&key(a))
    });
    idx
}

/// The standings dialog; `auto` closes it by itself after ten seconds (the game flow shows it that way).
pub fn standings_dialog(auto: bool) -> Dialog {
    let mut d = Dialog::new(Kind::Standings, "TCLASSIFICATIONDLG");
    if auto {
        d.st.insert("auto".into(), "1".into());
    }
    d
}

pub fn bind(c: &Classic, d: &mut Dialog) {
    match d.kind {
        Kind::Standings => bind_standings(c, d),
        _ => {
            if let Some(spec) = spec_for(c, d) {
                bind_list(d, spec);
            }
        }
    }
}

/// The rows of the kinds this module owns; `None` once built.
fn spec_for(c: &Classic, d: &Dialog) -> Option<Spec> {
    if d.st.contains_key("built") {
        return None;
    }
    let w = &c.world;
    Some(match d.kind {
        Kind::Scorers => scorers(w, false, 0),
        Kind::AllTimeScorers => scorers(w, true, 0),
        Kind::Calendar => {
            let club = if d.st.contains_key("opponent") {
                let (dv, j, _) = main_win::next_opponent(c)?;
                w.divisions[dv][j].name.clone()
            } else {
                w.human.as_ref()?.club.clone()
            };
            calendar(w, &club)
        }
        Kind::Winners => winners(w),
        Kind::Transactions => transactions(w),
        Kind::Acquisitions => acquisitions(w),
        _ => return None,
    })
}

/// Geometry of the form after `1050:3938`: the list is as high as its rows (but not above the screen), 200 pixels wider for wide
/// lists.
pub struct Layout {
    pub client: (i32, i32),
    pub list: (i32, i32),
    pub bevel: (i32, i32),
    pub ok: (i32, i32),
}

pub fn layout(count: usize, flags: u32) -> Layout {
    let wide = if flags & WIDE != 0 { 200 } else { 0 };
    let (client_h, list_h) = (297, 225);
    // the form is 29 pixels taller than its client area (caption and thick frame)
    let form_h = client_h + 29;
    let delta = (600 - form_h).min(count as i32 * 16 + 2 - list_h);
    Layout {
        client: (374 + wide, client_h + delta),
        list: (345 + wide, list_h + delta),
        bevel: (361 + wide, 241 + delta),
        ok: (152 + wide / 2, 260 + delta),
    }
}

/// Fills in a `TPrintListDlg`: the caption, the rows, the size of the form and the selection of the last row.
pub fn bind_list(d: &mut Dialog, spec: Spec) {
    let l = layout(spec.rows.len(), spec.flags);
    d.text("@caption", spec.caption);
    // `@client` is the real client size: the thick frame of the original hides four pixels of the declared one
    d.text("@client", format!("{}x{}", l.client.0 + 4, l.client.1 + 4));
    d.text("listOfItems.size", format!("{},{}", l.list.0, l.list.1));
    d.text("Bevel1.size", format!("{},{}", l.bevel.0, l.bevel.1));
    d.text("OKBtn.pos", format!("{},{}", l.ok.0, l.ok.1));
    let n = spec.rows.len();
    if spec.flags & SELECT_LAST != 0 && n > 0 {
        // the list ends on its last row; the original's `DrawTheItem` shows no selection
        let fit = ((l.list.1 - 2) / 16) as usize;
        d.text("listOfItems.top", n.saturating_sub(fit).to_string());
    }
    d.set_num("flags", spec.flags as i64);
    d.set_num("count", n as i64);
    d.set_num("list_h", l.list.1 as i64);
    d.lists.insert("listOfItems".into(), spec.rows);
    d.st.insert("built".into(), "1".into());
}

fn top(d: &Dialog) -> usize {
    d.texts
        .get("listOfItems.top")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Scrolls a list to the row `t` (clamped).
fn scroll_to(d: &mut Dialog, t: i64) {
    let count = d.num("count");
    let client = d.num("list_h") as i32 - 2;
    let max = (count - (client / 16) as i64).max(0);
    d.text("listOfItems.top", t.clamp(0, max).to_string());
}

pub fn click(_c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match d.kind {
        Kind::Standings => click_standings(d, name),
        _ => click_list(d, name),
    }
}

/// Clicks of a `TPrintListDlg` (also used by the coach lists).
pub fn click_list(d: &mut Dialog, name: &str) -> Outcome {
    let t = top(d) as i64;
    let page = visible_rows(d.num("list_h") as i32 - 2, 16) as i64 - 1;
    if let Some(row) = name.strip_prefix("listOfItems#") {
        d.text("listOfItems.sel", row);
        return Outcome::Keep;
    }
    match name {
        "OKBtn" => Outcome::Close,
        "listOfItems.up" => {
            scroll_to(d, t - 1);
            Outcome::Keep
        }
        "listOfItems.down" => {
            scroll_to(d, t + 1);
            Outcome::Keep
        }
        "listOfItems.pgup" => {
            scroll_to(d, t - page);
            Outcome::Keep
        }
        "listOfItems.pgdn" => {
            scroll_to(d, t + page);
            Outcome::Keep
        }
        _ => Outcome::Keep,
    }
}

pub fn key(_c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    if d.kind == Kind::Standings {
        return matches!(key, Key::Enter | Key::Escape | Key::Char(' ')).then_some(Outcome::Close);
    }
    key_list(d, key)
}

/// Arrow keys scroll the list (the original's focus is on the OK button, so this is a convenience of the port).
pub fn key_list(d: &mut Dialog, key: Key) -> Option<Outcome> {
    let t = top(d) as i64;
    match key {
        Key::Up => scroll_to(d, t - 1),
        Key::Down => scroll_to(d, t + 1),
        _ => return None,
    }
    Some(Outcome::Keep)
}

/// Timers: the standings close after ten seconds when shown by the game flow, a list when it was asked to (`Timer1Timer`).
pub fn tick(_c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    if timer_closes(d, ms) {
        Outcome::Close
    } else {
        Outcome::Keep
    }
}

/// Advances the one-second timer of the form; true when it closes the dialog.
pub fn timer_closes(d: &mut Dialog, ms: u32) -> bool {
    let mut acc = d.num("ms") + ms as i64;
    let mut seconds = d.num("seconds");
    while acc >= 1000 {
        acc -= 1000;
        seconds += 1;
        let close = match d.kind {
            Kind::Standings => d.st.contains_key("auto") && seconds == 10,
            _ => d.num("flags") as u32 & AUTOCLOSE != 0 && seconds > d.num("count") + 3,
        };
        if close {
            return true;
        }
    }
    d.set_num("ms", acc);
    d.set_num("seconds", seconds);
    false
}

// ---- TClassificationDlg -------------------------------------------------------------------------------------------------

fn bind_standings(c: &Classic, d: &mut Dialog) {
    if d.st.contains_key("built") {
        return;
    }
    let w = &c.world;
    d.bg = Some(Rgb(0, 128, 0));
    // the form's own size (without it the window comes out four pixels larger; measured on the capture of the original)
    d.text("@client", "592x375");
    // after round 14 the world is already in the next season: the dialog shows the final tables of the last one
    if let (0, Some(s)) = (w.round, w.last_season.as_ref()) {
        d.text("@caption", "Classificação - 14ª jornada");
        for (k, table) in s.tables.iter().enumerate() {
            let rows = table
                .iter()
                .map(|t| {
                    let mut club = Club::new(&t.name, Vec::new());
                    (club.wins, club.draws, club.losses) = (t.wins, t.draws, t.losses);
                    (club.goals_for, club.goals_against, club.points) =
                        (t.goals_for, t.goals_against, t.points);
                    if let Some(real) = c
                        .world
                        .divisions
                        .iter()
                        .flatten()
                        .chain(c.world.pool.iter())
                        .find(|x| x.name == t.name)
                    {
                        club.colors = real.colors;
                    }
                    standing_row(&club)
                })
                .collect();
            d.lists.insert(format!("ListBoxDivision{}", k + 1), rows);
        }
        d.focus = "ListBoxDivision1".into();
        d.st.insert("built".into(), "1".into());
        return;
    }
    d.text(
        "@caption",
        if w.round > 0 {
            format!("Classificação - {}ª jornada", w.round)
        } else {
            "Classificação".to_string()
        },
    );
    for (k, div) in w.divisions.iter().enumerate() {
        let rows = w
            .table_order(k)
            .into_iter()
            .map(|i| standing_row(&div[i]))
            .collect();
        d.lists.insert(format!("ListBoxDivision{}", k + 1), rows);
    }
    d.focus = "ListBoxDivision1".into();
    d.st.insert("built".into(), "1".into());
}

fn click_standings(d: &mut Dialog, name: &str) -> Outcome {
    // a click on a row selects it (the original shows that team on the main page); a click elsewhere (`GoOn`) closes
    if let Some((list, row)) = name.split_once('#') {
        if list.starts_with("ListBoxDivision") {
            d.text(format!("{list}.sel"), row);
            return Outcome::Keep;
        }
    }
    Outcome::Close
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut w = World::synthetic(3);
        for _ in 0..3 {
            w.play_round();
        }
        w
    }

    #[test]
    fn the_calendar_row_is_the_originals_field_string() {
        let f = Fixture {
            opponent: "Santos".into(),
            venue: 1,
            goals: (2, 1),
            gate: 10730,
            cup: false,
            result: 2,
            round: 3,
        };
        let r = calendar_row(&f);
        let (fg, bg, fields) = crate::win31::printlist::split_colors(&r);
        assert_eq!(fg, Some(Rgb(0, 0, 255)));
        assert_eq!(bg, Some(Rgb(255, 255, 255)));
        assert_eq!(
            fields,
            "\u{6}\u{14}3\t\u{78}Santos\t\u{0f}F\t\u{7}\u{0f}2\t\u{7}\u{0a}:\t\u{7}\u{0f}1\t\u{1e}  v. pen.\t\u{6}\u{46}10730\t"
        );
        // an unplayed home game of the cup: the number is blank and nothing follows the venue letter
        let g = Fixture {
            goals: (-1, -1),
            cup: true,
            round: 0,
            venue: 0,
            ..f
        };
        let row = calendar_row(&g);
        let (fg, bg, fields) = crate::win31::printlist::split_colors(&row);
        assert_eq!((fg, bg), (Some(Rgb(255, 255, 255)), Some(Rgb(255, 0, 0))));
        assert_eq!(fields, "\u{6}\u{14} \t\u{78}Santos\t\u{0f}C\t");
    }

    #[test]
    fn top_scorers_group_by_goals_and_stop_at_the_threshold() {
        let mut w = World::synthetic(4);
        // ten goals for one player, seven for another, three for a third
        w.divisions[0][0].slots[0].goals = 10;
        w.divisions[1][2].slots[1].goals = 7;
        w.divisions[2][3].slots[2].goals = 3;
        let t = top_scorers(&w, false);
        // threshold round(10 * 0.7) = 7: the first two; fewer than six, so down to round(10 * 0.51) = 5 too (nobody there)
        assert_eq!(t.iter().map(|x| x.2).collect::<Vec<_>>(), [10, 7]);
        // nobody scored: an empty list
        assert!(top_scorers(&World::synthetic(4), false).is_empty());
        let s = scorers(&w, false, 0);
        assert_eq!(s.rows.len(), 2);
        assert!(s.rows[0].starts_with("\u{6}\u{14}1\t\u{0} \t\u{2}\u{78}"));
        assert_eq!(s.flags & NUMBERED, NUMBERED);
    }

    #[test]
    fn career_scorers_use_the_history_counter() {
        let mut w = World::synthetic(4);
        w.divisions[0][1].slots[0].history[1] = 40;
        w.divisions[3][0].slots[4].history[1] = 35;
        w.divisions[3][1].slots[4].history[1] = 12;
        let t = top_scorers(&w, true);
        // threshold round(40 * 0.85) = 34
        assert_eq!(t.iter().map(|x| x.2).collect::<Vec<_>>(), [40, 35]);
    }

    #[test]
    fn standings_rows_carry_the_club_colours_and_the_order_uses_goal_difference() {
        let w = world();
        let d = &w.divisions[0];
        let order = table_order(d);
        for p in order.windows(2) {
            let (a, b) = (&d[p[0]], &d[p[1]]);
            assert!(
                (a.points, a.goals_for - a.goals_against, a.goals_for)
                    >= (b.points, b.goals_for - b.goals_against, b.goals_for)
            );
        }
        assert!(standing_row(&d[0]).starts_with('\u{1}'));
    }

    #[test]
    fn the_list_form_is_sized_to_its_rows() {
        let l = layout(4, 0);
        assert_eq!((l.list.1, l.client.1), (66, 297 + 66 - 225));
        let l = layout(100, WIDE);
        // never taller than the screen allows: 600 - (297 + 29) = 274 more pixels
        assert_eq!(l.list, (545, 225 + 274));
        assert_eq!(l.ok.0, 252);
        assert_eq!(layout(14, 0).list.1, 226);
    }

    #[test]
    fn a_list_dialog_selects_the_last_row_scrolls_and_closes() {
        let rows: Vec<String> = (0..40).map(|i| field("", 0x28, &i.to_string())).collect();
        let mut d = Dialog::new(Kind::Transactions, "TPRINTLISTDLG");
        bind_list(
            &mut d,
            Spec {
                caption: "x".into(),
                rows,
                flags: SELECT_LAST,
            },
        );
        // 40 rows: the list takes the 274 pixels the screen leaves and ends on the last row
        assert_eq!(d.texts["listOfItems.top"], (40 - 497 / 16).to_string());
        assert_eq!(d.texts["listOfItems.size"], "345,499");
        assert_eq!(d.texts["@client"], "378x575");
        assert!(matches!(
            click_list(&mut d, "listOfItems.pgup"),
            Outcome::Keep
        ));
        let t: usize = d.texts["listOfItems.top"].parse().unwrap();
        assert!(t < 40 - 501 / 16);
        // a click selects, the arrow keys scroll without going past either end
        click_list(&mut d, "listOfItems#3");
        assert_eq!(d.texts["listOfItems.sel"], "3");
        for _ in 0..100 {
            key_list(&mut d, Key::Up);
        }
        assert_eq!(d.texts["listOfItems.top"], "0");
        for _ in 0..100 {
            key_list(&mut d, Key::Down);
        }
        assert_eq!(d.texts["listOfItems.top"], (40 - 497 / 16).to_string());
        assert!(matches!(click_list(&mut d, "OKBtn"), Outcome::Close));
    }

    #[test]
    fn the_standings_close_after_ten_seconds_only_when_started_by_the_game() {
        let mut manual = standings_dialog(false);
        let mut auto = standings_dialog(true);
        let mut closed_at = None;
        for k in 0..60 {
            assert!(!timer_closes(&mut manual, 500));
            if timer_closes(&mut auto, 500) && closed_at.is_none() {
                closed_at = Some(k);
            }
        }
        assert_eq!(closed_at, Some(19), "after ten seconds");
        // a list with the auto-close flag closes after its rows plus three seconds
        let mut l = Dialog::new(Kind::Scorers, "TPRINTLISTDLG");
        bind_list(
            &mut l,
            Spec {
                caption: String::new(),
                rows: vec![field("", 5, "a"); 2],
                flags: AUTOCLOSE,
            },
        );
        let n = (0..100).find(|_| timer_closes(&mut l, 1000)).unwrap();
        assert_eq!(n, 5, "the sixth tick is the first beyond rows + 3");
    }

    #[test]
    fn transfers_and_acquisitions_are_rows_too() {
        let mut w = World::synthetic(6);
        w.play_season();
        let n = w.transfers.len();
        assert!(n > 0, "a season of the market has transfers");
        let s = transactions(&w);
        assert_eq!(s.rows.len(), n);
        assert_eq!(s.flags, WIDE | SELECT_LAST);
        assert!(s.rows[0].contains('\t'));
        assert!(acquisitions(&w).rows.is_empty());
    }
}
