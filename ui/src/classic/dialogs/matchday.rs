//! Dialogs of the match day: the squad dialog of a game (`TSubstitutionDlg`: at the break, after the sending-off of the goalkeeper, or
//! opened by a click on a club of the round view), the forced substitution after an injury (`TForcedSubstitutionDlg`), the referee aid,
//! the penalty of a primary chance (`TPenaltyDlg`), the shootout of a cup tie (`TPenaltiesDlg`), the draw of the cup (`TSorteioDlg`),
//! the end of the season (`TEndOfSeasonDlg`), the coach invitation (`TInviteCoachDlg`) and the list of sacked coaches. The flow that
//! opens them is in `classic/session.rs`; see there how the user's decisions are applied (the round is played again with them).
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::session::{find_club, info, Info, Step};
use crate::classic::{main_win, text, Classic, Key, Screen};
use elifoot_core::matchsim::{Decision, EventKind, Side};
use elifoot_core::outcome::Outcome as Shot;
use elifoot_core::season::Club;
use elifoot_core::sector::{Slot, Status};
use elifoot_core::world::ROUNDS;

fn rgb(c: [u8; 3]) -> String {
    format!("{},{},{}", c[0], c[1], c[2])
}

/// `(lettering, form colour)` of a club as texts: the original paints the windows of a club's game with its colours.
fn colors(club: Option<&Club>) -> (String, String) {
    match club {
        Some(c) => (rgb(c.colors.0), rgb(c.colors.1)),
        None => ("0,0,0".into(), "192,192,192".into()),
    }
}

fn parse_rgb(s: &str) -> Option<crate::win31::fb::Rgb> {
    let v: Vec<u8> = s.split(',').filter_map(|p| p.parse().ok()).collect();
    (v.len() == 3).then(|| crate::win31::fb::Rgb(v[0], v[1], v[2]))
}

fn star(s: &Slot) -> bool {
    s.g >= 8 && s.position >= 2
}

fn player_label(info: &Info, side: usize, i: usize) -> String {
    let n = info.names[side].get(i).cloned().unwrap_or_default();
    if star(&info.start[side][i]) {
        format!("{n}*")
    } else {
        n
    }
}

/// How far a game has gone when a squad is read from it.
#[derive(Clone, Copy)]
enum At {
    /// At the beginning of the minute: the events of earlier minutes, the planned substitutions of this one.
    Start(u16),
    /// Just after the events of the minute and before the automatic reply to them (the user is deciding).
    Reply(u16),
    /// After everything of the minute.
    End(u16),
}

fn squad_at(info: &Info, side: usize, at: At) -> Vec<Status> {
    let (minute, events_upto, auto) = match at {
        At::Start(m) => (m, m.saturating_sub(1), false),
        At::Reply(m) => (m, m, false),
        At::End(m) => (m, m, true),
    };
    let mut st: Vec<Status> = info.start[side].iter().map(|s| s.status).collect();
    let mut items: Vec<(u16, u8, usize)> = Vec::new();
    for (i, e) in info.events.iter().enumerate() {
        if (e.side == Side::Away) as usize == side
            && e.minute <= events_upto
            && matches!(e.kind, EventKind::RedCard | EventKind::Injury)
        {
            items.push((e.minute, 0, i));
        }
    }
    for (i, s) in info.subs.iter().enumerate() {
        if (s.side == Side::Away) as usize == side
            && (s.minute < minute || (s.minute == minute && (auto || !s.auto)))
        {
            items.push((s.minute, 1, i));
        }
    }
    items.sort();
    for (_, kind, i) in items {
        if kind == 0 {
            st[info.events[i].player] = Status::Out;
        } else {
            let s = info.subs[i];
            st[s.out] = Status::Out;
            st[s.inn] = Status::Starter;
        }
    }
    st
}

/// The squad of `side` after everything of `minute` (for the tests of the session).
#[cfg(test)]
pub fn squad_after(info: &Info, side: usize, minute: u16) -> Vec<Status> {
    squad_at(info, side, At::End(minute))
}

/// Slots with `status`, in the order of the lists (position, then name).
fn listed(info: &Info, side: usize, st: &[Status], status: Status) -> Vec<usize> {
    let mut v: Vec<usize> = (0..st.len()).filter(|&i| st[i] == status).collect();
    v.sort_by_key(|&i| {
        (
            info.start[side][i].position,
            info.names[side].get(i).map(|n| n.to_lowercase()),
        )
    });
    v
}

fn rows(info: &Info, side: usize, order: &[usize]) -> Vec<String> {
    order
        .iter()
        .map(|&i| {
            format!(
                "{}|{}|{}",
                ["G", "D", "M", "A"][info.start[side][i].position as usize],
                player_label(info, side, i),
                info.start[side][i].strength
            )
        })
        .collect()
}

/// The side of the user in a game of the session, when he plays in it.
fn human_side(c: &Classic, cup: bool, src: usize) -> Option<usize> {
    let sess = c.md.session.as_ref()?;
    let me = sess.snapshot.human.as_ref()?.club.as_str();
    let info = info(&c.world, cup, src)?;
    info.clubs.iter().position(|n| n == me)
}

fn goals(info: &Info, minute: u16) -> [i32; 2] {
    let mut g = [0, 0];
    for e in info.events {
        if e.kind == EventKind::Goal && e.minute <= minute {
            g[(e.side == Side::Away) as usize] += 1;
        }
    }
    g
}

fn side_of(s: usize) -> Side {
    if s == 0 {
        Side::Home
    } else {
        Side::Away
    }
}

/// The squad dialog. `mode` as in the original: 1 a club of the round view was clicked, 3 the user's goalkeeper was sent off, 4 the
/// break. `minute` is the minute on the clock, `next` the minute the substitutions made now would take place.
fn squad_dialog(cup: bool, src: usize, side: usize, minute: u16, mode: u8, next: u16) -> Dialog {
    let mut d = Dialog::new(Kind::Halftime, "TSUBSTITUTIONDLG");
    d.set_num("cup", cup as i64);
    d.set_num("src", src as i64);
    d.set_num("side", side as i64);
    d.set_num("minute", minute as i64);
    d.set_num("mode", mode as i64);
    d.set_num("next", next as i64);
    d.set_num("sel1", -1);
    d.set_num("sel2", -1);
    d.st.insert("pend".into(), String::new());
    d
}

