//! Post-match strength evolution, a port of `1010:4a41`.
use crate::rng::DelphiRng;
use crate::sector::Slot;

/// `rating` is +1 (improve), 0 (stay) or -1 (decline), as produced by `1068:1079`.
pub fn evolve(
    rng: &mut DelphiRng,
    strength: u8,
    goalkeeper: bool,
    rating: i8,
    team_level: i32,
) -> u8 {
    let mut s = strength as i32;
    match rating {
        1 => {
            if (!goalkeeper || rng.random(2) == 0) && s < 50 {
                s += 1;
                if team_level + 5 < s || s > 50 {
                    s -= 1;
                }
            }
        }
        -1 => {
            // the original draws Random(2) first and checks the floor afterwards
            let hit = rng.random(2) == 0;
            if hit && s > 1 {
                s -= 1;
                if s < team_level - 5 || s < 1 {
                    s += 1;
                }
            }
        }
        _ => {}
    }
    s as u8
}

/// `1068:1079`: the sign of the rating accumulator (`slot+0xc`).
pub fn rating_sign(rating: i32) -> i8 {
    rating.signum() as i8
}

/// `1010:4a41`: end-of-season evolution of a whole squad (every slot, starters and bench).
pub fn evolve_squad(rng: &mut DelphiRng, slots: &mut [Slot], team_level: i32) {
    for p in slots.iter_mut() {
        p.strength = evolve(
            rng,
            p.strength.min(255) as u8,
            p.position == 0,
            rating_sign(p.rating),
            team_level,
        ) as u16;
    }
}

/// `1010:6fb8`: the clamp level of the team at `rank` (0 = best) in a division with base level `base`.
pub fn level_for_rank(base: i32, rank: usize) -> i32 {
    base - 2 * rank as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_exceeds_cap_or_level_window() {
        let mut rng = DelphiRng::new(3);
        assert_eq!(evolve(&mut rng, 50, false, 1, 20), 50);
        assert_eq!(evolve(&mut rng, 12, false, 1, 5), 12); // 5 + 5 < 13
        assert_eq!(evolve(&mut rng, 10, false, 1, 5), 10); // 10 -> 11 exceeds level+5 = 10, so it is undone
        assert_eq!(evolve(&mut rng, 9, false, 1, 5), 10); // 9 -> 10 is within the window
    }

    #[test]
    fn goalkeepers_improve_half_as_often() {
        let (mut f, mut g) = (0, 0);
        let mut rng = DelphiRng::new(4);
        for _ in 0..20_000 {
            f += (evolve(&mut rng, 5, false, 1, 10) == 6) as u32;
            g += (evolve(&mut rng, 5, true, 1, 10) == 6) as u32;
        }
        assert_eq!(f, 20_000);
        assert!((9_000..11_000).contains(&g), "{g}");
    }

    #[test]
    fn decline_always_consumes_one_draw() {
        // strength 1 cannot decline, but the original still draws Random(2) first
        let (mut a, mut b) = (DelphiRng::new(5), DelphiRng::new(5));
        evolve(&mut a, 1, false, -1, 1);
        b.random(2);
        assert_eq!(a.seed(), b.seed());
    }

    #[test]
    fn decline_is_floored() {
        let mut rng = DelphiRng::new(8);
        for _ in 0..1_000 {
            assert_eq!(evolve(&mut rng, 1, false, -1, 1), 1);
        }
        let low = (0..1_000)
            .map(|_| evolve(&mut rng, 5, false, -1, 10))
            .min()
            .unwrap();
        assert_eq!(low, 5); // 4 < 10-5 would revert
    }
}
