//! Main chance generator, a port of `1010:46df` (see `docs/match-engine.md`).
//! Uses `f64`; the original used x87 extended precision (differences are far below one `Random` step).
use crate::rng::DelphiRng;
use crate::sector::Sectors;

/// Per-side match state used by the generator.
#[derive(Clone, Copy, Debug)]
pub struct SideState {
    pub sectors: Sectors,
    /// `team+0x4f` (shots / chances so far).
    pub shots: i32,
    /// `team+0x59` (goals).
    pub goals: i32,
    /// `team+0x5b`, a Real48 multiplier; 1.0 at the start.
    pub k: f64,
}

/// `T = (K/2)^(1/4) * ln(1 + R) * S` with `R = 4*ownAtt / (oppDef + 1)`.
pub fn threshold(own: &Sectors, opp: &Sectors, k: f64) -> f64 {
    let (o_def, o_mid, o_att) = (own.def as f64, own.mid as f64, own.att as f64);
    let (p_gk, p_def, p_mid, p_att) = (
        opp.gk as f64,
        opp.def as f64,
        opp.mid as f64,
        opp.att as f64,
    );

    let own_att = o_att + o_mid / 2.0;
    let opp_def = 3.0 * p_gk + p_def + p_mid / 2.0;
    let r = 4.0 * own_att / (opp_def + 1.0);

    // 45b8: running sums that are NOT reset between the three lines; both start at 1.
    let (mut lo, mut hi, mut s) = (1.0f64, 1.0f64, 0.0f64);
    lo += o_def;
    hi += p_att;
    s += lo / (lo + hi + 1.0);
    lo += o_mid;
    hi += p_mid;
    s += 2.0 * lo / (lo + hi + 1.0);
    lo += o_att;
    hi += 3.0 * p_gk + p_def;
    s += lo / (lo + hi + 1.0);

    let x = (r + 1.0).ln() * s;
    if x <= 0.0 {
        return 0.0; // Ln of a non-positive value is a runtime error in the original; no chance here
    }
    (k / 2.0).sqrt().sqrt() * x
}

/// Stages 1 and 2 of the generator. The `Random` draws happen in exactly the original order.
/// Returns whether a chance is created (the original then also needs a shooter, decided elsewhere).
pub fn chance(
    rng: &mut DelphiRng,
    own: &SideState,
    opp: &Sectors,
    home: bool,
    minute: u16,
) -> bool {
    let (a, b) = if home { (90, 180) } else { (120, 270) };
    let t = threshold(&own.sectors, opp, own.k);
    let candidate =
        (rng.random(a) as f64) < t - (own.shots + own.goals) as f64 || rng.random(b) == 0;
    // warm-up gate: Random(45) < minute (signed compare in the original)
    candidate && (rng.random(45) as i32) < minute as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn even(v: u16) -> Sectors {
        Sectors {
            gk: v,
            def: v * 4,
            mid: v * 4,
            att: v * 3,
        }
    }

    #[test]
    fn stronger_attack_means_higher_threshold() {
        let weak = even(10);
        let strong = even(30);
        assert!(threshold(&strong, &weak, 1.0) > threshold(&weak, &weak, 1.0));
        assert!(threshold(&weak, &strong, 1.0) < threshold(&weak, &weak, 1.0));
    }

    #[test]
    fn zero_attack_gives_no_threshold() {
        let none = Sectors::default();
        assert_eq!(threshold(&none, &even(10), 1.0), 0.0);
    }

    #[test]
    fn home_side_creates_more_chances() {
        let side = SideState {
            sectors: even(20),
            shots: 0,
            goals: 0,
            k: 1.0,
        };
        let opp = even(20);
        let (mut h, mut a) = (0, 0);
        let mut rng = DelphiRng::new(2026);
        for i in 0..200_000u32 {
            let minute = 60 + (i % 30) as u16;
            h += chance(&mut rng, &side, &opp, true, minute) as u32;
            a += chance(&mut rng, &side, &opp, false, minute) as u32;
        }
        assert!(h > a, "home {h} away {a}");
    }

    #[test]
    fn early_minutes_are_suppressed() {
        let side = SideState {
            sectors: even(20),
            shots: 0,
            goals: 0,
            k: 1.0,
        };
        let opp = even(20);
        let mut rng = DelphiRng::new(7);
        let early: u32 = (0..100_000)
            .map(|_| chance(&mut rng, &side, &opp, true, 1) as u32)
            .sum();
        let late: u32 = (0..100_000)
            .map(|_| chance(&mut rng, &side, &opp, true, 80) as u32)
            .sum();
        assert!(early * 10 < late, "early {early} late {late}");
    }

    #[test]
    fn shots_and_goals_dampen_the_threshold() {
        let mut side = SideState {
            sectors: even(20),
            shots: 0,
            goals: 0,
            k: 1.0,
        };
        let opp = even(20);
        let count = |side: &SideState| {
            let mut rng = DelphiRng::new(99);
            (0..100_000)
                .map(|_| chance(&mut rng, side, &opp, true, 90) as u32)
                .sum::<u32>()
        };
        let fresh = count(&side);
        side.shots = 12;
        side.goals = 3;
        assert!(count(&side) < fresh);
    }
}