/// The break of the user's game (`next`: 46, or 91 before the extra time).
pub fn halftime(_c: &Classic, cup: bool, src: usize, side: usize, next: u16) -> Dialog {
    squad_dialog(cup, src, side, next - 1, 4, next)
}

/// The user's goalkeeper was sent off at `minute`: the dialog lets him bring another player on (`mode` 3).
pub fn substitution(c: &Classic, cup: bool, src: usize, minute: u16, mode: u8) -> Dialog {
    let side = human_side(c, cup, src).unwrap_or(0);
    squad_dialog(cup, src, side, minute, mode, minute)
}

/// A click on a club of the round view: its squad and the events so far.
pub fn substitution_view(_c: &Classic, cup: bool, src: usize, side: usize, minute: u16) -> Dialog {
    squad_dialog(cup, src, side, minute, 1, minute + 1)
}

fn pending(d: &Dialog) -> Vec<(usize, usize)> {
    d.st.get("pend")
        .map(|p| {
            p.split(',')
                .filter_map(|x| x.split_once(':'))
                .filter_map(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                .collect()
        })
        .unwrap_or_default()
}

/// The squad of the dialog with the substitutions made in it applied, and how many the club can still make.
fn dialog_squad(info: &Info, d: &Dialog) -> (Vec<Status>, usize) {
    let side = d.num("side") as usize;
    let minute = d.num("minute") as u16;
    let at = match d.num("mode") {
        3 => At::Reply(minute),
        _ => At::End(minute),
    };
    let mut st = squad_at(info, side, at);
    let made = pending(d);
    for (o, i) in &made {
        st[*o] = Status::Out;
        st[*i] = Status::Starter;
    }
    let bench = info.start[side]
        .iter()
        .filter(|s| s.status == Status::Sub && s.available())
        .count()
        .min(3);
    let used = info
        .subs
        .iter()
        .filter(|s| {
            (s.side == Side::Away) as usize == side
                && (s.minute < minute || (s.minute == minute && d.num("mode") != 3))
        })
        .count()
        + made.len();
    (st, bench.saturating_sub(used))
}

fn event_lines(info: &Info, minute: u16, red: bool, injuries: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let mut score = [0, 0];
    for e in info.events.iter().filter(|e| e.minute <= minute) {
        let side = (e.side == Side::Away) as usize;
        let name = info.names[side].get(e.player).cloned().unwrap_or_default();
        match e.kind {
            EventKind::Goal => {
                score[side] += 1;
                lines.push(format!(
                    "{}:{} {} {}'{}",
                    score[0],
                    score[1],
                    name,
                    e.minute,
                    if e.primary { " (pen.)" } else { "" }
                ));
            }
            EventKind::RedCard if red => lines.push(format!("{name} expulso {}'", e.minute)),
            EventKind::Injury if injuries => {
                lines.push(format!("{name} lesionou-se {}'", e.minute))
            }
            EventKind::Annulled => lines.push(format!("{name} {}' - Anulado", e.minute)),
            _ => {}
        }
    }
    lines
}

fn referee_of(c: &Classic, cup: bool, info: &Info) -> String {
    let (Some(h), Some(a)) = (
        find_club(&c.world, &info.clubs[0]),
        find_club(&c.world, &info.clubs[1]),
    ) else {
        return String::new();
    };
    if !cup {
        if let Some(s) = c.md.session.as_ref() {
            let me = s.snapshot.human.as_ref().map(|h| h.club.as_str());
            if me.is_some_and(|m| info.clubs.iter().any(|n| n == m)) {
                return s.referee.clone();
            }
        }
    }
    main_win::referee(c, h, a)
}

pub fn bind(c: &Classic, d: &mut Dialog) {
    match d.kind {
        Kind::Halftime => bind_squad(c, d),
        Kind::ForcedSubstitution => bind_forced(c, d),
        Kind::Penalty => bind_penalty(c, d),
        Kind::Shootout => bind_shootout(c, d),
        Kind::CupDraw => bind_cup_draw(c, d),
        Kind::EndOfSeason => bind_end_of_season(c, d),
        Kind::Invitation => bind_invitation(c, d),
        Kind::Lashes => bind_lashes(c, d),
        Kind::RefereeAid => bind_referee_aid(c, d),
        _ => {}
    }
}

/// The text keys of a list box painted in the colours of a club.
fn club_list(d: &mut Dialog, name: &str, ink: &str, form: &str, cols: &str, ih: u32) {
    d.text(format!("{name}.color"), ink);
    d.text(format!("{name}.bgcolor"), form);
    d.text(format!("{name}.flat"), "1");
    d.text(format!("{name}.cols"), cols);
    // (an upper-case letter draws the column in bold: the name and the strength)
    d.text(format!("{name}.align"), "lLR");
    d.text(format!("{name}.itemheight"), ih.to_string());
    d.text(format!("{name}.inset"), "1");
}

fn set_sel(d: &mut Dialog, name: &str, row: Option<usize>) {
    match row {
        Some(r) => d.text(format!("{name}.sel"), r.to_string()),
        None => {
            d.texts.remove(&format!("{name}.sel"));
        }
    }
}

fn bind_squad(c: &Classic, d: &mut Dialog) {
    let (cup, src, side, minute) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
        d.num("minute") as u16,
    );
    let Some(info) = info(&c.world, cup, src) else {
        return;
    };
    let g = goals(&info, minute);
    d.text(
        "@caption",
        format!("{}, {} - {}, {}", info.clubs[0], g[0], info.clubs[1], g[1]),
    );
    d.text("textReferee", referee_of(c, cup, &info));
    d.text(
        "checkViewRedCards.checked",
        (c.md.view_red as u8).to_string(),
    );
    d.text(
        "checkViewInjuries.checked",
        (c.md.view_injuries as u8).to_string(),
    );
    d.lists.insert(
        "ListBoxOfEvents".into(),
        event_lines(&info, minute, c.md.view_red, c.md.view_injuries),
    );
    let me = human_side(c, cup, src) == Some(side);
    // the squads show only for the user's club (the original cuts the form short for the others)
    // (measured on the captures: the real form is 470 pixels wide; `Height` is the bottom of the squads' box + the caption's
    // 20 pixels + 5, or only the caption + the box's top - 2 for the clubs of the others)
    d.text("@client", if me { "470x377" } else { "470x169" });
    // (the squads' box lies below the cut-short form)
    for n in [
        "groupBoxSubstitution",
        "ListBoxOfSelectedPlayers",
        "listBoxOfSubstitutePlayers",
    ] {
        d.text(format!("{n}.visible"), (me as u8).to_string());
    }
    let (ink, form) = colors(find_club(&c.world, &info.clubs[side]));
    d.text("groupBoxSubstitution", info.clubs[side].clone());
    d.text("groupBoxSubstitution.color", ink.clone());
    d.text("groupBoxSubstitution.bgcolor", form.clone());
    let (st, left) = dialog_squad(&info, d);
    let on = listed(&info, side, &st, Status::Starter);
    let bench = listed(&info, side, &st, Status::Sub);
    for (name, order) in [
        ("ListBoxOfSelectedPlayers", &on),
        ("listBoxOfSubstitutePlayers", &bench),
    ] {
        d.lists.insert(name.into(), rows(&info, side, order));
        club_list(d, name, &ink, &form, "3,18,157", 16);
    }
    let sel = |key: &str, order: &[usize]| order.iter().position(|&i| i as i64 == d.num(key));
    let (s1, s2) = (sel("sel1", &on), sel("sel2", &bench));
    set_sel(d, "ListBoxOfSelectedPlayers", s1);
    set_sel(d, "listBoxOfSubstitutePlayers", s2);
    // the Substituir button: a player on the pitch and one on the bench of the same kind (a goalkeeper for a goalkeeper); after
    // the sending-off of the goalkeeper any field player may make way for a goalkeeper
    let mode = d.num("mode");
    let ok = match (s1.map(|r| on[r]), s2.map(|r| bench[r])) {
        (Some(o), Some(i)) => {
            let (po, pi) = (info.start[side][o].position, info.start[side][i].position);
            (po == 0) == (pi == 0) || (mode == 3 && pi == 0)
        }
        _ => false,
    };
    let can = me && left > 0 && ok && (mode != 3 || pending(d).is_empty());
    d.text("substituteBtn.enabled", (can as u8).to_string());
    d.text("OKBtn", "Continuar");
    if d.focus.is_empty() {
        d.focus = "OKBtn".into();
    }
}

