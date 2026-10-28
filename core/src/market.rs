//! The computer-managed transfer market, decoded from `ELIFOOT.EXE` (see `docs/finance.md`):
//! who puts a player up for sale (`1010:3aad` / `3e9c`), the asking price (`1040:371a`), the bids of the other clubs
//! (`1010:633d`, `5bb1`, `5f77`) and how the auction is settled (`1008:373e`, `351a`, `39c7`).
//! The original shows an auction dialog to the human; here only computer-managed clubs bid, all at once, and the
//! highest bid (first in the master list on a tie) wins.
use crate::season::Club;
use crate::sector::Slot;

/// Countries counted as "the same group" (`CTRGROUP/BOSMAN.TXE` and `PLOP.TXE` of the game).
pub const BOSMAN: [&str; 18] = [
    "POR", "ESP", "FRA", "ITA", "ALE", "AUT", "GRE", "ING", "ESC", "WAL", "ILN", "IRL", "DIN",
    "BEL", "HOL", "LUX", "FIN", "SUE",
];
pub const PLOP: [&str; 9] = [
    "POR", "BRA", "ANG", "MOC", "CAV", "STP", "GBI", "GNE", "TIM",
];

/// `1058:0a2f`: 0 same country, 1 both in the same group, 2 foreign.
pub fn country_class(club: [u8; 3], nat: [u8; 3]) -> u8 {
    if club == nat {
        return 0;
    }
    let (c, n) = (
        std::str::from_utf8(&club).unwrap_or(""),
        std::str::from_utf8(&nat).unwrap_or(""),
    );
    if (BOSMAN.contains(&c) && BOSMAN.contains(&n)) || (PLOP.contains(&c) && PLOP.contains(&n)) {
        1
    } else {
        2
    }
}

/// A country code as the three bytes the game stores (shorter codes are padded with spaces).
pub fn code(s: &str) -> [u8; 3] {
    let mut c = [b' '; 3];
    for (i, b) in s.bytes().take(3).enumerate() {
        c[i] = b;
    }
    c
}

/// Squad shape table at `DS:0x300`: rows by level `L = rating / 10` (0..5), columns
/// `[GK, DEF, MID, ATT, -, total]`.
pub const SHAPE: [[u8; 6]; 6] = [
    [3, 8, 7, 3, 0, 22],
    [3, 8, 7, 4, 0, 22],
    [3, 8, 8, 5, 0, 23],
    [4, 8, 8, 5, 0, 24],
    [4, 9, 8, 5, 0, 25],
    [4, 9, 8, 5, 0, 26],
];

fn level(rating: i64) -> usize {
    ((rating / 10).clamp(0, 5)) as usize
}

pub fn rating(club: &Club) -> i64 {
    crate::finance::rating(&club.slots)
}

/// Players of a position (`team+0x83+pos`).
pub fn count(club: &Club, position: u8) -> usize {
    club.slots.iter().filter(|s| s.position == position).count()
}

/// Players of class 2 (`team+0x87`).
pub fn foreigners(club: &Club) -> usize {
    club.slots.iter().filter(|s| s.foreign == 2).count()
}

/// `1010:5992`: the wage bill.
pub fn wage_sum(club: &Club) -> i64 {
    club.slots.iter().map(|s| s.wage).sum()
}

/// `1010:5de1`: how little the club needs the player (the lowest score is sold first).
pub fn sale_score(club: &Club, slot: &Slot) -> f64 {
    let r = rating(club);
    let l = level(r);
    let pos = slot.position as usize;
    let aggression = (5.0 / (slot.aggression as f64 + 1.0)).sqrt().sqrt();
    let mut score = aggression * SHAPE[l][pos] as f64 / (count(club, slot.position) as f64 + 1.0)
        * SHAPE[l][5] as f64
        / (club.slots.len() as f64 + 1.0)
        * slot.strength as f64
        / (r as f64 + 1.0);
    match slot.foreign {
        0 => score *= 1.5,
        1 => score *= 1.2,
        _ => {}
    }
    score
}

/// `1010:5a6a`: 0 when the player may be sold; otherwise the reason.
pub fn sale_blocker(club: &Club, idx: usize) -> u8 {
    let s = &club.slots[idx];
    if club.slots.len() < 12 {
        1
    } else if s.fresh {
        3
    } else if s.position == 0 && count(club, 0) < 2 {
        5
    } else if s.position != 0 && club.slots.len() as i64 - count(club, 0) as i64 <= 10 {
        6
    } else {
        0
    }
}

