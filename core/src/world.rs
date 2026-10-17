//! The whole league world: four divisions of eight clubs, an outside pool, the year and the round.
//!
//! Verified: division base levels `52, 38, 24, 10` (`1010:0b60` sets `division+4 = index * -14 + 52`; the outside
//! pool is index 4 with base -4 and is never evolved), 14 league rounds, evolution after every round, and the
//! season end of `1020:038e`. **New-game rules (verified):** teams are ordered by EFT level descending, ties in file-name order
//! (`1028:002a` loops the level from 20 to 1), cut into divisions of eight (`1028:0544`: division `min(4, i/8)`), and the
//! starting strength of every player is `clamp(base(i) + 3 - Random(7), 1, 50)` with `base(i) = max(5, round((1 - i/32) * 50) - 1)`
//! for the first 33 teams and 1 for the rest (`1028:0373`). **Assumed:** squad composition of the fictional world
//! (2 GK, 7 DEF, 6 MID, 5 ATT), fictional names, no cup (so only the champion's `K += 1.0` applies).
use crate::economy::{walk_index, INITIAL_INDEX};
use crate::eft::{EftTeam, Position};
use crate::rng::DelphiRng;
use crate::season::apply_plan;
use crate::season::{
    best_formation, end_of_round, end_of_season, plan_for_formation, play_round, standings, Club,
    MatchRecord, Plan, FORMATIONS,
};
use crate::sector::{Slot, Status};
use std::collections::HashMap;

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

/// The club the user manages: found by name (it changes division at season ends), with the chosen formation and plan.
#[derive(Clone, Debug)]
pub struct Human {
    pub club: String,
    /// Index into `FORMATIONS`.
    pub formation: usize,
    pub plan: Plan,
}

#[derive(Debug)]
pub struct World {
    pub divisions: [Vec<Club>; 4],
    pub pool: Vec<Club>,
    pub year: u16,
    /// Global economy index (`10c8:2b46`), random-walked every round.
    pub economy: f64,
    /// Rounds already played this season (0..=13); a new season starts after round 14.
    pub round: usize,
    pub rng: DelphiRng,
    /// Matches of the last round played, tagged with their division.
    pub last_round: Vec<(usize, MatchRecord)>,
    pub last_season: Option<SeasonSummary>,
    pub human: Option<Human>,
}

fn player_name(rng: &mut DelphiRng) -> String {
    format!(
        "{} {}",
        FIRST[rng.random(FIRST.len() as u16) as usize],
        LAST[rng.random(LAST.len() as u16) as usize]
    )
}

fn make_club(rng: &mut DelphiRng, name: String, base: i32) -> Club {
    let mut slots = Vec::new();
    let mut names = Vec::new();
    for (position, count) in [(0u8, 2), (1, 7), (2, 6), (3, 5)] {
        for _ in 0..count {
            let n = player_name(rng);
            let strength = player_strength(rng, base);
            slots.push(Slot::new(&n, position, strength, true, Status::Out));
            names.push(n);
        }
    }
    let mut club = Club::new(&name, slots);
    club.names = names;
    club
}

/// `1028:0373`: base strength of the team at position `i` of the master list (sorted by level, best first):
/// `max(5, round((1 - i/32) * 50) - 1)` for `i <= 32`, else 1 (`FRNDINT` rounds half to even).
pub fn base_strength(i: usize) -> i32 {
    if i > 32 {
        return 1;
    }
    let g = (1.0 - i as f64 / 32.0) * 50.0; // (c3 - i / c1 / c2) * c4 with c1 = 4, c2 = 8, c3 = 1, c4 = 50
    (g.round_ties_even() as i32 - 1).max(5)
}

/// Player strength for a team of base `base`: `clamp(base + 3 - Random(7), 1, 50)`.
fn player_strength(rng: &mut DelphiRng, base: i32) -> u16 {
    (base + 3 - rng.random(7) as i32).clamp(1, 50) as u16
}

/// The game accepts 34 to 128 teams (`1028:18d9`: the OK button needs more than `0x21`, and it warns above `0x80`).
pub const MIN_TEAMS: usize = 34;
pub const MAX_TEAMS: usize = 128;