fn pos_word(position: u8) -> &'static str {
    text::POSITIONS[position as usize % 4]
}

/// `TForcedSubstitutionDlg`: a player of the user is injured and the user chooses who comes on.
pub fn forced(c: &Classic, cup: bool, src: usize, minute: u16, player: usize) -> Dialog {
    let mut d = Dialog::new(Kind::ForcedSubstitution, "TFORCEDSUBSTITUTIONDLG");
    let side = human_side(c, cup, src).unwrap_or(0);
    d.set_num("cup", cup as i64);
    d.set_num("src", src as i64);
    d.set_num("side", side as i64);
    d.set_num("minute", minute as i64);
    d.set_num("player", player as i64);
    d.set_num("sel", -1);
    d
}

fn forced_bench(info: &Info, d: &Dialog) -> Vec<usize> {
    let side = d.num("side") as usize;
    let st = squad_at(info, side, At::Reply(d.num("minute") as u16));
    listed(info, side, &st, Status::Sub)
}

fn bind_forced(c: &Classic, d: &mut Dialog) {
    let (cup, src, side) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
    );
    let Some(info) = info(&c.world, cup, src) else {
        return;
    };
    let player = d.num("player") as usize;
    let (ink, form) = colors(find_club(&c.world, &info.clubs[side]));
    d.bg = parse_rgb(&form);
    d.text(
        "Label1",
        format!(
            "{} ({}) lesionou-se e tem de ser substituído.\nEscolha o jogador a entrar.",
            info.names[side].get(player).cloned().unwrap_or_default(),
            pos_word(info.start[side][player].position)
        ),
    );
    d.text("Label1.color", ink.clone());
    let bench = forced_bench(&info, d);
    let name = "ListBoxOfSubstitutePlayers";
    d.lists.insert(name.into(), rows(&info, side, &bench));
    club_list(d, name, &ink, &form, "3,18,200", 15);
    let sel = bench.iter().position(|&i| i as i64 == d.num("sel"));
    set_sel(d, name, sel);
    d.text("OKBtn.enabled", (sel.is_some() as u8).to_string());
}

/// The penalty of a primary chance at `minute` of the user's game: `attack` when the user's club shoots (he picks the shooter).
pub fn penalty(c: &Classic, cup: bool, src: usize, minute: u16, attack: bool) -> Dialog {
    let mut d = Dialog::new(Kind::Penalty, "TPENALTYDLG");
    let me = human_side(c, cup, src).unwrap_or(0);
    let side = if attack { me } else { 1 - me };
    d.set_num("cup", cup as i64);
    d.set_num("src", src as i64);
    d.set_num("minute", minute as i64);
    d.set_num("side", side as i64);
    d.set_num("attack", attack as i64);
    d.set_num("sel", -1);
    d.set_num("ms", 0);
    d.set_num("shot", 0);
    d
}

/// The primary chance of the dialog as the game has it now.
fn primary_of(c: &Classic, d: &Dialog) -> Option<elifoot_core::matchsim::Primary> {
    let (cup, src, minute) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("minute") as u16,
    );
    let list = if cup {
        &c.world.last_cup.as_ref()?.matches.get(src)?.primaries
    } else {
        &c.world.last_round.get(src)?.1.primaries
    };
    list.iter().find(|p| p.minute == minute).copied()
}

fn penalty_starters(info: &Info, d: &Dialog) -> Vec<usize> {
    let side = d.num("side") as usize;
    let st = squad_at(info, side, At::Start(d.num("minute") as u16));
    listed(info, side, &st, Status::Starter)
}

fn outcome_text(o: Shot) -> &'static str {
    match o {
        Shot::Goal => "GOLO",
        Shot::Saved => "Defendeu",
        Shot::Post => "Ao poste",
        Shot::Crossbar => "À barra",
        Shot::Wide => "Ao lado",
        Shot::Over => "Por cima",
    }
}

