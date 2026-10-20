//! `cargo run --release -p rustfoot-core --example sim` — match statistics for a few strength pairings.
use rustfoot_core::matchsim::{fixture_value, EventKind, Match};
use rustfoot_core::rng::DelphiRng;
use rustfoot_core::sector::{Slot, Status};

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
            // per-fixture bonus through the real c95c formula, with random strings, domestic league match
            let (a, h, v) = (name(&mut rng), name(&mut rng), name(&mut rng));
            let hb = fixture_value(a.as_bytes(), h.as_bytes(), true) + 2;
            let ab = fixture_value(a.as_bytes(), v.as_bytes(), true);
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
