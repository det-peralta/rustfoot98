//! Season layer: the league fixture table, rounds, standings and per-round evolution.
//!
//! **Verified in the original:** the 8-team double round-robin table (`DS:0x1838`, read by `1010:1d20`),
//! the 14 league rounds (`1020:11b8` loops 0..=14), **3-1-0 points** (`1010:9396` adds 3 to `team+0x20` for a win and
//! 1 for a draw), wins/draws/losses at `+0x16/+0x18/+0x1a`, standings sorted by points only (`1050:1a61`, stable sort
//! assumed), morale `K = 0.85*K + 0.15*result` after every game (`1068:10d1`), evolution after every round with
//! `level = base - 2 * rank`, and the season end (`1020:038e`): champion `K += 1.0`, cup winner `K += 0.5`, the top two
//! and bottom two of adjacent divisions swap (`1020:02a6`) and two random outside teams join the lowest division.
//! **Assumed (not yet decoded):** the auto line-up rule (the real `1010:4b73`; we pick the strongest of the game's twelve
//! formations), tie order, and that ranks are taken after the round's results.
use crate::evolution::{evolve_squad, level_for_rank};
use crate::finance::{self, Category, Finance};
use crate::matchsim::{
    fixture_value, morale_results, update_morale, Decision, Event, Match, Primary, Side, SubEvent,
};
use crate::rng::DelphiRng;
use crate::sector::{Slot, Status};
use std::collections::HashMap;

/// `DS:0x1838..`: `SCHEDULE[team][round-1]`, `|v| mod 8` = opponent index, negative = away.
pub const SCHEDULE: [[i8; 14]; 8] = [
    [2, -7, 5, -3, -1, -4, 6, -2, 7, -5, 3, 1, 4, -6],
    [5, -4, 3, -2, 8, -6, 7, -5, 4, -3, 2, -8, 6, -7],
    [-8, 6, -4, 1, 7, 3, -5, 8, -6, 4, -1, -7, -3, 5],
    [7, -5, -1, 8, -6, -2, 4, -7, 5, 1, -8, 6, 2, -4],
    [-6, 1, 2, -7, 5, 8, -3, 6, -1, -2, 7, -5, -8, 3],
    [-1, 3, -8, 6, -4, -7, 2, 1, -3, 8, -6, 4, 7, -2],
    [4, -2, 7, -5, 3, 1, -8, -4, 2, -7, 5, -3, -1, 8],
    [-3, 8, -6, 4, -2, 5, -1, 3, -8, 6, -4, 2, -5, 1],
];

