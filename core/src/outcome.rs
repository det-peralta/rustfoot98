//! Shot outcome, a port of `1010:81ed` (draw order preserved: attacker, defender, then `Random(4)`).
use crate::rng::DelphiRng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Goal = 1,
    Saved = 2,
    Post = 3,
    Crossbar = 4,
    Wide = 5,
    Over = 6,
}

impl Outcome {
    /// Portuguese text shown by the original game.
    pub fn text(self) -> &'static str {
        match self {
            Outcome::Goal => "GOLO",
            Outcome::Saved => "Defendeu",
            Outcome::Post => "Ao poste",
            Outcome::Crossbar => "Na barra",
            Outcome::Wide => "Ao lado",
            Outcome::Over => "Por cima",
        }
    }
}

pub fn shot(rng: &mut DelphiRng, attacker_force: u16, defender_strength: u16) -> Outcome {
    let a = rng.random(attacker_force) as i32;
    let d = rng.random(defender_strength) as i32;
    if rng.random(4) == 0 {
        if a < 4 {
            return Outcome::Wide;
        } else if a < 10 {
            return Outcome::Over;
        } else if a < 12 {
            return Outcome::Crossbar;
        } else if a < 13 {
            return Outcome::Post;
        }
    }
    if a < d {
        Outcome::Saved
    } else {
        Outcome::Goal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stronger_attacker_scores_more() {
        let mut rng = DelphiRng::new(5);
        let goals = |f: u16, rng: &mut DelphiRng| {
            (0..50_000)
                .filter(|_| shot(rng, f, 20) == Outcome::Goal)
                .count()
        };
        assert!(goals(60, &mut rng) > goals(20, &mut rng));
    }

    #[test]
    fn miss_types_only_come_from_the_lottery() {
        let mut rng = DelphiRng::new(11);
        let (mut wide, mut over, mut bar, mut post) = (0, 0, 0, 0);
        for _ in 0..200_000 {
            match shot(&mut rng, 50, 25) {
                Outcome::Wide => wide += 1,
                Outcome::Over => over += 1,
                Outcome::Crossbar => bar += 1,
                Outcome::Post => post += 1,
                _ => {}
            }
        }
        // 25% lottery, then a<4 / 4..9 / 10..11 / 12 out of 50 equally likely values
        assert!(
            over > wide && wide > bar && bar > post && post > 0,
            "{wide} {over} {bar} {post}"
        );
    }
}
