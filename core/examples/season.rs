//! `cargo run --release -p elifoot-core --example season -- [base] [seasons]`
//! Plays whole 8-team seasons (14 rounds, evolution after each round). `base` is the division base level
//! (`division+4`, source not decoded yet; default 12). Points are 3-1-0 and the line-up is a plain 4-4-2 (both assumed).
use elifoot_core::rng::DelphiRng;
use elifoot_core::season::{end_of_round, play_round, standings, Club};
use elifoot_core::sector::{Slot, Status};

fn name(rng: &mut DelphiRng) -> String {
    (0..5 + rng.random(8))
        .map(|_| (b'a' + rng.random(26) as u8) as char)
        .collect()
}

fn club(label: &str, strength: u16, rng: &mut DelphiRng) -> Club {
    let shape = [0u8, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3];
    let slots = shape
        .iter()
        .map(|&pos| {
            let s = (strength as i32 + rng.random(5) as i32 - 2).clamp(1, 50) as u16;
            Slot::new(&name(rng), pos, s, true, Status::Out)
        })
        .collect();
    Club::new(label, slots)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(12);
    let seasons: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let mut rng = DelphiRng::new(98);
    // club strengths from strong to weak
    let levels = [9u16, 8, 7, 6, 6, 5, 4, 3];
    let mut champion = [0u32; 8];
    let (mut first_avg, mut last_avg) = ([0f64; 8], [0f64; 8]);
    for season in 0..seasons {
        let mut clubs: Vec<Club> = levels
            .iter()
            .enumerate()
            .map(|(i, &l)| club(&format!("C{i}"), l, &mut rng))
            .collect();
        let start: Vec<f64> = clubs
            .iter()
            .map(|c| c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len() as f64)
            .collect();
        for round in 1..=14 {
            let mut seed = DelphiRng::new(season as u32 * 31 + round as u32);
            let mut names = move |_k: usize| {
                let n = |r: &mut DelphiRng| name(r).into_bytes();
                (n(&mut seed), n(&mut seed), n(&mut seed))
            };
            play_round(&mut rng, &mut clubs, round, &mut names);
            end_of_round(&mut rng, &mut clubs, base);
        }
        let table = standings(&clubs);
        champion[table[0]] += 1;
        for (i, c) in clubs.iter().enumerate() {
            first_avg[i] += start[i];
            last_avg[i] +=
                c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len() as f64;
        }
        if season == 0 {
            println!("season 1 table (base {base}):");
            for (pos, &i) in table.iter().enumerate() {
                let c = &clubs[i];
                println!(
                    "{:>2}. {} start-strength {:.1}  pts {:>2}  gf {:>2}  ga {:>2}",
                    pos + 1,
                    c.name,
                    start[i],
                    c.points,
                    c.goals_for,
                    c.goals_against
                );
            }
        }
    }
    println!("\nafter {seasons} seasons: titles and mean squad strength start -> end");
    for i in 0..8 {
        println!(
            "C{i} (level {}): {:>5.1}% titles   {:.2} -> {:.2}",
            levels[i],
            100.0 * champion[i] as f64 / seasons as f64,
            first_avg[i] / seasons as f64,
            last_avg[i] / seasons as f64
        );
    }
}