/// Home/away pairs of a league round (1..=14), as `(home, away)` club indices.
pub fn fixtures(round: usize) -> Vec<(usize, usize)> {
    (0..8)
        .filter_map(|t| {
            let v = SCHEDULE[t][round - 1];
            (v > 0).then_some((t, v.unsigned_abs() as usize % 8))
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Club {
    pub name: String,
    /// The `.EFT` short name as the file has it (`name` is trimmed and made unique); the referee's values read it.
    pub short_name: String,
    pub slots: Vec<Slot>,
    /// Player names, parallel to `slots`.
    pub names: Vec<String>,
    pub k: f64,
    pub points: u16,
    pub wins: u16,
    pub draws: u16,
    pub losses: u16,
    pub goals_for: i32,
    pub goals_against: i32,
    /// Games played by the current coach (`coach+0x0c`, saturating at `i16::MAX`); reset when the coach is sacked.
    pub coach_games: u16,
    /// The coach's score (`coach+0x0e`): +2 for a home win, +3 for an away win, +1 / +2 for a home / away draw. It
    /// travels with the coach when two clubs swap coaches and ranks the candidates for a vacancy (lowest first).
    pub coach_score: i64,
    /// The club's colours (background, foreground) from its `.EFT` team; the original paints the user's windows with them.
    pub colors: ([u8; 3], [u8; 3]),
    /// The `.EFT` team's long name (`FIGUEIRENSE F.C.`) and its coach's name, shown in the windows.
    pub full_name: String,
    pub coach_name: String,
    pub fin: Finance,
    /// The club's country code (`team+4` object `+0x4a`); compared with the players' nationalities.
    pub country: [u8; 3],
    /// Position in the master list of all clubs (the order the transfer auction visits buyers).
    pub master: u16,
    /// `+0xc7` / `+0xc9`: signings (all / foreign) counted against the squad and foreigner limits.
    pub bought: u16,
    pub bought_foreign: u16,
}

/// Coach sacking (`1010:8b5d`): an AI coach with at least this many games ...
pub const SACK_MIN_GAMES: u16 = 10;
/// ... whose team's morale `K` is at most this (checked before the game's own morale update) is replaced.
pub const SACK_MORALE: f64 = 0.65;

impl Club {
    /// `1010:8b5d` + `4f40` for a division played on its own ([`play_round`]): replaces the coach when the results were bad
    /// for long enough; the new coach starts with `K = 1.0` and zero games. The game itself swaps the coach with another
    /// club's (`5343`, `World::replace_coach`), which needs the other divisions.
    pub fn maybe_sack_coach(&mut self) -> bool {
        if self.sack_due() {
            self.k = 1.0;
            self.coach_games = 0;
            self.coach_score = 0;
            true
        } else {
            false
        }
    }

    /// `1010:8b5d` for a league game: the coach has coached enough games and the morale has sunk.
    pub fn sack_due(&self) -> bool {
        self.coach_games >= SACK_MIN_GAMES && self.k <= SACK_MORALE
    }

    pub fn new(name: &str, slots: Vec<Slot>) -> Self {
        Self {
            name: name.to_string(),
            short_name: name.to_string(),
            names: (0..slots.len())
                .map(|i| format!("Player {}", i + 1))
                .collect(),
            slots,
            k: 1.0,
            points: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            goals_for: 0,
            goals_against: 0,
            coach_games: 0,
            coach_score: 0,
            colors: ([0, 0, 0], [255, 255, 255]),
            full_name: name.to_string(),
            coach_name: String::new(),
            fin: Finance::default(),
            country: *b"???",
            master: 0,
            bought: 0,
            bought_foreign: 0,
        }
    }
}

/// A substitution planned by the user: at `minute`, `inn` (a bench player) replaces `out` (a starter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlannedSub {
    pub minute: u16,
    pub out: usize,
    pub inn: usize,
}

/// The user's line-up for a club: indices into the club's slots.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub starters: Vec<usize>,
    pub bench: Vec<usize>,
    pub subs: Vec<PlannedSub>,
}

/// Best line-up for one formation: the strongest available players per position, a bench of one goalkeeper and the four
/// best remaining outfield players. `None` when the squad cannot field the formation.
pub fn plan_for_formation(slots: &[Slot], formation: (u8, u8, u8)) -> Option<Plan> {
    let ranked = |pos: u8| -> Vec<usize> {
        let mut idx: Vec<usize> = (0..slots.len())
            .filter(|&i| slots[i].position == pos && slots[i].available())
            .collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
        idx
    };
    let by_pos: Vec<Vec<usize>> = (0..4).map(ranked).collect();
    let (d, m, a) = (
        formation.0 as usize,
        formation.1 as usize,
        formation.2 as usize,
    );
    if by_pos[0].is_empty() || by_pos[1].len() < d || by_pos[2].len() < m || by_pos[3].len() < a {
        return None;
    }
    let mut starters = vec![by_pos[0][0]];
    starters.extend(by_pos[1].iter().take(d));
    starters.extend(by_pos[2].iter().take(m));
    starters.extend(by_pos[3].iter().take(a));
    let mut bench = Vec::new();
    if let Some(&g) = by_pos[0].get(1) {
        bench.push(g);
    }
    let mut rest: Vec<usize> = (0..slots.len())
        .filter(|&i| slots[i].position != 0 && slots[i].available() && !starters.contains(&i))
        .collect();
    rest.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
    bench.extend(rest.into_iter().take(4));
    Some(Plan {
        starters,
        bench,
        subs: Vec::new(),
    })
}

/// Applies a user plan to the squad statuses, repairing it like the game must when players are unavailable: invalid,
/// duplicate or unavailable starters are replaced by the best available player of the same position; a missing goalkeeper
/// is fixed; the bench is topped up to five from the best remaining players.
pub fn apply_plan(slots: &mut [Slot], plan: &Plan) {
    for s in slots.iter_mut() {
        s.status = Status::Out;
    }
    let ok = |slots: &[Slot], i: usize| {
        i < slots.len() && slots[i].available() && slots[i].status == Status::Out
    };
    let mut wanted = Vec::new();
    for &i in &plan.starters {
        if ok(slots, i) && !wanted.contains(&i) {
            slots[i].status = Status::Starter;
            wanted.push(i);
        }
    }
    // replace the starters that were dropped, position by position
    let positions: Vec<u8> = plan
        .starters
        .iter()
        .filter_map(|&i| slots.get(i).map(|s| s.position))
        .collect();
    for pos in positions {
        let have = slots
            .iter()
            .filter(|s| s.status == Status::Starter && s.position == pos)
            .count();
        let want = plan
            .starters
            .iter()
            .filter(|&&i| slots.get(i).is_some_and(|s| s.position == pos))
            .count();
        if have < want {
            let best = (0..slots.len())
                .filter(|&i| {
                    slots[i].position == pos
                        && slots[i].available()
                        && slots[i].status == Status::Out
                })
                .max_by_key(|&i| slots[i].strength);
            if let Some(i) = best {
                slots[i].status = Status::Starter;
            }
        }
    }
    // a plan entry that pointed nowhere cannot be repaired by position: top up with the best available outfield players
    while slots.iter().filter(|s| s.starter()).count() < 11 {
        let best = (0..slots.len())
            .filter(|&i| {
                slots[i].position != 0 && slots[i].available() && slots[i].status == Status::Out
            })
            .max_by_key(|&i| slots[i].strength);
        match best {
            Some(i) => slots[i].status = Status::Starter,
            None => break,
        }
    }
    if !slots.iter().any(|s| s.starter() && s.position == 0) {
        if let Some(g) = (0..slots.len())
            .filter(|&i| slots[i].position == 0 && slots[i].available())
            .max_by_key(|&i| slots[i].strength)
        {
            slots[g].status = Status::Starter;
        }
    }
    for &i in &plan.bench {
        if ok(slots, i) && slots.iter().filter(|s| s.status == Status::Sub).count() < 5 {
            slots[i].status = Status::Sub;
        }
    }
    let mut rest: Vec<usize> = (0..slots.len())
        .filter(|&i| slots[i].available() && slots[i].status == Status::Out)
        .collect();
    rest.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
    for i in rest {
        if slots.iter().filter(|s| s.status == Status::Sub).count() >= 5 {
            break;
        }
        slots[i].status = Status::Sub;
    }
}

/// What happened in one game of a round (club indices refer to the slice passed to `play_round`).
#[derive(Clone, Debug)]
pub struct MatchRecord {
    pub home: usize,
    pub away: usize,
    pub home_goals: i32,
    pub away_goals: i32,
    pub events: Vec<Event>,
    pub subs: Vec<SubEvent>,
    /// Attendance and gate receipts of the home club (0 when finances are not simulated).
    pub attendance: i64,
    pub gate: i64,
    /// The players' names of both squads as they were in the game (events index into them).
    pub names: [Vec<String>; 2],
    /// Both squads as they were at the kick-off (status = who started and who was on the bench, strengths as played).
    pub start: [Vec<Slot>; 2],
    /// Every primary chance of the game (goals and misses).
    pub primaries: Vec<Primary>,
    /// The fixture bonus values `(home, away)` of the game (`match+0x4c` / `+0x4e`), shown by the referee aid.
    pub bonus: (i32, i32),
}

/// What a round needs to book the money: the division's ticket price, the economy index and the currency scale.
#[derive(Clone, Copy, Debug)]
pub struct MoneyCtx {
    pub price: f64,
    pub e: f64,
    pub g: i64,
}

/// The twelve formations of the game's *Seleccionar* menu (verified from the main form's menu items), as
/// `(defenders, midfielders, attackers)`; 4-4-2 comes first so that it wins ties.
pub const FORMATIONS: [(u8, u8, u8); 12] = [
    (4, 4, 2),
    (3, 3, 4),
    (3, 4, 3),
    (4, 2, 4),
    (4, 3, 3),
    (4, 5, 1),
    (5, 2, 3),
    (5, 3, 2),
    (5, 4, 1),
    (5, 5, 0),
    (6, 3, 1),
    (6, 4, 0),
];

/// Whether some formation can be fielded with these available counts `[gk, def, mid, att]`.
pub fn can_field(count: [usize; 4]) -> bool {
    count[0] >= 1
        && FORMATIONS.iter().any(|&(d, m, a)| {
            count[1] >= d as usize && count[2] >= m as usize && count[3] >= a as usize
        })
}

/// Index into `FORMATIONS` of the strongest fieldable formation (ties to the earlier entry), as `auto_lineup` chooses.
pub fn best_formation(slots: &[Slot]) -> Option<usize> {
    let mut best: Option<(i32, usize)> = None;
    for (k, &f) in FORMATIONS.iter().enumerate() {
        if let Some(plan) = plan_for_formation(slots, f) {
            let total: i32 = plan
                .starters
                .iter()
                .map(|&i| slots[i].strength as i32)
                .sum();
            if best.is_none_or(|(t, _)| total > t) {
                best = Some((total, k));
            }
        }
    }
    best.map(|(_, k)| k)
}

/// `1010:4388`: the computer's line-up (see `lineup.rs`); the squad list is shuffled and left in the final order.
pub fn auto_lineup(rng: &mut DelphiRng, slots: &mut Vec<Slot>, names: &mut Vec<String>) {
    crate::lineup::auto_lineup(rng, slots, names);
}

/// Two disjoint mutable references into one slice.
fn two_mut<T>(v: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert_ne!(i, j);
    if i < j {
        let (a, b) = v.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = v.split_at_mut(i);
        (&mut b[0], &mut a[j])
    }
}

/// The outcome of one played game (before league or cup bookkeeping).
#[derive(Clone, Debug)]
pub struct Game {
    pub home_goals: i32,
    pub away_goals: i32,
    pub events: Vec<Event>,
    pub subs: Vec<SubEvent>,
    /// The game went to extra time (minutes 91..=120).
    pub extra_time: bool,
    /// Who is on the pitch at the end (`Status::Starter`) for each side, in squad order (used by the penalty shootout).
    pub pitch: [Vec<Status>; 2],
    /// Both squads as they were at the kick-off.
    pub start: [Vec<Slot>; 2],
    pub primaries: Vec<Primary>,
}

/// Plays one game between two clubs whose line-ups are already set (`apply_plan` / `auto_lineup`): `hb` / `ab` are the
/// fixture bonuses, plans carry the user's planned substitutions. With `extra_time_if_tied` a drawn game continues to
/// minute 120 (`1048:069b` extends cup games). What persists is copied back to the clubs: ratings, strength loss, bans and
/// injuries. Points, goals and morale are left to the caller.
#[allow(clippy::too_many_arguments)]
pub fn play_game(
    rng: &mut DelphiRng,
    home: &mut Club,
    away: &mut Club,
    hb: i32,
    ab: i32,
    home_plan: Option<&Plan>,
    away_plan: Option<&Plan>,
    extra_time_if_tied: bool,
) -> Game {
    play_game_with(
        rng,
        home,
        away,
        hb,
        ab,
        home_plan,
        away_plan,
        extra_time_if_tied,
        &[],
    )
}

/// [`play_game`] with the user's `decisions` (shooters of primary chances, substitutions after injuries). Without decisions the
/// game is exactly the one `play_game` plays.
#[allow(clippy::too_many_arguments)]
pub fn play_game_with(
    rng: &mut DelphiRng,
    home: &mut Club,
    away: &mut Club,
    hb: i32,
    ab: i32,
    home_plan: Option<&Plan>,
    away_plan: Option<&Plan>,
    extra_time_if_tied: bool,
    decisions: &[Decision],
) -> Game {
    let mut m = Match::new(home.slots.clone(), away.slots.clone(), hb, ab);
    m.decisions = decisions.to_vec();
    let start = [m.home.slots.clone(), m.away.slots.clone()];
    m.home.k = home.k;
    m.away.k = away.k;
    for (plan, side) in [(home_plan, Side::Home), (away_plan, Side::Away)] {
        if let Some(plan) = plan {
            m.planned.extend(
                plan.subs
                    .iter()
                    .take(3)
                    .map(|s| (side, s.minute, s.out, s.inn)),
            );
        }
    }
    m.play(rng);
    let mut extra_time = false;
    if extra_time_if_tied && m.score().0 == m.score().1 {
        extra_time = true;
        m.play_extra_time(rng);
    }
    // carry over what persists between matches: ratings, strength loss, bans, injuries
    for (club, played) in [(&mut *home, &m.home), (&mut *away, &m.away)] {
        for (slot, p) in club.slots.iter_mut().zip(played.slots.iter()) {
            if slot.starter() || p.starter() {
                slot.history[0] += 1;
            }
            slot.history[1] += (p.goals as u32).saturating_sub(slot.goals as u32);
            slot.history[2] += (p.ban > 0 && slot.ban == 0) as u32;
            slot.history[3] += (p.injured > 0 && slot.injured == 0) as u32;
            slot.rating = p.rating;
            slot.goals = p.goals;
            slot.strength = p.strength;
            slot.ban = p.ban;
            slot.injured = p.injured;
        }
    }
    let (x, y) = m.score();
    Game {
        home_goals: x,
        away_goals: y,
        events: m.events,
        subs: m.subs,
        extra_time,
        pitch: [
            m.home.slots.iter().map(|p| p.status).collect(),
            m.away.slots.iter().map(|p| p.status).collect(),
        ],
        start,
        primaries: m.primaries,
    }
}

/// Coach scores after a league game (`1010:9396`): `(home, away)`.
pub fn coach_points(x: i32, y: i32) -> (i64, i64) {
    match x.cmp(&y) {
        std::cmp::Ordering::Greater => (2, 0),
        std::cmp::Ordering::Less => (0, 3),
        std::cmp::Ordering::Equal => (1, 2),
    }
}

/// The bonus values of a game played without a referee list (a world made without the game's files): `c95c` with a stand-in
/// string for the referee's name and every club domestic.
pub fn standin_bonus(stand_in: &[u8], home: &[u8], away: &[u8]) -> (i32, i32) {
    (
        fixture_value(stand_in, home, true) + 2,
        fixture_value(stand_in, away, true),
    )
}

/// Plays one league round with the local fallback for sackings (the coach is replaced by a fresh one); see
/// [`play_round_with`] for the original's coach swap between clubs.
pub fn play_round(
    rng: &mut DelphiRng,
    clubs: &mut [Club],
    round: usize,
    bonus: &mut dyn FnMut(usize, usize) -> (i32, i32),
    plans: &HashMap<usize, Plan>,
    money: Option<MoneyCtx>,
) -> Vec<MatchRecord> {
    play_round_with(rng, clubs, round, bonus, plans, money, &mut |clubs, i| {
        clubs[i].maybe_sack_coach();
    })
}

/// Plays one league round; updates ratings, injuries, bans, goals, points (3-1-0), W/D/L and morale. After each game
/// the sack check of `1010:8b5d` runs for the loser (both clubs after a draw), before the morale update, and
/// `on_sack(clubs, i)` is called for every club whose coach is due to go (`1010:4f40`).
#[allow(clippy::too_many_arguments)]
pub fn play_round_with(
    rng: &mut DelphiRng,
    clubs: &mut [Club],
    round: usize,
    bonus: &mut dyn FnMut(usize, usize) -> (i32, i32),
    plans: &HashMap<usize, Plan>,
    money: Option<MoneyCtx>,
    on_sack: &mut dyn FnMut(&mut [Club], usize),
) -> Vec<MatchRecord> {
    play_round_ctl(
        rng,
        clubs,
        round,
        bonus,
        plans,
        money,
        on_sack,
        &mut Control::default(),
    )
}

/// What the user decides while a round is played, and what a replay of the round keeps as it was. The whole round is computed
/// in one go (`World::play_round_ctl`); to let the user act at a minute of his game, the front end plays the round once, shows it,
/// and when the user decides something replays the round from a copy of the world with the decision added. Everything the
/// decision cannot reach stays identical: the random state is recorded at fixed points of the round (`checkpoint`) and restored
/// at the same points of the replay, so the other games of the round are not disturbed by the user's game consuming
/// a different number of random numbers.
#[derive(Clone, Debug, Default)]
pub struct Control {
    /// Decisions in the humans' league games, each with the home club of the game it belongs to.
    pub league: Vec<(String, Decision)>,
    /// Decisions in the humans' cup ties of the same round, tagged the same way.
    pub cup: Vec<(String, Decision)>,
    /// Random state at every checkpoint of the round, recorded by a run with `frozen == None`.
    pub log: Vec<DelphiRng>,
    /// When set, every checkpoint restores the recorded state of the same index.
    pub frozen: Option<Vec<DelphiRng>>,
    cursor: usize,
}

impl Control {
    /// A replay of the round that recorded `log`, with the user's decisions of the league game and the cup tie.
    pub fn replay(
        log: Vec<DelphiRng>,
        league: Vec<(String, Decision)>,
        cup: Vec<(String, Decision)>,
    ) -> Control {
        Control {
            league,
            cup,
            log: Vec::new(),
            frozen: Some(log),
            cursor: 0,
        }
    }

    /// The decisions of the game (league or `cup`) whose home club is `home`.
    pub fn decisions(&self, cup: bool, home: &str) -> Vec<Decision> {
        let all = if cup { &self.cup } else { &self.league };
        all.iter()
            .filter(|(h, _)| h == home)
            .map(|&(_, d)| d)
            .collect()
    }

    /// Records the random state (a first run) or restores the recorded one (a replay). Called at the start of the league
    /// round, before every game of it, at the start of the cup draw and before every tie.
    pub fn checkpoint(&mut self, rng: &mut DelphiRng) {
        match &self.frozen {
            Some(f) => {
                if let Some(s) = f.get(self.cursor) {
                    *rng = s.clone();
                }
            }
            None => self.log.push(rng.clone()),
        }
        self.cursor += 1;
    }
}

/// [`play_round_with`] under the user's [`Control`].
#[allow(clippy::too_many_arguments)]
pub fn play_round_ctl(
    rng: &mut DelphiRng,
    clubs: &mut [Club],
    round: usize,
    bonus: &mut dyn FnMut(usize, usize) -> (i32, i32),
    plans: &HashMap<usize, Plan>,
    money: Option<MoneyCtx>,
    on_sack: &mut dyn FnMut(&mut [Club], usize),
    ctl: &mut Control,
) -> Vec<MatchRecord> {
    let mut records = Vec::new();
    ctl.checkpoint(rng);
    for (i, c) in clubs.iter_mut().enumerate() {
        match plans.get(&i) {
            Some(plan) => apply_plan(&mut c.slots, plan),
            None => auto_lineup(rng, &mut c.slots, &mut c.names),
        }
    }
    for (h, a) in fixtures(round) {
        ctl.checkpoint(rng);
        let (hb, ab) = bonus(h, a);
        let (hc, ac) = two_mut(clubs, h, a);
        let (attendance, gate) = match money {
            Some(m) => {
                let att = finance::attendance(
                    rng,
                    finance::rating(&hc.slots),
                    finance::rating(&ac.slots),
                    hc.k,
                    ac.k,
                    false,
                    hc.fin.capacity(),
                );
                (att, (att as f64 * m.price).round_ties_even() as i64)
            }
            None => (0, 0),
        };
        let decisions = if plans.contains_key(&h) || plans.contains_key(&a) {
            ctl.decisions(false, &hc.name)
        } else {
            Vec::new()
        };
        let g = play_game_with(
            rng,
            hc,
            ac,
            hb,
            ab,
            plans.get(&h),
            plans.get(&a),
            false,
            &decisions,
        );
        let (x, y) = (g.home_goals, g.away_goals);
        hc.goals_for += x;
        hc.goals_against += y;
        ac.goals_for += y;
        ac.goals_against += x;
        match x.cmp(&y) {
            std::cmp::Ordering::Greater => {
                hc.points += 3;
                hc.wins += 1;
                ac.losses += 1;
            }
            std::cmp::Ordering::Less => {
                ac.points += 3;
                ac.wins += 1;
                hc.losses += 1;
            }
            std::cmp::Ordering::Equal => {
                hc.points += 1;
                ac.points += 1;
                hc.draws += 1;
                ac.draws += 1;
            }
        }
        if money.is_some() {
            hc.fin.book(gate, Category::Gate);
            for c in [&mut *hc, &mut *ac] {
                let wages = finance::wage_bill(&c.slots);
                c.fin.book(-wages, Category::Wages);
                let interest = finance::loan_interest(c.fin.loan);
                c.fin.book(-interest, Category::Interest);
            }
            finance::update_fill(&mut hc.fin, attendance);
        }
        let (ph, pa) = coach_points(x, y);
        hc.coach_score += ph;
        ac.coach_score += pa;
        let checks: &[usize] = match x.cmp(&y) {
            std::cmp::Ordering::Greater => &[a],
            std::cmp::Ordering::Less => &[h],
            std::cmp::Ordering::Equal => &[h, a],
        };
        for &i in checks {
            if clubs[i].sack_due() {
                on_sack(clubs, i);
            }
        }
        let (rh, ra) = morale_results(x, y);
        let (hc, ac) = two_mut(clubs, h, a);
        hc.k = update_morale(hc.k, rh);
        ac.k = update_morale(ac.k, ra);
        hc.coach_games = hc.coach_games.saturating_add(1).min(i16::MAX as u16);
        ac.coach_games = ac.coach_games.saturating_add(1).min(i16::MAX as u16);
        records.push(MatchRecord {
            home: h,
            away: a,
            home_goals: x,
            away_goals: y,
            events: g.events,
            subs: g.subs,
            attendance,
            gate,
            names: [clubs[h].names.clone(), clubs[a].names.clone()],
            start: g.start,
            primaries: g.primaries,
            bonus: (hb, ab),
        });
    }
    records
}

/// Club indices ordered by points (stable: ties keep the club order), best first.
pub fn standings(clubs: &[Club]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..clubs.len()).collect();
    idx.sort_by_key(|&i| std::cmp::Reverse(clubs[i].points));
    idx
}