fn bind_penalty(c: &Classic, d: &mut Dialog) {
    let (cup, src, side) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
    );
    let Some(info) = info(&c.world, cup, src) else {
        return;
    };
    let (ink, form) = colors(find_club(&c.world, &info.clubs[side]));
    d.text("@caption", info.clubs[side].clone());
    d.bg = parse_rgb(&form);
    for n in ["Label1", "LabelChoose", "labelResult"] {
        d.text(format!("{n}.color"), ink.clone());
    }
    let attack = d.num("attack") != 0;
    let shot = d.num("shot") != 0;
    let ticks = d.num("ms") / 1000;
    let prim = primary_of(c, d);
    let name_of = |p: usize| info.names[side].get(p).cloned().unwrap_or_default();
    // the user's club chooses the shooter first; otherwise the shooter is announced after a second
    let choosing = attack && !shot;
    let (show_label, label) = match prim {
        _ if choosing => (
            true,
            format!("{}, escolha o jogador para marcar o penalti!", c.coach),
        ),
        Some(p) => (ticks >= 1, format!("Marcador: {}", name_of(p.shooter))),
        None => (false, String::new()),
    };
    d.text("LabelChoose", label);
    d.text("LabelChoose.visible", (show_label as u8).to_string());
    d.text("Bevel1.visible", (choosing as u8).to_string());
    d.text("ListBoxOfPlayers.visible", (choosing as u8).to_string());
    d.text("shootBtn.visible", (choosing as u8).to_string());
    if choosing {
        let order = penalty_starters(&info, d);
        let name = "ListBoxOfPlayers";
        d.lists.insert(name.into(), rows(&info, side, &order));
        club_list(d, name, &ink, &form, "3,18,140", 16);
        let sel = order.iter().position(|&i| i as i64 == d.num("sel"));
        set_sel(d, name, sel);
        d.text("shootBtn.enabled", (sel.is_some() as u8).to_string());
    }
    let show_result = !choosing && ticks >= 3;
    d.text("labelResult.visible", (show_result as u8).to_string());
    if let (true, Some(p)) = (show_result, prim) {
        d.text("labelResult", outcome_text(p.outcome));
    }
    if d.focus.is_empty() {
        d.focus = "shootBtn".into();
    }
}

fn bind_referee_aid(c: &Classic, d: &mut Dialog) {
    let (cup, src) = (d.num("cup") != 0, d.num("src") as usize);
    let Some(info) = info(&c.world, cup, src) else {
        return;
    };
    for (k, (name, gauge, bonus)) in [
        ("textHomeTeam", "gaugeHomeAid", info.bonus.0),
        ("textAwayTeam", "gaugeAwayAid", info.bonus.1),
    ]
    .into_iter()
    .enumerate()
    {
        let (ink, form) = colors(find_club(&c.world, &info.clubs[k]));
        d.text(name, info.clubs[k].clone());
        d.text(format!("{name}.color"), ink);
        d.text(format!("{name}.bgcolor"), form);
        d.text(format!("{gauge}.value"), bonus.clamp(0, 15).to_string());
    }
}

/// The shootout of a cup tie (`TPenaltiesDlg`): the recorded kicks come one after the other.
fn shootout(c: &Classic, src: usize) -> Dialog {
    let mut d = Dialog::new(Kind::Shootout, "TPENALTIESDLG");
    d.set_num("src", src as i64);
    d.set_num("ms", 0);
    d.set_num("human", human_side(c, true, src).is_some() as i64);
    d
}

fn bind_shootout(c: &Classic, d: &mut Dialog) {
    let src = d.num("src") as usize;
    let Some(m) = c.world.last_cup.as_ref().and_then(|r| r.matches.get(src)) else {
        return;
    };
    let Some(p) = &m.pens else {
        return;
    };
    let per_kick = if d.num("human") != 0 { 3000 } else { 1000 };
    let shown = ((d.num("ms") / per_kick) as usize + 1).min(p.kicks.len());
    let (mut h, mut a) = (0, 0);
    // a row per pair of kicks: home taker, his result, away taker, his result
    let mut table: Vec<[String; 4]> = Vec::new();
    let names = [&m.home_players, &m.away_players];
    for k in p.kicks.iter().take(shown) {
        let side = k.side as usize;
        let who = names[side].get(k.player).cloned().unwrap_or_default();
        let result = outcome_text(k.outcome).to_string();
        let hit = k.outcome == Shot::Goal;
        if side == 0 {
            h += hit as i32;
            table.push([who, result, String::new(), String::new()]);
        } else {
            a += hit as i32;
            match table.last_mut().filter(|r| r[2].is_empty()) {
                Some(r) => {
                    r[2] = who;
                    r[3] = result;
                }
                None => table.push([String::new(), String::new(), who, result]),
            }
        }
    }
    let (hc, ac) = (
        colors(find_club(&c.world, &m.home)),
        colors(find_club(&c.world, &m.away)),
    );
    d.text("textHomeTeamName", m.home.clone());
    d.text("textHomeTeamName.color", hc.0);
    d.text("textHomeTeamName.bgcolor", hc.1);
    d.text("textAwayTeamName", m.away.clone());
    d.text("textAwayTeamName.color", ac.0);
    d.text("textAwayTeamName.bgcolor", ac.1);
    d.text("textResult", format!("{h}:{a}"));
    d.lists.insert(
        "ListBoxPenalties".into(),
        table.iter().map(|r| r.join("|")).collect(),
    );
    // the sizeable form at its design position (Left 0, Top 113), 640x357 with its frame
    d.text("@frame", "thick");
    d.text("@client", "634x332");
    d.text("@origin", "0,113");
    d.text("ListBoxPenalties.cols", "8,140,290,420");
    d.text("ListBoxPenalties.align", "llll");
    d.text("ListBoxPenalties.itemheight", "22");
}

