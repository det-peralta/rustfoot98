//! The whole league world: four divisions of eight clubs, an outside pool, the year and the round.
//!
//! Verified: division base levels `52, 38, 24, 10` (`1010:0b60` sets `division+4 = index * -14 + 52`; the outside
//! pool is index 4 with base -4 and is never evolved), 14 league rounds, evolution after every round, and the
//! season end of `1020:038e`. **Assumed:** squad composition (2 GK, 7 DEF, 6 MID, 5 ATT), starting strength
//! `level + Random(7) - 3` (the deviations seen in a real save are uniform in -3..=3), club/player names are fictional,
//! no cup (so only the champion's `K += 1.0` applies).
use crate::eft::{EftTeam, Position};
use crate::rng::DelphiRng;
use crate::season::{end_of_round, end_of_season, play_round, standings, Club, MatchRecord};
use crate::sector::{Slot, Status};

pub const DIVISION_BASE: [i32; 4] = [52, 38, 24, 10];
pub const DIVISION_NAMES: [&str; 4] = ["1ª Divisão", "2ª Divisão", "3ª Divisão", "4ª Divisão"];
pub const ROUNDS: usize = 14;

const CITIES: [&str; 40] = [
    "Aveiro",
    "Braga",
    "Coimbra",
    "Évora",
    "Faro",
    "Guarda",
    "Lagos",
    "Leiria",
    "Porto",
    "Viseu",
    "Tomar",
    "Sintra",
    "Beja",
    "Chaves",
    "Elvas",
    "Espinho",
    "Fafe",
    "Gaia",
    "Loulé",
    "Mafra",
    "Olhão",
    "Penafiel",
    "Setúbal",
    "Tavira",
    "Vila Real",
    "Barreiro",
    "Cascais",
    "Moura",
    "Seixal",
    "Estoril",
    "Abrantes",
    "Covilhã",
    "Peniche",
    "Portimão",
    "Santarém",
    "Torres Vedras",
    "Amadora",
    "Oeiras",
    "Maia",
    "Lamego",
];
const KINDS: [&str; 6] = [
    "Atlético",
    "Sporting",
    "União",
    "Académica",
    "Desportivo",
    "Estrela",
];
const FIRST: [&str; 20] = [
    "Rui", "João", "Pedro", "Nuno", "Hugo", "Tiago", "Paulo", "Ricardo", "André", "Bruno",
    "Carlos", "Diogo", "Fábio", "Gil", "Luís", "Miguel", "Rafael", "Sérgio", "Tomás", "Vasco",
];
const LAST: [&str; 16] = [
    "Silva",
    "Santos",
    "Ferreira",
    "Pereira",
    "Costa",
    "Oliveira",
    "Rodrigues",
    "Martins",
    "Sousa",
    "Fernandes",
    "Gonçalves",
    "Lopes",
    "Marques",
    "Alves",
    "Almeida",
    "Ribeiro",
];

#[derive(Clone, Debug)]
pub struct TableRow {
    pub name: String,
    pub points: u16,
    pub wins: u16,
    pub draws: u16,
    pub losses: u16,
    pub goals_for: i32,
    pub goals_against: i32,
}

/// A club that changed division at a season end (`4` = the outside pool).
#[derive(Clone, Debug)]
pub struct Move {
    pub name: String,
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug)]
pub struct SeasonSummary {
    pub year: u16,
    pub tables: [Vec<TableRow>; 4],
    pub moves: Vec<Move>,
}

#[derive(Debug)]
pub struct World {
    pub divisions: [Vec<Club>; 4],
    pub pool: Vec<Club>,
    pub year: u16,
    /// Rounds already played this season (0..=13); a new season starts after round 14.
    pub round: usize,
    pub rng: DelphiRng,
    /// Matches of the last round played, tagged with their division.
    pub last_round: Vec<(usize, MatchRecord)>,
    pub last_season: Option<SeasonSummary>,
}

fn player_name(rng: &mut DelphiRng) -> String {
    format!(
        "{} {}",
        FIRST[rng.random(FIRST.len() as u16) as usize],
        LAST[rng.random(LAST.len() as u16) as usize]
    )
}