/// End of round: injuries and bans tick down, then every squad evolves with `level = base - 2 * rank`.
pub fn end_of_round(rng: &mut DelphiRng, clubs: &mut [Club], base: i32) {
    for c in clubs.iter_mut() {
        for s in c.slots.iter_mut() {
            s.ban = s.ban.saturating_sub(1);
            s.injured = s.injured.saturating_sub(1);
        }
    }
    for (rank, i) in standings(clubs).into_iter().enumerate() {
        evolve_squad(rng, &mut clubs[i].slots, level_for_rank(base, rank));
        // `1010:3e9c` clears `slot+0xc..0x13` of every player of a team whose coach has coached more than one game.
        // The sample save shows the accumulators zeroed after round 2 for all but two teams, while the evolution of that
        // round still used them, so the clear is applied after the evolution.
        if clubs[i].coach_games > 1 {
            for s in clubs[i].slots.iter_mut() {
                s.rating = 0;
            }
        }
    }
}

/// `1020:02a6`: the team at `ia` of `a` and the team at `ib` of `b` swap lists (each is appended to the other list).
fn swap_between(a: &mut Vec<Club>, ia: usize, b: &mut Vec<Club>, ib: usize) {
    let x = a.remove(ia);
    let y = b.remove(ib);
    b.push(x);
    a.push(y);
}