/// The cup draw dialog, when the options ask for it (`1020:0ed4`): always, or when the user's club is drawn.
pub fn cup_draw(c: &Classic) -> Option<Dialog> {
    let cup = c.world.last_cup.as_ref()?;
    let sess = c.md.session.as_ref()?;
    let me = sess.snapshot.human.as_ref()?.club.clone();
    let human_drawn = cup.matches.iter().any(|m| m.home == me || m.away == me);
    let show = c.opts.cup_draw == 2 || (c.opts.cup_draw == 1 && human_drawn);
    if !show || sess.snapshot.cup.len() <= 2 {
        return None;
    }
    let mut d = Dialog::new(Kind::CupDraw, "TSORTEIODLG");
    d.set_num("ms", 0);
    d.set_num("human", human_drawn as i64);
    Some(d)
}

fn bind_cup_draw(c: &Classic, d: &mut Dialog) {
    let (Some(cup), Some(sess)) = (c.world.last_cup.as_ref(), c.md.session.as_ref()) else {
        return;
    };
    let tick = if d.num("human") != 0 { 1500 } else { 400 };
    // two timer ticks per game: the home team is named, then the away team and the game goes into the list
    let ticks = (d.num("ms") / tick) as usize;
    let (done, home_only) = (ticks / 2, ticks % 2 == 1);
    let mut teams: Vec<String> = sess.snapshot.cup.clone();
    teams.sort_by_key(|n| n.to_lowercase());
    let games = cup.matches.len();
    let current = cup.matches.get(done.min(games.saturating_sub(1)));
    let (home, away) = match current {
        Some(m) if done < games && home_only => (Some(m), None),
        Some(_) if done > 0 && !home_only => {
            let m = &cup.matches[(done - 1).min(games - 1)];
            (Some(m), Some(m))
        }
        Some(_) | None => (None, None),
    };
    let sel = home.and_then(|m| teams.iter().position(|n| *n == m.home));
    d.lists.insert("listBoxOfCupTeams".into(), teams);
    set_sel(d, "listBoxOfCupTeams", sel);
    let shown = if home_only { done } else { done.min(games) };
    let list: Vec<String> = cup
        .matches
        .iter()
        .take(shown)
        .map(|m| format!("{} - {}", m.home, m.away))
        .collect();
    let last = list.len().checked_sub(1);
    d.lists.insert("listBoxOfGames".into(), list);
    set_sel(d, "listBoxOfGames", last);
    let mut label = |name: &str, team: Option<&str>| match team {
        Some(t) => {
            let (ink, form) = colors(find_club(&c.world, t));
            d.text(name, t);
            d.text(format!("{name}.color"), ink);
            d.text(format!("{name}.bgcolor"), form);
        }
        None => {
            d.text(name, "");
            d.texts.remove(&format!("{name}.bgcolor"));
        }
    };
    label("textHomeTeam", home.map(|m| m.home.as_str()));
    label("textAwayTeam", away.map(|m| m.away.as_str()));
}

fn bind_end_of_season(c: &Classic, d: &mut Dialog) {
    let Some(s) = c.world.last_season.as_ref() else {
        return;
    };
    let champion = s.tables[0]
        .first()
        .map(|r| r.name.clone())
        .unwrap_or_default();
    let cup = s.cup_winner.clone().unwrap_or_default();
    for (name, club, times, coach_times, coach, kind) in [
        (
            "textLeagueWinnerName",
            &champion,
            "textLeagueWinnerTimes",
            "textLeagueWinnerCoach",
            &s.honours.league_coach,
            0,
        ),
        (
            "textCupWinnerName",
            &cup,
            "textCupWinnerTimes",
            "textCupWinnerCoach",
            &s.honours.cup_coach,
            1,
        ),
    ] {
        let (ink, form) = colors(find_club(&c.world, club));
        d.text(name, club.clone());
        d.text(format!("{name}.color"), ink);
        d.text(format!("{name}.bgcolor"), form);
        let t = c.md.titles.get(club).copied().unwrap_or_default();
        let k = c.md.coach_titles.get(coach).copied().unwrap_or_default();
        let (n, nk) = if kind == 0 { (t.0, k.0) } else { (t.1, k.1) };
        let word = if kind == 0 {
            "vitória no campeonato"
        } else {
            "taça"
        };
        d.text(times, format!("{n}ª {word}"));
        d.text(coach_times, format!("{coach} ({nk}ª {word})"));
    }
    d.text(
        "labelBestScorer",
        format!("Melhor marcador ({} golos)", s.honours.top_goals),
    );
    d.lists.insert(
        "ListBoxBestScorers".into(),
        s.honours
            .top_scorers
            .iter()
            .map(|(p, club)| format!("{p}  ({club})"))
            .collect(),
    );
    let [p1, p2, p3] = s.honours.prizes;
    d.text("textPrizeLeagueWinner", text::money_words(p1, "reais"));
    d.text("textPrizeCupWinner", text::money_words(p2, "reais"));
    d.text("textPrizeBestScorer", text::money_words(p3, "reais"));
}

fn bind_invitation(_c: &Classic, d: &mut Dialog) {
    d.text(
        "textCoachName",
        d.st.get("coach").cloned().unwrap_or_default(),
    );
    d.text(
        "textTeamName",
        format!("{} ?", d.st.get("team").cloned().unwrap_or_default()),
    );
}

/// The dialog of an invitation to coach `team` (a club whose coach was sacked).
pub fn invitation(c: &Classic, team: &str) -> Dialog {
    let mut d = Dialog::new(Kind::Invitation, "TINVITECOACHDLG");
    d.st.insert("coach".into(), c.coach.clone());
    d.st.insert("team".into(), team.into());
    d
}

fn bind_lashes(_c: &Classic, d: &mut Dialog) {
    d.text("@caption", "Chicotadas Psicológicas");
    let lines = d.lists.get("lines").cloned().unwrap_or_default();
    d.lists.insert("listOfItems".into(), lines);
}