fn make_club(rng: &mut DelphiRng, name: String, level: i32) -> Club {
    let mut slots = Vec::new();
    let mut names = Vec::new();
    for (position, count) in [(0u8, 2), (1, 7), (2, 6), (3, 5)] {
        for _ in 0..count {
            let n = player_name(rng);
            let strength = (level + rng.random(7) as i32 - 3).clamp(1, 50) as u16;
            slots.push(Slot::new(&n, position, strength, true, Status::Out));
            names.push(n);
        }
    }
    let mut club = Club::new(&name, slots);
    club.names = names;
    club
}

/// Minimum clubs for a real-team world: 4 divisions of 8 plus an outside pool of at least 3 (`end_of_season`).
pub const MIN_TEAMS: usize = 35;

/// Builds a club from a `.EFT` team: real player names and positions, strength `level + Random(7) - 3`
/// (inferred from a real save, not decoded). Errors when no formation of the game can be fielded.
fn club_from_eft(rng: &mut DelphiRng, t: &EftTeam, name: String) -> Result<Club, String> {
    let mut slots = Vec::new();
    let mut names = Vec::new();
    let mut count = [0usize; 4];
    for p in &t.players {
        let position = match p.position {
            Position::Goalkeeper => 0u8,
            Position::Defender => 1,
            Position::Midfielder => 2,
            Position::Attacker => 3,
        };
        count[position as usize] += 1;
        let strength = (t.level as i32 + rng.random(7) as i32 - 3).clamp(1, 50) as u16;
        slots.push(Slot::new(
            &p.name,
            position,
            strength,
            p.nationality == t.country,
            Status::Out,
        ));
        names.push(p.name.clone());
    }
    if !crate::season::can_field(count) {
        return Err(format!(
            "{}: cannot field any of the 12 formations (gk {}, def {}, mid {}, att {})",
            t.short_name, count[0], count[1], count[2], count[3]
        ));
    }
    let mut club = Club::new(&name, slots);
    club.names = names;
    Ok(club)
}

impl World {
    /// A world made of real teams: shuffled (ties between equal levels are random), the strongest 32 fill the
    /// four divisions in level order (8 each), the rest form the outside pool.
    /// **Assumption:** how the original assigns teams to divisions is not decoded; a real save shows the 4th division
    /// mixing levels 1..6, so this is only an approximation.
    pub fn from_teams(seed: u32, teams: &[EftTeam]) -> Result<World, String> {
        if teams.len() < MIN_TEAMS {
            return Err(format!(
                "need at least {MIN_TEAMS} teams, got {}",
                teams.len()
            ));
        }
        let mut rng = DelphiRng::new(seed);
        let mut order: Vec<usize> = (0..teams.len()).collect();
        // Fisher-Yates with the Delphi RNG, then a stable sort by level (descending)
        for i in (1..order.len()).rev() {
            let j = rng.random(i as u16 + 1) as usize;
            order.swap(i, j);
        }
        order.sort_by_key(|&i| std::cmp::Reverse(teams[i].level));
        let mut used = std::collections::HashSet::new();
        let mut clubs = Vec::new();
        for &i in &order {
            let mut name = teams[i].short_name.trim().to_string();
            let mut n = 2;
            while !used.insert(name.clone()) {
                name = format!("{} ({n})", teams[i].short_name.trim());
                n += 1;
            }
            clubs.push(club_from_eft(&mut rng, &teams[i], name)?);
        }
        let mut it = clubs.into_iter();
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for d in divisions.iter_mut() {
            *d = it.by_ref().take(8).collect();
        }
        let pool: Vec<Club> = it.collect();
        Ok(World {
            divisions,
            pool,
            year: 1998,
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
        })
    }

