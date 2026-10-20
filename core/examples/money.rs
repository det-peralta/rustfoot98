//! Prints the first rounds of money for a synthetic world: `cargo run -p rustfoot-core --example money`.
use rustfoot_core::world::World;

fn main() {
    let mut w = World::synthetic(1998);
    println!("E {:.3} G {}", w.economy, w.scale);
    for round in 1..=3 {
        w.play_round();
        println!("-- round {round}");
        for d in 0..4 {
            let c = &w.divisions[d][0];
            let gates: Vec<i64> = w
                .last_round
                .iter()
                .filter(|(dd, _)| *dd == d)
                .map(|(_, r)| r.gate)
                .collect();
            let atts: Vec<i64> = w
                .last_round
                .iter()
                .filter(|(dd, _)| *dd == d)
                .map(|(_, r)| r.attendance)
                .collect();
            println!(
                "D{} price {:.1} F {:.4} {:>14} cash {:>10} wages {:>9} cap {:>6}  gates {:?} att {:?}",
                d + 1, w.money_ctx(d).price, c.fin.f, c.name, c.fin.cash, c.fin.wages, c.fin.capacity(), gates, atts
            );
        }
    }
}
