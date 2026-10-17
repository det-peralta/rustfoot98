//! Team sector strength, verified from `1010:397d`.
//! `sector[pos] += strength * 2 + (same country ? 3 : 0) + slot adjustment` over the starters.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sectors {
    pub gk: u16,
    pub def: u16,
    pub mid: u16,
    pub att: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct SquadSlot {
    pub starter: bool,
    /// 0 goalkeeper, 1 defender, 2 midfielder, 3 attacker (the position the player is playing in).
    pub position: u8,
    pub strength: u8,
    pub same_country: bool,
    pub adjust: u16,
}

pub fn compute(squad: &[SquadSlot]) -> Sectors {
    let mut s = [0u16; 4];
    for p in squad.iter().filter(|p| p.starter) {
        let v = p.strength as u16 * 2 + if p.same_country { 3 } else { 0 } + p.adjust;
        s[(p.position & 3) as usize] = s[(p.position & 3) as usize].wrapping_add(v);
    }
    Sectors {
        gk: s[0],
        def: s[1],
        mid: s[2],
        att: s[3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_strength_and_adds_nationality_bonus() {
        let squad = [
            SquadSlot {
                starter: true,
                position: 0,
                strength: 5,
                same_country: true,
                adjust: 0,
            },
            SquadSlot {
                starter: true,
                position: 3,
                strength: 8,
                same_country: false,
                adjust: 1,
            },
            SquadSlot {
                starter: true,
                position: 3,
                strength: 7,
                same_country: true,
                adjust: 0,
            },
            SquadSlot {
                starter: false,
                position: 3,
                strength: 50,
                same_country: true,
                adjust: 0,
            },
        ];
        let s = compute(&squad);
        assert_eq!(
            s,
            Sectors {
                gk: 13,
                def: 0,
                mid: 0,
                att: 17 + 17
            }
        );
    }
}
