//! `cargo run --release -p rustfoot-core --example world -- [seasons]`: mean strength per division over the years.
use rustfoot_core::world::{World, DIVISION_NAMES};

fn main() {
    let seasons: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let mut w = World::synthetic(1998);
    let mean = |w: &World| -> Vec<f64> {
        w.divisions
            .iter()
            .map(|d| {
                d.iter()
                    .flat_map(|c| c.slots.iter())
                    .map(|s| s.strength as f64)
                    .sum::<f64>()
                    / (d.len() * 20) as f64
            })
            .collect()
    };
    println!("{:>5} {}", "year", DIVISION_NAMES.join("   "));
    println!(
        "{:>5} {}",
        w.year,
        mean(&w)
            .iter()
            .map(|m| format!("{m:>9.1}"))
            .collect::<Vec<_>>()
            .join("   ")
    );
    for _ in 0..seasons {
        w.play_season();
        println!(
            "{:>5} {}",
            w.year,
            mean(&w)
                .iter()
                .map(|m| format!("{m:>9.1}"))
                .collect::<Vec<_>>()
                .join("   ")
        );
    }
}
