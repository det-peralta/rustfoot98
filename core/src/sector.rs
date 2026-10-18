//! Squad slots, hidden attributes, team sector strength (`1010:397d`) and attacker force (`1010:818c`).
//!
//! Verified from the disassembly and against a real save (15/15 players):
//! * `H` = sum of the bytes of the player's name.
//! * `g = H mod 10 + 1` (hidden talent), `proneness = H mod 11` (injury weight, `slot+0x2f`),
//!   `aggression = 5 - trunc(sqrt(H mod 36))`, then `(aggression + 2) mod 6` for defenders (`slot+0x2d`).
//! * `star` ("craque") = `g >= 8 && position >= 2`.
//! * Sector value of a starter: `strength + 2*star + 3*sameCountry`.
//!
//! (Earlier notes and third-party write-ups said `strength * 2` and "no hidden attributes"; both are wrong.)

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sectors {
    pub gk: u16,
    pub def: u16,
    pub mid: u16,
    pub att: u16,
}

/// `slot+0x10`: 0 not playing (bench-less/out), 1 starter, 2 designated substitute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Out = 0,
    Starter = 1,
    Sub = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hidden {
    pub g: u8,
    pub aggression: u8,
    pub proneness: u8,
}

/// Sum of the name's bytes (Latin-1), the `H` of `1010:7516`.
pub fn name_hash(name: &str) -> u32 {
    name.chars().map(|c| c as u32 & 0xff).sum()
}

pub fn hidden(name: &str, position: u8) -> Hidden {
    let h = name_hash(name);
    let mut aggression = 5 - ((h % 36) as f64).sqrt() as i32;
    if position == 1 {
        aggression = (aggression + 2) % 6;
    }
    Hidden {
        g: (h % 10) as u8 + 1,
        aggression: aggression as u8,
        proneness: (h % 11) as u8,
    }
}

/// One squad entry (the "slot" object: strength `+0x0a`, status `+0x10`, ban `+0x27`, injury `+0x29`, `g` `+0x2b`, ...).
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    pub status: Status,
    /// 0 goalkeeper, 1 defender, 2 midfielder, 3 attacker.
    pub position: u8,
    pub strength: u16,
    pub g: u8,
    pub aggression: u8,
    pub proneness: u8,
    pub same_country: bool,
    /// Games of suspension left (`+0x27`).
    pub ban: u8,
    /// Games of injury left (`+0x29`).
    pub injured: u8,
    /// Season performance accumulator (`+0x0c`): -1 per pick while a starter, +1 while not (`1010:49e2`).
    pub rating: i32,
    /// Nationality code (the class object's string at `+0x1b`).
    pub nat: [u8; 3],
    /// Stored wage (`+0x31`), which also drives the sale and bid logic. Lags behind the live value until `3e9c` renegotiates.
    pub wage: i64,
    /// `+0x16`: set when the wage was just renegotiated or the player changed club; such a player cannot be sold again.
    pub fresh: bool,
    /// `+0x35`: 0 same country as the club, 1 same country group (Bosman / PLOP), 2 foreigner.
    pub foreign: u8,
}

impl Slot {
    pub fn new(
        name: &str,
        position: u8,
        strength: u16,
        same_country: bool,
        status: Status,
    ) -> Self {
        let h = hidden(name, position);
        Self {
            status,
            position,
            strength,
            g: h.g,
            aggression: h.aggression,
            proneness: h.proneness,
            same_country,
            ban: 0,
            injured: 0,
            rating: 0,
            nat: *b"???",
            wage: 0,
            fresh: false,
            foreign: if same_country { 0 } else { 2 },
        }
    }

    pub fn starter(&self) -> bool {
        self.status == Status::Starter
    }

    /// Available to be picked as a starter or substitute (`+0x27 == 0 && +0x29 == 0`).
    pub fn available(&self) -> bool {
        self.ban == 0 && self.injured == 0
    }

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
    for p in squad.iter().filter(|p| p.starter()) {
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
                let replace = if squad[b].starter() {
                    p.starter() && squad[b].attack_force() <= p.attack_force()
                } else {
                    true
                };
                Some(if replace { i } else { b })
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
                let replace = n.starter()
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

    fn slot(position: u8, strength: u16, g: u8, same: bool, status: Status) -> Slot {
        Slot {
            status,
            position,
            strength,
            g,
            aggression: 3,
            proneness: 3,
            same_country: same,
            ban: 0,
            injured: 0,
            rating: 0,
            nat: *b"???",
            wage: 0,
            fresh: false,
            foreign: 0,
        }
    }
    const S: Status = Status::Starter;

    /// Golden values read from a real save (`.e98` tail offsets +23, +17, +21) of 15 players.
    #[test]
    fn hidden_attributes_match_the_real_save() {
        let rows: [(&str, u8, u8, u8, u8); 15] = [
            ("Carlos Alberto", 0, 8, 0, 4),
            ("Gilberto", 0, 5, 0, 10),
            ("Dionisio", 1, 1, 0, 5),
            ("Leonardo", 1, 1, 2, 6),
            ("Luciano", 1, 6, 2, 0),
            ("Nass", 1, 6, 4, 9),
            ("Nilson", 1, 8, 4, 0),
            ("Nilson Costa", 1, 6, 4, 10),
            ("Marquinhos", 2, 4, 1, 7),
            ("Petry", 2, 3, 0, 4),
            ("Sandro Ventura", 2, 9, 1, 2),
            ("Vinicius", 2, 3, 2, 6),
            ("Marcelinho", 3, 7, 1, 3),
            ("Nando", 3, 7, 0, 1),
            ("Willian", 3, 1, 5, 5),
        ];
        for (name, pos, g, aggr, prone) in rows {
            assert_eq!(
                hidden(name, pos),
                Hidden {
                    g,
                    aggression: aggr,
                    proneness: prone
                },
                "{name}"
            );
        }
        // the only star in the save is the midfielder with g = 9
        assert!(Slot::new("Sandro Ventura", 2, 3, true, S).star());
        assert!(!Slot::new("Carlos Alberto", 0, 5, true, S).star());
    }

    #[test]
    fn sector_is_strength_plus_star_plus_nationality() {
        let squad = [
            slot(0, 5, 8, true, S),             // GK with g=8 is not a star: 5 + 3
            slot(3, 8, 9, false, S),            // attacker g=9 -> star: 8 + 2
            slot(3, 7, 3, true, S),             // 7 + 3
            slot(3, 50, 10, true, Status::Sub), // bench: ignored
        ];
        assert_eq!(
            compute(&squad),
            Sectors {
                gk: 8,
                def: 0,
                mid: 0,
                att: 10 + 10
            }
        );
    }

    #[test]
    fn attack_force_follows_818c() {
        assert_eq!(slot(3, 8, 10, false, S).attack_force(), 32);
        assert_eq!(slot(0, 5, 1, false, S).attack_force(), 3);
        assert_eq!(slot(0, 5, 0, false, S).attack_force(), 2); // 2.5 rounds to even
    }

    #[test]
    fn shooter_and_defender_selection() {
        let squad = [
            slot(0, 9, 5, false, S),
            slot(3, 4, 5, false, S),
            slot(3, 8, 5, false, S),
            slot(3, 8, 5, false, Status::Sub),
            slot(0, 6, 5, false, S),
        ];
        assert_eq!(best_attacker(&squad), Some(2));
        assert_eq!(best_defender(&squad), Some(0));
    }
}
