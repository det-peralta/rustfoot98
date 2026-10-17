//! The knock-out cup (`Taça`), decoded from `1020:0ed4` (draw), `1010:9396` (result handling), `1050:0d46` (shootout) and
//! `1048:069b` (extra time).
//!
//! * The cup is played on every **even** round, after that round's league games. It involves **every** club of the world
//!   (divisions and outside pool), `34..=128` of them.
//! * The draw of round `r` happens only when `N > p`, with `p = 2^((14 - r) / 2)` and `N` the number of clubs of the world, so the
//!   final always falls on round 14 whatever `N` is.
//! * The survivors are shuffled (`1068:0f68`); with `n` survivors, `k` = the smallest power of two `>= n / 2` and `n - k`
//!   consecutive pairs of the shuffled list play (first of the pair at home); the others get a bye.
//! * Single leg. A draw after 90 minutes goes to extra time (to minute 120); if still level, a shootout decides.
//! * **The automatic shootout is inverted in the original**: each side draws three `Random(squad mean strength)` and the home
//!   side is eliminated when its sum is **greater than or equal to** the away side's, so the lower total wins and ties go to
//!   the away side. This port keeps that behaviour (it is what the program does).
//! * Morale after a tie (`1068:10d1`): the winner's result is 1.5 when the home side advanced and 2.0 when the away side
//!   advanced; the loser's is 0.
use crate::rng::DelphiRng;
use crate::sector::Slot;

/// `p`: number of clubs the world must exceed for the cup to be drawn in `round` (even, 2..=14).
pub fn threshold(round: usize) -> usize {
    1usize << ((14usize.saturating_sub(round)) / 2)
}

/// `(k, pairs)` for `n` survivors: `k` is the smallest power of two `>= n/2`, `pairs = n - k` games are played.
pub fn round_size(n: usize) -> (usize, usize) {
    let mut k = 1usize;
    while (n as f64 / 2.0) > k as f64 {
        k <<= 1;
    }
    (k, n.saturating_sub(k))
}

/// Name of the round for `k` (`1020:0ed4`).
pub fn round_name(k: usize) -> String {
    match k {
        1 => "FINAL".to_string(),
        2 | 3 => "Meias-finais".to_string(),
        4..=7 => "Quartos de final".to_string(),
        8..=15 => "Oitavos de final".to_string(),
        _ => format!("{k} avos de final"),
    }
}

/// `1068:0f68`: `3n` times take a random element and move it to the end.
pub fn shuffle<T>(rng: &mut DelphiRng, v: &mut Vec<T>) {
    let n = v.len();
    if n == 0 {
        return;
    }
    for _ in 0..n * 3 {
        let i = rng.random(n as u16) as usize;
        let x = v.remove(i);
        v.push(x);
    }
}

/// `1010:4ec3`: the team rating, the rounded mean strength of the whole squad (bench included).
pub fn team_rating(slots: &[Slot]) -> u16 {
    if slots.is_empty() {
        return 0;
    }
    let sum: u32 = slots.iter().map(|s| s.strength as u32).sum();
    (sum as f64 / slots.len() as f64).round_ties_even() as u16
}

/// `1050:0d46` (automatic branch): true when the **home** side is eliminated. Home draws first.
pub fn shootout_home_eliminated(rng: &mut DelphiRng, home_rating: u16, away_rating: u16) -> bool {
    let home: u32 = (0..3).map(|_| rng.random(home_rating) as u32).sum();
    let away: u32 = (0..3).map(|_| rng.random(away_rating) as u32).sum();
    home >= away
}

/// Morale results `(home, away)` after a cup tie (`1010:9396`).
pub fn morale_results(home_advances: bool) -> (f64, f64) {
    if home_advances {
        (1.5, 0.0)
    } else {
        (0.0, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_final_always_falls_on_round_14() {
        assert_eq!(
            (2..=14).step_by(2).map(threshold).collect::<Vec<_>>(),
            [64, 32, 16, 8, 4, 2, 1]
        );
        for total in 34..=128usize {
            // simulate the bracket: a round is drawn when total > p
            let mut n = total;
            let mut last = 0;
            for r in (2..=14).step_by(2) {
                if total > threshold(r) && n >= 2 {
                    let (_, pairs) = round_size(n);
                    n -= pairs;
                    last = r;
                }
            }
            assert_eq!(n, 1, "{total} clubs leave one winner");
            assert_eq!(last, 14, "{total} clubs end with the final in round 14");
        }
    }

    #[test]
    fn round_sizes_and_names() {
        assert_eq!(round_size(40), (32, 8));
        assert_eq!(round_size(32), (16, 16));
        assert_eq!(round_size(100), (64, 36));
        assert_eq!(round_size(2), (1, 1));
        assert_eq!(round_size(3), (2, 1));
        assert_eq!(round_name(1), "FINAL");
        assert_eq!(round_name(2), "Meias-finais");
        assert_eq!(round_name(4), "Quartos de final");
        assert_eq!(round_name(8), "Oitavos de final");
        assert_eq!(round_name(32), "32 avos de final");
    }

    #[test]
    fn shuffle_keeps_the_elements_and_depends_on_the_seed() {
        let mut a: Vec<u32> = (0..40).collect();
        let mut b = a.clone();
        shuffle(&mut DelphiRng::new(1), &mut a);
        shuffle(&mut DelphiRng::new(2), &mut b);
        assert_ne!(a, b);
        let mut sorted = a.clone();
        sorted.sort();
        assert_eq!(sorted, (0..40).collect::<Vec<_>>());
    }

    #[test]
    fn the_automatic_shootout_is_inverted_like_the_original() {
        // the side with the lower rating (lower expected sum) wins more often: home eliminated often when home rating is high
        let mut rng = DelphiRng::new(5);
        let eliminated = |h: u16, a: u16, rng: &mut DelphiRng| {
            (0..20_000)
                .filter(|_| shootout_home_eliminated(rng, h, a))
                .count()
        };
        let strong_home = eliminated(40, 10, &mut rng);
        let weak_home = eliminated(10, 40, &mut rng);
        assert!(
            strong_home > 18_000 && weak_home < 2_000,
            "{strong_home} {weak_home}"
        );
        // equal ratings: ties go to the away side, so the home side is eliminated slightly more than half the time
        let even = eliminated(20, 20, &mut rng);
        assert!(even > 10_000, "{even}");
    }

    #[test]
    fn rating_is_the_mean_strength_of_the_whole_squad() {
        use crate::sector::Status;
        let slots: Vec<Slot> = [5u16, 6, 7, 8]
            .iter()
            .map(|&s| Slot::new("x", 1, s, true, Status::Out))
            .collect();
        assert_eq!(team_rating(&slots), 6); // 6.5 rounds to even
        assert_eq!(morale_results(true), (1.5, 0.0));
        assert_eq!(morale_results(false), (0.0, 2.0));
    }
}
