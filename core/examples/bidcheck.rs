use elifoot_core::e98;
use elifoot_core::market::*;
use elifoot_core::season::Club;
use elifoot_core::sector::{Slot, Status};

fn club(t: &e98::E98Team, skip: Option<&str>) -> Club {
    let mut slots = Vec::new();
    let mut names = Vec::new();
    for p in &t.players {
        if Some(p.name.as_str()) == skip {
            continue;
        }
        let mut s = Slot::new(
            &p.name,
            p.position,
            p.strength,
            p.nationality == t.country,
            Status::Out,
        );
        s.nat = code(&p.nationality);
        s.wage = p.wage;
        s.fresh = p.fresh;
        s.foreign = country_class(code(&t.country), s.nat);
        slots.push(s);
        names.push(p.name.clone());
    }
    let mut c = Club::new(&t.short_name, slots);
    c.names = names;
    c.country = code(&t.country);
    c.fin.cash = t.cash;
    c
}

fn main() {
    let a = e98::parse(&std::fs::read("original/saves/snap2.e98").unwrap()).unwrap();
    let (e, g) = (2.0054, 5);
    // (player id, seller, buyer, fee)
    for (pid, seller, buyer_name, fee) in [
        (272u16, "PALMEIRAS", "SÃO PAULO", 5_171_000i64),
        (714, "CEARÁ", "PALMEIRAS", 592_000),
        (657, "SPORT", "PALMEIRAS", 1_352_000),
    ] {
        // the player as seen in his new club
        let (nt, pl) = a
            .teams
            .iter()
            .find_map(|t| {
                t.players
                    .iter()
                    .find(|p| p.id == pid)
                    .map(|p| (t, p.clone()))
            })
            .unwrap();
        let mut slot = Slot::new(&pl.name, pl.position, pl.strength, false, Status::Out);
        slot.nat = code(&pl.nationality);
        let v = elifoot_core::economy::player_value(pl.strength, 1.0, e, g);
        println!(
            "== {} ({} -> {}) fee {} ask {}",
            pl.name,
            seller,
            nt.short_name,
            fee,
            floor_1000(market_value(&slot, e, g))
        );
        let mut results = Vec::new();
        for t in &a.teams {
            if t.short_name.trim() == seller || t.players.is_empty() {
                continue;
            }
            let skip = if t.id == nt.id {
                Some(pl.name.as_str())
            } else {
                None
            };
            let mut c = club(t, skip);
            if t.id == nt.id && buyer_name == nt.short_name.trim() {
                c.fin.cash = t.cash + fee;
            }
            if !may_bid(&c, &slot) {
                eprintln!(
                    "  no-bid {} squad {} foreign {}",
                    t.short_name,
                    c.slots.len(),
                    foreigners(&c)
                );
                continue;
            }
            if c.fin.cash < floor_1000(market_value(&slot, e, g)) {
                continue;
            }
            let b = floor_1000(bid(&c, v, &slot, e, g));
            results.push((
                b,
                t.short_name.clone(),
                c.fin.cash,
                wage_sum(&c),
                rating(&c),
            ));
        }
        results.sort_by_key(|x| std::cmp::Reverse(x.0));
        for r in results.iter().take(4) {
            println!(
                "   {:>10} {:<14} cash {:>10} wages {:>7} rating {}",
                r.0, r.1, r.2, r.3, r.4
            );
        }
    }
}
