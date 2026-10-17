//! Season layer: the league fixture table, rounds, standings and per-round evolution.
//!
//! **Verified in the original:** the 8-team double round-robin table (`DS:0x1838`, read by `1010:1d20`),
//! the 14 league rounds (`1020:11b8` loops 0..=14), standings sorted by points only (`1050:1a61`, stable sort assumed),
//! and evolution after every round with `level = base - 2 * rank` (`0c41 -> 0d9c -> 6fb8 -> 4a41`).
//! **Assumed (not yet decoded):** 3-1-0 points, the 4-4-2 auto line-up (the real `1010:4b73` picks the formation),
//! tie order, and that ranks are taken after the round's results.
use crate::evolution::{evolve_squad, level_for_rank};
use crate::matchsim::{fixture_value, Match};
use crate::rng::DelphiRng;
use crate::sector::{Slot, Status};

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
    pub k: f64,
    pub points: u16,
    pub goals_for: i32,
    pub goals_against: i32,
}

impl Club {
    pub fn new(name: &str, slots: Vec<Slot>) -> Self {
        Self {
            name: name.to_string(),
            slots,
            k: 1.0,
            points: 0,
            goals_for: 0,
            goals_against: 0,
        }
    }
}

/// Simplified 4-4-2 line-up (stand-in for `1010:4b73`): best fit players by strength per position,
/// then a bench of one goalkeeper and the four best remaining outfield players; unavailable players are out.
pub fn auto_lineup(slots: &mut [Slot]) {
    for s in slots.iter_mut() {
        s.status = Status::Out;
    }
    let take = |slots: &mut [Slot], pos: u8, n: usize, status: Status| {
        let mut idx: Vec<usize> = (0..slots.len())
            .filter(|&i| {
                slots[i].position == pos && slots[i].status == Status::Out && slots[i].available()
            })
            .collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(slots[i].strength));
        for &i in idx.iter().take(n) {
            slots[i].status = status;
        }
    };
    for (pos, n) in [(0u8, 1), (1, 4), (2, 4), (3, 2)] {
        take(slots, pos, n, Status::Starter);
    }
    take(slots, 0, 1, Status::Sub);
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

/// Plays one league round; updates ratings, injuries, bans, goals and points (3-1-0, assumed).
pub fn play_round(
    rng: &mut DelphiRng,
    clubs: &mut [Club],
    round: usize,
    names: &mut dyn FnMut(usize) -> (Vec<u8>, Vec<u8>, Vec<u8>),
) {
    for c in clubs.iter_mut() {
        auto_lineup(&mut c.slots);
    }
    for (h, a) in fixtures(round) {
        let (division, hn, an) = names(round * 8 + h);
        let hb = fixture_value(&division, &hn, true) + 2;
        let ab = fixture_value(&division, &an, true);
        let mut m = Match::new(clubs[h].slots.clone(), clubs[a].slots.clone(), hb, ab);
        m.home.k = clubs[h].k;
        m.away.k = clubs[a].k;
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
            std::cmp::Ordering::Greater => clubs[h].points += 3,
            std::cmp::Ordering::Less => clubs[a].points += 3,
            std::cmp::Ordering::Equal => {
                clubs[h].points += 1;
                clubs[a].points += 1;
            }
        }
    }
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
}
