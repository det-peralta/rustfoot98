//! Reader for the original game's `.e98` saves (`JOGOS\*.e98`), and the import into a [`World`].
//!
//! The whole file is walked with the structure of the original loader (`1020:1d8d`; layout in `docs/e98-format.md`) and
//! must be consumed to the last byte, which was checked on real saves. Imported: rosters (status, goals, bans, injuries,
//! wages), standings, morale `K`, finances, the cup entrants and the division lists. Skipped (read but unused): the
//! coach/event tables, the played-round records and the news lists.
use crate::cipher::read_pstr_n;
use crate::economy::DEFAULT_SCALE;
use crate::finance::Finance;
use crate::market::{code, country_class};
use crate::season::Club;
use crate::sector::{Slot, Status};
use crate::world::base_strength;
use crate::world::World;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct E98Player {
    pub id: u16,
    pub nationality: String,
    pub name: String,
    pub position: u8,
    pub strength: u16,
    /// Stored wage (`+0x31`) and the "fresh" flag (`+0x16`).
    pub wage: i64,
    pub fresh: bool,
    /// Goals scored (`+0x14`).
    pub goals: u16,
    /// `slot+0x10`: 0 out, 1 starter, 2 substitute.
    pub status: u8,
    /// Games of suspension and injury left (`+0x27`, `+0x29`).
    pub ban: u8,
    pub injured: u8,
}

#[derive(Clone, Debug)]
pub struct E98Team {
    pub id: u16,
    pub full_name: String,
    pub short_name: String,
    pub country: String,
    pub level: u8,
    pub wins: u16,
    pub draws: u16,
    pub losses: u16,
    pub goals_for: u16,
    pub goals_against: u16,
    pub points: u16,
    pub k: f64,
    pub cash: i64,
    pub loan: i64,
    pub factor: f64,
    /// Season totals of the ledger: gate, sales, purchases, wages.
    pub ledger: [i64; 4],
    pub players: Vec<E98Player>,
    /// The team's coach (`coach+0x0c` games coached, `+0x0e` score, `+0x07` set for the user's coaches).
    pub coach: u16,
    pub coach_games: u16,
    pub coach_score: i64,
    pub coach_human: bool,
    /// Team ids of the opponents in the fixture list (each twice); empty for clubs outside the league.
    pub opponents: Vec<u16>,
}

#[derive(Clone, Debug)]
pub struct E98 {
    pub description: String,
    pub year: u16,
    /// Economy index (extended at header offset `0x28`; 2.0 in a new game).
    pub economy: f64,
    pub human_id: u16,
    pub teams: Vec<E98Team>,
    /// Ids of the clubs in the cup draw (`26f2`).
    pub cup: Vec<u16>,
    /// Division lists as saved: name and club ids (the fifth is the pool of unemployed-coach clubs).
    pub divisions: Vec<(String, Vec<u16>)>,
}

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*d.get(o)?, *d.get(o + 1)?]))
}

/// x87 80-bit extended (Delphi `Extended`).
pub fn extended(b: &[u8]) -> f64 {
    if b.len() < 10 {
        return 0.0;
    }
    let m = u64::from_le_bytes(b[..8].try_into().unwrap());
    let se = u16::from_le_bytes([b[8], b[9]]);
    let e = (se & 0x7fff) as i32;
    if e == 0 {
        return 0.0;
    }
    let v = m as f64 / 2f64.powi(63) * 2f64.powi(e - 16383);
    if se & 0x8000 != 0 {
        -v
    } else {
        v
    }
}

/// Delphi `Real48`: exponent byte, then a 39-bit mantissa and a sign bit.
pub fn real48(b: &[u8]) -> f64 {
    if b.len() < 6 || b[0] == 0 {
        return 0.0;
    }
    let mut m = 0u64;
    for (i, &x) in b[1..6].iter().enumerate() {
        m |= (x as u64) << (8 * i);
    }
    let sign = m >> 39 & 1;
    let frac = (m & ((1 << 39) - 1)) as f64 / (1u64 << 39) as f64;
    let v = (1.0 + frac) * 2f64.powi(b[0] as i32 - 129);
    if sign == 1 {
        -v
    } else {
        v
    }
}

