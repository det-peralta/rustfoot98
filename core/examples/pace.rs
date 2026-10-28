//! The computer market's pace from an original save: plays `ROUNDS` rounds (cup days included) with several seeds and prints
//! the sales per day: `cargo run --release -p rustfoot-core --example pace -- save.e98 [rounds] [seeds]`.
use rustfoot_core::e98;
use rustfoot_core::world::World;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: pace save.e98 [rounds] [seeds]")?;
    let rounds: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(14);
    let seeds: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(20);
    let save = e98::parse(&std::fs::read(&path).map_err(|e| e.to_string())?)?;
    let (mut sales, mut days) = (0usize, 0usize);
    for seed in 0..seeds {
        let mut w = World::from_e98(seed + 1, &save)?;
        for _ in 0..rounds {
            let before = w.transfers.len();
            let cup = w.round % 2 == 1; // an even round follows: its cup day too
            w.play_round();
            sales += w.transfers.len().saturating_sub(before);
            days += 1 + cup as usize;
        }
    }
    println!(
        "{seeds} runs of {rounds} rounds: {:.2} sales per round, {:.2} per match day",
        sales as f64 / (seeds as usize * rounds) as f64,
        sales as f64 / days as f64
    );
    Ok(())
}