/// `1020:038e`: season end. Lists are sorted by points first (`1020:38b8`). The first division's champion gets
/// `K += 1.0` and the cup winner `K += 0.5`; then the **top two of each lower division swap with the bottom two of the
/// division above** (calls `02a6` with indices 6, 4, 4 because the lists shift by two after the first pair), and two
/// distinct random outside teams replace the lowest division's bottom two. Finally the table is cleared for a new season.
/// Divisions must hold 8 clubs each and the pool at least 3.
pub fn end_of_season(
    rng: &mut DelphiRng,
    divs: &mut [Vec<Club>; 4],
    pool: &mut Vec<Club>,
    cup_winner: Option<&str>,
) {
    for d in divs.iter_mut() {
        let order = standings(d);
        let sorted: Vec<Club> = order.into_iter().map(|i| d[i].clone()).collect();
        *d = sorted;
    }
    divs[0][0].k += 1.0;
    if let Some(name) = cup_winner {
        if let Some(c) = divs
            .iter_mut()
            .flatten()
            .chain(pool.iter_mut())
            .find(|c| c.name == name)
        {
            c.k += 0.5;
        }
    }
    for upper in 0..3 {
        let (head, tail) = divs.split_at_mut(upper + 1);
        let (a, b) = (&mut tail[0], &mut head[upper]);
        let idx_in_upper = if upper == 0 { 6 } else { 4 };
        swap_between(a, 0, b, idx_in_upper);
        swap_between(a, 0, b, idx_in_upper);
    }
    let first = rng.random(pool.len() as u16) as usize;
    let second = loop {
        let j = rng.random(pool.len() as u16) as usize;
        if j != first {
            break j;
        }
    };
    swap_between(pool, first, &mut divs[3], 4);
    swap_between(pool, second - (first < second) as usize, &mut divs[3], 4);
    for c in divs.iter_mut().flatten().chain(pool.iter_mut()) {
        c.points = 0;
        c.wins = 0;
        c.draws = 0;
        c.losses = 0;
        c.goals_for = 0;
        c.goals_against = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::needless_range_loop)] // the loops index several tables with the same (team, round)
    fn schedule_is_a_valid_double_round_robin() {
        let opp = |v: i8| v.unsigned_abs() as usize % 8;
        let mut met = [[0u8; 8]; 8];
        let mut home_games = [0u8; 8];
        for t in 0..8 {
            for r in 0..14 {
                let (o, v) = (opp(SCHEDULE[t][r]), SCHEDULE[t][r]);
                assert_ne!(o, t, "team {t} plays itself in round {}", r + 1);
                // the opponent's row mirrors it with the opposite ground
                assert_eq!(opp(SCHEDULE[o][r]), t);
                assert_ne!(SCHEDULE[o][r] > 0, v > 0);
                met[t][o] += 1;
                home_games[t] += (v > 0) as u8;
            }
        }
        for t in 0..8 {
            assert_eq!(home_games[t], 7);
            for o in (0..8).filter(|&o| o != t) {
                assert_eq!(met[t][o], 2, "{t} vs {o}");
            }
        }
    }

    #[test]
    fn every_round_has_four_games_and_every_team_plays_once() {
        for round in 1..=14 {
            let f = fixtures(round);
            assert_eq!(f.len(), 4);
            let mut seen = [false; 8];
            for (h, a) in f {
                assert!(!seen[h] && !seen[a]);
                seen[h] = true;
                seen[a] = true;
            }
            assert!(seen.iter().all(|&s| s));
        }
        // each ordered (home, away) pair occurs exactly once in the season
        let mut pairs = std::collections::HashSet::new();
        for round in 1..=14 {
            for p in fixtures(round) {
                assert!(pairs.insert(p));
            }
        }
        assert_eq!(pairs.len(), 56);
    }

    #[test]
    fn auto_lineup_names_eleven_and_a_bench_and_skips_the_injured() {
        let mut rng = DelphiRng::new(1);
        let mut slots = crate::matchsim::tests::squad(6, &mut rng);
        let mut names: Vec<String> = (0..slots.len()).map(|i| format!("P{i}")).collect();
        slots[3].injured = 2; // an injured defender is never picked
        let injured_name = names[3].clone();
        auto_lineup(&mut rng, &mut slots, &mut names);
        assert_eq!(slots.iter().filter(|s| s.starter()).count(), 11);
        assert!(slots.iter().filter(|s| s.status == Status::Sub).count() >= 4);
        let k = names.iter().position(|n| *n == injured_name).unwrap();
        assert_eq!(slots[k].status, Status::Out);
    }

    fn named_division(prefix: &str, rng: &mut DelphiRng) -> Vec<Club> {
        (0..8)
            .map(|i| {
                let mut c = Club::new(
                    &format!("{prefix}{i}"),
                    crate::matchsim::tests::squad(5, rng),
                );
                c.points = 100 - i as u16 * 10; // sorted best first by construction
                c
            })
            .collect()
    }

    #[test]
    fn promotion_and_relegation_move_two_clubs_each_way() {
        let mut rng = DelphiRng::new(3);
        let mut divs = [
            named_division("A", &mut rng),
            named_division("B", &mut rng),
            named_division("C", &mut rng),
            named_division("D", &mut rng),
        ];
        let mut pool: Vec<Club> = (0..6)
            .map(|i| Club::new(&format!("P{i}"), crate::matchsim::tests::squad(4, &mut rng)))
            .collect();
        let names = |d: &Vec<Club>| {
            let mut v: Vec<String> = d.iter().map(|c| c.name.clone()).collect();
            v.sort();
            v
        };
        end_of_season(&mut rng, &mut divs, &mut pool, Some("B3"));
        // division sizes are preserved
        assert!(divs.iter().all(|d| d.len() == 8));
        assert_eq!(pool.len(), 6);
        // D1 = old top six + the two best of D2; D2 = old ranks 3..6 + D1's bottom two + the two best of D3
        assert_eq!(
            names(&divs[0]),
            ["A0", "A1", "A2", "A3", "A4", "A5", "B0", "B1"]
        );
        assert_eq!(
            names(&divs[1]),
            ["A6", "A7", "B2", "B3", "B4", "B5", "C0", "C1"]
        );
        assert_eq!(
            names(&divs[2]),
            ["B6", "B7", "C2", "C3", "C4", "C5", "D0", "D1"]
        );
        // D4 lost its bottom two (to the pool) and gained two random outside clubs
        let d4 = names(&divs[3]);
        assert!(d4.contains(&"C6".to_string()) && d4.contains(&"C7".to_string()));
        assert_eq!(d4.iter().filter(|n| n.starts_with('P')).count(), 2);
        assert!(
            names(&pool).contains(&"D6".to_string()) && names(&pool).contains(&"D7".to_string())
        );
        // the table is cleared
        assert!(divs.iter().flatten().all(|c| c.points == 0));
    }

    #[test]
    fn titles_raise_morale() {
        let mut rng = DelphiRng::new(4);
        let mut divs = [
            named_division("A", &mut rng),
            named_division("B", &mut rng),
            named_division("C", &mut rng),
            named_division("D", &mut rng),
        ];
        let mut pool: Vec<Club> = (0..4)
            .map(|i| Club::new(&format!("P{i}"), crate::matchsim::tests::squad(4, &mut rng)))
            .collect();
        end_of_season(&mut rng, &mut divs, &mut pool, Some("A0")); // champion also wins the cup
        let champion = divs.iter().flatten().find(|c| c.name == "A0").unwrap();
        assert_eq!(champion.k, 2.5);
        let other = divs.iter().flatten().find(|c| c.name == "A1").unwrap();
        assert_eq!(other.k, 1.0);
    }

    #[test]
    fn a_season_round_updates_points_wdl_and_morale() {
        let mut rng = DelphiRng::new(5);
        let mut clubs: Vec<Club> = (0..8)
            .map(|i| Club::new(&format!("C{i}"), crate::matchsim::tests::squad(6, &mut rng)))
            .collect();
        let mut seed = DelphiRng::new(6);
        let mut bonus = move |_h: usize, _a: usize| {
            let mut n = || {
                (0..8)
                    .map(|_| b'a' + seed.random(26) as u8)
                    .collect::<Vec<u8>>()
            };
            standin_bonus(&n(), &n(), &n())
        };
        play_round(&mut rng, &mut clubs, 1, &mut bonus, &HashMap::new(), None);
        for c in &clubs {
            assert_eq!(c.wins + c.draws + c.losses, 1);
            assert_eq!(c.points, c.wins * 3 + c.draws);
        }
        assert_eq!(
            clubs.iter().map(|c| c.wins).sum::<u16>(),
            clubs.iter().map(|c| c.losses).sum::<u16>()
        );
        assert!(clubs.iter().any(|c| (c.k - 1.0).abs() > 1e-9));
    }

    #[test]
    fn a_plan_for_each_formation_has_the_right_shape() {
        let mut rng = DelphiRng::new(11);
        let slots = crate::matchsim::tests::squad(6, &mut rng);
        for &f in FORMATIONS.iter() {
            let Some(plan) = plan_for_formation(&slots, f) else {
                continue;
            };
            assert_eq!(plan.starters.len(), 11);
            let count = |pos: u8| {
                plan.starters
                    .iter()
                    .filter(|&&i| slots[i].position == pos)
                    .count()
            };
            assert_eq!(
                (count(0), count(1), count(2), count(3)),
                (1, f.0 as usize, f.1 as usize, f.2 as usize)
            );
            assert!(plan.bench.len() <= 5 && plan.bench.iter().all(|i| !plan.starters.contains(i)));
        }
        // 4-4-2 is feasible, 6-4-0 is not (only five defenders in this squad)
        assert!(plan_for_formation(&slots, (4, 4, 2)).is_some());
        assert!(plan_for_formation(&slots, (6, 4, 0)).is_none());
    }

    #[test]
    fn applying_a_plan_repairs_unavailable_starters() {
        let mut rng = DelphiRng::new(12);
        let mut slots = crate::matchsim::tests::squad(6, &mut rng);
        let plan = plan_for_formation(&slots, (4, 4, 2)).unwrap();
        // a planned starter is injured, another index is bogus and one is listed twice
        slots[plan.starters[3]].injured = 2;
        let mut broken = plan.clone();
        broken.starters[5] = 999;
        broken.starters[6] = broken.starters[7];
        apply_plan(&mut slots, &broken);
        assert_eq!(slots.iter().filter(|s| s.starter()).count(), 11);
        assert!(slots.iter().filter(|s| s.starter()).all(|s| s.available()));
        assert_eq!(
            slots
                .iter()
                .filter(|s| s.starter() && s.position == 0)
                .count(),
            1
        );
        assert!(slots.iter().filter(|s| s.status == Status::Sub).count() <= 5);
    }

    #[test]
    fn a_coach_is_sacked_after_ten_games_with_low_morale() {
        let mut c = Club::new("X", Vec::new());
        c.k = 0.5;
        c.coach_games = 9;
        assert!(!c.maybe_sack_coach(), "too few games");
        c.coach_games = 10;
        c.k = 0.66;
        assert!(!c.maybe_sack_coach(), "morale above the threshold");
        c.k = 0.65;
        assert!(c.maybe_sack_coach());
        assert_eq!((c.k, c.coach_games), (1.0, 0));
    }
}
