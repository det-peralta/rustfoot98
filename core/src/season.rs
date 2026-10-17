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
use crate::matchsim::{fixture_value, morale_results, update_morale, Event, Match, Side, SubEvent};
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
}

impl Club {
    pub fn new(name: &str, slots: Vec<Slot>) -> Self {
        Self {
            name: name.to_string(),
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

/// Stand-in for the auto line-up `1010:4b73` (its choice rule is not decoded): the formation whose starters have the
/// largest total strength (ties go to the earlier entry of `FORMATIONS`), best players per position, a bench of one
/// goalkeeper plus the four best remaining outfield players; unavailable players are out.
pub fn auto_lineup(slots: &mut [Slot]) {
    for s in slots.iter_mut() {
        s.status = Status::Out;
    }
    let ranked = |slots: &[Slot], pos: u8| -> Vec<usize> {
        let mut idx: Vec<usize> = (0..slots.len())
            .filter(|&i| slots[i].position == pos && slots[i].available())
            .collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
        idx
    };
    let by_pos: Vec<Vec<usize>> = (0..4).map(|p| ranked(slots, p)).collect();
    let top = |pos: usize, n: usize| -> i32 {
        by_pos[pos]
            .iter()
            .take(n)
            .map(|&i| slots[i].strength as i32)
            .sum()
    };
    let best = FORMATIONS
        .iter()
        .filter(|&&(d, m, a)| {
            by_pos[1].len() >= d as usize
                && by_pos[2].len() >= m as usize
                && by_pos[3].len() >= a as usize
        })
        .max_by_key(|&&(d, m, a)| {
            (
                top(1, d as usize) + top(2, m as usize) + top(3, a as usize),
                std::cmp::Reverse(FORMATIONS.iter().position(|&f| f == (d, m, a)).unwrap()),
            )
        })
        .copied();
    if let Some(&g) = by_pos[0].first() {
        slots[g].status = Status::Starter;
    }
    let Some((d, m, a)) = best else { return };
    for (pos, n) in [(1usize, d as usize), (2, m as usize), (3, a as usize)] {
        for &i in by_pos[pos].iter().take(n) {
            slots[i].status = Status::Starter;
        }
    }
    if let Some(&g) = by_pos[0].get(1) {
        slots[g].status = Status::Sub;
    }
    let mut rest: Vec<usize> = (0..slots.len())
        .filter(|&i| {
            slots[i].position != 0 && slots[i].status == Status::Out && slots[i].available()
        })
        .collect();
    rest.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
    for &i in rest.iter().take(4) {
        slots[i].status = Status::Sub;
    }
}

/// Plays one league round; updates ratings, injuries, bans, goals, points (3-1-0), W/D/L and morale.
pub fn play_round(
    rng: &mut DelphiRng,
    clubs: &mut [Club],
    round: usize,
    names: &mut dyn FnMut(usize, usize) -> (Vec<u8>, Vec<u8>, Vec<u8>),
    plans: &HashMap<usize, Plan>,
) -> Vec<MatchRecord> {
    let mut records = Vec::new();
    for (i, c) in clubs.iter_mut().enumerate() {
        match plans.get(&i) {
            Some(plan) => apply_plan(&mut c.slots, plan),
            None => auto_lineup(&mut c.slots),
        }
    }
    for (h, a) in fixtures(round) {
        let (division, hn, an) = names(h, a);
        let hb = fixture_value(&division, &hn, true) + 2;
        let ab = fixture_value(&division, &an, true);
        let mut m = Match::new(clubs[h].slots.clone(), clubs[a].slots.clone(), hb, ab);
        m.home.k = clubs[h].k;
        m.away.k = clubs[a].k;
        for (club, side) in [(h, Side::Home), (a, Side::Away)] {
            if let Some(plan) = plans.get(&club) {
                m.planned.extend(
                    plan.subs
                        .iter()
                        .take(3)
                        .map(|s| (side, s.minute, s.out, s.inn)),
                );
            }
        }
        m.play(rng);
        let (x, y) = m.score();
        // carry over what persists between matches: ratings, strength loss, bans, injuries
        for (club, played) in [(h, &m.home), (a, &m.away)] {
            for (slot, p) in clubs[club].slots.iter_mut().zip(played.slots.iter()) {
                slot.rating = p.rating;
                slot.strength = p.strength;
                slot.ban = p.ban;
                slot.injured = p.injured;
            }
        }
        clubs[h].goals_for += x;
        clubs[h].goals_against += y;
        clubs[a].goals_for += y;
        clubs[a].goals_against += x;
        match x.cmp(&y) {
            std::cmp::Ordering::Greater => {
                clubs[h].points += 3;
                clubs[h].wins += 1;
                clubs[a].losses += 1;
            }
            std::cmp::Ordering::Less => {
                clubs[a].points += 3;
                clubs[a].wins += 1;
                clubs[h].losses += 1;
            }
            std::cmp::Ordering::Equal => {
                clubs[h].points += 1;
                clubs[a].points += 1;
                clubs[h].draws += 1;
                clubs[a].draws += 1;
            }
        }
        let (rh, ra) = morale_results(x, y);
        clubs[h].k = update_morale(clubs[h].k, rh);
        clubs[a].k = update_morale(clubs[a].k, ra);
        records.push(MatchRecord {
            home: h,
            away: a,
            home_goals: x,
            away_goals: y,
            events: m.events,
            subs: m.subs,
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
    }
}

/// `1020:02a6`: the team at `ia` of `a` and the team at `ib` of `b` swap lists (each is appended to the other list).
fn swap_between(a: &mut Vec<Club>, ia: usize, b: &mut Vec<Club>, ib: usize) {
    let x = a.remove(ia);
    let y = b.remove(ib);
    b.push(x);
    a.push(y);
}

/// Where the cup winner sits: division `0..=3` or the outside pool (`4`), and its index in that list.
pub type ClubRef = (usize, usize);

/// `1020:038e`: season end. Lists are sorted by points first (`1020:38b8`). The first division's champion gets
/// `K += 1.0` and the cup winner `K += 0.5`; then the **top two of each lower division swap with the bottom two of the
/// division above** (calls `02a6` with indices 6, 4, 4 because the lists shift by two after the first pair), and two
/// distinct random outside teams replace the lowest division's bottom two. Finally the table is cleared for a new season.
/// Divisions must hold 8 clubs each and the pool at least 3.
pub fn end_of_season(
    rng: &mut DelphiRng,
    divs: &mut [Vec<Club>; 4],
    pool: &mut Vec<Club>,
    cup_winner: Option<ClubRef>,
) {
    for d in divs.iter_mut() {
        let order = standings(d);
        let sorted: Vec<Club> = order.into_iter().map(|i| d[i].clone()).collect();
        *d = sorted;
    }
    divs[0][0].k += 1.0;
    if let Some((d, i)) = cup_winner {
        let list = if d < 4 { &mut divs[d] } else { &mut *pool };
        list[i].k += 0.5;
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
    fn auto_lineup_picks_a_442_with_a_bench() {
        let mut rng = DelphiRng::new(1);
        let mut slots = crate::matchsim::tests::squad(6, &mut rng);
        slots[3].injured = 2; // an injured defender is never picked
        auto_lineup(&mut slots);
        assert_eq!(slots.iter().filter(|s| s.starter()).count(), 11);
        // one injured player less: a goalkeeper plus the remaining outfield reserves
        assert_eq!(slots.iter().filter(|s| s.status == Status::Sub).count(), 4);
        assert_eq!(slots[3].status, Status::Out);
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
        end_of_season(&mut rng, &mut divs, &mut pool, Some((1, 3)));
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
        end_of_season(&mut rng, &mut divs, &mut pool, Some((0, 0))); // champion also wins the cup
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
        let mut names = move |_h: usize, _a: usize| {
            let mut n = || {
                (0..8)
                    .map(|_| b'a' + seed.random(26) as u8)
                    .collect::<Vec<u8>>()
            };
            (n(), n(), n())
        };
        play_round(&mut rng, &mut clubs, 1, &mut names, &HashMap::new());
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
        // the automatic choice agrees with the best formation
        let best = best_formation(&slots).unwrap();
        let mut auto = slots.clone();
        auto_lineup(&mut auto);
        let plan = plan_for_formation(&slots, FORMATIONS[best]).unwrap();
        let mut by_plan = slots.clone();
        apply_plan(&mut by_plan, &plan);
        let starters = |v: &Vec<Slot>| {
            v.iter()
                .enumerate()
                .filter(|(_, s)| s.starter())
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        };
        assert_eq!(starters(&auto), starters(&by_plan));
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
}