/// Byte cursor over the save; every read is bounds-checked.
struct Cur<'a> {
    d: &'a [u8],
    o: usize,
}

impl<'a> Cur<'a> {
    fn raw(&mut self, n: usize) -> Result<&'a [u8], String> {
        let r = self
            .d
            .get(self.o..self.o + n)
            .ok_or_else(|| format!("truncated save at {:#x}", self.o))?;
        self.o += n;
        Ok(r)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.raw(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        let b = self.raw(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn pstr(&mut self) -> Result<String, String> {
        let (s, o) = read_pstr_n(self.d, self.o, false).ok_or("truncated string")?;
        self.o = o;
        Ok(s)
    }
    fn list<T>(
        &mut self,
        mut f: impl FnMut(&mut Self) -> Result<T, String>,
    ) -> Result<Vec<T>, String> {
        let n = self.u16()?;
        (0..n).map(|_| f(self)).collect()
    }
}

fn long(b: &[u8], at: usize) -> i64 {
    i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as i64
}

fn player(c: &mut Cur) -> Result<E98Player, String> {
    let id = c.u16()?;
    let nat = c.pstr()?;
    let name = c.pstr()?;
    let tail = c.raw(48)?;
    Ok(E98Player {
        id,
        nationality: nat,
        name,
        position: tail[0],
        strength: u16::from_le_bytes([tail[2], tail[3]]),
        wage: long(tail, 25),
        fresh: tail[14] != 0,
        goals: u16::from_le_bytes([tail[12], tail[13]]),
        status: tail[8],
        ban: tail[15],
        injured: tail[19],
    })
}

fn team(c: &mut Cur) -> Result<E98Team, String> {
    let id = c.u16()?;
    // the .EFT object: 50-byte anchor, full and short name, colours, country, level
    c.raw(50)?;
    let full_name = c.pstr()?;
    let short_name = c.pstr()?;
    c.raw(8)?;
    let country = c.pstr()?;
    let level = c.u8()?;
    let block = c.raw(0x37)?; // standings, ledger (+0x16..)
    c.raw(14)?;
    let k = real48(c.raw(6)?);
    let k = if (0.3..=11.0).contains(&k) { k } else { 1.0 };
    let coach = c.u16()?;
    c.raw(28 + 28)?; // two blocks of 28 bytes
    let money = c.raw(4 + 4 + 4 + 10)?; // loan, cash, start-of-season cash, F
    let factor = extended(&money[12..22]);
    let factor = if (0.1..=10.0).contains(&factor) {
        factor
    } else {
        1.0
    };
    c.raw(6 + 4 + 2 + 2)?;
    let w = |k: usize| u16::from_le_bytes([block[2 * k], block[1 + 2 * k]]);
    let ledger = [
        long(block, 0x31 - 0x16),
        long(block, 0x35 - 0x16),
        long(block, 0x39 - 0x16),
        long(block, 0x41 - 0x16),
    ];
    let players = c.list(player)?;
    c.list(|c| c.raw(7).map(|_| ()))?;
    c.list(|c| c.raw(8).map(|_| ()))?;
    let opponents = c.list(|c| {
        let e = c.raw(16)?;
        Ok(u16::from_le_bytes([e[0], e[1]]))
    })?;
    c.raw(1 + 6)?; // stadium level and fill
    Ok(E98Team {
        id,
        full_name,
        short_name,
        country,
        level,
        wins: w(0),
        draws: w(1),
        losses: w(2),
        goals_for: w(3),
        goals_against: w(4),
        points: w(5),
        k,
        cash: long(money, 4),
        loan: long(money, 0),
        factor,
        ledger,
        players,
        coach,
        coach_games: 0,
        coach_score: 0,
        coach_human: false,
        opponents,
    })
}

/// Reads a whole save following the original loader (`1020:1d8d`, see `docs/e98-format.md`); the file must be consumed exactly.
pub fn parse(d: &[u8]) -> Result<E98, String> {
    let (description, _) = read_pstr_n(d, 0, true).ok_or("truncated header")?;
    let year = u16_at(d, 0x24).ok_or("truncated header")?;
    let human_id = u16_at(d, 0x1f).ok_or("truncated header")?;
    if !(1990..=2100).contains(&year) {
        return Err(format!("not an Elifoot 98 save (year field reads {year})"));
    }
    let mut c = Cur { d, o: 0 };
    let n = c.u8()? as usize;
    c.raw(n + 2 + 2 + 1 + 2 + 4 + 10 + 10 + 101)?;
    let coaches = c.list(|c| {
        let id = c.u16()?;
        c.pstr()?;
        let human = c.u8()? != 0;
        c.u8()?;
        let games = c.u16()?;
        let rest = c.raw(4 + 4 + 5)?;
        let score = long(rest, 0);
        c.list(|c| c.raw(7).map(|_| ()))?;
        Ok((id, human, games, score))
    })?;
    c.list(|c| {
        c.raw(3)?;
        c.pstr()?;
        c.pstr()?;
        Ok(())
    })?;
    let mut teams = c.list(team)?;
    for t in &mut teams {
        if let Some(&(_, human, games, score)) = coaches.iter().find(|k| k.0 == t.coach) {
            t.coach_human = human;
            t.coach_games = games;
            t.coach_score = score;
        }
    }
    let cup = c.list(|c| c.u16())?;
    let divisions = c.list(|c| {
        c.raw(4 + 2)?;
        let b = c.raw(21)?;
        let name = String::from_utf8_lossy(&b[1..1 + (b[0] as usize).min(20)]).into_owned();
        Ok((name, c.list(|c| c.u16())?))
    })?;
    c.list(|c| {
        c.raw(1 + 41 + 2 + 2 + 10 + 4 + 2 + 2 + 2 + 1)?;
        Ok(())
    })?;
    while c.u16()? != 0 {
        c.list(|c| c.u16())?;
    }
    c.list(|c| c.raw(10).map(|_| ()))?;
    c.list(|c| c.raw(12).map(|_| ()))?;
    if c.o != d.len() {
        return Err(format!("{} unread bytes after the save", d.len() - c.o));
    }
    if teams.len() < 33 {
        return Err(format!("only {} team records; not a save", teams.len()));
    }
    let economy = extended(&d[0x28..0x32]);
    let economy = if (0.5..=10.0).contains(&economy) {
        economy
    } else {
        crate::economy::INITIAL_INDEX
    };
    Ok(E98 {
        description,
        year,
        economy,
        human_id,
        teams,
        cup,
        divisions,
    })
}

impl E98 {
    /// The four divisions as indices into `teams`, from the save's own division lists (sorted by club id).
    pub fn divisions(&self) -> Result<[Vec<usize>; 4], String> {
        let by_key: HashMap<u16, usize> = self
            .teams
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id, i))
            .collect();
        let mut out: [Vec<usize>; 4] = Default::default();
        for (k, (_, ids)) in self.divisions.iter().take(4).enumerate() {
            let mut v: Vec<usize> = ids.iter().filter_map(|i| by_key.get(i).copied()).collect();
            v.sort_by_key(|&i| self.teams[i].id);
            out[k] = v;
        }
        if out.iter().any(|g| g.len() != 8) {
            return Err("the save does not hold four divisions of eight".into());
        }
        Ok(out)
    }
}

