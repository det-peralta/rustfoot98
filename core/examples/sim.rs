//! `cargo run --release -p elifoot-core --example sim` — match statistics for a few strength pairings.
use elifoot_core::matchsim::Match;
use elifoot_core::rng::DelphiRng;
use elifoot_core::sector::Slot;

fn squad(strength: u16, rng: &mut DelphiRng) -> Vec<Slot> {
    [0u8, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3]
        .iter()
        .map(|&position| Slot {
            starter: true,
            position,
            strength,
            g: rng.random(10) as u8 + 1,
            same_country: true,
        })
        .collect()
}

fn main() {
    println!(
        "{:>9} {:>9} {:>8} {:>8} {:>7} {:>7} {:>7} {:>8}",
        "home str", "away str", "home g", "away g", "home W", "draw", "away W", "total g"
    );
    for (hs, as_) in [(6, 6), (3, 3), (9, 9), (8, 5), (5, 8), (10, 3)] {
        let mut rng = DelphiRng::new(1998);
        let n = 100_000;
        let (mut hg, mut ag, mut hw, mut dr, mut aw) = (0u64, 0u64, 0u32, 0u32, 0u32);
        for _ in 0..n {
            let mut m = Match::new(squad(hs, &mut rng), squad(as_, &mut rng), 2, 0);
            m.play(&mut rng);
            let (x, y) = m.score();
            hg += x as u64;
            ag += y as u64;
            match x.cmp(&y) {
                std::cmp::Ordering::Greater => hw += 1,
                std::cmp::Ordering::Equal => dr += 1,
                _ => aw += 1,
            }
        }
        let f = n as f64;
        println!(
            "{:>9} {:>9} {:>8.2} {:>8.2} {:>6.1}% {:>6.1}% {:>6.1}% {:>8.2}",
            hs,
            as_,
            hg as f64 / f,
            ag as f64 / f,
            100.0 * hw as f64 / f,
            100.0 * dr as f64 / f,
            100.0 * aw as f64 / f,
            (hg + ag) as f64 / f
        );
    }
}
