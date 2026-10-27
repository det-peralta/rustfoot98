//! Economy index and player prices (verified in the original; see `docs/finance.md`).
use crate::rng::DelphiRng;

/// `10c8:2b46`: global economy index, saved in the `.e98` header (extended at `0x28`); **2.0** in a new game
/// (read from a new-game save; the player values below only come out right with this start).
pub const INITIAL_INDEX: f64 = 2.0;
/// `10c8:2d87`: currency scale `G` (a long). The sample saves use `Reais` with `G = 5`; new-game cash is `base * 50_000 * G`.
pub const DEFAULT_SCALE: i64 = 5;
/// A currency of *Dinheiro* (`TCurrencyDlg`): the five 101-byte records at `DS:0062` of the original, one of which is copied to
/// `DS:2d40` for the game (and into the `.e98` header): name, the word written after amounts, an abbreviation, the loan step (`+63`,
/// `2d7f`), a value used by the player-search dialog (`+67`), the scale `G` (`+71`, `2d87`), the unit of the ticket price shown
/// (`+75`, 1000 for escudos counted in contos), the season prizes (`+79..+87`: champion, cup winner, top scorer) and a factor of the
/// ticket price (`+91`, extended).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Currency {
    pub name: &'static str,
    /// Written after amounts (`1040:335a`), without the original's leading space.
    pub unit: &'static str,
    pub short: &'static str,
    pub loan_step: i64,
    pub search: i64,
    pub scale: i64,
    pub price_unit: i64,
    pub prizes: [i64; 3],
    pub factor: f64,
}

/// The currencies in the order of the dialog.
pub const CURRENCIES: [Currency; 5] = [
    Currency {
        name: "Escudos (contos)",
        unit: "contos",
        short: "esc.",
        loan_step: 100_000,
        search: 1,
        scale: 1,
        price_unit: 1000,
        prizes: [3_000_000, 1_000_000, 500_000],
        factor: 0.5,
    },
    Currency {
        name: "Euros",
        unit: "euros",
        short: "",
        loan_step: 500_000,
        search: 4,
        scale: 4,
        price_unit: 1,
        prizes: [15_000_000, 5_000_000, 3_000_000],
        factor: 2.0,
    },
    Currency {
        name: "Dólares",
        unit: "dólares",
        short: "",
        loan_step: 500_000,
        search: 4,
        scale: 5,
        price_unit: 1,
        prizes: [20_000_000, 6_000_000, 3_000_000],
        factor: 2.0,
    },
    Currency {
        name: "Reais",
        unit: "reais",
        short: "",
        loan_step: 500_000,
        search: 4,
        scale: 5,
        price_unit: 1,
        prizes: [20_000_000, 6_000_000, 3_000_000],
        factor: 2.0,
    },
    Currency {
        name: "Libras",
        unit: "libras",
        short: "",
        loan_step: 250_000,
        search: 4,
        scale: 4,
        price_unit: 1,
        prizes: [10_000_000, 4_000_000, 2_000_000],
        factor: 2.0,
    },
];

/// Index of *Reais* in [`CURRENCIES`] (the sample saves' currency and the default).
pub const REAIS: usize = 3;

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

/// `1010:7ff4`: the value (and wage per game) of a player in currency units:
/// `G * 50 * round(sqrt(F) * E * 35 * 1000^0.04 * strength^1.04 / 50)`, clamped to `[50 G, 50000 G]`.
/// `f` is the team's economic factor (extended at `team+0x79`), `e` the economy index, `g` the currency scale.
/// Verified on every strength of a new-game save (`E = 2`, `G = 5`, `F = 1`: 1 -> 500, 12 -> 6000, 13 -> 6750, 34 -> 18000).
/// (The original also keeps the previous value for injured players when it is higher.)
pub fn player_value(strength: u16, f: f64, e: f64, g: i64) -> i64 {
    let y = f.sqrt() * (g as f64 * e) * 35.0 * 1000f64.powf(0.04) * (strength as f64).powf(1.04);
    let units = (y / 50.0 / g as f64).round_ties_even() as i64;
    (units * 50 * g).clamp(50 * g, 50_000 * g)
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
        assert!(
            mean_after(2026) > INITIAL_INDEX + 0.2,
            "{}",
            mean_after(2026)
        );
        assert!(
            mean_after(1998) < INITIAL_INDEX - 0.2,
            "{}",
            mean_after(1998)
        );
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
    fn player_values_match_a_new_game_save() {
        let v = |s| player_value(s, 1.0, INITIAL_INDEX, DEFAULT_SCALE);
        for (s, want) in [
            (1, 500),
            (2, 1000),
            (5, 2500),
            (12, 6000),
            (13, 6750),
            (14, 7250),
            (20, 10500),
            (31, 16500),
            (34, 18000),
        ] {
            assert_eq!(v(s), want, "strength {s}");
        }
        // the economic factor of a poor division lowers values a little
        assert!(player_value(20, 0.9768, 2.0, 5) < v(20));
    }

    #[test]
    fn player_values_are_clamped_and_multiples_of_50_g() {
        assert_eq!(player_value(1, 0.0001, 0.5, 5), 250);
        assert_eq!(player_value(50, 1e9, 10.0, 5), 250_000);
        assert_eq!(player_value(8, 1.0, 1.0, 5) % 250, 0);
    }
}
