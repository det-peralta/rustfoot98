//! The computer clubs' auctions in consecutive saves of the original (`tools/capture/days.sh`): for every new transfer, the bid of
//! every computer-managed club (`market::bid`, the clubs as the later save has them with the sale undone, the player at the
//! strength of the transfer record) next to the fee the original booked, and in the end how often the buyer has the best bid and
//! how far the fees are from the buyer's bid:
//! `cargo run --release -p rustfoot-core --example auctions -- 001.e98 002.e98 003.e98 ...`.
//! `DIV=1.6` replaces `market::BID_DIVISOR`; `SPARE=0` leaves out the spare-cash factor of `5f77` (the two fit equally well);
//! `STATE=before` takes the bidders from the earlier save instead.
use rustfoot_core::e98;
use rustfoot_core::market::*;
use rustfoot_core::season::Club;
use rustfoot_core::sector::Slot;
use rustfoot_core::world::World;

/// `market::bid` with the divisor and the spare-cash factor of the environment.
fn bid_of(buyer: &Club, value: i64, slot: &Slot, e: f64, g: i64) -> i64 {
    let div: f64 = std::env::var("DIV")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(BID_DIVISOR);
    let spare = std::env::var("SPARE").as_deref() != Ok("0");
    if div == BID_DIVISOR && spare {
        return bid(buyer, value, slot, e, g);
    }
    let r = rating(buyer);
    if SHAPE[(r / 10).clamp(0, 5) as usize][slot.position as usize] as usize
        == count(buyer, slot.position)
    {
        return 0;
    }
    let cash = buyer.fin.cash as f64;
    let w = wage_sum(buyer);
    let mut x = market_value(slot, e, g) as f64 * need(buyer, slot);
    let ratio = (cash - x) / (w + 1 + value) as f64;
    if ratio < 3.0 {
        x = cash - 3.0 * (w + value) as f64;
    } else if ratio > 5.0 {
        x = x * ratio.ln() / div;
    }
    if spare && w > 100 && cash - x > 10.0 * w as f64 && cash - x > 1.0 {
        x *= ((cash - x) / w as f64 - 9.0).ln() / 10.0 + 1.0;
    }
    let a = (cash - x - 5.0 * w as f64).round_ties_even() as i64;
    a.min(x.round_ties_even() as i64).max(0)
}

fn main() -> Result<(), String> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.len() < 2 {
        return Err("usage: auctions first.e98 second.e98 [more saves in order]".into());
    }
    let (mut n, mut first, mut logs) = (0, 0, Vec::new());
    for pair in paths.windows(2) {
        let before = e98::parse(&std::fs::read(&pair[0]).map_err(|e| e.to_string())?)?;
        let after = e98::parse(&std::fs::read(&pair[1]).map_err(|e| e.to_string())?)?;
        let w = World::from_e98(1, &after)?;
        let w0 = World::from_e98(1, &before)?;
        // `STATE=before`: the bidders as the earlier save has them (exact for the auctions later in the same market, which
        // the save interrupts at the human's turn)
        let early = std::env::var("STATE").as_deref() == Ok("before");
        let (e, g) = (if early { before.economy } else { after.economy }, w.scale);
        let name_of = |s: &e98::E98, pid: u16| {
            s.teams
                .iter()
                .find(|t| t.players.iter().any(|p| p.id == pid))
                .map(|t| t.short_name.trim().to_string())
        };
        // the human coaches' clubs are asked in a dialog (the recorder bids nothing there)
        let humans: Vec<String> = after
            .teams
            .iter()
            .filter(|t| t.coach_human)
            .map(|t| t.short_name.trim().to_string())
            .collect();
        let clubs: Vec<&Club> = w.divisions.iter().flatten().chain(&w.pool).collect();
        let bidders: Vec<&Club> = if early {
            w0.divisions.iter().flatten().chain(&w0.pool).collect()
        } else {
            clubs.clone()
        };
        println!(
            "-- {} (round {} {}) -> {} (round {} {}): {} sales",
            pair[0],
            w0.round,
            w0.year,
            pair[1],
            w.round,
            w.year,
            after.transfers.len().saturating_sub(before.transfers.len())
        );
        for t in after.transfers.iter().skip(before.transfers.len()) {
            let (Some(buyer), Some(seller)) =
                (name_of(&after, t.player), name_of(&before, t.player))
            else {
                continue;
            };
            let Some(bc) = clubs.iter().find(|c| c.name == buyer) else {
                continue;
            };
            let pname = &after
                .teams
                .iter()
                .flat_map(|t| &t.players)
                .find(|p| p.id == t.player)
                .map_or("", |p| p.name.as_str());
            let Some(i) = bc.names.iter().position(|n| n == pname) else {
                continue;
            };
            let mut slot = bc.slots[i];
            slot.strength = t.strength;
            let ask = asking_price(&slot, e, g);
            // `2fec`: the wage the player asks, in the seller's economy
            let f = w0
                .divisions
                .iter()
                .flatten()
                .chain(&w0.pool)
                .find(|c| c.name == seller)
                .map_or(1.0, |c| c.fin.f);
            let value = rustfoot_core::economy::player_value(slot.strength, f, e, g);
            let mut bids = Vec::new();
            for c in &bidders {
                if c.name == seller || humans.contains(&c.name) {
                    continue;
                }
                let mut c = (*c).clone();
                if c.name == buyer && !early {
                    c.slots.remove(i);
                    c.names.remove(i);
                    c.fin.cash += t.fee;
                }
                let mut s = slot;
                s.foreign = country_class(c.country, s.nat);
                if !may_bid(&c, &s) || c.fin.cash < ask {
                    continue;
                }
                bids.push((floor_1000(bid_of(&c, value, &s, e, g)), c.name.clone()));
            }
            bids.sort_by_key(|x| std::cmp::Reverse(x.0));
            let rank = bids.iter().position(|x| x.1 == buyer);
            let own = rank.map_or(0, |r| bids[r].0);
            n += 1;
            first += (rank == Some(0)) as usize;
            if own > 0 {
                logs.push((t.fee as f64 / own as f64).ln());
            }
            println!(
                "{pname:<20} str {:>2} {seller:<14} -> {buyer:<14} fee {:>9} ask {:>9} buyer's bid {own:>9} (rank {rank:?})  best {:?}",
                slot.strength,
                t.fee,
                ask,
                &bids[..bids.len().min(3)]
            );
        }
    }
    let mean = logs.iter().sum::<f64>() / logs.len().max(1) as f64;
    let sd =
        (logs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / logs.len().max(1) as f64).sqrt();
    println!(
        "{n} auctions: the buyer has the best bid in {first}; fee / buyer's bid {:.3} (geometric mean), spread {:.1} %",
        mean.exp(),
        sd * 100.0
    );
    Ok(())
}