/// The sackings of a round as the list of the original: the sacked coach, the coach who takes over, an empty line.
fn lashes(c: &Classic, from: usize, to: usize) -> Option<Dialog> {
    let sess = c.md.session.as_ref()?;
    let me = sess.snapshot.human.as_ref().map(|h| h.club.clone());
    let all = c.opts.lashes == 2;
    let coach = |club: &str| {
        let n = sess.coaches.get(club).cloned().unwrap_or_default();
        if n.is_empty() {
            "Treinador".to_string()
        } else {
            n
        }
    };
    let mut lines = Vec::new();
    for (sacked, gave) in c
        .world
        .coach_changes
        .iter()
        .skip(from)
        .take(to.saturating_sub(from))
    {
        let human = me.as_deref() == Some(sacked.as_str()) || me.as_deref() == Some(gave.as_str());
        if !(all || (c.opts.lashes == 1 && human)) {
            continue;
        }
        lines.push(format!("{} foi despedido do {sacked}", coach(sacked)));
        lines.push(format!("Troca com {} ({gave})", coach(gave)));
        lines.push(String::new());
    }
    (!lines.is_empty()).then(|| lashes_dialog(lines))
}

/// The list of sacked coaches ("Chicotadas Psicológicas") with these lines.
pub fn lashes_dialog(lines: Vec<String>) -> Dialog {
    let mut d = Dialog::new(Kind::Lashes, "TPRINTLISTDLG");
    d.lists.insert("lines".into(), lines);
    d
}

pub fn key(_c: &mut Classic, _d: &mut Dialog, _key: Key) -> Option<Outcome> {
    None
}

pub fn tick(c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    match d.kind {
        Kind::Penalty => {
            // the shooter is chosen first when the user's club shoots; then the result comes after three seconds and the window
            // closes after five (`Timer1`)
            if d.num("attack") == 0 || d.num("shot") != 0 {
                d.set_num("ms", d.num("ms") + ms as i64);
            }
            if d.num("ms") / 1000 >= 5 {
                return Outcome::Close;
            }
            Outcome::Keep
        }
        Kind::Shootout => {
            let src = d.num("src") as usize;
            let n = c
                .world
                .last_cup
                .as_ref()
                .and_then(|r| r.matches.get(src))
                .and_then(|m| m.pens.as_ref())
                .map_or(0, |p| p.kicks.len()) as i64;
            let per_kick = if d.num("human") != 0 { 3000 } else { 1000 };
            d.set_num("ms", d.num("ms") + ms as i64);
            // after the last kick three seconds more (`Timer2`)
            if d.num("ms") >= n * per_kick + 3000 {
                return Outcome::Close;
            }
            Outcome::Keep
        }
        Kind::CupDraw => {
            let games = c.world.last_cup.as_ref().map_or(0, |r| r.matches.len()) as i64;
            let tick = if d.num("human") != 0 { 1500 } else { 400 };
            d.set_num("ms", d.num("ms") + ms as i64);
            // two ticks per game, then five more before the window closes
            if d.num("ms") / tick >= games * 2 + 5 {
                return Outcome::Close;
            }
            Outcome::Keep
        }
        Kind::EndOfSeason => {
            d.set_num("ms", d.num("ms") + ms as i64);
            // `TimerCount` closes the window after eleven seconds
            if d.num("ms") > 11_000 {
                return Outcome::Close;
            }
            Outcome::Keep
        }
        _ => Outcome::Keep,
    }
}

pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match d.kind {
        Kind::Halftime => click_squad(c, d, name),
        Kind::ForcedSubstitution => click_forced(c, d, name),
        Kind::Penalty => click_penalty(c, d, name),
        Kind::Invitation => click_invitation(c, d, name),
        Kind::Shootout | Kind::CupDraw => Outcome::Keep,
        _ => {
            if name == "OKBtn" {
                Outcome::Close
            } else {
                Outcome::Keep
            }
        }
    }
}

fn click_squad(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let (cup, src, side) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
    );
    match name {
        "checkViewRedCards" => c.md.view_red = !c.md.view_red,
        "checkViewInjuries" => c.md.view_injuries = !c.md.view_injuries,
        "textReferee" => {
            let mut r = Dialog::new(Kind::RefereeAid, "TREFEREEAIDDLG");
            r.set_num("cup", cup as i64);
            r.set_num("src", src as i64);
            return Outcome::Open(Box::new(r));
        }
        "substituteBtn" => {
            let Some(i) = info(&c.world, cup, src) else {
                return Outcome::Keep;
            };
            let (st, _) = dialog_squad(&i, d);
            let on = listed(&i, side, &st, Status::Starter);
            let bench = listed(&i, side, &st, Status::Sub);
            let (s1, s2) = (d.num("sel1"), d.num("sel2"));
            if on.iter().any(|&k| k as i64 == s1) && bench.iter().any(|&k| k as i64 == s2) {
                let mut p = d.st.get("pend").cloned().unwrap_or_default();
                if !p.is_empty() {
                    p.push(',');
                }
                p.push_str(&format!("{s1}:{s2}"));
                d.st.insert("pend".into(), p);
                d.set_num("sel1", -1);
                d.set_num("sel2", -1);
            }
        }
        "OKBtn" => return close_squad(c, d),
        _ => {
            let pick = |prefix: &str| {
                name.strip_prefix(prefix)
                    .and_then(|r| r.parse::<usize>().ok())
            };
            if let Some(i) = info(&c.world, cup, src) {
                let (st, _) = dialog_squad(&i, d);
                if let Some(r) = pick("ListBoxOfSelectedPlayers#") {
                    if let Some(&slot) = listed(&i, side, &st, Status::Starter).get(r) {
                        d.set_num("sel1", slot as i64);
                    }
                } else if let Some(r) = pick("listBoxOfSubstitutePlayers#") {
                    if let Some(&slot) = listed(&i, side, &st, Status::Sub).get(r) {
                        d.set_num("sel2", slot as i64);
                    }
                }
            }
        }
    }
    Outcome::Keep
}

/// Continuar: what the user did in the dialog becomes decisions of the game and the round is played again with them.
fn close_squad(c: &mut Classic, d: &mut Dialog) -> Outcome {
    let (cup, side, mode) = (
        d.num("cup") != 0,
        side_of(d.num("side") as usize),
        d.num("mode"),
    );
    let made = pending(d);
    let minute = d.num("minute") as u16;
    let next = d.num("next") as u16;
    // the break is over: the clock turns red (or purple before the extra time)
    if mode == 4 {
        if let Screen::Round(rv) = &mut c.screen {
            if next == 91 {
                rv.extra_seen = true;
            } else {
                rv.halftime_seen = true;
            }
        }
    }
    let mut ds = Vec::new();
    match mode {
        3 => match made.first() {
            Some(&(o, i)) => ds.push(Decision::Substitute {
                side,
                minute,
                out: Some(o),
                inn: i,
            }),
            None => ds.push(Decision::NoSubstitution { side, minute }),
        },
        _ => {
            for (o, i) in made {
                ds.push(Decision::Planned {
                    side,
                    minute: next,
                    out: o,
                    inn: i,
                });
            }
        }
    }
    if !ds.is_empty() && human_side(c, cup, d.num("src") as usize).is_some() {
        c.decide(cup, ds);
    }
    Outcome::Close
}

