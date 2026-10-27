//! The money of a season change, from two saves of the original on either side of it (see `tools/capture/days.sh`):
//! `cargo run --release -p rustfoot-core --example seasonend -- before.e98 after.e98`. The original clears the ledger when a season
//! ends, so per club the cash change less the new season's ledger is what the season change paid (plus the old season's games played
//! after the first save, shown by their count).
use rustfoot_core::e98;

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [a, b] = &args[..] else {
        return Err("usage: seasonend before.e98 after.e98".into());
    };
    let before = e98::parse(&std::fs::read(a).map_err(|e| e.to_string())?)?;
    let after = e98::parse(&std::fs::read(b).map_err(|e| e.to_string())?)?;
    println!(
        "years {} -> {}, E {:.5} -> {:.5}, winners {:?}",
        before.year,
        after.year,
        before.economy,
        after.economy,
        after.winners.last()
    );
    let played = |t: &e98::E98Team| t.calendar.iter().filter(|f| f.goals_a >= 0).count();
    let (mut ok, mut n) = (0, 0);
    let div_of = |s: &e98::E98, id: u16| s.divisions.iter().position(|(_, ids)| ids.contains(&id));
    for (d, (_, ids)) in before.divisions.iter().enumerate() {
        println!("-- division {}", d + 1);
        let mut rows: Vec<&e98::E98Team> = ids
            .iter()
            .filter_map(|id| before.teams.iter().find(|t| t.id == *id))
            .collect();
        rows.sort_by_key(|t| {
            std::cmp::Reverse((t.points, t.goals_for as i32 - t.goals_against as i32))
        });
        for (pos, t0) in rows.iter().enumerate() {
            let Some(t1) = after.teams.iter().find(|t| t.short_name == t0.short_name) else {
                continue;
            };
            let rest = t1.cash - t0.cash - t1.ledger.iter().sum::<i64>();
            // what the season change itself added to the old season's ledger (its last round, the prizes)
            let last: Vec<i64> = (0..7).map(|i| t1.past[0][i] - t0.ledger[i]).collect();
            n += 1;
            if rest == last.iter().sum::<i64>() && t1.past[1] == t0.past[0] {
                ok += 1;
            } else {
                println!("   ** the cash change is not the kept ledgers' change");
            }
            println!(
                "   last season's ledger kept {:?} (old season after the first save {:?}); the one before {:?} (was {:?})",
                t1.past[0], last, t1.past[1], t0.past[0]
            );
            println!(
                "{:>2} {:<20} pts {:>2} played {:>2} -> {:>2}  div {:?}  cash {:>10} -> {:>10}  ledger after {:?}  rest {:>10}  loan {} -> {}  squad {} -> {}  wages {} -> {}",
                pos + 1,
                t0.short_name,
                t0.points,
                played(t0),
                played(t1),
                div_of(&after, t1.id).map(|x| x + 1),
                t0.cash,
                t1.cash,
                t1.ledger,
                rest,
                t0.loan,
                t1.loan,
                t0.players.len(),
                t1.players.len(),
                t0.players.iter().map(|p| p.wage).sum::<i64>(),
                t1.players.iter().map(|p| p.wage).sum::<i64>(),
            );
        }
    }
    println!("{ok} of {n} clubs: cash change = last season's ledger change + the new ledger, the older ledger moved down");
    Ok(())
}
