//! `cargo run --release -p elifoot-core --example sim` — match statistics for a few strength pairings.
use elifoot_core::matchsim::{EventKind, Match};
use elifoot_core::rng::DelphiRng;
use elifoot_core::sector::{Slot, Status};

fn name(rng: &mut DelphiRng) -> String {
    (0..5 + rng.random(8))
        .map(|_| (b'a' + rng.random(26) as u8) as char)
        .collect()
}

/// 4-4-2 starters plus a bench of one goalkeeper and four outfield players.
fn squad(strength: u16, rng: &mut DelphiRng) -> Vec<Slot> {
    let mut v = Vec::new();
    for &pos in &[0u8, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3] {
        v.push(Slot::new(&name(rng), pos, strength, true, Status::Starter));
    }
    for &pos in &[0u8, 1, 2, 3, 3] {
        v.push(Slot::new(
            &name(rng),
            pos,
            strength.saturating_sub(1).max(1),
            true,
            Status::Sub,
        ));
    }
    v
}

fn main() {
    println!(
        "{:>8} {:>8} {:>7} {:>7} {:>7} {:>6} {:>7} {:>8} {:>6} {:>6}",
        "home str",
        "away str",
        "home g",
        "away g",
        "home W",
        "draw",
        "away W",
        "total g",
        "reds",
        "injur"
    );
    for (hs, as_) in [(6, 6), (3, 3), (9, 9), (8, 5), (5, 8), (10, 3)] {
        let mut rng = DelphiRng::new(1998);
        let n = 100_000;
        let (mut hg, mut ag, mut hw, mut dr, mut aw, mut reds, mut inj) =
            (0u64, 0u64, 0u32, 0u32, 0u32, 0u64, 0u64);
        for _ in 0..n {
            // c95c is not decoded yet: approximate it with a uniform 0..=10 per side (+2 for the home side)
            let (hb, ab) = (rng.random(11) as i32 + 2, rng.random(11) as i32);
            let mut m = Match::new(squad(hs, &mut rng), squad(as_, &mut rng), hb, ab);
            m.play(&mut rng);
            let (x, y) = m.score();
            hg += x as u64;
            ag += y as u64;
            match x.cmp(&y) {
                std::cmp::Ordering::Greater => hw += 1,
                std::cmp::Ordering::Equal => dr += 1,
                _ => aw += 1,
            }
            reds += m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::RedCard)
                .count() as u64;
            inj += m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Injury)
                .count() as u64;
        }
        let f = n as f64;
        println!(
            "{:>8} {:>8} {:>7.2} {:>7.2} {:>6.1}% {:>5.1}% {:>6.1}% {:>8.2} {:>6.2} {:>6.2}",
            hs,
            as_,
            hg as f64 / f,
            ag as f64 / f,
            100.0 * hw as f64 / f,
            100.0 * dr as f64 / f,
            100.0 * aw as f64 / f,
            (hg + ag) as f64 / f,
            reds as f64 / f,
            inj as f64 / f
        );
    }
}
