//! Post-match strength evolution, a port of `1010:4a41`.
use crate::rng::DelphiRng;

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
        -1 if s > 1 && rng.random(2) == 0 => {
            s -= 1;
            if s < team_level - 5 || s < 1 {
                s += 1;
            }
        }
        _ => {}
    }
    s as u8
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
