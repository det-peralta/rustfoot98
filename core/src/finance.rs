//! Club finances: cash, the ledger, wages, gate receipts, stadium. What is here was verified against real saves
//! (see `docs/finance.md`): the per-game wage bill (`5a0a`), the gate (`9396` + `b0a1`), the division ticket price
//! (`35e1`), new-game cash (`0373`) and the stadium capacity. Transfers, loans and the economic-factor smoothing
//! (`6e48`) are only partly decoded and are approximated or left out where noted.
use crate::rng::DelphiRng;
use crate::sector::Slot;

/// Ledger categories of `1010:58bf` that the port books.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Gate,
    Sales,
    Purchases,
    Wages,
    Interest,
    /// Stadium works (category `0x17`).
    Works,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finance {
    pub cash: i64,
    /// Outstanding loan (`+0x6d`). Loans cannot be taken in the port yet; the interest is 5 % per game.
    pub loan: i64,
    /// Economic factor `F` (`+0x79`), shared by the clubs of a division.
    pub f: f64,
    /// Stadium level (`+4` of the stadium object): capacity is `level * 5000`.
    pub stadium: u8,
    /// Stadium filling, an exponential average of attendance over capacity (`stadium+5`, `1010:c0c0`); 0.8 or more asks for an expansion.
    pub fill: f64,
    pub gate: i64,
    pub sales: i64,
    pub purchases: i64,
    pub wages: i64,
    pub interest: i64,
    pub works: i64,
}

impl Default for Finance {
    fn default() -> Self {
        Finance {
            cash: 0,
            loan: 0,
            f: 1.0,
            stadium: 1,
            fill: 0.6,
            gate: 0,
            sales: 0,
            purchases: 0,
            wages: 0,
            interest: 0,
            works: 0,
        }
    }
}

pub const SEATS_PER_LEVEL: i64 = 5000;
pub const MAX_STADIUM_LEVEL: u8 = 24;

impl Finance {
    /// New-game state for the club at master-list position with base strength `base` (`1028:0373`).
    pub fn new_game(base: i32, g: i64) -> Finance {
        Finance {
            cash: base as i64 * 50_000 * g,
            stadium: ((base as f64 * 0.3).round_ties_even() as i64)
                .clamp(1, MAX_STADIUM_LEVEL as i64) as u8,
            ..Finance::default()
        }
    }

    pub fn capacity(&self) -> i64 {
        self.stadium as i64 * SEATS_PER_LEVEL
    }

    /// `1010:58bf`: adds to the cash and to the category total.
    pub fn book(&mut self, amount: i64, category: Category) {
        self.cash += amount;
        match category {
            Category::Gate => self.gate += amount,
            Category::Sales => self.sales += amount,
            Category::Purchases => self.purchases += amount,
            Category::Wages => self.wages += amount,
            Category::Interest => self.interest += amount,
            Category::Works => self.works += amount,
        }
    }
}

/// `1010:5992`: what a club pays in wages per game: the sum of the players' stored wages (`slot+0x31`).
pub fn wage_bill(slots: &[Slot]) -> i64 {
    slots.iter().map(|s| s.wage).sum()
}

/// `1010:4ec3`: the rounded mean strength of a squad.
pub fn rating(slots: &[Slot]) -> i64 {
    if slots.is_empty() {
        return 0;
    }
    let sum: i64 = slots.iter().map(|s| s.strength as i64).sum();
    (sum as f64 / slots.len() as f64).round_ties_even() as i64
}

/// `1040:35e1` as called by `1010:0875`: the ticket price of a division's games,
/// `round(G * E / sqrt(max(avgF, 0.1)) * avgRating / 5) / 2`, at least `G / 2`.
/// The original multiplies by `1 + 0.5 * [flag == 1]` for a flag we could not trace; applying it to the user's division gave
/// prices 30 % too high against the sample save, so it is left out. Against the sample the formula is within 3 % for the
/// three upper divisions (44 / 32 / 18) and about 15 % low for the fourth (7 against ~8).
pub fn ticket_price(avg_f: f64, avg_rating: i64, e: f64, g: i64) -> f64 {
    let a = avg_f.max(0.1);
    let x = (g as f64 * e) / a.sqrt() * avg_rating as f64 / 5.0;
    let price = x.round_ties_even() / 2.0;
    price.max(g as f64 * 0.5)
}

/// `1010:c0c0`: after a home game the stadium's filling moves toward `attendance / (capacity + 1)` (weight 0.4).
pub fn update_fill(f: &mut Finance, attendance: i64) {
    let ratio = attendance as f64 / (f.capacity() + 1) as f64;
    f.fill = 0.6 * f.fill + 0.4 * ratio;
}

/// `1010:c030`: the stadium asks for an expansion when it is at least 80 % full and not at the maximum level.
pub fn wants_expansion(f: &Finance) -> bool {
    f.fill >= 0.8 && f.stadium < MAX_STADIUM_LEVEL
}