fn click_forced(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let (cup, src, side) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
    );
    if let Some(r) = name
        .strip_prefix("ListBoxOfSubstitutePlayers#")
        .and_then(|r| r.parse::<usize>().ok())
    {
        if let Some(i) = info(&c.world, cup, src) {
            if let Some(&slot) = forced_bench(&i, d).get(r) {
                d.set_num("sel", slot as i64);
            }
        }
        return Outcome::Keep;
    }
    if name == "OKBtn" && d.num("sel") >= 0 {
        c.decide(
            cup,
            vec![Decision::Substitute {
                side: side_of(side),
                minute: d.num("minute") as u16,
                out: None,
                inn: d.num("sel") as usize,
            }],
        );
        return Outcome::Close;
    }
    Outcome::Keep
}

fn click_penalty(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let (cup, src, side) = (
        d.num("cup") != 0,
        d.num("src") as usize,
        d.num("side") as usize,
    );
    let choosing = d.num("attack") != 0 && d.num("shot") == 0;
    if !choosing {
        return Outcome::Keep;
    }
    if let Some(r) = name
        .strip_prefix("ListBoxOfPlayers#")
        .and_then(|r| r.parse::<usize>().ok())
    {
        if let Some(i) = info(&c.world, cup, src) {
            if let Some(&slot) = penalty_starters(&i, d).get(r) {
                d.set_num("sel", slot as i64);
            }
        }
    } else if name == "shootBtn" && d.num("sel") >= 0 {
        c.decide(
            cup,
            vec![Decision::Shooter {
                side: side_of(side),
                minute: d.num("minute") as u16,
                player: d.num("sel") as usize,
            }],
        );
        d.set_num("shot", 1);
        d.set_num("ms", 0);
    }
    Outcome::Keep
}

fn click_invitation(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "yesBtn" => Outcome::Close,
        "noBtn" => {
            // the user declines: the round is played again without the invitation
            let (l, cp) = match c.md.session.as_mut() {
                Some(s) => {
                    s.accept = false;
                    (s.league.clone(), s.cup.clone())
                }
                None => return Outcome::Close,
            };
            c.replay_round(l, cp);
            Outcome::Close
        }
        "plantelBtn" => {
            let team = d.st.get("team").cloned().unwrap_or_default();
            Outcome::Open(Box::new(super::market::open_plantel(c, &team, None)))
        }
        _ => Outcome::Keep,
    }
}

/// The round view is over (league part): queue what the original shows next, in its order.
pub fn end_of_round(c: &mut Classic) {
    let Some(sess) = c.md.session.as_ref() else {
        c.md.todo.push_back(Step::Main);
        return;
    };
    let round = sess.snapshot.round + 1;
    let human_played = sess.snapshot.human.is_some();
    let first_cup_change = c
        .world
        .last_cup
        .as_ref()
        .map_or(c.world.coach_changes.len(), |r| r.coach_changes_from);
    if let Some((_, to)) = c.is_invitation() {
        let d = invitation(c, &to);
        c.md.todo.push_back(Step::Open(Box::new(d)));
    }
    if let Some(d) = lashes(c, 0, first_cup_change) {
        c.md.todo.push_back(Step::Open(Box::new(d)));
    }
    if human_played || round == ROUNDS {
        let d = super::tables::standings_dialog(true);
        c.md.todo.push_back(Step::Open(Box::new(d)));
    }
    if c.world
        .last_cup
        .as_ref()
        .is_some_and(|r| !r.matches.is_empty())
    {
        c.md.todo.push_back(Step::CupDraw);
        c.md.todo.push_back(Step::CupView);
    } else {
        finish_steps(c, round);
    }
}

/// The cup view is over: the shootouts, the sackings of the cup round, the end of the season.
pub fn end_of_cup(c: &mut Classic) {
    let Some(sess) = c.md.session.as_ref() else {
        c.md.todo.push_back(Step::Main);
        return;
    };
    let round = sess.snapshot.round + 1;
    let ties: Vec<usize> = c
        .world
        .last_cup
        .as_ref()
        .map(|r| {
            r.matches
                .iter()
                .enumerate()
                .filter(|(_, m)| m.pens.is_some())
                .map(|(i, _)| i)
                .collect()
        })
        .unwrap_or_default();
    for i in ties {
        let d = shootout(c, i);
        c.md.todo.push_back(Step::Open(Box::new(d)));
    }
    let from = c
        .world
        .last_cup
        .as_ref()
        .map_or(c.world.coach_changes.len(), |r| r.coach_changes_from);
    if let Some(d) = lashes(c, from, c.world.coach_changes.len()) {
        c.md.todo.push_back(Step::Open(Box::new(d)));
    }
    finish_steps(c, round);
}

fn finish_steps(c: &mut Classic, round: usize) {
    if round == ROUNDS {
        if let Some(s) = c.world.last_season.as_ref() {
            // the season is over: count the titles and show the winners
            let champion = s.tables[0].first().map(|r| r.name.clone());
            let (cup, lc, cc) = (
                s.cup_winner.clone(),
                s.honours.league_coach.clone(),
                s.honours.cup_coach.clone(),
            );
            if let Some(ch) = champion {
                c.md.titles.entry(ch).or_default().0 += 1;
            }
            if let Some(cp) = cup {
                c.md.titles.entry(cp).or_default().1 += 1;
            }
            c.md.coach_titles.entry(lc).or_default().0 += 1;
            c.md.coach_titles.entry(cc).or_default().1 += 1;
            let mut d = Dialog::new(Kind::EndOfSeason, "TENDOFSEASONDLG");
            d.set_num("ms", 0);
            c.md.todo.push_back(Step::Open(Box::new(d)));
        }
    }
    c.md.todo.push_back(Step::Auctions);
    c.md.todo.push_back(Step::Main);
}

