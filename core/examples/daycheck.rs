//! Checks the port's money rules against two consecutive saves of the original (one day apart, see `tools/capture/days.sh`):
//! `cargo run --release -p rustfoot-core --example daycheck -- before.e98 after.e98`. Per club: the ledger's wage change against the
//! port's wage bill of the squad before the day, and for each home game the ticket price the ledger implies (gate change over the
//! attendance stored in the fixture list) against the port's price of the division.
use rustfoot_core::e98;
use rustfoot_core::finance;
use rustfoot_core::world::World;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = &args[..] else {
        return Err("usage: daycheck before.e98 after.e98".into());
    };
    let before = e98::parse(&std::fs::read(a).map_err(|e| e.to_string())?)?;
    let after = e98::parse(&std::fs::read(b).map_err(|e| e.to_string())?)?;
    let w = World::from_e98(1, &before)?;
    let w1 = World::from_e98(1, &after)?;
    // the original fixes the next round's price in `6e48`: after the F update, before the evolution of the squads
    for d in 0..4 {
        let n = w.divisions[d].len().max(1) as f64;
        let avg_f = w1.divisions[d].iter().map(|c| c.fin.f).sum::<f64>() / n;
        let r0 = (w.divisions[d]
            .iter()
            .map(|c| finance::rating(&c.slots))
            .sum::<i64>() as f64
            / n)
            .round_ties_even() as i64;
        let r1 = (w1.divisions[d]
            .iter()
            .map(|c| finance::rating(&c.slots))
            .sum::<i64>() as f64
            / n)
            .round_ties_even() as i64;
        println!(
            "next round, division {}: F after {:.5}  price with ratings before {:.1} / after {:.1}  (port now {:.1})",
            d + 1,
            avg_f,
            finance::ticket_price(avg_f, r0, w1.economy, w1.scale),
            finance::ticket_price(avg_f, r1, w1.economy, w1.scale),
            w1.money_ctx(d).price
        );
    }
    println!("E {:.4} G {}", w.economy, w.scale);
    {
        // the cup's clubs (the draw list of the save): their mean F and rounded mean rating
        let clubs: Vec<&rustfoot_core::season::Club> = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .filter(|c| w.cup.contains(&c.name))
            .collect();
        let n = clubs.len().max(1) as f64;
        let f = clubs.iter().map(|c| c.fin.f).sum::<f64>() / n;
        let r = clubs.iter().map(|c| finance::rating(&c.slots)).sum::<i64>() as f64 / n;
        let x = w.economy * r.round_ties_even() / (2.0 * f.sqrt());
        println!(
            "cup: {} clubs, mean F {:.5}, mean rating {:.3} -> x/2 {:.3}, x1.5 {:.3}",
            clubs.len(),
            f,
            r,
            x,
            1.5 * x
        );
    }
    let (mut wage_ok, mut wage_n) = (0, 0);
    for d in 0..4 {
        let ctx = w.money_ctx(d);
        let clubs = &w.divisions[d];
        let n = clubs.len() as f64;
        let avg_f = clubs.iter().map(|c| c.fin.f).sum::<f64>() / n;
        let ratings: Vec<i64> = clubs.iter().map(|c| finance::rating(&c.slots)).collect();
        let mean_strength: Vec<f64> = clubs
            .iter()
            .map(|c| {
                c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len().max(1) as f64
            })
            .collect();
        println!(
            "-- division {} price {:.1}  avgF {:.5} ratings {:?} (mean {:.3}) raw strengths {:?}",
            d + 1,
            ctx.price,
            avg_f,
            ratings,
            ratings.iter().sum::<i64>() as f64 / n,
            mean_strength
                .iter()
                .map(|v| format!("{v:.2}"))
                .collect::<Vec<_>>()
        );
        let top = |k: usize| -> f64 {
            clubs
                .iter()
                .map(|c| {
                    let mut v: Vec<f64> = c.slots.iter().map(|s| s.strength as f64).collect();
                    v.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    v.truncate(k);
                    v.iter().sum::<f64>() / v.len().max(1) as f64
                })
                .sum::<f64>()
                / n
        };
        let levels: Vec<u8> = clubs
            .iter()
            .filter_map(|c| {
                before
                    .teams
                    .iter()
                    .find(|t| t.short_name == c.name)
                    .map(|t| t.level)
            })
            .collect();
        let sizes: Vec<usize> = clubs.iter().map(|c| c.slots.len()).collect();
        println!(
            "   top11 {:.3} top14 {:.3} top16 {:.3} levels {:?} sizes {:?}",
            top(11),
            top(14),
            top(16),
            levels,
            sizes
        );
        let by_status = |want: &[u8]| -> (f64, Vec<String>) {
            let per: Vec<f64> = clubs
                .iter()
                .filter_map(|c| before.teams.iter().find(|t| t.short_name == c.name))
                .map(|t| {
                    let v: Vec<f64> = t
                        .players
                        .iter()
                        .filter(|p| want.contains(&p.status))
                        .map(|p| p.strength as f64)
                        .collect();
                    v.iter().sum::<f64>() / v.len().max(1) as f64
                })
                .collect();
            (
                per.iter().sum::<f64>() / n,
                per.iter().map(|v| format!("{v:.2}")).collect(),
            )
        };
        println!("   starters {:?}", by_status(&[1]));
        println!("   starters+subs {:?}", by_status(&[1, 2]));
        {
            // variants: injured / suspended players out of the mean, or counted as 0
            let teams: Vec<&e98::E98Team> = clubs
                .iter()
                .filter_map(|c| before.teams.iter().find(|t| t.short_name == c.name))
                .collect();
            let var = |zero: bool| -> Vec<i64> {
                teams
                    .iter()
                    .map(|t| {
                        let avail = |p: &&e98::E98Player| p.injured == 0 && p.ban == 0;
                        let (sum, cnt) = if zero {
                            (
                                t.players
                                    .iter()
                                    .filter(avail)
                                    .map(|p| p.strength as f64)
                                    .sum::<f64>(),
                                t.players.len(),
                            )
                        } else {
                            let v: Vec<f64> = t
                                .players
                                .iter()
                                .filter(avail)
                                .map(|p| p.strength as f64)
                                .collect();
                            (v.iter().sum::<f64>(), v.len())
                        };
                        (sum / cnt.max(1) as f64).round_ties_even() as i64
                    })
                    .collect()
            };
            let (a, z) = (var(false), var(true));
            println!(
                "   VAR d{} available-only {:?} mean {:.3}; unavailable as 0 {:?} mean {:.3}",
                d + 1,
                a,
                a.iter().sum::<i64>() as f64 / n,
                z,
                z.iter().sum::<i64>() as f64 / n
            );
            let rr = ratings.iter().sum::<i64>() as f64 / n;
            let raw = mean_strength.iter().sum::<f64>() / n;
            let k = w.scale as f64 * w.economy / avg_f.max(0.1).sqrt() / 5.0;
            println!(
                "   FIT d{} E {:.6} avgF {:.6} meanRounded {:.4} meanRaw {:.4} -> x/2 with meanRounded {:.4} with meanRaw {:.4}",
                d + 1, w.economy, avg_f, rr, raw, k * rr / 2.0, k * raw / 2.0
            );
        }
        for c in &w.divisions[d] {
            let (Some(t0), Some(t1)) = (
                before.teams.iter().find(|t| t.short_name == c.name),
                after.teams.iter().find(|t| t.short_name == c.name),
            ) else {
                continue;
            };
            let wages = t1.ledger[4] - t0.ledger[4];
            let bill = finance::wage_bill(&c.slots);
            let games = t1.calendar.iter().filter(|f| f.goals_a >= 0).count()
                - t0.calendar.iter().filter(|f| f.goals_a >= 0).count();
            wage_n += 1;
            if wages == -bill * games as i64 {
                wage_ok += 1;
            }
            // the home games of the day: their stored attendance
            let home: Vec<&e98::E98Fixture> = t1
                .calendar
                .iter()
                .zip(t0.calendar.iter())
                .filter(|(f1, f0)| f1.goals_a >= 0 && f0.goals_a < 0 && f1.venue == 0)
                .map(|(f1, _)| f1)
                .collect();
            let gate = t1.ledger[0] - t0.ledger[0];
            let att: i64 = home.iter().map(|f| f.gate).sum();
            let implied = if att > 0 {
                gate as f64 / att as f64
            } else {
                0.0
            };
            let cash = t1.cash - t0.cash;
            let booked: i64 = (0..7).map(|i| t1.ledger[i] - t0.ledger[i]).sum();
            println!(
                "{:<22} games {} wages {:>9} bill {:>9} {}  gate {:>9} att {:>6} price {:>7.2}  cash {:>10} booked {:>10}{}",
                c.name,
                games,
                wages,
                -bill * games as i64,
                if wages == -bill * games as i64 { "ok" } else { "XX" },
                gate,
                att,
                implied,
                cash,
                booked,
                if cash == booked { "" } else { "  (cash differs)" }
            );
        }
    }
    println!("wages exact for {wage_ok} of {wage_n} clubs");
    Ok(())
}