/// Builds a club from a `.EFT` team: real player names and positions, strength from the team's list position
/// (`base`). Errors when no formation of the game can be fielded.
fn club_from_eft(
    rng: &mut DelphiRng,
    t: &EftTeam,
    name: String,
    base: i32,
) -> Result<Club, String> {
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
        let strength = player_strength(rng, base);
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
    /// A world made of real teams. **Pass the teams in directory order** (file names, case-insensitive): like the original,
    /// the master list is ordered by level descending and ties keep that order. At most 128 teams are used.
    /// The first 32 fill the four divisions (8 each), the rest form the outside pool. Verified against a real save, where the
    /// 52 teams appear in strictly descending level order and the 4th division is the fourth group of eight.
    pub fn from_teams(seed: u32, teams: &[EftTeam]) -> Result<World, String> {
        if teams.len() < MIN_TEAMS {
            return Err(format!(
                "need at least {MIN_TEAMS} teams, got {}",
                teams.len()
            ));
        }
        let mut rng = DelphiRng::new(seed);
        let mut order: Vec<usize> = (0..teams.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(teams[i].level)); // stable: ties keep the given (directory) order
        order.truncate(MAX_TEAMS);
        let mut used = std::collections::HashSet::new();
        let mut clubs = Vec::new();
        for (pos, &i) in order.iter().enumerate() {
            let mut name = teams[i].short_name.trim().to_string();
            let mut n = 2;
            while !used.insert(name.clone()) {
                name = format!("{} ({n})", teams[i].short_name.trim());
                n += 1;
            }
            clubs.push(club_from_eft(
                &mut rng,
                &teams[i],
                name,
                base_strength(pos),
            )?);
        }
        let mut it = clubs.into_iter();
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for d in divisions.iter_mut() {
            *d = it.by_ref().take(8).collect();
        }
        let pool: Vec<Club> = it.collect();
        let mut w = World {
            divisions,
            pool,
            year: 1998,
            economy: INITIAL_INDEX,
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
        };
        w.pick_human();
        Ok(w)
    }

    /// A fictional world with the original's starting strengths: the club at list position `i` (division `i / 8`) gets
    /// `base_strength(i)`, so the first division starts around 49..38 and the fourth around 11..5.
    pub fn synthetic(seed: u32) -> World {
        let mut rng = DelphiRng::new(seed);
        let mut n = 0;
        let mut next_name = || {
            let i = n;
            n += 1;
            format!("{} {}", KINDS[i % KINDS.len()], CITIES[i % CITIES.len()])
        };
        let mut divisions: [Vec<Club>; 4] = Default::default();
        for (d, division) in divisions.iter_mut().enumerate() {
            *division = (0..8)
                .map(|j| make_club(&mut rng, next_name(), base_strength(d * 8 + j)))
                .collect();
        }
        let pool = (0..6)
            .map(|i| make_club(&mut rng, next_name(), base_strength(32 + i)))
            .collect();
        let mut w = World {
            divisions,
            pool,
            year: 1998,
            economy: INITIAL_INDEX,
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
        };
        w.pick_human();
        w
    }

    /// Where the user's club is: `(division 0..=3, index)`, or `None` when it is in the outside pool.
    pub fn human_location(&self) -> Option<(usize, usize)> {
        let name = &self.human.as_ref()?.club;
        self.divisions
            .iter()
            .enumerate()
            .find_map(|(d, v)| v.iter().position(|c| &c.name == name).map(|i| (d, i)))
    }

    /// Makes `club` of `division` the user's club with the automatic line-up.
    pub fn set_human(&mut self, division: usize, club: usize) {
        let c = &self.divisions[division][club];
        let formation = best_formation(&c.slots).unwrap_or(0);
        let plan = plan_for_formation(&c.slots, FORMATIONS[formation]).unwrap_or_default();
        self.human = Some(Human {
            club: c.name.clone(),
            formation,
            plan,
        });
    }

    /// The original's start rule (`1028:0b26`): a random one of the bottom six clubs of the 4th division (`7 - Random(6)`).
    pub fn pick_human(&mut self) {
        let club = 7 - self.rng.random(6) as usize;
        self.set_human(3, club);
    }

    /// Writes the line-up the game would actually field next round back into the user's plan (injured, suspended or
    /// missing players replaced), so the displayed plan is the truth. Also updates the formation label.
    fn refresh_plan(&mut self) {
        let Some((d, i)) = self.human_location() else {
            return;
        };
        let Some(h) = self.human.as_mut() else { return };
        let mut slots = self.divisions[d][i].slots.clone();
        apply_plan(&mut slots, &h.plan);
        let with = |st: Status| -> Vec<usize> {
            (0..slots.len())
                .filter(|&k| slots[k].status == st)
                .collect()
        };
        h.plan.starters = with(Status::Starter);
        h.plan.bench = with(Status::Sub);
        let count = |pos: u8| {
            h.plan
                .starters
                .iter()
                .filter(|&&k| slots[k].position == pos)
                .count() as u8
        };
        if let Some(k) = FORMATIONS
            .iter()
            .position(|&f| f == (count(1), count(2), count(3)))
        {
            h.formation = k;
        }
    }

    /// Picks the best line-up for formation number `formation` (index into `FORMATIONS`); false when it cannot be fielded.
    pub fn set_formation(&mut self, formation: usize) -> bool {
        let Some((d, i)) = self.human_location() else {
            return false;
        };
        let Some(&f) = FORMATIONS.get(formation) else {
            return false;
        };
        match plan_for_formation(&self.divisions[d][i].slots, f) {
            Some(plan) => {
                if let Some(h) = self.human.as_mut() {
                    h.formation = formation;
                    h.plan = plan;
                }
                true
            }
            None => false,
        }
    }

    /// Replaces the user's plan (starters, bench and up to three planned substitutions).
    pub fn set_plan(&mut self, plan: Plan) {
        if let Some(h) = self.human.as_mut() {
            h.plan = plan;
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
            let mut plans = HashMap::new();
            if let (Some((hd, hi)), Some(h)) = (self.human_location(), self.human.as_ref()) {
                if hd == d {
                    plans.insert(hi, h.plan.clone());
                }
            }
            let records = play_round(
                &mut self.rng,
                &mut self.divisions[d],
                round,
                &mut names,
                &plans,
            );
            self.last_round.extend(records.into_iter().map(|r| (d, r)));
            end_of_round(&mut self.rng, &mut self.divisions[d], DIVISION_BASE[d]);
        }
        self.economy = walk_index(&mut self.rng, self.economy, self.year);
        if let Some(h) = self.human.as_mut() {
            h.plan.subs.clear(); // planned substitutions are for one round only
        }
        self.refresh_plan();
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
        if self.human_location().is_none() {
            self.pick_human(); // the user's club left the league: the game hands them another one (`4f40`)
        }
        self.refresh_plan();
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

    #[test]
    fn starting_strengths_follow_the_list_position() {
        // computed from the binary's constants (1028:0373): max(5, round((1 - i/32) * 50) - 1), then 1 beyond i = 32
        let expect = [
            49, 47, 46, 44, 43, 41, 40, 38, 37, 35, 33, 32, 30, 29, 27, 26, 24, 22, 21, 19, 18, 16,
            15, 13, 11, 10, 8, 7, 5, 5, 5, 5, 5, 1, 1, 1,
        ];
        for (i, e) in expect.iter().enumerate() {
            assert_eq!(base_strength(i), *e, "position {i}");
        }
        // the real save: Figueirense is the 30th team (index 29) and its squad averaged 5.4
        assert_eq!(base_strength(29), 5);
    }

    #[test]
    fn division_assignment_matches_the_real_save() {
        // levels of the 52 teams of a real save in file order (pre-match save): strictly descending, groups of 8
        let levels = [
            16u8, 14, 14, 13, 13, 13, 12, 12, 12, 12, 12, 11, 10, 10, 9, 9, 9, 9, 8, 7, 7, 7, 6, 6,
            6, 6, 5, 5, 5, 5, 5, 5, 5, 4, 4, 4, 3, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        // shuffle the input: the world must sort it back by level (stable for ties)
        let mut teams: Vec<EftTeam> = levels
            .iter()
            .enumerate()
            .map(|(i, &l)| fake_team(i, l))
            .collect();
        teams.reverse();
        teams.sort_by_key(|t| t.level); // not descending on purpose
        let w = World::from_teams(1, &teams).unwrap();
        let mean_level = |d: &Vec<Club>| d.len();
        assert_eq!(mean_level(&w.divisions[3]), 8);
        assert_eq!(w.pool.len(), 52 - 32);
        // 4th division = positions 24..31 of the sorted list = levels 6,6,5,5,5,5,5,5
        let d4: Vec<f64> = w.divisions[3]
            .iter()
            .map(|c| c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len() as f64)
            .collect();
        assert!(d4.iter().all(|m| (3.0..=12.5).contains(m)), "{d4:?}");
        // first division starts strong
        let mins: Vec<u16> = w.divisions[0]
            .iter()
            .map(|c| c.slots.iter().map(|s| s.strength).min().unwrap())
            .collect();
        assert!(mins.iter().all(|&m| m >= 35), "{mins:?}");
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

    #[test]
    fn the_user_starts_in_the_bottom_six_of_the_fourth_division() {
        for seed in 1..40 {
            let w = World::synthetic(seed);
            let (d, i) = w.human_location().unwrap();
            assert_eq!(d, 3);
            assert!((2..=7).contains(&i), "seed {seed} gave index {i}");
            assert_eq!(w.human.as_ref().unwrap().plan.starters.len(), 11);
        }
    }

    #[test]
    fn the_user_can_change_formation_and_play_with_a_planned_substitution() {
        use crate::season::PlannedSub;
        let mut w = World::synthetic(7);
        let (d, i) = w.human_location().unwrap();
        assert!(w.set_formation(2)); // 3-4-3
        let plan = w.human.as_ref().unwrap().plan.clone();
        let count = |pos: u8| {
            plan.starters
                .iter()
                .filter(|&&k| w.divisions[d][i].slots[k].position == pos)
                .count()
        };
        assert_eq!((count(1), count(2), count(3)), (3, 4, 3));
        // plan one substitution: the first starter outfield player for the first outfield bench player
        let out = *plan
            .starters
            .iter()
            .find(|&&k| w.divisions[d][i].slots[k].position == 3)
            .unwrap();
        let inn = *plan
            .bench
            .iter()
            .find(|&&k| w.divisions[d][i].slots[k].position != 0)
            .unwrap();
        let mut p = plan.clone();
        p.subs.push(PlannedSub {
            minute: 46,
            out,
            inn,
        });
        w.set_plan(p);
        w.play_round();
        assert!(
            w.human.as_ref().unwrap().plan.subs.is_empty(),
            "planned subs are consumed"
        );
        // the human club's match was played with its plan: it appears in the last round
        let name = w.human.as_ref().unwrap().club.clone();
        let (d2, i2) = w.human_location().unwrap();
        assert!(
            w.last_round
                .iter()
                .any(|(dd, r)| *dd == d2 && (r.home == i2 || r.away == i2)),
            "{name} played"
        );
        assert!(!w.set_formation(99));
    }

    #[test]
    fn the_user_keeps_a_club_across_seasons() {
        let mut w = World::synthetic(8);
        for _ in 0..6 {
            w.play_season();
            assert!(
                w.human_location().is_some(),
                "the user always has a club in the league"
            );
        }
    }

    #[test]
    fn the_displayed_plan_is_repaired_after_a_round() {
        let mut w = World::synthetic(21);
        for _ in 0..5 {
            w.play_round();
            let h = w.human.as_ref().unwrap();
            let (d, i) = w.human_location().unwrap();
            let slots = &w.divisions[d][i].slots;
            assert_eq!(h.plan.starters.len(), 11);
            assert!(
                h.plan.starters.iter().all(|&k| slots[k].available()),
                "an unavailable player is still listed"
            );
            assert!(h.plan.bench.iter().all(|&k| slots[k].available()));
            assert!(h.plan.bench.len() <= 5);
        }
    }
}