fn club_of(t: &E98Team, name: String, rank: usize) -> Club {
    let base = base_strength(rank);
    let slots: Vec<Slot> = t
        .players
        .iter()
        .map(|p| {
            let mut s = Slot::new(
                &p.name,
                p.position,
                p.strength,
                p.nationality == t.country,
                Status::Out,
            );
            s.nat = code(&p.nationality);
            s.foreign = country_class(code(&t.country), s.nat);
            s.wage = p.wage;
            s.fresh = p.fresh;
            s.goals = p.goals;
            s.ban = p.ban;
            s.injured = p.injured;
            s.status = match p.status {
                1 => Status::Starter,
                2 => Status::Sub,
                _ => Status::Out,
            };
            s
        })
        .collect();
    let mut c = Club::new(&name, slots);
    c.names = t.players.iter().map(|p| p.name.clone()).collect();
    c.k = t.k;
    c.points = t.points;
    c.wins = t.wins;
    c.draws = t.draws;
    c.losses = t.losses;
    c.goals_for = t.goals_for as i32;
    c.goals_against = t.goals_against as i32;
    c.coach_games = t.coach_games;
    c.coach_score = t.coach_score;
    c.fin = Finance::new_game(base, DEFAULT_SCALE); // stadium level: the save's stadium object was not located
    c.fin.cash = t.cash;
    c.fin.loan = t.loan;
    c.fin.f = t.factor;
    c.fin.gate = t.ledger[0];
    c.fin.sales = t.ledger[1];
    c.fin.purchases = t.ledger[2];
    c.fin.wages = t.ledger[3];
    c.country = code(&t.country);
    c.master = rank as u16;
    c
}

