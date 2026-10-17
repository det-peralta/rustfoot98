//! Season layer: the league fixture table, rounds, standings and per-round evolution.
//!
//! **Verified in the original:** the 8-team double round-robin table (`DS:0x1838`, read by `1010:1d20`),
//! the 14 league rounds (`1020:11b8` loops 0..=14), **3-1-0 points** (`1010:9396` adds 3 to `team+0x20` for a win and
//! 1 for a draw), wins/draws/losses at `+0x16/+0x18/+0x1a`, standings sorted by points only (`1050:1a61`, stable sort
//! assumed), morale `K = 0.85*K + 0.15*result` after every game (`1068:10d1`), evolution after every round with
//! `level = base - 2 * rank`, and the season end (`1020:038e`): champion `K += 1.0`, cup winner `K += 0.5`, the top two
//! and bottom two of adjacent divisions swap (`1020:02a6`) and two random outside teams join the lowest division.
//! **Assumed (not yet decoded):** the 4-4-2 auto line-up (the real `1010:4b73` picks the formation), tie order, and that
//! ranks are taken after the round's results.
use crate::evolution::{evolve_squad, level_for_rank};
use crate::matchsim::{fixture_value, morale_results, update_morale, Match};
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

/// Plays one league round; updates ratings, injuries, bans, goals, points (3-1-0), W/D/L and morale.
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
        let mut names = move |_k: usize| {
            let mut n = || {
                (0..8)
                    .map(|_| b'a' + seed.random(26) as u8)
                    .collect::<Vec<u8>>()
            };
            (n(), n(), n())
        };
        play_round(&mut rng, &mut clubs, 1, &mut names);
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
}