/// `1010:c172` / `1040:37ee`: price of the next 5000 seats.
pub fn expansion_cost(f: &Finance, e: f64, g: i64) -> i64 {
    let bracket = if f.stadium >= 14 {
        3.0
    } else if f.stadium >= 6 {
        2.0
    } else {
        1.0
    };
    let units = (f64::sqrt(bracket) * e * 35_000.0 * g as f64 / 50_000.0).round_ties_even() as i64;
    (units * 50_000).max(50_000)
}

/// `1010:c081`: one more level; the filling falls back to 0.6.
pub fn expand(f: &mut Finance) {
    f.stadium += 1;
    f.fill = 0.6;
}

/// `1040:3865`: interest per game, 5 % of the loan.
pub fn loan_interest(loan: i64) -> i64 {
    (loan as f64 * 0.05).round_ties_even() as i64
}

/// The loan step (`10c8:2d7f`): the header of the sample saves holds 500 000 with `G = 5`.
pub fn loan_step(g: i64) -> i64 {
    100_000 * g
}

/// `1040:3899`: the bank's limit, `rating * K * E * G * 20000` rounded down to a whole step (at least one step).
pub fn max_loan(rating: i64, k: f64, e: f64, g: i64) -> i64 {
    let step = loan_step(g);
    let raw = (rating as f64 * k * e * g as f64 * 20_000.0).round_ties_even() as i64;
    (raw / step * step).max(step)
}

/// `1010:b0a1`: attendance of a game. `w = 3 * rating(home) + rating(away)`, a random part up to 3, both morales under a
/// root, then `^1.04`, cup games +30 %, capped by the home stadium.
pub fn attendance(
    rng: &mut DelphiRng,
    home_rating: i64,
    away_rating: i64,
    home_k: f64,
    away_k: f64,
    cup: bool,
    capacity: i64,
) -> i64 {
    let w = (3 * home_rating + away_rating) as f64 + rng.random_real() * 3.0;
    let p = w * home_k.sqrt() * away_k.sqrt() * 200.0;
    let att = (p.powf(1.04) * (1.0 + if cup { 0.3 } else { 0.0 })).round_ties_even() as i64;
    att.min(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_cash_and_stadium_follow_the_base_strength() {
        let best = Finance::new_game(49, 5);
        assert_eq!(best.cash, 12_250_000);
        assert_eq!(best.stadium, 15);
        assert_eq!(best.capacity(), 75_000);
        let low = Finance::new_game(1, 5);
        assert_eq!((low.cash, low.stadium), (250_000, 1));
        assert_eq!(Finance::new_game(5, 5).cash, 1_250_000);
        assert_eq!(Finance::new_game(5, 5).stadium, 2);
    }

    #[test]
    fn ticket_prices_match_the_first_round_of_a_real_save() {
        // first division: rating 44 -> 44, third: 18 -> 18 (the sample save: ~44.8 and ~18.5)
        assert_eq!(ticket_price(1.0, 44, 2.0, 5), 44.0);
        assert_eq!(ticket_price(1.0, 18, 2.0, 5), 18.0);
        assert_eq!(ticket_price(0.9345, 7, 2.0, 5), 7.0);
        // never below half the currency scale
        assert_eq!(ticket_price(1.0, 0, 2.0, 5), 2.5);
    }

    #[test]
    fn attendance_is_capped_by_the_stadium_and_grows_with_morale() {
        let mut rng = DelphiRng::new(1);
        let a = attendance(&mut rng, 5, 6, 1.0, 1.0, false, 10_000);
        assert!((5800..6800).contains(&a), "{a}"); // the real save's first home game of a 4th-division club: 5931
        let mut rng = DelphiRng::new(1);
        assert!(attendance(&mut rng, 5, 6, 2.0, 2.0, false, 10_000) > a);
        let mut rng = DelphiRng::new(1);
        assert_eq!(attendance(&mut rng, 44, 44, 1.0, 1.0, false, 5_000), 5_000);
    }

    #[test]
    fn loans_and_stadium_follow_the_decoded_formulas() {
        assert_eq!(loan_interest(1_000_000), 50_000);
        assert_eq!(max_loan(5, 1.0, 2.0, 5), 1_000_000);
        assert_eq!(max_loan(0, 1.0, 2.0, 5), 500_000);
        let mut f = Finance::new_game(5, 5);
        assert_eq!(expansion_cost(&f, 2.0, 5), 350_000);
        assert!(!wants_expansion(&f));
        for _ in 0..20 {
            let cap = f.capacity();
            update_fill(&mut f, cap);
        }
        assert!(wants_expansion(&f));
        expand(&mut f);
        assert_eq!((f.stadium, f.fill), (3, 0.6));
        f.stadium = 14;
        assert_eq!(expansion_cost(&f, 2.0, 5), 600_000);
    }

    #[test]
    fn booking_updates_cash_and_the_ledger() {
        let mut f = Finance::new_game(5, 5);
        f.book(47_448, Category::Gate);
        f.book(-40_500, Category::Wages);
        assert_eq!(f.cash, 1_250_000 + 47_448 - 40_500);
        assert_eq!((f.gate, f.wages), (47_448, -40_500));
    }
}
