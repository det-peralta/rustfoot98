//! Reader for the original game's `.e98` saves (`JOGOS\*.e98`), and the import into a [`World`].
//!
//! Only part of the format is decoded (see `docs/findings.md`): what is read here was verified on two real saves (a new game
//! and the same game after two rounds). Not decoded yet and therefore *not* imported: injuries and bans, the finances,
//! the cup, the fixture results. The imported world starts a fresh cup.
//!
//! Layout of what is used, found by scanning for the `EFa` + zeros anchor that opens every team record:
//! - header: double-delta pstr (save description), `u16` id of the first human club at `0x1f`, `u16` year at `0x24`,
//!   the economy index as an extended at `0x28`;
//! - a coach table near the start (one row per team plus the two human managers) that is not needed here;
//! - per team: `u16` team id, the `.EFT` header (names, colours, country), `level u8`, `wins, draws, losses, goals for,
//!   goals against, points` as `u16`, the 14 opponent positions, 57 bytes later the morale `K` (Real48), then the squad: per
//!   player `u16 id, nat pstr, name pstr` + 48 bytes (`+0` position, `+2` strength `u16`);
//! - league teams (the first 32 of the master list) end with a count (14) and 14 fixture entries of 16 bytes whose `u16` value at `+0` is the
//!   team id of the opponent. Clubs that play each other form one division.
use crate::cipher::read_pstr_n;
use crate::season::Club;
use crate::sector::{Slot, Status};
use crate::world::World;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct E98Player {
    pub nationality: String,
    pub name: String,
    pub position: u8,
    pub strength: u16,
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
    pub players: Vec<E98Player>,
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
}

const ANCHOR_ZEROS: usize = 20;

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

fn anchors(d: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut pat = b"EFa".to_vec();
    pat.extend(std::iter::repeat_n(0u8, ANCHOR_ZEROS));
    let mut o = 0;
    while o + pat.len() <= d.len() {
        if d[o..o + pat.len()] == pat[..] {
            out.push(o);
            o += pat.len();
        } else {
            o += 1;
        }
    }
    out
}

/// Tries to read a player record at `o`: `(player, id, end of the record)`.
fn player_at(d: &[u8], o: usize) -> Option<(E98Player, u16, usize)> {
    let id = u16_at(d, o)?;
    if *d.get(o + 2)? != 3 {
        return None;
    }
    let (nat, o2) = read_pstr_n(d, o + 2, false)?;
    if !(nat.len() == 3 && nat.chars().all(|c| c.is_ascii_uppercase())) {
        return None;
    }
    let len = *d.get(o2)? as usize;
    if !(1..=30).contains(&len) {
        return None;
    }
    let (name, o3) = read_pstr_n(d, o2, false)?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphabetic() || matches!(c, ' ' | '.' | '\'' | '-'))
    {
        return None;
    }
    let tail = d.get(o3..o3 + 48)?;
    if tail[0] > 3 {
        return None;
    }
    let strength = u16::from_le_bytes([tail[2], tail[3]]);
    if !(1..=60).contains(&strength) {
        return None;
    }
    Some((
        E98Player {
            nationality: nat,
            name,
            position: tail[0],
            strength,
        },
        id,
        o3 + 48,
    ))
}