/// `1040:371a`: what a player is worth on the market (before the auction), `strength^1.04 * G * E * f * 2000` with `f`
/// lowered by aggression and injury proneness and raised by talent.
pub fn market_value(slot: &Slot, e: f64, g: i64) -> i64 {
    let f = 1.0 - slot.aggression as f64 / 5.0 * 0.2 - slot.proneness as f64 / 10.0 * 0.2
        + slot.g as f64 / 10.0 * 0.2;
    ((slot.strength as f64).powf(1.04) * (g as f64 * e) * f * 2000.0).round_ties_even() as i64
}

/// The club picks the player to put up (`3aad`): the sellable player with the lowest score; with exactly five
/// foreigners only a foreigner may go.
pub fn pick_for_sale(club: &Club) -> Option<usize> {
    let five = foreigners(club) == 5;
    let mut best: Option<(usize, f64)> = None;
    for (i, s) in club.slots.iter().enumerate() {
        if sale_blocker(club, i) != 0 || (five && s.foreign != 2) {
            continue;
        }
        let score = sale_score(club, s);
        if best.is_none_or(|(_, b)| score < b) {
            best = Some((i, score));
        }
    }
    best.map(|(i, _)| i)
}

/// `1010:633d` (without the human rules): may `buyer` bid for `slot`?
pub fn may_bid(buyer: &Club, slot: &Slot) -> bool {
    bid_refusal(buyer, slot).is_none()
}

/// Why `buyer` may not take `slot`, in the original's words: a full squad (24 with the players bought) or the fifth foreigner
/// outside the Bosman group.
pub fn bid_refusal(buyer: &Club, slot: &Slot) -> Option<&'static str> {
    if buyer.slots.len() + buyer.bought as usize >= 24 {
        return Some("Tem demasiados jogadores");
    }
    let class = country_class(buyer.country, slot.nat);
    (foreigners(buyer) + buyer.bought_foreign as usize >= 5 && class == 2)
        .then_some("Tem demasiados jogadores estrangeiros")
}

/// `1010:5bb1`: how much the buyer wants the player.
pub fn need(buyer: &Club, slot: &Slot) -> f64 {
    let r = rating(buyer);
    let l = level(r);
    let pos = slot.position as usize;
    let mut x = slot.strength as f64 / (r as f64 + 5.0) * (SHAPE[l][pos] as f64 + 1.0)
        / (count(buyer, slot.position) as f64 + 1.0)
        * SHAPE[l][5] as f64
        / (buyer.slots.len() as f64 + 1.0);
    x = x.sqrt().sqrt() * (r as f64).powf(0.2);
    if slot.position == 0 && count(buyer, 0) == 1 {
        x *= 1.5;
    }
    let foreign = foreigners(buyer) as f64;
    let f = match country_class(buyer.country, slot.nat) {
        0 => foreign * 0.05 + 1.5,
        1 => foreign * 0.05 + 1.15,
        _ => 1.0,
    };
    x *= f;
    if slot.star() {
        x *= 1.1;
    }
    x
}

/// The binary divides by 1.6 here (`CS:5f57`), but with 1.6 the bids come out about 40 % above the fees the original booked. On the
/// 36 computer-to-computer auctions of a season and a half of the original's saves (`core/examples/auctions.rs`) 2.2 gives the fees
/// with a median error of 3 % and a spread of 7 % (see `docs/finance.md`). Kept as a calibrated constant until the gap is explained.
pub const BID_DIVISOR: f64 = 2.2;

/// `1010:5f77`: the bid of a computer-managed club for `slot`, whose value (`7ff4` in the seller's economy) is `value`.
pub fn bid(buyer: &Club, value: i64, slot: &Slot, e: f64, g: i64) -> i64 {
    let r = rating(buyer);
    let l = level(r);
    if SHAPE[l][slot.position as usize] as usize == count(buyer, slot.position) {
        return 0;
    }
    let cash = buyer.fin.cash as f64;
    let w = wage_sum(buyer);
    let mut x = market_value(slot, e, g) as f64 * need(buyer, slot);
    // spare cash measured in wage periods (`FSUBRP` leaves cash - X)
    let ratio = (cash - x) / (w + 1 + value) as f64;
    if ratio < 3.0 {
        x = cash - 3.0 * (w + value) as f64;
    } else if ratio > 5.0 {
        x = x * ratio.ln() / BID_DIVISOR;
    }
    if w > 100 {
        let spare = cash - x;
        if spare > 10.0 * w as f64 && spare > 1.0 {
            x *= ratio_term(spare, w);
        }
    }
    let a = (cash - x - 5.0 * w as f64).round_ties_even() as i64;
    let b = x.round_ties_even() as i64;
    a.min(b).max(0)
}

