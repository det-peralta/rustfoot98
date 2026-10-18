//! Transfer market over several seasons: `cargo run -p elifoot-core --release --example market`.
use elifoot_core::world::World;

fn main() {
    let mut w = World::synthetic(1998);
    for season in 0..12 {
        let mut per_round = Vec::new();
        for _ in 0..14 {
            let before = w.transfers.len();
            w.play_round();
            per_round.push(w.transfers.len().saturating_sub(before));
            if w.round == 0 {
                break;
            }
        }
        let n = w.transfers.len();
        let fee: i64 = w.transfers.iter().map(|t| t.fee).sum();
        let fresh = w
            .divisions
            .iter()
            .flatten()
            .flat_map(|c| &c.slots)
            .filter(|s| s.fresh)
            .count();
        let total = w
            .divisions
            .iter()
            .flatten()
            .map(|c| c.slots.len())
            .sum::<usize>();
        let mean: Vec<String> = (0..4)
            .map(|d| {
                format!(
                    "{:.1}",
                    w.divisions[d]
                        .iter()
                        .flat_map(|c| &c.slots)
                        .map(|s| s.strength as f64)
                        .sum::<f64>()
                        / w.divisions[d].iter().map(|c| c.slots.len()).sum::<usize>() as f64
                )
            })
            .collect();
        let cash: Vec<String> = (0..4)
            .map(|d| {
                format!(
                    "{}",
                    w.divisions[d].iter().map(|c| c.fin.cash).sum::<i64>() / 8 / 1000
                )
            })
            .collect();
        let st: Vec<u32> = (0..4)
            .map(|d| w.divisions[d].iter().map(|c| c.fin.stadium as u32).sum())
            .collect();
        println!("stadium levels {st:?}");
        println!("season {season:>2} ({}): transfers {n:>3} fees {fee:>11} fresh {fresh}/{total} per-round {per_round:?} mean str {mean:?} avg cash(k) {cash:?}", w.year);
        w.transfers.clear();
    }
}
