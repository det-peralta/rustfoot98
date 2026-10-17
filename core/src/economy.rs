//! Economy index and player prices (verified in the original; see `docs/finance.md`).
use crate::rng::DelphiRng;

/// `10c8:2b46`: global economy index, saved in the `.e98` header; 1.0 in a new game.
pub const INITIAL_INDEX: f64 = 1.0;
/// Clamp of the index (`1020:0c3d` / `1020:0c39`).
pub const INDEX_MIN: f64 = 0.5;
pub const INDEX_MAX: f64 = 10.0;
/// Step of the random walk (`1020:0c2f`).
pub const INDEX_STEP: f64 = 0.01;

/// `1020:0c41`, once per round: `E += 0.01 * d`, then clamp to `[0.5, 10]`.
/// `d = r1 + r2 - r3` when `year mod 100 < 50` (drifts up), else `r1 - r2 - r3` (drifts down), with `r` = `Random()` reals
/// drawn in that order. (The year-based drift is a quirk of the original.)
pub fn walk_index(rng: &mut DelphiRng, e: f64, year: u16) -> f64 {
    let r1 = rng.random_real();
    let r2 = rng.random_real();
    let r3 = rng.random_real();
    let d = if year % 100 < 50 {
        r1 + r2 - r3
    } else {
        r1 - r2 - r3
    };
    (e + INDEX_STEP * d).clamp(INDEX_MIN, INDEX_MAX)
}

/// `1010:7ff4`: the value of a player in units of the currency scale `G` (`10c8:2d87`; `G` cancels out of the formula):
/// `clamp(50 * round(sqrt(F) * E * 35 * strength^1.04 * 10^0.04 / 50), 50, 50000)`.
/// `f` is the team's economic factor (extended at `team+0x79`, how it is initialised is not decoded).
/// (The original also keeps the previous value for injured players when it is higher.)
pub fn player_value(strength: u16, f: f64, e: f64) -> i64 {
    let r = (f.sqrt() * e * 35.0 * (strength as f64).powf(1.04) * 10f64.powf(0.04) / 50.0)
        .round_ties_even();
    (r * 50.0).clamp(50.0, 50_000.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_index_stays_in_bounds_and_drifts_with_the_year() {
        let mean_after = |year: u16| {
            let mut rng = DelphiRng::new(9);
            let mut sum = 0.0;
            for _ in 0..200 {
                let mut e = INITIAL_INDEX;
                for _ in 0..14 * 5 {
                    e = walk_index(&mut rng, e, year);
                    assert!((INDEX_MIN..=INDEX_MAX).contains(&e));
                }
                sum += e;
            }
            sum / 200.0
        };
        // years 00..49 drift up, years 50..99 drift down (mean step +/- 0.5 * 0.01 per round)
        assert!(mean_after(2026) > 1.2, "{}", mean_after(2026));
        assert!(mean_after(1998) < 0.8, "{}", mean_after(1998));
    }

    #[test]
    fn the_index_is_clamped() {
        let mut rng = DelphiRng::new(1);
        let mut e = INDEX_MAX;
        for _ in 0..1000 {
            e = walk_index(&mut rng, e, 2000);
        }
        assert_eq!(e, INDEX_MAX);
        let mut e = INDEX_MIN;
        for _ in 0..1000 {
            e = walk_index(&mut rng, e, 1999);
        }
        assert_eq!(e, INDEX_MIN);
    }

    #[test]
    fn player_values_grow_with_strength_and_are_clamped() {
        let v = |s, f, e| player_value(s, f, e);
        assert!(v(10, 100.0, 1.0) > v(5, 100.0, 1.0));
        assert!(v(5, 400.0, 1.0) > v(5, 100.0, 1.0));
        assert!(v(5, 100.0, 2.0) > v(5, 100.0, 1.0));
        assert_eq!(v(1, 0.0001, 0.5), 50);
        assert_eq!(v(50, 1e9, 10.0), 50_000);
        assert_eq!(v(8, 100.0, 1.0) % 50, 0);
    }
}