    /// A fictional world. Levels follow the real data: the 4th division holds levels 1..6 (read from a real save).
    pub fn synthetic(seed: u32) -> World {
        let mut rng = DelphiRng::new(seed);
        let levels: [[i32; 8]; 4] = [
            [16, 15, 14, 13, 13, 12, 12, 11],
            [10, 9, 9, 8, 8, 7, 7, 6],
            [6, 5, 5, 4, 4, 4, 3, 3],
            [6, 5, 3, 2, 2, 1, 1, 1],
        ];
        let mut n = 0;
        let mut next_name = || {
            let i = n;
            n += 1;
            format!("{} {}", KINDS[i % KINDS.len()], CITIES[i % CITIES.len()])
        };
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for (d, lv) in levels.iter().enumerate() {
            divisions[d] = lv
                .iter()
                .map(|&l| make_club(&mut rng, next_name(), l))
                .collect();
        }
        let pool = (0..6)
            .map(|i| make_club(&mut rng, next_name(), 1 + i % 3))
            .collect();
        World {
            divisions,
            pool,
            year: 1998,
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
        }
    }

    /// Plays the next round of every division, evolves the squads, and runs the season end after round 14.
    pub fn play_round(&mut self) {
        self.last_round.clear();
        let round = self.round + 1;
        for d in 0..4 {
            let division_name = DIVISION_NAMES[d].as_bytes().to_vec();
            let team_names: Vec<Vec<u8>> = self.divisions[d]
                .iter()
                .map(|c| c.name.as_bytes().to_vec())
                .collect();
            let mut names = |h: usize, a: usize| {
                (
                    division_name.clone(),
                    team_names[h].clone(),
                    team_names[a].clone(),
                )
            };
            let records = play_round(&mut self.rng, &mut self.divisions[d], round, &mut names);
            self.last_round.extend(records.into_iter().map(|r| (d, r)));
            end_of_round(&mut self.rng, &mut self.divisions[d], DIVISION_BASE[d]);
        }
        self.round = round;
        if round == ROUNDS {
            self.finish_season();
        }
    }

    fn finish_season(&mut self) {
        let table = |d: &Vec<Club>| -> Vec<TableRow> {
            standings(d)
                .into_iter()
                .map(|i| {
                    let c = &d[i];
                    TableRow {
                        name: c.name.clone(),
                        points: c.points,
                        wins: c.wins,
                        draws: c.draws,
                        losses: c.losses,
                        goals_for: c.goals_for,
                        goals_against: c.goals_against,
                    }
                })
                .collect()
        };
        let tables = [
            table(&self.divisions[0]),
            table(&self.divisions[1]),
            table(&self.divisions[2]),
            table(&self.divisions[3]),
        ];
        let before: Vec<(String, usize)> = self
            .divisions
            .iter()
            .enumerate()
            .flat_map(|(d, v)| v.iter().map(move |c| (c.name.clone(), d)))
            .chain(self.pool.iter().map(|c| (c.name.clone(), 4)))
            .collect();
        end_of_season(&mut self.rng, &mut self.divisions, &mut self.pool, None);
        let after: Vec<(String, usize)> = self
            .divisions
            .iter()
            .enumerate()
            .flat_map(|(d, v)| v.iter().map(move |c| (c.name.clone(), d)))
            .chain(self.pool.iter().map(|c| (c.name.clone(), 4)))
            .collect();
        let moves = before
            .iter()
            .filter_map(|(name, from)| {
                let to = after.iter().find(|(n, _)| n == name).map(|(_, d)| *d)?;
                (to != *from).then(|| Move {
                    name: name.clone(),
                    from: *from,
                    to,
                })
            })
            .collect();
        self.last_season = Some(SeasonSummary {
            year: self.year,
            tables,
            moves,
        });
        self.year += 1;
        self.round = 0;
    }