impl World {
    /// Builds a world from an original save. `seed` drives the random number generator from here on (the original's seed is
    /// not stored in a place we have decoded). The user's club is the first human club of the save.
    pub fn from_e98(seed: u32, save: &E98) -> Result<World, String> {
        let divs = save.divisions()?;
        let mut used = std::collections::HashSet::new();
        let mut name_of = |t: &E98Team| {
            let mut name = t.short_name.trim().to_string();
            let mut n = 2;
            while !used.insert(name.clone()) {
                name = format!("{} ({n})", t.short_name.trim());
                n += 1;
            }
            name
        };
        // team ids follow the master list, so the rank of the id is the list position that fixed the new-game values
        let mut by_id: Vec<usize> = (0..save.teams.len()).collect();
        by_id.sort_by_key(|&i| save.teams[i].id);
        let base_of = |i: usize| by_id.iter().position(|&x| x == i).unwrap_or(0);
        let mut in_league = std::collections::HashSet::new();
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for (d, idx) in divs.iter().enumerate() {
            for &i in idx {
                in_league.insert(i);
                divisions[d].push(club_of(&save.teams[i], name_of(&save.teams[i]), base_of(i)));
            }
        }
        let mut rest: Vec<usize> = (0..save.teams.len())
            .filter(|i| !in_league.contains(i))
            .collect();
        rest.sort_by_key(|&i| std::cmp::Reverse(save.teams[i].level));
        let pool: Vec<Club> = rest
            .iter()
            .map(|&i| club_of(&save.teams[i], name_of(&save.teams[i]), base_of(i)))
            .collect();
        let round = divs
            .iter()
            .flatten()
            .map(|&i| {
                let t = &save.teams[i];
                (t.wins + t.draws + t.losses) as usize
            })
            .max()
            .unwrap_or(0);
        let mut w = World::empty(seed, divisions, pool, save.year, round.min(13));
        w.economy = save.economy;
        w.refresh_cash_ref();
        let human = divs.iter().enumerate().find_map(|(d, idx)| {
            idx.iter()
                .position(|&i| save.teams[i].id == save.human_id)
                .map(|p| (d, p))
        });
        match human {
            Some((d, p)) => w.set_human(d, p),
            None => w.set_human(3, 7),
        }
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real48_decodes_known_values() {
        assert_eq!(real48(&[0x81, 0, 0, 0, 0, 0]), 1.0);
        assert!((real48(&[0x80, 0x9a, 0x99, 0x99, 0x99, 0x19]) - 0.6).abs() < 1e-9);
        assert_eq!(real48(&[0, 1, 2, 3, 4, 5]), 0.0);
    }

    #[test]
    fn rejects_foreign_files() {
        assert!(parse(&[0u8; 100]).is_err());
    }

    /// Runs against the author's local samples when present (they are copyrighted game saves and are not in the repo).
    #[test]
    fn local_samples_when_available() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original/saves");
        for name in ["snap1.e98", "snap2.e98"] {
            let Ok(bytes) = std::fs::read(root.join(name)) else {
                continue;
            };
            let save = parse(&bytes).unwrap();
            assert_eq!(save.year, 2026);
            assert!(
                (save.economy - if name == "snap1.e98" { 2.0 } else { 2.0054 }).abs() < 0.001,
                "{}",
                save.economy
            );
            assert_eq!(save.teams.len(), 52);
            assert_eq!(save.cup.len(), 52);
            assert_eq!(save.divisions.len(), 5);
            if name == "snap2.e98" {
                let t = save.teams.iter().find(|t| t.coach_human).unwrap();
                assert_eq!(t.coach_games, 2, "two rounds played");
                assert!(t.coach_score <= 6);
                assert_eq!(save.teams.iter().filter(|t| t.coach_human).count(), 2);
            }
            let divs = save.divisions().unwrap();
            assert!(divs.iter().all(|g| g.len() == 8));
            let w = World::from_e98(1, &save).unwrap();
            assert_eq!(w.pool.len(), 20);
            // the standings are internally consistent
            for c in w.divisions.iter().flatten() {
                assert_eq!(c.points, 3 * c.wins + c.draws, "{}", c.name);
            }
            let human = w.divisions[3]
                .iter()
                .find(|c| c.name == "FIGUEIRENSE")
                .unwrap();
            if name == "snap1.e98" {
                assert_eq!(human.fin.cash, 1_250_000);
                assert_eq!(
                    w.divisions[0].iter().map(|c| c.fin.cash).max(),
                    Some(12_250_000)
                );
                assert!((human.fin.f - 0.97679).abs() < 1e-4);
            } else {
                assert_eq!(human.fin.cash, 1_216_448);
                assert_eq!(human.fin.gate, 47_448);
                assert_eq!(human.fin.wages, -81_000);
            }
            if name == "snap2.e98" {
                // the computer's line-up rule reproduces the saved statuses of the clubs (list order = the saved order)
                let (mut ok, mut total) = (0, 0);
                for idx in divs.iter() {
                    for &i in idx {
                        let t = &save.teams[i];
                        if t.players.iter().all(|p| p.status == 0) {
                            continue;
                        }
                        let mut slots: Vec<Slot> = t
                            .players
                            .iter()
                            .map(|p| {
                                let mut s =
                                    Slot::new(&p.name, p.position, p.strength, true, Status::Out);
                                s.goals = p.goals;
                                s.ban = p.ban;
                                s.injured = p.injured;
                                s
                            })
                            .collect();
                        crate::lineup::default_statuses(&mut slots);
                        let want: Vec<u8> = t.players.iter().map(|p| p.status).collect();
                        let got: Vec<u8> = slots
                            .iter()
                            .map(|s| match s.status {
                                Status::Out => 0,
                                Status::Starter => 1,
                                Status::Sub => 2,
                            })
                            .collect();
                        total += 1;
                        ok += (want == got) as usize;
                    }
                }
                assert!(
                    total >= 28 && ok + 1 >= total,
                    "line-up rule matched {ok} of {total} clubs"
                );
            }
            let (d, _) = w.human_location().unwrap();
            assert_eq!(d, 3, "Figueirense starts in the fourth division");
            if name == "snap1.e98" {
                assert_eq!(w.round, 0);
                assert!(w.divisions[0].iter().any(|c| c.name == "SÃO PAULO"));
            } else {
                assert_eq!(w.round, 2);
            }
        }
    }
}
