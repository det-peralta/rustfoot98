//! The computer's line-up, a port of `1010:4388` / `1010:4b73` (see `docs/match-engine.md`).
//!
//! `4388` shuffles the squad list (3n remove-and-append moves), then runs the selection `4b73` twice:
//! first the core of the team with quotas goalkeeper 1, defenders 3, midfielders 3, attackers 2 (no bench), then with quotas
//! 1 / 10 / 10 / 10 and the bench enabled, which completes the eleven (at most ten outfield players in all) with the best
//! remaining players of any position and picks the bench: one goalkeeper (if any is left) and outfield players until five
//! substitutes are named. Players are ranked by `strength + goals scored` (`1050:1958`) with a stable bubble sort (`1068:0e0d`).
//! Injured and suspended players are never chosen. The number of substitutions allowed is `min(3, bench)`.
use crate::rng::DelphiRng;
use crate::sector::{Slot, Status};
use std::cmp::Reverse;

fn key(s: &Slot) -> i32 {
    s.strength as i32 + s.goals as i32
}

/// `1068:0f68`: 3n times, take a random element and move it to the end.
pub fn shuffle(rng: &mut DelphiRng, order: &mut Vec<usize>) {
    let n = order.len();
    if n == 0 {
        return;
    }
    for _ in 0..3 * n {
        let j = rng.random(n as u16) as usize;
        let v = order.remove(j);
        order.push(v);
    }
}

/// `1010:4b73`: one selection pass over the squad in `order` (which it re-sorts). Returns the number of substitutes named so far.
pub fn select(slots: &mut [Slot], order: &mut [usize], bench: bool, quota: [i32; 4]) -> usize {
    let fit = |s: &Slot| s.injured == 0 && s.ban == 0;
    let (mut candidates, mut want, mut sub_room) = ([0i32; 4], quota, [0i32; 4]);
    let (mut outfield, mut subs) = (0i32, 0usize);
    for &i in order.iter() {
        let s = &slots[i];
        if !fit(s) {
            continue;
        }
        let p = s.position as usize;
        match s.status {
            Status::Out => candidates[p] += 1,
            Status::Starter => {
                want[p] -= 1;
                if p != 0 {
                    outfield += 1;
                }
            }
            Status::Sub => {
                sub_room[p] -= 1;
                subs += 1;
            }
        }
    }
    for p in 0..4 {
        if want[p] < candidates[p] {
            sub_room[p] = 1;
        }
    }
    order.sort_by_key(|&i| Reverse(key(&slots[i]))); // stable, like the original's bubble sort
    for &i in order.iter() {
        let p = slots[i].position as usize;
        let free = fit(&slots[i]) && slots[i].status == Status::Out;
        if want[p] < 1 {
            if bench && free && sub_room[p] > 0 && (subs as i32) < (p == 0) as i32 + 4 {
                slots[i].status = Status::Sub;
                sub_room[p] -= 1;
                subs += 1;
            }
        } else if free && (p == 0 || outfield < 10) {
            slots[i].status = Status::Starter;
            want[p] -= 1;
            if p != 0 {
                outfield += 1;
            }
        }
    }
    if bench {
        for &i in order.iter() {
            if subs < 5
                && fit(&slots[i])
                && slots[i].status == Status::Out
                && slots[i].position != 0
            {
                slots[i].status = Status::Sub;
                subs += 1;
            }
        }
    }
    subs
}

/// The two passes of `4388` on a list order (no shuffle, nothing is reordered in the squad itself): statuses are set from scratch.
pub fn apply(slots: &mut [Slot], order: &mut [usize]) {
    for s in slots.iter_mut() {
        s.status = Status::Out;
    }
    select(slots, order, false, [1, 3, 3, 2]);
    select(slots, order, true, [1, 10, 10, 10]);
}

/// `1010:4388` for a computer-managed club: shuffles the squad list (squad and names stay parallel), selects the line-up and
/// leaves the squad in the final list order.
pub fn auto_lineup(rng: &mut DelphiRng, slots: &mut Vec<Slot>, names: &mut Vec<String>) {
    let mut order: Vec<usize> = (0..slots.len()).collect();
    shuffle(rng, &mut order);
    apply(slots, &mut order);
    let new_slots: Vec<Slot> = order.iter().map(|&i| slots[i]).collect();
    if names.len() == slots.len() {
        let new_names: Vec<String> = order.iter().map(|&i| names[i].clone()).collect();
        *names = new_names;
    }
    *slots = new_slots;
}

/// The same selection without shuffling or reordering, for the user's default line-up.
pub fn default_statuses(slots: &mut [Slot]) {
    let mut order: Vec<usize> = (0..slots.len()).collect();
    apply(slots, &mut order);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squad() -> Vec<Slot> {
        // 2 GK, 6 DEF, 6 MID, 4 ATT with distinct strengths
        let mut v = Vec::new();
        let mut n = 0u16;
        for (pos, count) in [(0u8, 2), (1, 6), (2, 6), (3, 4)] {
            for _ in 0..count {
                n += 1;
                v.push(Slot::new(&format!("P{n}"), pos, 10 + n, true, Status::Out));
            }
        }
        v
    }

    #[test]
    fn the_core_is_one_goalkeeper_three_three_two_plus_the_two_best_of_the_rest() {
        let mut s = squad();
        default_statuses(&mut s);
        let starters: Vec<&Slot> = s.iter().filter(|p| p.status == Status::Starter).collect();
        assert_eq!(starters.len(), 11);
        let n = |pos| starters.iter().filter(|p| p.position == pos).count();
        assert_eq!(n(0), 1);
        // 3-3-2 plus the best two remaining outfield players (the strongest here are attackers / midfielders)
        assert!(n(1) >= 3 && n(2) >= 3 && n(3) >= 2);
        assert_eq!(n(1) + n(2) + n(3), 10);
        // the best goalkeeper plays and the other one is on the bench
        assert_eq!(s[1].status, Status::Starter);
        assert_eq!(s[0].status, Status::Sub);
        assert_eq!(s.iter().filter(|p| p.status == Status::Sub).count(), 5);
    }

    #[test]
    fn injured_and_suspended_players_are_never_picked() {
        let mut s = squad();
        s[17].injured = 2;
        s[16].ban = 1;
        default_statuses(&mut s);
        assert_eq!(s[17].status, Status::Out);
        assert_eq!(s[16].status, Status::Out);
        assert_eq!(s.iter().filter(|p| p.status == Status::Starter).count(), 11);
    }

    #[test]
    fn goals_scored_raise_a_players_rank() {
        let mut s = squad();
        s[2].goals = 40; // a weak defender who scores a lot is picked
        s[2].strength = 1;
        default_statuses(&mut s);
        assert_eq!(s[2].status, Status::Starter);
    }

    #[test]
    fn shuffling_keeps_squad_and_names_together() {
        let mut rng = DelphiRng::new(5);
        let mut s = squad();
        let mut names: Vec<String> = (0..s.len()).map(|i| format!("P{}", i + 1)).collect();
        auto_lineup(&mut rng, &mut s, &mut names);
        for (slot, name) in s.iter().zip(&names) {
            let n: u16 = name[1..].parse().unwrap();
            assert_eq!(slot.strength, 10 + n);
        }
        assert_eq!(s.iter().filter(|p| p.status == Status::Starter).count(), 11);
    }
}