    pub fn play_season(&mut self) {
        loop {
            self.play_round();
            if self.round == 0 {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_world_has_four_divisions_of_eight_and_a_pool() {
        let w = World::synthetic(1);
        assert!(w.divisions.iter().all(|d| d.len() == 8));
        assert_eq!(w.pool.len(), 6);
        assert!(w.divisions.iter().flatten().all(|c| c.slots.len() == 20));
    }

    fn fake_team(i: usize, level: u8) -> EftTeam {
        use crate::eft::EftPlayer;
        let mut players = Vec::new();
        for (pos, n) in [
            (Position::Goalkeeper, 2),
            (Position::Defender, 6),
            (Position::Midfielder, 6),
            (Position::Attacker, 5),
        ] {
            for k in 0..n {
                players.push(EftPlayer {
                    nationality: "POR".into(),
                    name: format!("Jogador {i}-{k}-{}", pos as u8),
                    position: pos,
                });
            }
        }
        EftTeam {
            full_name: format!("Clube {i}"),
            short_name: format!("Clube {}", i % 40),
            background: [255; 3],
            foreground: [0; 3],
            country: "POR".into(),
            level,
            players,
            coach: "Treinador".into(),
        }
    }

    #[test]
    fn real_teams_fill_divisions_by_level_and_a_season_runs() {
        let teams: Vec<EftTeam> = (0..45).map(|i| fake_team(i, 1 + (i % 20) as u8)).collect();
        let mut w = World::from_teams(5, &teams).unwrap();
        assert!(w.divisions.iter().all(|d| d.len() == 8));
        assert_eq!(w.pool.len(), 13);
        // duplicate short names were made unique, so no club is lost when tracking moves
        let mut names: Vec<&String> = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .map(|c| &c.name)
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 45);
        let mean = |d: &Vec<Club>| {
            d.iter()
                .map(|c| {
                    c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len() as f64
                })
                .sum::<f64>()
                / 8.0
        };
        assert!(mean(&w.divisions[0]) > mean(&w.divisions[3]));
        w.play_season();
        assert_eq!(w.year, 1999);
    }

    #[test]
    fn real_teams_are_validated() {
        let teams: Vec<EftTeam> = (0..10).map(|i| fake_team(i, 5)).collect();
        assert!(World::from_teams(1, &teams)
            .unwrap_err()
            .contains("need at least"));
        let mut teams: Vec<EftTeam> = (0..40).map(|i| fake_team(i, 5)).collect();
        teams[3]
            .players
            .retain(|p| p.position != Position::Goalkeeper);
        assert!(World::from_teams(1, &teams)
            .unwrap_err()
            .contains("cannot field"));
    }

    /// With the shipped game files present (git-ignored) several seasons of real teams must run.
    #[test]
    fn shipped_teams_play_seasons_if_available() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original/elifoot98/EQUIPAS");
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let teams: Vec<EftTeam> = rd
            .flatten()
            .filter_map(|e| crate::eft::parse(&std::fs::read(e.path()).ok()?).ok())
            .collect();
        assert!(teams.len() > 100);
        let mut w = World::from_teams(1998, &teams).unwrap();
        w.play_season();
        w.play_season();
        assert_eq!(w.year, 2000);
    }

    #[test]
    fn a_season_runs_and_rolls_over() {
        let mut w = World::synthetic(2);
        for r in 1..=13 {
            w.play_round();
            assert_eq!(w.round, r);
            assert_eq!(w.last_round.len(), 16); // 4 divisions x 4 games
        }
        w.play_round();
        assert_eq!((w.round, w.year), (0, 1999));
        let summary = w.last_season.as_ref().unwrap();
        assert_eq!(summary.year, 1998);
        // each table has every club with 14 games played
        for t in &summary.tables {
            assert_eq!(t.len(), 8);
            assert!(t.iter().all(|r| r.wins + r.draws + r.losses == 14));
            assert!(t.windows(2).all(|p| p[0].points >= p[1].points));
        }
        // 3 swaps of two clubs each way plus two outside clubs in and out: 6*2 + 4 moves
        assert_eq!(summary.moves.len(), 16);
        assert!(w.divisions.iter().all(|d| d.len() == 8));
        assert!(w.divisions.iter().flatten().all(|c| c.points == 0));
    }

    #[test]
    fn many_seasons_keep_the_world_consistent() {
        let mut w = World::synthetic(3);
        for _ in 0..20 {
            w.play_season();
        }
        assert_eq!(w.year, 2018);
        let mut names: Vec<&String> = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .map(|c| &c.name)
            .collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 38, "no club is lost or duplicated");
        assert!(w
            .divisions
            .iter()
            .flatten()
            .all(|c| c.slots.iter().all(|s| (1..=50).contains(&s.strength))));
    }
}