#[cfg(test)]
mod tests {
    use super::*;
    use elifoot_core::matchsim::{Event, SubEvent};

    fn slot(position: u8, strength: u16, status: Status) -> Slot {
        Slot::new("x", position, strength, true, status)
    }

    /// Eleven starters (keeper first), three substitutes: a keeper, a defender and an attacker.
    fn squad() -> Vec<Slot> {
        let mut v = vec![slot(0, 5, Status::Starter)];
        v.extend((0..4).map(|_| slot(1, 5, Status::Starter)));
        v.extend((0..4).map(|_| slot(2, 5, Status::Starter)));
        v.extend((0..2).map(|_| slot(3, 5, Status::Starter)));
        v.extend([
            slot(0, 4, Status::Sub),
            slot(1, 4, Status::Sub),
            slot(3, 4, Status::Sub),
        ]);
        v
    }

    fn ev(minute: u16, side: Side, kind: EventKind, player: usize) -> Event {
        Event {
            minute,
            side,
            kind,
            player,
            position: 1,
            primary: false,
        }
    }

    fn info<'a>(
        start: &'a [Slot],
        names: &'a [String],
        events: &'a [Event],
        subs: &'a [SubEvent],
    ) -> Info<'a> {
        Info {
            clubs: ["Home".into(), "Away".into()],
            names: [names, names],
            start: [start, start],
            events,
            subs,
            bonus: (7, 5),
        }
    }

    #[test]
    fn the_squad_at_a_minute_follows_injuries_and_substitutions() {
        let (s, names) = (
            squad(),
            (0..14).map(|i| format!("P{i}")).collect::<Vec<_>>(),
        );
        let events = [ev(30, Side::Home, EventKind::Injury, 2)];
        let subs = [SubEvent {
            minute: 30,
            side: Side::Home,
            out: 2,
            inn: 12,
            auto: true,
        }];
        let i = info(&s, &names, &events, &subs);
        // before the injury everybody started; the other side never changes
        assert_eq!(squad_at(&i, 0, At::End(29))[2], Status::Starter);
        assert_eq!(squad_at(&i, 1, At::End(60))[2], Status::Starter);
        // while the user decides the injured player is off and the reply (a substitution) is not yet made
        let reply = squad_at(&i, 0, At::Reply(30));
        assert_eq!((reply[2], reply[12]), (Status::Out, Status::Sub));
        // afterwards the substitute is on
        let after = squad_at(&i, 0, At::End(30));
        assert_eq!((after[2], after[12]), (Status::Out, Status::Starter));
        // at the start of the minute of a planned substitution only the planned ones count
        let planned = [SubEvent {
            minute: 46,
            side: Side::Home,
            out: 3,
            inn: 11,
            auto: false,
        }];
        let i = info(&s, &names, &[], &planned);
        assert_eq!(squad_at(&i, 0, At::End(45))[3], Status::Starter);
        assert_eq!(squad_at(&i, 0, At::Start(46))[3], Status::Out);
    }

    #[test]
    fn the_substitutions_left_count_the_ones_made() {
        let (s, names) = (
            squad(),
            (0..14).map(|i| format!("P{i}")).collect::<Vec<_>>(),
        );
        let subs = [SubEvent {
            minute: 20,
            side: Side::Away,
            out: 4,
            inn: 13,
            auto: true,
        }];
        let i = info(&s, &names, &[], &subs);
        // three fit substitutes: three changes, minus the one the away side made
        assert_eq!(i.subs_left(0, 60, true), 3);
        assert_eq!(i.subs_left(1, 60, true), 2);
        assert_eq!(
            i.subs_left(1, 20, false),
            3,
            "the automatic reply of the minute is not counted yet"
        );
    }

    #[test]
    fn the_event_list_has_the_score_and_the_filters_of_the_check_boxes() {
        let (s, names) = (
            squad(),
            (0..14).map(|i| format!("P{i}")).collect::<Vec<_>>(),
        );
        let mut pen = ev(40, Side::Away, EventKind::Goal, 9);
        pen.primary = true;
        let events = [
            ev(10, Side::Home, EventKind::Goal, 10),
            ev(20, Side::Home, EventKind::RedCard, 3),
            ev(25, Side::Away, EventKind::Injury, 4),
            pen,
            ev(41, Side::Away, EventKind::Annulled, 9),
        ];
        let i = info(&s, &names, &events, &[]);
        assert_eq!(
            event_lines(&i, 90, false, false),
            vec!["1:0 P10 10'", "1:1 P9 40' (pen.)", "P9 41' - Anulado"]
        );
        let all = event_lines(&i, 90, true, true);
        assert_eq!(all.len(), 5);
        assert_eq!(all[1], "P3 expulso 20'");
        assert_eq!(all[2], "P4 lesionou-se 25'");
        // only what happened so far
        assert_eq!(event_lines(&i, 15, true, true), vec!["1:0 P10 10'"]);
        assert_eq!(goals(&i, 45), [1, 1]);
    }

    #[test]
    fn the_squad_lists_are_sorted_by_position_and_then_by_name() {
        let names: Vec<String> = ["Zed", "Bob", "Ann", "Cid"].map(String::from).to_vec();
        let start = vec![
            slot(2, 9, Status::Starter),
            slot(0, 8, Status::Starter),
            slot(2, 7, Status::Starter),
            slot(1, 6, Status::Starter),
        ];
        let i = info(&start, &names, &[], &[]);
        let st = vec![Status::Starter; 4];
        let order = listed(&i, 0, &st, Status::Starter);
        assert_eq!(order, vec![1, 3, 2, 0]);
        assert_eq!(rows(&i, 0, &order)[0], "G|Bob|8");
        assert_eq!(rows(&i, 0, &order)[2], "M|Ann|7");
    }

    #[test]
    fn colours_are_read_from_the_texts() {
        assert_eq!(parse_rgb("1,2,3"), Some(crate::win31::fb::Rgb(1, 2, 3)));
        assert_eq!(parse_rgb("1,2"), None);
    }
}
