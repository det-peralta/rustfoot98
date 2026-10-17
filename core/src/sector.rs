//! Squad slots, team sector strength (`1010:397d`) and attacker force (`1010:818c`).
//!
//! Verified from the disassembly: a starter adds
//! `slot.strength + 2 * star + (same country ? 3 : 0)` to the sector of its position, where
//! `star` is the "craque" flag (`player+0x1a`), set at creation (`1010:7516`) when
//! `g >= 8 && position >= 2`, with `g = Random(10) + 1` (slot `+0x2b`).
//! (Earlier notes and third-party write-ups said `strength * 2`; the code does not do that.)

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sectors {
    pub gk: u16,
    pub def: u16,
    pub mid: u16,
    pub att: u16,
}

/// One squad entry (the "slot" object: strength at `+0x0a`, starter flag at `+0x10`, `g` at `+0x2b`).
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub starter: bool,
    /// 0 goalkeeper, 1 defender, 2 midfielder, 3 attacker.
    pub position: u8,
    pub strength: u16,
    /// Hidden talent `Random(10)+1`, fixed at creation.
    pub g: u8,
    pub same_country: bool,
}

impl Slot {
    /// The "craque" mark (`*` next to the name).
    pub fn star(&self) -> bool {
        self.g >= 8 && self.position >= 2
    }

    /// `1010:818c`: `round((pos+1) * strength * (g/10 + 1) / 2)` (round half to even, as FRNDINT).
    pub fn attack_force(&self) -> u16 {
        let x = (self.position as u32 + 1) * self.strength as u32;
        (x as f64 * (self.g as f64 / 10.0 + 1.0) / 2.0).round_ties_even() as u16
    }
}

pub fn compute(squad: &[Slot]) -> Sectors {
    let mut s = [0u16; 4];
    for p in squad.iter().filter(|p| p.starter) {
        let v = p.strength + 2 * p.star() as u16 + if p.same_country { 3 } else { 0 };
        let i = (p.position & 3) as usize;
        s[i] = s[i].wrapping_add(v);
    }
    Sectors {
        gk: s[0],
        def: s[1],
        mid: s[2],
        att: s[3],
    }
}

/// Shooter for AI-resolved chances (`0ee6` with comparer `201d`): the starter with the highest force;
/// ties go to the later slot; a non-starter is always replaced.
pub fn best_attacker(squad: &[Slot]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, p) in squad.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(b) => {
                let replace = if squad[b].starter {
                    p.starter && squad[b].attack_force() <= p.attack_force()
                } else {
                    true
                };
                if replace {
                    Some(i)
                } else {
                    Some(b)
                }
            }
        };
    }
    best
}

/// Defender for the shot duel (`0ee6` with comparer `1f88`): the starter with the lowest position
/// (the goalkeeper), strongest on ties.
pub fn best_defender(squad: &[Slot]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, n) in squad.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(b) => {
                let y = &squad[b];
                let replace = n.starter
                    && (n.position < y.position
                        || (n.position == y.position && n.strength > y.strength));
                Some(if replace { i } else { b })
            }
        };
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(position: u8, strength: u16, g: u8, same: bool, starter: bool) -> Slot {
        Slot {
            starter,
            position,
            strength,
            g,
            same_country: same,
        }
    }

    #[test]
    fn sector_is_strength_plus_star_plus_nationality() {
        let squad = [
            slot(0, 5, 8, true, true),    // GK with g=8 is not a star
            slot(3, 8, 9, false, true),   // attacker, g=9 -> star (+2)
            slot(3, 7, 3, true, true),    // +3
            slot(3, 50, 10, true, false), // bench: ignored
        ];
        assert_eq!(
            compute(&squad),
            Sectors {
                gk: 8,
                def: 0,
                mid: 0,
                att: (8 + 2) + (7 + 3)
            }
        );
    }

    #[test]
    fn matches_the_real_save() {
        // Sandro Ventura (midfielder, strength 3, g=9) is the only star in the snapshot; Carlos Alberto (GK, g=8) is not.
        assert!(slot(2, 3, 9, true, true).star());
        assert!(!slot(0, 5, 8, true, true).star());
        assert!(!slot(3, 7, 7, true, true).star());
    }

    #[test]
    fn attack_force_follows_818c() {
        // attacker strength 8, g=10: 4*8*(2)/2 = 32 ; goalkeeper strength 5, g=1: 1*5*1.1/2 = 2.75 -> 3
        assert_eq!(slot(3, 8, 10, false, true).attack_force(), 32);
        assert_eq!(slot(0, 5, 1, false, true).attack_force(), 3);
        // exact .5 rounds to even: 1*5*1.0... g=0 gives 5*1/2 = 2.5 -> 2
        assert_eq!(slot(0, 5, 0, false, true).attack_force(), 2);
    }

    #[test]
    fn shooter_and_defender_selection() {
        let squad = [
            slot(0, 9, 5, false, true),
            slot(3, 4, 5, false, true),
            slot(3, 8, 5, false, true),
            slot(3, 8, 5, false, false), // not a starter, never chosen over a starter
            slot(0, 6, 5, false, true),
        ];
        assert_eq!(best_attacker(&squad), Some(2));
        assert_eq!(best_defender(&squad), Some(0)); // strongest starting goalkeeper
    }
}