pub fn parse(d: &[u8]) -> Result<E98, String> {
    let (description, _) = read_pstr_n(d, 0, true).ok_or("truncated header")?;
    let year = u16_at(d, 0x24).ok_or("truncated header")?;
    let human_id = u16_at(d, 0x1f).ok_or("truncated header")?;
    if !(1990..=2100).contains(&year) {
        return Err(format!("not an Elifoot 98 save (year field reads {year})"));
    }
    let a = anchors(d);
    if a.len() < 34 {
        return Err(format!("only {} team records found; not a save", a.len()));
    }

    let ids: Vec<u16> = a
        .iter()
        .filter_map(|&p| u16_at(d, p.wrapping_sub(2)))
        .collect();

    let mut teams = Vec::new();
    for (i, &p) in a.iter().enumerate() {
        let end = a.get(i + 1).copied().unwrap_or(d.len());
        let id = u16_at(d, p.wrapping_sub(2)).ok_or("truncated team")?;
        let (full_name, o) = read_pstr_n(d, p + 0x32, false).ok_or("bad team name")?;
        let (short_name, o) = read_pstr_n(d, o, false).ok_or("bad team name")?;
        let (country, o) = read_pstr_n(d, o + 8, false).ok_or("bad country")?;
        let stats = d.get(o..o + 13).ok_or("truncated team")?;
        let w = |k: usize| u16::from_le_bytes([stats[1 + 2 * k], stats[2 + 2 * k]]);
        let level = stats[0];
        // morale K: Real48, 57 bytes after the 14 opponent positions that follow the standings
        let ko = o + 13 + 14 + 43;
        let k = d.get(ko..ko + 6).map(real48).unwrap_or(1.0);
        let k = if (0.3..=11.0).contains(&k) { k } else { 1.0 };

        // squad: scan forward for a run of player records
        let mut players = Vec::new();
        let mut q = ko + 6;
        let mut last_end = q;
        while q < end {
            if let Some((pl, _, e)) = player_at(d, q) {
                players.push(pl);
                q = e;
                last_end = e;
            } else if players.is_empty() {
                q += 1;
            } else {
                break;
            }
        }
        if players.is_empty() {
            return Err(format!("{short_name}: no players found"));
        }

        // fixture list: count 14, then 16-byte entries
        let mut opponents = Vec::new();
        for s in last_end..(last_end + 32).min(end) {
            if u16_at(d, s) == Some(14) {
                let vals: Vec<u16> = (0..14).map_while(|j| u16_at(d, s + 2 + 16 * j)).collect();
                if vals.len() == 14 && vals.iter().all(|v| ids.contains(v)) {
                    opponents = vals;
                    break;
                }
            }
        }
        teams.push(E98Team {
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
            players,
            opponents,
        });
    }
    let economy = d.get(0x28..0x32).map(extended).unwrap_or(0.0);
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
    })
}

impl E98 {
    /// Splits the league teams into four divisions: clubs that appear in each other's fixture lists form a group; groups are
    /// ranked by mean player strength (division 1 is the strongest). Returns indices into `teams`.
    pub fn divisions(&self) -> Result<[Vec<usize>; 4], String> {
        let by_key: HashMap<u16, usize> = self
            .teams
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id, i))
            .collect();
        let league: Vec<usize> = (0..self.teams.len())
            .filter(|&i| !self.teams[i].opponents.is_empty())
            .collect();
        if league.len() != 32 {
            return Err(format!("expected 32 league clubs, found {}", league.len()));
        }
        let mut groups: Vec<Vec<usize>> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &i in &league {
            if !seen.insert(i) {
                continue;
            }
            let mut group = vec![i];
            let mut stack = vec![i];
            while let Some(x) = stack.pop() {
                for k in &self.teams[x].opponents {
                    if let Some(&j) = by_key.get(k) {
                        if seen.insert(j) {
                            group.push(j);
                            stack.push(j);
                        }
                    }
                }
            }
            groups.push(group);
        }
        if groups.len() != 4 || groups.iter().any(|g| g.len() != 8) {
            return Err("the fixture lists do not form four divisions of eight".into());
        }
        let mean = |g: &Vec<usize>| -> f64 {
            let (s, n) = g
                .iter()
                .flat_map(|&i| &self.teams[i].players)
                .fold((0.0, 0.0), |(s, n), p| (s + p.strength as f64, n + 1.0));
            s / n
        };
        groups.sort_by(|a, b| mean(b).total_cmp(&mean(a)));
        for g in &mut groups {
            g.sort_by_key(|&i| self.teams[i].id);
        }
        let mut it = groups.into_iter();
        Ok([
            it.next().unwrap(),
            it.next().unwrap(),
            it.next().unwrap(),
            it.next().unwrap(),
        ])
    }
}

fn club_of(t: &E98Team, name: String) -> Club {
    let slots: Vec<Slot> = t
        .players
        .iter()
        .map(|p| {
            Slot::new(
                &p.name,
                p.position,
                p.strength,
                p.nationality == t.country,
                Status::Out,
            )
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
        let mut in_league = std::collections::HashSet::new();
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for (d, idx) in divs.iter().enumerate() {
            for &i in idx {
                in_league.insert(i);
                divisions[d].push(club_of(&save.teams[i], name_of(&save.teams[i])));
            }
        }
        let mut rest: Vec<usize> = (0..save.teams.len())
            .filter(|i| !in_league.contains(i))
            .collect();
        rest.sort_by_key(|&i| std::cmp::Reverse(save.teams[i].level));
        let pool: Vec<Club> = rest
            .iter()
            .map(|&i| club_of(&save.teams[i], name_of(&save.teams[i])))
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
            let divs = save.divisions().unwrap();
            assert!(divs.iter().all(|g| g.len() == 8));
            let w = World::from_e98(1, &save).unwrap();
            assert_eq!(w.pool.len(), 20);
            // the standings are internally consistent
            for c in w.divisions.iter().flatten() {
                assert_eq!(c.points, 3 * c.wins + c.draws, "{}", c.name);
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