fn ratio_term(over: f64, w: i64) -> f64 {
    ((over / w as f64 - 9.0).ln() / 10.0) + 1.0
}

/// `1010:63fa`: the price a computer-managed club asks when the user wants to buy `slot` directly (0 = not for sale).
/// Only players the club could do without (sale score at most 1.95) are for sale; the price is
/// `market value * 3.5 * score`, raised for a buyer of the same country (`(p + 30000) * 1.6`) or country group
/// (`(p + 15000) * 1.3`), then rounded down to 100 000 from 2 M, 50 000 from 1 M, 25 000 above 500 000, 10 000 above 100 000
/// and 1000 below.
pub fn direct_price(seller: &Club, idx: usize, buyer: Option<&Club>, e: f64, g: i64) -> i64 {
    if sale_blocker(seller, idx) != 0 {
        return 0;
    }
    let slot = &seller.slots[idx];
    let score = sale_score(seller, slot);
    if score > 1.95 {
        return 0;
    }
    let mut price = (market_value(slot, e, g) as f64 * 3.5 * score).round_ties_even() as i64;
    if let Some(b) = buyer {
        match country_class(b.country, slot.nat) {
            0 => price = ((price + 30_000) as f64 * 1.6).round_ties_even() as i64,
            1 => price = ((price + 15_000) as f64 * 1.3).round_ties_even() as i64,
            _ => {}
        }
    }
    let unit = if price >= 2_000_000 {
        100_000
    } else if price >= 1_000_000 {
        50_000
    } else if price > 500_000 {
        25_000
    } else if price > 100_000 {
        10_000
    } else {
        1000
    };
    price / unit * unit
}

/// `3aad`: the asking price of a computer-managed club's auction, `371a * 1.6` (times the header's extended at `2b50`, 1.0 in
/// every save seen) rounded down to 1000 by `5b37`: the *Preço base* of the auction dialog.
pub fn asking_price(slot: &Slot, e: f64, g: i64) -> i64 {
    floor_1000((market_value(slot, e, g) as f64 * 1.6).round_ties_even() as i64)
}

/// Rounds a price down to a multiple of 1000 (`1882` then `1845`).
pub fn floor_1000(v: i64) -> i64 {
    v.div_euclid(1000) * 1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_bid_gives_the_originals_reason() {
        let w = crate::world::World::synthetic(1);
        let mut c = w.divisions[0][0].clone();
        c.country = *b"BRA";
        let mut s = c.slots[0];
        s.nat = *b"BRA";
        c.bought = 0;
        c.bought_foreign = 0;
        for x in c.slots.iter_mut() {
            x.nat = *b"BRA";
        }
        assert_eq!(bid_refusal(&c, &s), None);
        c.bought = (24 - c.slots.len()) as u16;
        assert_eq!(bid_refusal(&c, &s), Some("Tem demasiados jogadores"));
        c.bought = 0;
        c.bought_foreign = 5;
        s.nat = *b"JPN";
        assert_eq!(
            bid_refusal(&c, &s),
            Some("Tem demasiados jogadores estrangeiros")
        );
    }

    #[test]
    fn country_classes_follow_the_group_files() {
        assert_eq!(country_class(*b"BRA", *b"BRA"), 0);
        assert_eq!(country_class(*b"BRA", *b"POR"), 1); // PLOP
        assert_eq!(country_class(*b"BEL", *b"HOL"), 1); // Bosman
        assert_eq!(country_class(*b"BRA", *b"COL"), 2);
        assert_eq!(country_class(*b"BEL", *b"BRA"), 2);
    }

    #[test]
    fn rounding_is_down_to_thousands() {
        assert_eq!(floor_1000(5_171_999), 5_171_000);
        assert_eq!(floor_1000(999), 0);
    }
}
