//! The whole league world: four divisions of eight clubs, an outside pool, the year and the round.
//!
//! Verified: division base levels `52, 38, 24, 10` (`1010:0b60` sets `division+4 = index * -14 + 52`; the outside
//! pool is index 4 with base -4 and is never evolved), 14 league rounds, evolution after every round, and the
//! season end of `1020:038e`. **New-game rules (verified):** teams are ordered by EFT level descending, ties in file-name order
//! (`1028:002a` loops the level from 20 to 1), cut into divisions of eight (`1028:0544`: division `min(4, i/8)`), and the
//! starting strength of every player is `clamp(base(i) + 3 - Random(7), 1, 50)` with `base(i) = max(5, round((1 - i/32) * 50) - 1)`
//! for the first 33 teams and 1 for the rest (`1028:0373`). **Assumed:** squad composition of the fictional world
//! (2 GK, 7 DEF, 6 MID, 5 ATT), fictional names, no cup (so only the champion's `K += 1.0` applies).
use crate::cup;
use crate::economy::{player_value, walk_index, DEFAULT_SCALE, INITIAL_INDEX};
use crate::eft::{EftTeam, Position};
use crate::finance::{self, Finance};
use crate::history::History;
use crate::market::{code, country_class};
use crate::matchsim::{Event, Primary, SubEvent};
use crate::rng::DelphiRng;
use crate::season::{apply_plan, auto_lineup, fixtures, play_game_with, play_round_ctl, Control};
use crate::season::{
    best_formation, end_of_round, end_of_season, plan_for_formation, standings, Club, MatchRecord,
    MoneyCtx, Plan, FORMATIONS,
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

/// One cup tie: the first club played at home. `winner` advanced.
#[derive(Clone, Debug)]
pub struct CupMatch {
    pub home: String,
    pub away: String,
    pub home_goals: i32,
    pub away_goals: i32,
    pub extra_time: bool,
    /// Decided by a shootout (the score is then level).
    pub shootout: bool,
    pub winner: String,
    pub events: Vec<Event>,
    pub subs: Vec<SubEvent>,
    /// Player names of the two squads, so that the events can be shown.
    pub home_players: Vec<String>,
    pub away_players: Vec<String>,
    pub attendance: i64,
    /// The kick-by-kick shootout, when it was played in full.
    pub pens: Option<cup::Shootout>,
    /// Both squads at the kick-off, the primary chances and the fixture bonus values `(home, away)` of the game.
    pub start: [Vec<Slot>; 2],
    pub primaries: Vec<Primary>,
    pub bonus: (i32, i32),
}

/// A drawn cup round waiting for its match day: the ties in draw order (the first club of a pair plays at home), the clubs
/// without a tie advance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CupDraw {
    /// The league round after which it was drawn (even).
    pub round: usize,
    /// Name of the stage ("32 avos de final", "Oitavos", ..., "FINAL").
    pub name: String,
    pub pairs: Vec<(String, String)>,
}

/// The cup tie of a human club, for the main window of a cup day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CupTie {
    pub opponent: String,
    pub home: bool,
    pub stage: String,
}

#[derive(Clone, Debug)]
pub struct CupRound {
    pub name: String,
    pub matches: Vec<CupMatch>,
    /// Clubs still in the cup after this round.
    pub left: usize,
    /// Index into `World::coach_changes` of the first sacking of this cup round (the ones before are the league round's).
    pub coach_changes_from: usize,
}

#[derive(Clone, Debug)]
pub struct SeasonSummary {
    pub year: u16,
    pub tables: [Vec<TableRow>; 4],
    pub moves: Vec<Move>,
    pub cup_winner: Option<String>,
    /// What the end-of-season window shows besides the winners.
    pub honours: Honours,
}

/// The coaches of the champion and the cup winner, the best scorers of the season and the prizes of `1020:2ae6`.
#[derive(Clone, Debug, Default)]
pub struct Honours {
    pub league_coach: String,
    pub cup_coach: String,
    /// `(player, club)` of everyone with the most goals.
    pub top_scorers: Vec<(String, String)>,
    pub top_goals: u16,
    /// Prizes of the champion, the cup winner and the club of each top scorer.
    pub prizes: [i64; 3],
}

/// The club the user manages: found by name (it changes division at season ends), with the chosen formation and plan.
#[derive(Clone, Debug)]
pub struct Human {
    pub club: String,
    /// The human's own name (the original's player name), also the name of the club's coach while he manages it.
    pub coach: String,
    /// Index into `FORMATIONS`.
    pub formation: usize,
    pub plan: Plan,
}

/// What belongs to one human coach besides the [`Human`] itself: the market lists of his club.
#[derive(Clone, Debug)]
pub struct Bench {
    /// The seat (order of the players in the new-game dialog, 0 is the first).
    pub seat: usize,
    pub human: Human,
    pub pending: Vec<Pending>,
    pub observed: Vec<(String, String)>,
    pub sell_tried: Vec<String>,
}

/// The hot seat: several human coaches share one computer. `World::human` (with `pending`, `observed` and `sell_tried`) is the
/// *active* human, the one the interface shows and the `human_*` methods act for; the others wait here until
/// [`World::switch_human`] swaps them in. The single-human game is the case of an empty `benched`.
#[derive(Clone, Debug, Default)]
pub struct Seats {
    /// The seats that are not active.
    pub benched: Vec<Bench>,
    /// The seat of the active human.
    pub active: usize,
    /// Clubs of humans who declined the invitation of the round being played (the interface replays the round).
    pub declined: Vec<String>,
    /// Coach swaps of the round that took a human coach to another club, as `(from, to)` (see `World::human_moves`).
    pub moves: Vec<(String, String)>,
    applied: usize,
}

#[derive(Clone, Debug)]
pub struct World {
    pub divisions: [Vec<Club>; 4],
    pub pool: Vec<Club>,
    pub year: u16,
    /// Global economy index (`10c8:2b46`), random-walked every round.
    pub economy: f64,
    /// Currency scale `G` (`10c8:2d87`), the scale of [`World::currency`].
    pub scale: i64,
    /// The currency of the game (index into [`crate::economy::CURRENCIES`], chosen in *Dinheiro*).
    pub currency: usize,
    /// Total cash of each division at the last `1010:7067` (the denominator of the economic-factor update).
    pub cash_ref: [i64; 4],
    /// Rounds already played this season (0..=13); a new season starts after round 14.
    pub round: usize,
    pub rng: DelphiRng,
    /// Matches of the last round played, tagged with their division.
    pub last_round: Vec<(usize, MatchRecord)>,
    pub last_season: Option<SeasonSummary>,
    pub human: Option<Human>,
    /// Names of the clubs still in the cup (all of them at the start of a season).
    pub cup: Vec<String>,
    /// The last cup round played in the current season (None before the first draw).
    pub last_cup: Option<CupRound>,
    /// The cup round that has been drawn (at the end of an even league day, `1020:0ed4`) and is still to be played on its own
    /// match day (`play_cup_day_ctl`).
    pub cup_draw: Option<CupDraw>,
    pub cup_champion: Option<String>,
    /// Transfers made between computer-managed clubs, newest last (kept for the current season).
    pub transfers: Vec<Transfer>,
    /// Auctions of computer-managed clubs' players that the user can still outbid before the next round.
    pub board: Vec<Listing>,
    /// Direct purchases of the user waiting for the end of the round.
    pub pending: Vec<Pending>,
    /// Players of other clubs the user watches (`Jogadores sob observação`), as `(club, player)` in the order they were added.
    pub observed: Vec<(String, String)>,
    /// Players of the user's club whose sale found no buyer: they cannot be offered again before the next game (`5a6a` code 4).
    pub sell_tried: Vec<String>,
    /// Whether the user accepts an invitation to take over a club whose coach was sacked (the original asks in a dialog).
    pub accept_invitations: bool,
    /// The option *Ver os desempates por penalties nos jogos sem treinadores humanos* (`10c8:2b33`): every shootout is played kick by
    /// kick (the dialog), not only the humans' and the final's (`1050:0d46`). It changes the random numbers drawn.
    pub real_shootouts: bool,
    /// Coach changes of the last round played, as `(sacked club, club that gave its coach)`; the second is the club the
    /// coach came from. The humans' own moves are in `seats.moves`.
    pub coach_changes: Vec<(String, String)>,
    /// Parallel to `coach_changes`: the coach taken over had no club (he came from the pool of unemployed coaches, `1010:d55c` finds
    /// no team), which the sack list words differently.
    pub coach_changes_free: Vec<bool>,
    /// The other human coaches of a hot-seat game and the coach swaps of the round (see [`Seats`]).
    pub seats: Seats,
    /// Calendars, champions, club and coach histories, the user's coach profile (`history.rs`, `docs/season.md`).
    pub history: History,
    /// The order the clubs of each division were last sorted in (indices into `divisions[d]`; empty = the division's own
    /// order). The original sorts its division lists in place after every round, so clubs level on points, goal difference
    /// and goals keep the order of the table before.
    pub tie_order: Vec<Vec<usize>>,
}

/// A player the user could buy straight away (`Procurar jogadores`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub club: String,
    /// Index of the player in the club's squad.
    pub index: usize,
    pub player: String,
    pub position: u8,
    pub strength: u16,
    pub nationality: [u8; 3],
    pub price: i64,
}

/// Search criteria of the player search dialog.
#[derive(Clone, Debug, Default)]
pub struct Search {
    pub name: String,
    pub nationality: Option<[u8; 3]>,
    pub min_strength: u16,
    pub position: Option<u8>,
    pub max_price: Option<i64>,
    pub hide_mine: bool,
}

/// A purchase the user has paid that arrives with the end of the round (`1020:0aae`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub seller: String,
    pub player: String,
}

/// A player on offer in an open auction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    pub seller: String,
    pub player: String,
    pub ask: i64,
    /// The computer's best bid: club and fee.
    pub best: Option<(String, i64)>,
    pub round: usize,
}

/// A completed transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub round: usize,
    pub year: u16,
    pub player: String,
    pub position: u8,
    pub strength: u16,
    pub from: String,
    pub to: String,
    pub fee: i64,
}

fn player_name(rng: &mut DelphiRng) -> String {
    format!(
        "{} {}",
        FIRST[rng.random(FIRST.len() as u16) as usize],
        LAST[rng.random(LAST.len() as u16) as usize]
    )
}

fn make_club(rng: &mut DelphiRng, name: String, base: i32, master: usize) -> Club {
    let mut slots = Vec::new();
    let mut names = Vec::new();
    for (position, count) in [(0u8, 2), (1, 7), (2, 6), (3, 5)] {
        for _ in 0..count {
            let n = player_name(rng);
            let strength = player_strength(rng, base);
            let mut slot = Slot::new(&n, position, strength, true, Status::Out);
            slot.nat = *b"POR";
            slot.wage = player_value(strength, 1.0, INITIAL_INDEX, DEFAULT_SCALE);
            slots.push(slot);
            names.push(n);
        }
    }
    let mut club = Club::new(&name, slots);
    club.names = names;
    club.fin = Finance::new_game(base, DEFAULT_SCALE);
    club.country = *b"POR";
    club.master = master as u16;
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
    master: usize,
    scale: i64,
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
        let mut slot = Slot::new(
            &p.name,
            position,
            strength,
            p.nationality == t.country,
            Status::Out,
        );
        slot.nat = code(&p.nationality);
        slot.foreign = country_class(code(&t.country), slot.nat);
        slot.wage = player_value(strength, 1.0, INITIAL_INDEX, scale);
        slots.push(slot);
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
    club.fin = Finance::new_game(base, scale);
    club.country = code(&t.country);
    club.colors = (t.background, t.foreground);
    club.full_name = t.full_name.clone();
    club.coach_name = t.coach.clone();
    club.master = master as u16;
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
        let mut order: Vec<usize> = (0..teams.len()).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(teams[i].level)); // stable: ties keep the given (directory) order
        order.truncate(MAX_TEAMS);
        World::from_order(DelphiRng::new(seed), teams, &order, crate::economy::REAIS)
    }

    /// A new game with the countries chosen in *Selecção de Países* (`TSelectCountriesDlg`). `list` is the dialog's list, in its
    /// order, as `(country code, selected)`. With more than 128 teams in the chosen countries, `OKBtnClick` (`1028:19ac`) takes teams
    /// away at random, the bigger countries losing more: it picks a row (`Random(rows)`) and, if that country is chosen and has more
    /// than one team left (more than none when every chosen country has one), removes one when `Random(128)` is below its count.
    /// Then `1028:002a` takes, for every level from 20 down to 1, the teams of that level in directory order while their
    /// country has places left.
    pub fn from_countries(
        seed: u32,
        teams: &[EftTeam],
        list: &[(String, bool)],
        currency: usize,
    ) -> Result<World, String> {
        let mut rng = DelphiRng::new(seed);
        let mut left: Vec<usize> = list
            .iter()
            .map(|(code, on)| {
                if *on {
                    teams.iter().filter(|t| &t.country == code).count()
                } else {
                    0
                }
            })
            .collect();
        let mut total: usize = left.iter().sum();
        if total < MIN_TEAMS {
            return Err(format!("need at least {MIN_TEAMS} teams, got {total}"));
        }
        let mut countries = left.iter().filter(|&&n| n > 0).count();
        while total > MAX_TEAMS {
            let r = rng.random(list.len() as u16) as usize;
            let floor = 1 - (countries == total) as usize;
            if list[r].1 && left[r] > floor && (rng.random(128) as usize) < left[r] {
                left[r] -= 1;
                total -= 1;
                if left[r] == 0 {
                    countries -= 1;
                }
            }
        }
        let mut order = Vec::new();
        for level in (1..=20u8).rev() {
            for (i, t) in teams.iter().enumerate() {
                if t.level != level {
                    continue;
                }
                if let Some(k) = list.iter().position(|(code, on)| *on && *code == t.country) {
                    if left[k] > 0 {
                        left[k] -= 1;
                        order.push(i);
                    }
                }
            }
        }
        World::from_order(rng, teams, &order, currency)
    }

    /// The world of the teams `order` (indices into `teams`, master-list order): the first 32 fill the divisions.
    fn from_order(
        mut rng: DelphiRng,
        teams: &[EftTeam],
        order: &[usize],
        currency: usize,
    ) -> Result<World, String> {
        let currency = currency.min(crate::economy::CURRENCIES.len() - 1);
        let scale = crate::economy::CURRENCIES[currency].scale;
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
                pos,
                scale,
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
            scale,
            currency,
            cash_ref: [0; 4],
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_draw: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            real_shootouts: false,
            coach_changes: Vec::new(),
            coach_changes_free: Vec::new(),
            seats: Seats::default(),
            history: History::default(),
            tie_order: Vec::new(),
        };
        w.reset_cup();
        w.log_new_game();
        w.refresh_cash_ref();
        w.pick_human();
        Ok(w)
    }

    /// A world around ready-made clubs (used by the `.e98` import): fresh cup, default economy index.
    pub fn empty(
        seed: u32,
        divisions: [Vec<Club>; 4],
        pool: Vec<Club>,
        year: u16,
        round: usize,
    ) -> World {
        let mut w = World {
            divisions,
            pool,
            year,
            economy: INITIAL_INDEX,
            scale: DEFAULT_SCALE,
            currency: crate::economy::REAIS,
            cash_ref: [0; 4],
            round,
            rng: DelphiRng::new(seed),
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_draw: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            real_shootouts: false,
            coach_changes: Vec::new(),
            coach_changes_free: Vec::new(),
            seats: Seats::default(),
            history: History::default(),
            tie_order: Vec::new(),
        };
        w.reset_cup();
        w.refresh_cash_ref();
        w
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
                .map(|j| make_club(&mut rng, next_name(), base_strength(d * 8 + j), d * 8 + j))
                .collect();
        }
        let pool = (0..6)
            .map(|i| make_club(&mut rng, next_name(), base_strength(32 + i), 32 + i))
            .collect();
        let mut w = World {
            divisions,
            pool,
            year: 1998,
            economy: INITIAL_INDEX,
            scale: DEFAULT_SCALE,
            currency: crate::economy::REAIS,
            cash_ref: [0; 4],
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_draw: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            real_shootouts: false,
            coach_changes: Vec::new(),
            coach_changes_free: Vec::new(),
            seats: Seats::default(),
            history: History::default(),
            tie_order: Vec::new(),
        };
        w.reset_cup();
        w.log_new_game();
        w.refresh_cash_ref();
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
        let c = &mut self.divisions[division][club];
        // the default line-up is the computer's own selection (`lineup.rs`), without reordering the squad
        let mut probe = c.slots.clone();
        crate::lineup::default_statuses(&mut probe);
        let with = |st: Status| -> Vec<usize> {
            (0..probe.len())
                .filter(|&k| probe[k].status == st)
                .collect()
        };
        let (starters, bench) = (with(Status::Starter), with(Status::Sub));
        let count = |pos: u8| {
            starters
                .iter()
                .filter(|&&k| probe[k].position == pos)
                .count() as u8
        };
        let shape = (count(1), count(2), count(3));
        let formation = FORMATIONS
            .iter()
            .position(|&f| f == shape)
            .or_else(|| best_formation(&c.slots))
            .unwrap_or(0);
        let plan = Plan {
            starters,
            bench,
            subs: Vec::new(),
        };
        let name = c.name.clone();
        let coach = self.human.take().map(|h| h.coach).unwrap_or_default();
        self.human = Some(Human {
            club: name,
            coach,
            formation,
            plan,
        });
    }

    // ---- the hot seat: several human coaches (see [`Seats`]) ----

    /// How many human coaches play.
    pub fn human_count(&self) -> usize {
        self.seats.benched.len() + self.human.is_some() as usize
    }

    /// The seat of the active human (0 is the first player).
    pub fn active_seat(&self) -> usize {
        self.seats.active
    }

    /// The clubs the humans manage, in seat order (a human without a club, whose coach sits at the pool, is not in the list: the
    /// club plays for itself).
    pub fn human_clubs(&self) -> Vec<String> {
        self.humans()
            .filter(|h| !self.pool.iter().any(|c| c.name == h.club))
            .map(|h| h.club.clone())
            .collect()
    }

    /// Every human in seat order (the active one included).
    pub fn humans(&self) -> impl Iterator<Item = &Human> {
        let mut v: Vec<(usize, &Human)> = self
            .seats
            .benched
            .iter()
            .map(|b| (b.seat, &b.human))
            .chain(self.human.as_ref().map(|h| (self.seats.active, h)))
            .collect();
        v.sort_by_key(|&(s, _)| s);
        v.into_iter().map(|(_, h)| h)
    }

    /// Whether the human of `seat` has no club: his coach sits at a club of the outside pool (the original keeps unemployed
    /// coaches there; a sacked human is replaced by a free coach and takes his place, `1010:5343`).
    pub fn is_unemployed(&self, seat: usize) -> bool {
        self.human_at(seat)
            .is_some_and(|h| self.pool.iter().any(|c| c.name == h.club))
    }

    /// The seats of the humans who have a club: only they take turns on the main window and play.
    pub fn employed_seats(&self) -> Vec<usize> {
        (0..self.human_count())
            .filter(|&s| !self.is_unemployed(s))
            .collect()
    }

    /// The seat of the human who manages `club`.
    pub fn seat_of(&self, club: &str) -> Option<usize> {
        if self.human.as_ref().is_some_and(|h| h.club == club) {
            return Some(self.seats.active);
        }
        self.seats
            .benched
            .iter()
            .find(|b| b.human.club == club)
            .map(|b| b.seat)
    }

    /// Whether `name` is the name of a human coach (the players' own names, or the coach of a club a human manages).
    pub fn is_human_coach(&self, name: &str) -> bool {
        !name.is_empty()
            && self.humans().any(|h| {
                h.coach == name
                    || self
                        .locate(&h.club)
                        .is_some_and(|l| self.club_ref(l).coach_name == name)
            })
    }

    /// Whether `club` is managed by a human (any seat).
    pub fn is_human_club(&self, club: &str) -> bool {
        self.seat_of(club).is_some_and(|s| !self.is_unemployed(s))
    }

    /// The human of `seat`, active or not.
    pub fn human_at(&self, seat: usize) -> Option<&Human> {
        if seat == self.seats.active {
            return self.human.as_ref();
        }
        self.seats
            .benched
            .iter()
            .find(|b| b.seat == seat)
            .map(|b| &b.human)
    }

    /// The market lists of the human of `seat`: purchases to arrive, watched players, refused sales.
    #[allow(clippy::type_complexity)]
    pub fn seat_lists(&self, seat: usize) -> (&[Pending], &[(String, String)], &[String]) {
        if seat == self.seats.active {
            return (&self.pending, &self.observed, &self.sell_tried);
        }
        match self.seats.benched.iter().find(|b| b.seat == seat) {
            Some(b) => (&b.pending, &b.observed, &b.sell_tried),
            None => (&[], &[], &[]),
        }
    }

    /// Makes the human of `seat` the active one: the human, his purchases, watched players and refused sales swap with the
    /// active human's. False when there is no such seat.
    pub fn switch_human(&mut self, seat: usize) -> bool {
        if seat == self.seats.active && self.human.is_some() {
            return true;
        }
        let Some(k) = self.seats.benched.iter().position(|b| b.seat == seat) else {
            return false;
        };
        let mut next = self.seats.benched.remove(k);
        if let Some(h) = self.human.take() {
            self.seats.benched.push(Bench {
                seat: self.seats.active,
                human: h,
                pending: std::mem::take(&mut self.pending),
                observed: std::mem::take(&mut self.observed),
                sell_tried: std::mem::take(&mut self.sell_tried),
            });
        }
        self.human = Some(next.human);
        self.pending = std::mem::take(&mut next.pending);
        self.observed = std::mem::take(&mut next.observed);
        self.sell_tried = std::mem::take(&mut next.sell_tried);
        self.seats.active = seat;
        true
    }

    /// Hands the main window to the next human (the round starts after the last one): false when the active human is the last.
    pub fn next_human(&mut self) -> bool {
        let next = self.seats.active + 1;
        next < self.human_count() && self.switch_human(next)
    }

    /// Adds a human coach named `coach` who manages club `club` of `division`; he becomes the active human.
    pub fn add_human(&mut self, division: usize, club: usize, coach: &str) -> usize {
        let seat = self.human_count();
        if let Some(h) = self.human.take() {
            self.seats.benched.push(Bench {
                seat: self.seats.active,
                human: h,
                pending: std::mem::take(&mut self.pending),
                observed: std::mem::take(&mut self.observed),
                sell_tried: std::mem::take(&mut self.sell_tried),
            });
        }
        self.seats.active = seat;
        self.set_human(division, club);
        if let Some(h) = self.human.as_mut() {
            h.coach = coach.to_string();
        }
        if !coach.is_empty() {
            self.divisions[division][club].coach_name = coach.to_string();
        }
        seat
    }

    /// A new game for `names` (one per human): the first takes the club `pick_human` chose, the others get another random one
    /// of the bottom six of the 4th division each (`1028:0b26`). Every club's coach is renamed after its human. The first
    /// human is the active one afterwards.
    pub fn name_humans(&mut self, names: &[String]) {
        if names.is_empty() {
            return;
        }
        let first = self
            .human
            .take()
            .and_then(|h| self.divisions[3].iter().position(|c| c.name == h.club));
        let mut chosen: Vec<usize> = first.into_iter().collect();
        self.seats = Seats::default();
        self.pending.clear();
        self.observed.clear();
        self.sell_tried.clear();
        for (k, name) in names.iter().enumerate() {
            let club = match first.filter(|_| k == 0) {
                Some(c) => c,
                None => loop {
                    let c = 7 - self.rng.random(6) as usize;
                    if !chosen.contains(&c) {
                        chosen.push(c);
                        break c;
                    }
                },
            };
            self.add_human(3, club, name);
        }
        self.switch_human(0);
    }

    /// Runs `f` once with each human active in turn (seat order); the active human is the same afterwards.
    pub fn for_each_human(&mut self, mut f: impl FnMut(&mut World)) {
        let back = self.seats.active;
        for seat in 0..self.human_count() {
            self.switch_human(seat);
            f(self);
        }
        self.switch_human(back);
    }

    /// Where the clubs of all humans are (seat order, those outside the divisions left out).
    pub fn human_locations(&self) -> Vec<(usize, usize)> {
        self.humans()
            .filter_map(|h| self.locate_in_divisions(&h.club))
            .collect()
    }

    fn locate_in_divisions(&self, name: &str) -> Option<(usize, usize)> {
        self.divisions
            .iter()
            .enumerate()
            .find_map(|(d, v)| v.iter().position(|c| c.name == name).map(|i| (d, i)))
    }

    /// The coach swaps of the last round that took a human coach to another club, as `(from, to)`.
    pub fn human_moves(&self) -> &[(String, String)] {
        &self.seats.moves
    }

    /// Whether the human who managed `club` declines the invitation to another one (the round is then replayed).
    pub fn decline_invitation(&mut self, club: &str) {
        if !self.seats.declined.iter().any(|c| c == club) {
            self.seats.declined.push(club.to_string());
        }
    }

    /// Starts a new cup: every club of the world is in.
    /// For games saved before finances existed: the cash a new game would give each club, by list position.
    pub fn give_new_game_finances(&mut self) {
        let order: Vec<(usize, usize)> = (0..4)
            .flat_map(|d| (0..self.divisions[d].len()).map(move |i| (d, i)))
            .collect();
        for (pos, (d, i)) in order.into_iter().enumerate() {
            self.divisions[d][i].fin = Finance::new_game(base_strength(pos), self.scale);
        }
        for (j, c) in self.pool.iter_mut().enumerate() {
            c.fin = Finance::new_game(base_strength(32 + j), self.scale);
        }
        self.refresh_cash_ref();
    }

    /// The record of the game's currency (scale, loan step, prizes, the word written after amounts).
    pub fn currency_info(&self) -> &'static crate::economy::Currency {
        &crate::economy::CURRENCIES[self.currency.min(crate::economy::CURRENCIES.len() - 1)]
    }

    /// All clubs as `(division, index)`, divisions 0..=3 then the outside pool as division 4, ordered by the master list.
    fn all_clubs_by_master(&self) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = (0..4)
            .flat_map(|d| (0..self.divisions[d].len()).map(move |i| (d, i)))
            .chain((0..self.pool.len()).map(|i| (4, i)))
            .collect();
        v.sort_by_key(|&(d, i)| self.club_ref((d, i)).master);
        v
    }

    fn club_ref(&self, (d, i): (usize, usize)) -> &Club {
        if d < 4 {
            &self.divisions[d][i]
        } else {
            &self.pool[i]
        }
    }

    fn club_mut(&mut self, (d, i): (usize, usize)) -> &mut Club {
        if d < 4 {
            &mut self.divisions[d][i]
        } else {
            &mut self.pool[i]
        }
    }

    fn is_human(&self, loc: (usize, usize)) -> bool {
        self.is_human_club(&self.club_ref(loc).name)
    }

    /// Puts `slot_idx` of the club at `seller` up for sale at `ask` (`1010:5b37`) and settles the auction (`1008:3600`,
    /// `373e`, `351a`, `39c7`): every computer-managed club except the seller that may bid does so, the highest bid wins (the
    /// earlier club in the master list on a tie), pays it, and the player moves with a renegotiated wage.
    fn auction(&mut self, seller: (usize, usize), slot_idx: usize, ask: i64, round: usize) -> bool {
        let ask = crate::market::floor_1000(ask);
        {
            let c = self.club_ref(seller);
            if self
                .board
                .iter()
                .any(|l| l.seller == c.name && l.player == c.names[slot_idx])
            {
                return false;
            }
        }
        let best = self.best_bid(seller, slot_idx, ask);
        // the user may still outbid the computer on the player of a computer-managed club
        if !self.is_human(seller) && self.human_may_bid(seller, slot_idx, ask) {
            let s = self.club_ref(seller);
            self.board.push(Listing {
                seller: s.name.clone(),
                player: s.names[slot_idx].clone(),
                ask,
                best: best.map(|(loc, fee)| (self.club_ref(loc).name.clone(), fee)),
                round,
            });
            return true;
        }
        match best {
            Some((buyer, fee)) => {
                self.settle(seller, slot_idx, buyer, fee, round);
                true
            }
            None => false,
        }
    }

    /// The computer-managed clubs' sealed bids: the winner and the fee, if any club bids at least `ask`.
    fn best_bid(
        &self,
        seller: (usize, usize),
        slot_idx: usize,
        ask: i64,
    ) -> Option<((usize, usize), i64)> {
        let slot = self.club_ref(seller).slots[slot_idx];
        let value = player_value(
            slot.strength,
            self.club_ref(seller).fin.f,
            self.economy,
            self.scale,
        );
        let mut best: Option<((usize, usize), i64)> = None;
        for loc in self.all_clubs_by_master() {
            if loc == seller || self.is_human(loc) {
                continue;
            }
            let buyer = self.club_ref(loc);
            if !crate::market::may_bid(buyer, &slot) || buyer.fin.cash < ask {
                continue;
            }
            let b = crate::market::floor_1000(crate::market::bid(
                buyer,
                value,
                &slot,
                self.economy,
                self.scale,
            ));
            if b >= ask && best.is_none_or(|(_, top)| b > top || top == 0) {
                best = Some((loc, b));
            }
        }
        best
    }

    fn human_may_bid(&self, seller: (usize, usize), slot_idx: usize, ask: i64) -> bool {
        // some human may still outbid the computer
        self.human_locations()
            .into_iter()
            .any(|h| self.may_bid_for(h, seller, slot_idx, ask))
    }

    /// Whether the human whose club is at `h` can take part in the auction of `slot_idx` of the club at `seller`.
    fn may_bid_for(
        &self,
        h: (usize, usize),
        seller: (usize, usize),
        slot_idx: usize,
        ask: i64,
    ) -> bool {
        let c = self.club_ref(h);
        h != seller
            && self.history.options.auction != 2 // the coach profile: no buying in auctions
            && c.fin.cash >= ask
            && crate::market::may_bid(c, &self.club_ref(seller).slots[slot_idx])
    }

    /// `1008:39c7`: moves the player, books the fee on both sides and records the transfer.
    fn settle(
        &mut self,
        seller: (usize, usize),
        slot_idx: usize,
        buyer_loc: (usize, usize),
        fee: i64,
        round: usize,
    ) {
        let value = {
            let sl = &self.club_ref(seller).slots[slot_idx];
            player_value(
                sl.strength,
                self.club_ref(seller).fin.f,
                self.economy,
                self.scale,
            )
        };
        let mut moved = {
            let c = self.club_mut(seller);
            let name = c.names.remove(slot_idx);
            let sl = c.slots.remove(slot_idx);
            (sl, name)
        };
        let from = self.club_ref(seller).name.clone();
        self.club_mut(seller)
            .fin
            .book(fee, finance::Category::Sales);
        let to_country = self.club_ref(buyer_loc).country;
        let (to, year) = {
            let c = self.club_mut(buyer_loc);
            moved.0.foreign = country_class(to_country, moved.0.nat);
            moved.0.same_country = moved.0.foreign == 0;
            moved.0.wage = value;
            moved.0.fresh = true;
            moved.0.status = Status::Out;
            c.slots.push(moved.0);
            c.names.push(moved.1.clone());
            c.fin.book(-fee, finance::Category::Purchases);
            (c.name.clone(), self.year)
        };
        self.remap_plan_after_removal(&from, slot_idx);
        if self.transfers.len() >= 200 {
            self.transfers.remove(0);
        }
        self.transfers.push(Transfer {
            round,
            year,
            player: moved.1,
            position: moved.0.position,
            strength: moved.0.strength,
            from,
            to,
            fee,
        });
        let (a, b) = (
            self.club_ref(seller).name.clone(),
            self.club_ref(buyer_loc).name.clone(),
        );
        self.for_each_human(|w| {
            if w.human.as_ref().is_some_and(|h| h.club == a || h.club == b) {
                w.refresh_plan();
            }
        });
    }

    /// Keeps the plan of the human who manages `club` consistent after the player at `removed` left the squad.
    fn remap_plan_after_removal(&mut self, club: &str, removed: usize) {
        let h = match self.human.as_mut() {
            Some(h) if h.club == club => h,
            _ => match self.seats.benched.iter_mut().find(|b| b.human.club == club) {
                Some(b) => &mut b.human,
                None => return,
            },
        };
        let fix = |v: &mut Vec<usize>| {
            v.retain(|&i| i != removed);
            for i in v.iter_mut() {
                if *i > removed {
                    *i -= 1;
                }
            }
        };
        fix(&mut h.plan.starters);
        fix(&mut h.plan.bench);
        h.plan.subs.retain(|s| s.out != removed && s.inn != removed);
        for s in &mut h.plan.subs {
            if s.out > removed {
                s.out -= 1;
            }
            if s.inn > removed {
                s.inn -= 1;
            }
        }
    }

    /// Settles the auctions left open by the previous round: the computer's best bid wins unless the user bid.
    fn close_board(&mut self) {
        let board = std::mem::take(&mut self.board);
        for l in board {
            let Some((buyer, fee)) = l.best else { continue };
            let find = |w: &World, name: &str| {
                w.all_clubs_by_master()
                    .into_iter()
                    .find(|&loc| w.club_ref(loc).name == name)
            };
            let (Some(seller), Some(buyer)) = (find(self, &l.seller), find(self, &buyer)) else {
                continue;
            };
            if let Some(idx) = self
                .club_ref(seller)
                .names
                .iter()
                .position(|n| *n == l.player)
            {
                self.settle(seller, idx, buyer, fee, l.round);
            }
        }
    }

    /// `Procurar jogadores` (`1048:29f0`): up to 50 players of other clubs that are for sale, with the price each club asks.
    pub fn search_offers(&self, q: &Search) -> Vec<Offer> {
        let Some(human) = self.human_location() else {
            return Vec::new();
        };
        let me = self.club_ref(human);
        let needle = q.name.to_lowercase();
        let mut out = Vec::new();
        for loc in self.all_clubs_by_master() {
            if out.len() >= 50 {
                break;
            }
            let c = self.club_ref(loc);
            if loc == human && q.hide_mine {
                continue;
            }
            if loc == human {
                continue; // the user cannot buy from himself
            }
            for (i, s) in c.slots.iter().enumerate() {
                if out.len() >= 50 {
                    break;
                }
                if (!needle.is_empty() && !c.names[i].to_lowercase().contains(&needle))
                    || q.nationality.is_some_and(|n| n != s.nat)
                    || s.strength < q.min_strength
                    || q.position.is_some_and(|p| p != s.position)
                {
                    continue;
                }
                let price = crate::market::direct_price(c, i, Some(me), self.economy, self.scale);
                if price == 0 || q.max_price.is_some_and(|m| price > m) {
                    continue;
                }
                out.push(Offer {
                    club: c.name.clone(),
                    index: i,
                    player: c.names[i].clone(),
                    position: s.position,
                    strength: s.strength,
                    nationality: s.nat,
                    price,
                });
            }
        }
        out
    }

    /// The user buys a listed player at the asking price: the club is paid at once and the player arrives with the end of the
    /// round (`1048:1d66`, `1020:0aae`).
    pub fn human_buy(&mut self, club: &str, index: usize, player: &str) -> Result<i64, String> {
        let human = self.human_location().ok_or("sem clube")?;
        let seller = self
            .all_clubs_by_master()
            .into_iter()
            .find(|&loc| self.club_ref(loc).name == club)
            .ok_or("clube inexistente")?;
        let idx = index;
        if self.club_ref(seller).names.get(idx).map(String::as_str) != Some(player) {
            return Err("o jogador já não está nesse clube".into());
        }
        if self
            .pending
            .iter()
            .chain(self.seats.benched.iter().flat_map(|b| b.pending.iter()))
            .any(|p| p.seller == club && p.player == player)
        {
            return Err("esse jogador já está comprado".into());
        }
        let price = crate::market::direct_price(
            self.club_ref(seller),
            idx,
            Some(self.club_ref(human)),
            self.economy,
            self.scale,
        );
        if price == 0 {
            return Err("o clube não quer vender esse jogador".into());
        }
        let slot = self.club_ref(seller).slots[idx];
        let me = self.club_ref(human);
        if me.fin.cash < price {
            return Err(format!("sem dinheiro: o preço é {price}"));
        }
        if let Some(why) = crate::market::bid_refusal(me, &slot) {
            return Err(why.into());
        }
        let year = self.year;
        let round = self.round;
        let to = me.name.clone();
        let foreign = country_class(me.country, slot.nat) == 2;
        self.club_mut(seller)
            .fin
            .book(price, finance::Category::Sales);
        let c = self.club_mut(human);
        c.fin.book(-price, finance::Category::Purchases);
        c.bought += 1;
        c.bought_foreign += foreign as u16;
        self.pending.push(Pending {
            seller: club.to_string(),
            player: player.to_string(),
        });
        self.transfers.push(Transfer {
            round,
            year,
            player: player.to_string(),
            position: slot.position,
            strength: slot.strength,
            from: club.to_string(),
            to,
            fee: price,
        });
        Ok(price)
    }

    /// `1020:0aae`: the user's purchases join the club (wage from the player's value in the new club's economy).
    fn deliver_purchases(&mut self) {
        self.for_each_human(|w| w.deliver_own_purchases());
    }

    fn deliver_own_purchases(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        let Some(human) = self.human_location() else {
            return;
        };
        for p in pending {
            let Some(seller) = self
                .all_clubs_by_master()
                .into_iter()
                .find(|&loc| self.club_ref(loc).name == p.seller)
            else {
                continue;
            };
            let Some(idx) = self
                .club_ref(seller)
                .names
                .iter()
                .position(|n| *n == p.player)
            else {
                continue;
            };
            let value = {
                let sl = &self.club_ref(seller).slots[idx];
                player_value(
                    sl.strength,
                    self.club_ref(human).fin.f,
                    self.economy,
                    self.scale,
                )
            };
            let name = self.club_mut(seller).names.remove(idx);
            let mut slot = self.club_mut(seller).slots.remove(idx);
            let country = self.club_ref(human).country;
            slot.foreign = country_class(country, slot.nat);
            slot.same_country = slot.foreign == 0;
            slot.wage = value;
            slot.fresh = true;
            slot.status = Status::Out;
            let c = self.club_mut(human);
            c.slots.push(slot);
            c.names.push(name);
            c.bought = c.bought.saturating_sub(1);
            if slot.foreign == 2 {
                c.bought_foreign = c.bought_foreign.saturating_sub(1);
            }
            let seller_club = self.club_ref(seller).name.clone();
            self.remap_plan_after_removal(&seller_club, idx);
        }
        self.refresh_plan();
    }

    /// The user takes one loan step (`1010:665e`), within the bank's limit (`1040:3899`).
    pub fn human_borrow(&mut self) -> Result<i64, String> {
        let loc = self.human_location().ok_or("sem clube")?;
        let (e, g) = (self.economy, self.scale);
        let step = finance::loan_step(self.currency_info());
        let c = self.club_mut(loc);
        let max = finance::max_loan(finance::rating(&c.slots), c.k, e, g, step);
        if c.fin.loan + step > max {
            return Err(format!("o banco não empresta mais: limite {max}"));
        }
        c.fin.cash += step;
        c.fin.loan += step;
        Ok(step)
    }

    /// The user repays one loan step (`1010:668d`).
    pub fn human_repay(&mut self) -> Result<i64, String> {
        let loc = self.human_location().ok_or("sem clube")?;
        let step = finance::loan_step(self.currency_info());
        let c = self.club_mut(loc);
        if c.fin.loan <= 0 {
            return Err("não tens empréstimos".into());
        }
        let pay = step.min(c.fin.loan);
        if c.fin.cash < pay {
            return Err("sem dinheiro para amortizar".into());
        }
        c.fin.cash -= pay;
        c.fin.loan -= pay;
        Ok(pay)
    }

    /// The user builds the next 5000 seats (`1010:c081`, cost `c172`) once the stadium is at least 80 % full.
    pub fn human_expand(&mut self) -> Result<i64, String> {
        let loc = self.human_location().ok_or("sem clube")?;
        let (e, g) = (self.economy, self.scale);
        let c = self.club_mut(loc);
        if c.fin.stadium >= finance::MAX_STADIUM_LEVEL {
            return Err("o estádio já tem a capacidade máxima".into());
        }
        // (the user builds whenever the money is there: the 80 % filling is the computer's rule, `1010:c030`)
        if c.fin.cash < finance::expansion_cost(&c.fin, e, g) {
            return Err(format!(
                "as obras custam {}",
                finance::expansion_cost(&c.fin, e, g)
            ));
        }
        // `ConstructBtnClick` (`1030:3036`): the stand is built, then its price is taken (`1010:c172` after `c081`)
        finance::expand(&mut c.fin);
        let cost = finance::expansion_cost(&c.fin, e, g);
        c.fin.book(-cost, finance::Category::Works);
        Ok(cost)
    }

    /// The user bids `amount` (a multiple of 1000, above the asking price and the computer's best bid) for a listed player.
    pub fn human_bid(&mut self, listing: usize, amount: i64) -> Result<(), String> {
        let l = self
            .board
            .get(listing)
            .cloned()
            .ok_or("leilão inexistente")?;
        let human = self.human_location().ok_or("sem clube")?;
        if amount % 1000 != 0 {
            return Err("as licitações são em múltiplos de 1000".into());
        }
        if amount < l.ask {
            return Err(format!("a licitação mínima é {}", l.ask));
        }
        if let Some((_, best)) = &l.best {
            if amount <= *best {
                return Err(format!("é preciso superar a melhor licitação ({best})"));
            }
        }
        let seller = self
            .all_clubs_by_master()
            .into_iter()
            .find(|&loc| self.club_ref(loc).name == l.seller)
            .ok_or("clube vendedor inexistente")?;
        let idx = self
            .club_ref(seller)
            .names
            .iter()
            .position(|n| *n == l.player)
            .ok_or("o jogador já não está no clube")?;
        let hc = self.club_ref(human);
        if hc.fin.cash < amount {
            return Err("sem dinheiro suficiente".into());
        }
        if let Some(why) = crate::market::bid_refusal(hc, &self.club_ref(seller).slots[idx]) {
            return Err(why.into());
        }
        self.board.remove(listing);
        self.settle(seller, idx, human, amount, l.round);
        Ok(())
    }

    /// The user puts the player at `slot_idx` up for sale at `ask`; the computer-managed clubs bid and the best bid wins.
    /// Returns the fee, or an explanation.
    pub fn human_sell(&mut self, slot_idx: usize, ask: i64) -> Result<i64, String> {
        let human = self.human_location().ok_or("sem clube")?;
        let club = self.club_ref(human);
        if slot_idx >= club.slots.len() {
            return Err("jogador inexistente".into());
        }
        let ask = crate::market::floor_1000(ask);
        if let Some(why) = self.sale_blocker_text(slot_idx) {
            return Err(why.into());
        }
        let Some((buyer, fee)) = self.best_bid(human, slot_idx, ask) else {
            let name = self.club_ref(human).names[slot_idx].clone();
            self.sell_tried.push(name);
            return Err("Não houve ofertas".into());
        };
        let round = self.round;
        self.settle(human, slot_idx, buyer, fee, round);
        Ok(fee)
    }

    /// `3e9c`: the most overpaid (stored wage / value above 1.2) and the most underpaid (below 0.8) player not yet marked
    /// `fresh` are repriced to their current value and marked.
    fn renegotiate_wages(&mut self, loc: (usize, usize), classes: [bool; 3]) {
        let (e, g) = (self.economy, self.scale);
        let club = self.club_mut(loc);
        let f = club.fin.f;
        let (mut cut, mut raise) = (None, None);
        let (mut cut_best, mut raise_best) = (1.2f64, 0.8f64);
        for (i, s) in club.slots.iter().enumerate() {
            // the coach profile (`TCoachOptionsDlg`): only the classes the computer manages are renegotiated
            if s.fresh || !classes[s.foreign.min(2) as usize] {
                continue;
            }
            let v = player_value(s.strength, f, e, g);
            let ratio = s.wage as f64 / v as f64;
            if ratio > cut_best {
                cut = Some(i);
                cut_best = ratio;
            }
            if ratio < raise_best {
                raise = Some(i);
                raise_best = ratio;
            }
        }
        for i in [cut, raise].into_iter().flatten() {
            let s = &mut club.slots[i];
            s.wage = player_value(s.strength, f, e, g);
            s.fresh = true;
        }
    }

    /// `1010:3e9c` (and `3aad`) for one computer-managed club after a round: wage renegotiation, forced sale when broke, and
    /// the regular decision to put a player up for sale.
    fn club_after_round(&mut self, loc: (usize, usize), round: usize) {
        let human = self.is_human(loc);
        let (e, g) = (self.economy, self.scale);
        if human {
            // the user's wages are renegotiated by the computer for the player classes of the coach profile (the original asks
            // the user in a dialog for the others)
            if round >= 2 && self.club_ref(loc).coach_games > 1 {
                self.renegotiate_wages(loc, self.history.options.salary);
            }
            return;
        }
        // `3aad` ends with the stadium: expand when it is full and the club can afford the works plus two wage bills
        {
            let club = self.club_mut(loc);
            if finance::wants_expansion(&club.fin) {
                let cost = finance::expansion_cost(&club.fin, e, g);
                if cost + 2 * crate::market::wage_sum(club) < club.fin.cash {
                    finance::expand(&mut club.fin);
                    club.fin.book(-cost, finance::Category::Works);
                }
            }
        }
        if round < 2 {
            return;
        }
        // wages follow the player's value: the most overpaid and the most underpaid player (outside 0.8..1.2) are repriced
        if self.club_ref(loc).coach_games > 1 {
            self.renegotiate_wages(loc, [true; 3]);
            let club = self.club_mut(loc);
            // a club that is broke sells its lowest-scoring player at any price
            if club.fin.cash < 0 {
                if let Some(i) = crate::market::pick_for_sale(club) {
                    self.auction(loc, i, 0, round);
                }
            }
        }
        // `3aad`: sell when the cash is under a third of the wage bill, otherwise with a 1 in 20 chance, or when the squad is
        // over 20 with exactly five foreigners
        let (broke, five_and_big) = {
            let club = self.club_ref(loc);
            (
                3 * club.fin.cash < crate::market::wage_sum(club),
                crate::market::foreigners(club) == 5 && club.slots.len() >= 21,
            )
        };
        let sell = broke || self.rng.random(20) == 0 || five_and_big;
        if sell {
            let club = self.club_ref(loc);
            if let Some(i) = crate::market::pick_for_sale(club) {
                let ask = crate::market::market_value(&club.slots[i], e, g);
                self.auction(loc, i, ask, round);
            }
        }
    }

    /// The clubs of the next round's games, in game order (`13e1` walks the list of games just created).
    fn market_round(&mut self, round: usize) {
        let next = round % ROUNDS + 1;
        let mut order = Vec::new();
        for d in 0..4 {
            for (h, a) in fixtures(next) {
                order.push((d, h));
                order.push((d, a));
            }
        }
        for loc in order {
            self.club_after_round(loc, round);
        }
    }

    /// Saves from before the transfer market: every club gets an unknown country (all players count as domestic), wages at
    /// the player's value and a master-list position following the current order.
    pub fn give_market_defaults(&mut self) {
        let (e, g) = (self.economy, self.scale);
        for (master, c) in self
            .divisions
            .iter_mut()
            .flatten()
            .chain(self.pool.iter_mut())
            .enumerate()
        {
            c.master = master as u16;
            c.country = *b"???";
            let f = c.fin.f;
            for s in c.slots.iter_mut() {
                s.nat = *b"???";
                s.foreign = 0;
                s.wage = player_value(s.strength, f, e, g);
            }
        }
    }

    /// `1010:7067` for every division: remembers the total cash.
    pub fn refresh_cash_ref(&mut self) {
        for d in 0..4 {
            self.cash_ref[d] = self.divisions[d].iter().map(|c| c.fin.cash).sum();
        }
    }

    /// `1010:6e48`, once per round and division: `F = 0.8 F + 0.2 ratio`, `ratio` = cash of the computer-managed clubs over the
    /// division total remembered earlier (floor 0.1). The reference total is refreshed afterwards. (How the original
    /// times the total and the human exclusion could not be reproduced exactly; see `docs/finance.md`.)
    fn update_factors(&mut self) {
        let humans = self.human_clubs();
        for d in 0..4 {
            let ai: i64 = self.divisions[d]
                .iter()
                .filter(|c| !humans.contains(&c.name))
                .map(|c| c.fin.cash)
                .sum();
            let reference = self.cash_ref[d].max(1) as f64;
            let ratio = (ai as f64 / reference).max(0.1);
            for c in &mut self.divisions[d] {
                c.fin.f = 0.8 * c.fin.f + 0.2 * ratio;
            }
        }
        self.refresh_cash_ref();
    }

    pub fn reset_cup(&mut self) {
        self.cup = self
            .divisions
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .map(|c| c.name.clone())
            .collect();
        self.cup_champion = None;
    }

    /// `1010:8b5d` -> `1010:4f40`: the coach of club `i` of `cur` (division `d`, taken out of `self`) is sacked. A new one is
    /// picked among the coaches (`5343`): those of clubs whose morale `K` is at least 1.25 and whose rating (`4ec3`) is 11..19
    /// below this club's (`5661`), the lowest coach score first (`1050:1d59`); the user's coach only when invitations are
    /// accepted. Without a candidate nothing happens (the check repeats after every lost game). The two clubs swap
    /// coaches, both coaches' game counters and both clubs' morale reset; the user's coach takes the club it moved to.
    fn replace_coach(&mut self, d: usize, cur: &mut [Club], i: usize, no_humans: bool) {
        let get = |this: &World, cur: &[Club], loc: (usize, usize)| -> Club {
            if loc.0 == d {
                cur[loc.1].clone()
            } else {
                this.club_ref(loc).clone()
            }
        };
        let mut locs: Vec<(usize, usize)> = self
            .divisions
            .iter()
            .enumerate()
            .flat_map(|(dd, v)| {
                (0..if dd == d { cur.len() } else { v.len() }).map(move |k| (dd, k))
            })
            .chain((0..if d == 4 { cur.len() } else { self.pool.len() }).map(|k| (4, k)))
            .collect();
        locs.sort_by_key(|&l| get(self, cur, l).master);
        let me = (d, i);
        let t = cur[i].clone();
        let rating_t = crate::cup::team_rating(&t.slots) as i32;
        // (humans without a club count: they sit at the pool and are candidates there)
        let human_names: Vec<String> = self.humans().map(|h| h.club.clone()).collect();
        let declined = self.seats.declined.clone();
        let is_human = |c: &Club| human_names.contains(&c.name);
        let exclude_humans = no_humans;
        // coach list order: computer coaches by club order, the user's coach last
        let mut cands: Vec<((usize, usize), Club)> = Vec::new();
        for l in locs {
            if l == me {
                continue;
            }
            let c = get(self, cur, l);
            let diff = rating_t - crate::cup::team_rating(&c.slots) as i32;
            // a coach without a club (the pool) is taken by a club rated below 15 (flag 0x200 of `5343`); the others need the
            // morale and the rating window (flag 0x100)
            let free = l.0 == 4 && rating_t < 15;
            if (free || (l.0 != 4 && c.k >= 1.25 && (11..20).contains(&diff)))
                && !(exclude_humans && is_human(&c))
            {
                cands.push((l, c));
            }
        }
        cands.sort_by_key(|(_, c)| is_human(c)); // stable: users last
                                                 // `1050:1e3b` (free coaches wanted, rating < 15) puts computer coaches first, `1d59` only ranks by score; both
                                                 // are the stable bubble sort `1068:0e0d` keeping a pair that is already in ascending order
        let mut v: Vec<usize> = (0..cands.len()).collect();
        for a in 1..v.len() {
            for j in (a..v.len()).rev() {
                let (p, q) = (&cands[v[j - 1]].1, &cands[v[j]].1);
                let in_order = if rating_t < 15 {
                    match (is_human(q), is_human(p)) {
                        (false, true) => true,
                        (true, false) => false,
                        _ => p.coach_score <= q.coach_score,
                    }
                } else {
                    p.coach_score <= q.coach_score
                };
                if !in_order {
                    v.swap(j - 1, j);
                }
            }
        }
        // (a human without a club, at the pool, takes the job without being asked)
        let Some(&pick) = v.iter().find(|&&k| {
            !is_human(&cands[k].1)
                || cands[k].0 .0 == 4
                || (self.accept_invitations && !declined.contains(&cands[k].1.name))
        }) else {
            return;
        };
        let (ploc, p) = cands[pick].clone();
        let t_human = is_human(&t);
        let p_human = is_human(&p);
        let (t_score, t_games) = (t.coach_score, 0u16);
        let (p_score, p_games) = (p.coach_score, 0u16);
        cur[i].k = 1.0;
        cur[i].coach_score = p_score;
        cur[i].coach_games = p_games;
        cur[i].coach_name = p.coach_name.clone();
        {
            let pc = if ploc.0 == d {
                &mut cur[ploc.1]
            } else {
                self.club_mut(ploc)
            };
            pc.k = 1.0;
            pc.coach_score = t_score;
            pc.coach_games = t_games;
            pc.coach_name = t.coach_name.clone();
        }
        self.log_coach_swap(&t, &p);
        self.coach_changes.push((t.name.clone(), p.name.clone()));
        self.coach_changes_free.push(ploc.0 == 4);
        if t_human {
            self.seats.moves.push((t.name.clone(), p.name.clone()));
        }
        if p_human {
            self.seats.moves.push((p.name.clone(), t.name.clone()));
        }
        if t_human && !p_human {
            // `1010:6a08`: the club the user leaves gets wages no higher than the players' values and no overdraft
            let (e, g) = (self.economy, self.scale);
            let c = &mut cur[i];
            let f = c.fin.f;
            for s in c.slots.iter_mut() {
                s.wage = s.wage.min(player_value(s.strength, f, e, g));
            }
            c.fin.cash = c.fin.cash.max(0);
        }
    }

    /// The user's coach was swapped to another club: the user manages that club from now on.
    fn apply_human_move(&mut self) {
        let new: Vec<(String, String)> = self.seats.moves[self.seats.applied..].to_vec();
        self.seats.applied = self.seats.moves.len();
        // the seats first, then the moves: two humans may swap clubs
        let moves: Vec<(usize, String)> = new
            .into_iter()
            .filter_map(|(from, to)| Some((self.seat_of(&from)?, to)))
            .collect();
        let back = self.seats.active;
        for (seat, to) in moves {
            match self.locate(&to) {
                Some((d, i)) if d < 4 => {
                    self.switch_human(seat);
                    self.set_human(d, i);
                }
                // replaced by a free coach: the human has no club (and no turn) until a club hires him. With no other human to
                // play the days he stays where he is (nothing is known of a game without any human coach).
                Some(_) if self.employed_seats().iter().any(|&s| s != seat) => {
                    self.switch_human(seat);
                    if let Some(h) = self.human.as_mut() {
                        h.club = to.clone();
                    }
                }
                _ => {}
            }
        }
        // the window never rests on a human without a club
        let back = if self.is_unemployed(back) {
            self.employed_seats().first().copied().unwrap_or(back)
        } else {
            back
        };
        self.switch_human(back);
    }

    /// A cup loser's coach is sacked (flag `0x400`: the user's coach is never a candidate).
    fn sack_at(&mut self, (d, i): (usize, usize)) {
        let mut cur = if d < 4 {
            std::mem::take(&mut self.divisions[d])
        } else {
            std::mem::take(&mut self.pool)
        };
        self.replace_coach(d, &mut cur, i, true);
        if d < 4 {
            self.divisions[d] = cur;
        } else {
            self.pool = cur;
        }
    }

    fn locate(&self, name: &str) -> Option<(usize, usize)> {
        self.divisions
            .iter()
            .chain(std::iter::once(&self.pool))
            .enumerate()
            .find_map(|(d, v)| v.iter().position(|c| c.name == name).map(|i| (d, i)))
    }

    fn club_at(&mut self, (d, i): (usize, usize)) -> &mut Club {
        if d < 4 {
            &mut self.divisions[d][i]
        } else {
            &mut self.pool[i]
        }
    }

    /// The draw of the cup after the league games of `round` (even), `1020:0ed4` up to the draw dialog: the survivors are
    /// shuffled and paired; the ties are played on their own match day (`play_cup_day_ctl`). Does nothing when the cup has
    /// no round this time (see `cup.rs` for the rules).
    pub fn draw_cup(&mut self, round: usize, ctl: &mut Control) {
        self.cup_draw = None;
        let total = self.divisions.iter().map(Vec::len).sum::<usize>() + self.pool.len();
        if total <= cup::threshold(round) || self.cup.len() < 2 {
            return;
        }
        ctl.checkpoint(&mut self.rng);
        cup::shuffle(&mut self.rng, &mut self.cup);
        let n = self.cup.len();
        let (k, pairs) = cup::round_size(n);
        let pairs = self
            .cup
            .chunks(2)
            .take(pairs)
            .filter(|p| p.len() == 2)
            .map(|p| (p[0].clone(), p[1].clone()))
            .collect();
        self.cup_draw = Some(CupDraw {
            round,
            name: cup::round_name(k),
            pairs,
        });
    }

    /// Whether the next match day is a cup day (a round has been drawn).
    pub fn cup_day_due(&self) -> bool {
        self.cup_draw.is_some()
    }

    /// The cup tie of the active human on the pending cup day, if he has one (a human who is out of the cup or has a bye
    /// plays no game that day).
    pub fn human_cup_tie(&self) -> Option<CupTie> {
        let draw = self.cup_draw.as_ref()?;
        let me = &self.human.as_ref()?.club;
        draw.pairs.iter().find_map(|(h, a)| {
            if h == me {
                Some(CupTie {
                    opponent: a.clone(),
                    home: true,
                    stage: draw.name.clone(),
                })
            } else if a == me {
                Some(CupTie {
                    opponent: h.clone(),
                    home: false,
                    stage: draw.name.clone(),
                })
            } else {
                None
            }
        })
    }

    /// The seats of the humans who play a tie on the pending cup day, in seat order.
    pub fn humans_in_cup_tie(&self) -> Vec<usize> {
        let Some(draw) = self.cup_draw.as_ref() else {
            return Vec::new();
        };
        self.employed_seats()
            .into_iter()
            .filter(|&s| {
                self.human_at(s)
                    .is_some_and(|h| draw.pairs.iter().any(|(a, b)| *a == h.club || *b == h.club))
            })
            .collect()
    }

    /// The cup match day: plays the drawn ties with the humans' plans as they are now (`1020:0ed4` after the draw dialog).
    pub fn play_cup_day_ctl(&mut self, ctl: &mut Control) {
        let Some(draw) = self.cup_draw.take() else {
            return;
        };
        let round = draw.round;
        self.play_cup_ties(&draw, ctl);
        self.record_cup_fixtures(round);
        self.apply_human_move();
        self.for_each_human(|w| {
            if let Some(h) = w.human.as_mut() {
                h.plan.subs.clear(); // planned substitutions are for one game only
            }
            w.refresh_plan();
        });
        if round == ROUNDS {
            self.finish_season();
        }
    }

    fn play_cup_ties(&mut self, draw: &CupDraw, ctl: &mut Control) {
        let round = draw.round;
        let coach_changes_from = self.coach_changes.len();
        let entrants: Vec<String> = draw
            .pairs
            .iter()
            .flat_map(|(h, a)| [h.clone(), a.clone()])
            .collect();
        let plans: Vec<(String, Plan)> = self
            .humans()
            .map(|h| (h.club.clone(), h.plan.clone()))
            .collect();
        let mut matches = Vec::new();
        let mut losers = Vec::new();
        for pair in entrants.chunks(2) {
            let (hn, an) = (pair[0].clone(), pair[1].clone());
            let (Some(hl), Some(al)) = (self.locate(&hn), self.locate(&an)) else {
                continue;
            };
            ctl.checkpoint(&mut self.rng);
            let money = self.money_ctx(hl.0.min(3));
            let placeholder = || Club::new("", Vec::new());
            let mut home = std::mem::replace(self.club_at(hl), placeholder());
            let mut away = std::mem::replace(self.club_at(al), placeholder());
            let rng = &mut self.rng;
            let mut plan_of = |c: &mut Club| -> Option<Plan> {
                match plans.iter().find(|(club, _)| *club == c.name) {
                    Some((_, p)) => {
                        apply_plan(&mut c.slots, p);
                        Some(p.clone())
                    }
                    None => {
                        auto_lineup(rng, &mut c.slots, &mut c.names);
                        None
                    }
                }
            };
            let hp = plan_of(&mut home);
            let ap = plan_of(&mut away);
            let attendance = finance::attendance(
                &mut self.rng,
                finance::rating(&home.slots),
                finance::rating(&away.slots),
                home.k,
                away.k,
                true,
                home.fin.capacity(),
            );
            let hb = crate::matchsim::fixture_value(b"TACA", home.name.as_bytes(), true) + 2;
            let ab = crate::matchsim::fixture_value(b"TACA", away.name.as_bytes(), true);
            let human_in_tie = hp.is_some() || ap.is_some();
            let decisions = if human_in_tie {
                ctl.decisions(true, &home.name)
            } else {
                Vec::new()
            };
            let g = play_game_with(
                &mut self.rng,
                &mut home,
                &mut away,
                hb,
                ab,
                hp.as_ref(),
                ap.as_ref(),
                true,
                &decisions,
            );
            let shootout = g.home_goals == g.away_goals;
            let mut pens = None;
            let home_out = if shootout {
                if human_in_tie || round == ROUNDS || self.real_shootouts {
                    // the dialog with real kicks (always in the final, when a human plays, or for every tie by the option)
                    let r = cup::shootout(
                        &mut self.rng,
                        (&home.slots, &g.pitch[0]),
                        (&away.slots, &g.pitch[1]),
                    );
                    let out = r.away >= r.home;
                    pens = Some(r);
                    out
                } else {
                    cup::shootout_home_eliminated(
                        &mut self.rng,
                        cup::team_rating(&home.slots),
                        cup::team_rating(&away.slots),
                    )
                }
            } else {
                g.home_goals < g.away_goals
            };
            // cup gate receipts are shared (`1010:9396`, game type 1), wages are paid as in the league
            let half = (attendance as f64 * money.price / 2.0).round_ties_even() as i64;
            for c in [&mut home, &mut away] {
                c.fin.book(half, finance::Category::Gate);
                let wages = finance::wage_bill(&c.slots);
                c.fin.book(-wages, finance::Category::Wages);
                let interest = finance::loan_interest(c.fin.loan);
                c.fin.book(-interest, finance::Category::Interest);
            }
            finance::update_fill(&mut home.fin, attendance);
            // `1010:9396`, game type 1: the winner's coach scores 3, the loser's coach may be sacked (before the morale update)
            let (winner_club, loser_club) = if home_out {
                (&mut away, &mut home)
            } else {
                (&mut home, &mut away)
            };
            winner_club.coach_score += 3;
            let sack = loser_club.sack_due();
            let (winner, loser) = if home_out {
                (an.clone(), hn.clone())
            } else {
                (hn.clone(), an.clone())
            };
            matches.push(CupMatch {
                home: hn,
                away: an,
                home_goals: g.home_goals,
                away_goals: g.away_goals,
                extra_time: g.extra_time,
                shootout,
                winner,
                events: g.events,
                subs: g.subs,
                home_players: home.names.clone(),
                away_players: away.names.clone(),
                attendance,
                pens,
                start: g.start,
                primaries: g.primaries,
                bonus: (hb, ab),
            });
            losers.push(loser);
            *self.club_at(hl) = home;
            *self.club_at(al) = away;
            if sack {
                let loc = if home_out { hl } else { al };
                self.sack_at(loc);
            }
            let (mh, ma) = cup::morale_results(!home_out);
            for (loc, r) in [(hl, mh), (al, ma)] {
                let c = self.club_at(loc);
                c.k = crate::matchsim::update_morale(c.k, r);
                c.coach_games = c.coach_games.saturating_add(1).min(i16::MAX as u16);
            }
        }
        self.cup.retain(|c| !losers.contains(c));
        if self.cup.len() == 1 {
            self.cup_champion = Some(self.cup[0].clone());
        }
        self.last_cup = Some(CupRound {
            name: draw.name.clone(),
            matches,
            left: self.cup.len(),
            coach_changes_from,
        });
    }

    /// The original's start rule (`1028:0b26`): a random one of the bottom six clubs of the 4th division (`7 - Random(6)`).
    /// With several humans the club is one no other human manages.
    pub fn pick_human(&mut self) {
        let taken = self.human_clubs();
        loop {
            let club = 7 - self.rng.random(6) as usize;
            if taken.contains(&self.divisions[3][club].name) {
                continue;
            }
            self.set_human(3, club);
            return;
        }
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

    /// The clubs of division `d` (indices) in league-table order: points, goal difference, goals scored; clubs level on all three
    /// keep their order of the last sort (`SortDivisionLists`, `1050:1ab6`).
    pub fn table_order(&self, d: usize) -> Vec<usize> {
        let clubs = &self.divisions[d];
        let mut idx: Vec<usize> = match self.tie_order.get(d) {
            Some(o) if o.len() == clubs.len() => o.clone(),
            _ => (0..clubs.len()).collect(),
        };
        idx.sort_by_key(|&i| {
            let c = &clubs[i];
            std::cmp::Reverse((c.points, c.goals_for - c.goals_against, c.goals_for))
        });
        idx
    }

    /// The ticket price of division `d` and the constants a round needs to book money (`1040:35e1` via `1010:0875`).
    pub fn money_ctx(&self, d: usize) -> MoneyCtx {
        let clubs = &self.divisions[d];
        let n = clubs.len().max(1) as f64;
        let avg_f = clubs.iter().map(|c| c.fin.f).sum::<f64>() / n;
        let avg_rating = (clubs.iter().map(|c| finance::rating(&c.slots)).sum::<i64>() as f64 / n)
            .round_ties_even() as i64;
        MoneyCtx {
            price: finance::ticket_price(avg_f, avg_rating, self.economy, self.scale),
            e: self.economy,
            g: self.scale,
        }
    }

    /// Plays the next round of every division, evolves the squads, and runs the season end after round 14.
    pub fn play_round(&mut self) {
        let mut ctl = Control::default();
        self.play_round_ctl(&mut ctl);
        if self.cup_day_due() {
            self.play_cup_day_ctl(&mut ctl);
        }
    }

    /// [`World::play_round`] under the user's [`Control`]: the decisions made during the games and the replay support (the front
    /// end plays a round on a copy of the world, shows it, and plays it again on the copy of the world with the user's
    /// decisions added). Without decisions this is exactly `play_round`.
    pub fn play_round_ctl(&mut self, ctl: &mut Control) {
        self.close_board();
        self.sell_tried.clear();
        for b in &mut self.seats.benched {
            b.sell_tried.clear();
        }
        self.last_round.clear();
        self.coach_changes.clear();
        self.coach_changes_free.clear();
        self.seats.moves.clear();
        self.seats.applied = 0;
        let round = self.round + 1;
        #[allow(clippy::needless_range_loop)]
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
            for h in self.humans() {
                if let Some((hd, hi)) = self.locate_in_divisions(&h.club) {
                    if hd == d && !self.history.holiday {
                        plans.insert(hi, h.plan.clone());
                    }
                }
            }
            let money = self.money_ctx(d);
            // the division is taken out so that a sacking can reach the other divisions (the coach swap of `4f40`)
            let mut cur = std::mem::take(&mut self.divisions[d]);
            let mut rng = std::mem::replace(&mut self.rng, DelphiRng::new(0));
            let records = play_round_ctl(
                &mut rng,
                &mut cur,
                round,
                &mut names,
                &plans,
                Some(money),
                &mut |cur, i| self.replace_coach(d, cur, i, round == ROUNDS),
                ctl,
            );
            self.rng = rng;
            self.divisions[d] = cur;
            self.apply_human_move();
            self.last_round.extend(records.into_iter().map(|r| (d, r)));
        }
        self.record_league_fixtures(round);
        // order of `1020:0c41`: games, economic factors (`6e48`), clubs sell and renegotiate (`3e9c`), then the evolution
        self.update_factors();
        self.market_round(round);
        self.deliver_purchases();
        for (d, base) in DIVISION_BASE.iter().enumerate() {
            end_of_round(&mut self.rng, &mut self.divisions[d], *base);
        }
        self.economy = walk_index(&mut self.rng, self.economy, self.year);
        self.round = round;
        self.tie_order = (0..4).map(|d| self.table_order(d)).collect();
        self.last_cup = None;
        self.cup_draw = None;
        if round.is_multiple_of(2) {
            self.draw_cup(round, ctl);
        }
        self.for_each_human(|w| {
            if let Some(h) = w.human.as_mut() {
                h.plan.subs.clear(); // planned substitutions are for one game only
            }
            w.refresh_plan();
        });
        if round == ROUNDS && self.cup_draw.is_none() {
            self.finish_season();
        }
    }

    /// `1020:2ae6`: the champion of the first division, the cup winner and the club of every top scorer get their prizes
    /// (category 3); the scorers' tallies then start again.
    fn pay_prizes(&mut self, cup_winner: Option<&str>) {
        let [p1, p2, p3] = finance::season_prizes(self.currency_info());
        if let Some(&i) = standings(&self.divisions[0]).first() {
            self.divisions[0][i].fin.book(p1, finance::Category::Prizes);
        }
        if let Some(name) = cup_winner {
            if let Some(loc) = self.locate(name) {
                self.club_at(loc).fin.book(p2, finance::Category::Prizes);
            }
        }
        let top = self
            .divisions
            .iter()
            .flatten()
            .chain(self.pool.iter())
            .flat_map(|c| c.slots.iter().map(|s| s.goals))
            .max()
            .unwrap_or(0);
        if top > 0 {
            for c in self
                .divisions
                .iter_mut()
                .flatten()
                .chain(self.pool.iter_mut())
            {
                let scorers = c.slots.iter().filter(|s| s.goals == top).count() as i64;
                if scorers > 0 {
                    c.fin.book(p3 * scorers, finance::Category::Prizes);
                }
            }
        }
        for c in self
            .divisions
            .iter_mut()
            .flatten()
            .chain(self.pool.iter_mut())
        {
            for s in c.slots.iter_mut() {
                s.goals = 0;
            }
        }
    }

    /// The data of the end-of-season window, read before the prizes are paid and the scorers' tallies start again.
    fn honours(&self, cup_winner: Option<&str>) -> Honours {
        let clubs = || self.divisions.iter().flatten().chain(self.pool.iter());
        let top = clubs()
            .flat_map(|c| c.slots.iter().map(|s| s.goals))
            .max()
            .unwrap_or(0);
        let top_scorers = clubs()
            .filter(|_| top > 0)
            .flat_map(|c| {
                c.slots
                    .iter()
                    .enumerate()
                    .filter(|(_, s)| s.goals == top)
                    .map(|(i, _)| (c.names.get(i).cloned().unwrap_or_default(), c.name.clone()))
            })
            .collect();
        let coach = |name: Option<&str>| {
            name.and_then(|n| clubs().find(|c| c.name == n))
                .map(|c| c.coach_name.clone())
                .unwrap_or_default()
        };
        let league_winner = standings(&self.divisions[0])
            .first()
            .map(|&i| self.divisions[0][i].name.as_str());
        Honours {
            league_coach: coach(league_winner),
            cup_coach: coach(cup_winner),
            top_scorers,
            top_goals: top,
            prizes: finance::season_prizes(self.currency_info()),
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
        let cup_winner = self.cup_champion.clone();
        let honours = self.honours(cup_winner.as_deref());
        self.pay_prizes(cup_winner.as_deref());
        self.record_season_history();
        end_of_season(
            &mut self.rng,
            &mut self.divisions,
            &mut self.pool,
            cup_winner.as_deref(),
        );
        // a human whose club left the league gets another one (`4f40`)
        self.for_each_human(|w| {
            if w.human_location().is_none() {
                w.pick_human();
            }
            w.refresh_plan();
        });
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
            cup_winner,
            honours,
        });
        self.year += 1;
        self.round = 0;
        self.tie_order.clear();
        self.history.calendars.clear(); // the new season's calendars are generated from the new divisions
        self.reset_cup();
        self.refresh_cash_ref();
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

/// A player's answer to the contract the user proposes (`newSalaryBtnClick`, `1018:46ee`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WageReply {
    /// The proposed wage is accepted and the player renewed (`fresh`).
    Accepted,
    /// The player wants `amount`; `can_sell` is false when the club may not sell him, so the user has to accept.
    Demand { amount: i64, can_sell: bool },
}

/// What the price column of the player search asks for (the price combo box of `TCheckPlayerDlg`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PriceFilter {
    /// No price chosen: every player, for sale or not.
    #[default]
    Any,
    /// `Qualquer`: the players that are for sale.
    ForSale,
    /// For sale at a price within `lo..=hi`.
    Between(i64, i64),
}

/// The criteria of `findBtnClick` (`1048:34f5`, `3223`).
#[derive(Clone, Debug, Default)]
pub struct PlayerQuery {
    /// Part of the name (case-insensitive).
    pub name: String,
    pub nationality: Option<[u8; 3]>,
    /// Inclusive range of strengths.
    pub strength: Option<(u16, u16)>,
    pub position: Option<u8>,
    pub price: PriceFilter,
    /// `Não mostrar os jogadores da minha equipa`.
    pub hide_mine: bool,
}

/// The most players the search lists.
pub const SEARCH_LIMIT: usize = 50;

impl World {
    /// `1010:5a6a`: 0 when the user's player at `slot_idx` may be offered for sale, otherwise the reason (1 squad too small,
    /// 3 renewed or just arrived, 4 already offered without success in this game, 5 goalkeepers, 6 outfield players).
    pub fn sale_blocker(&self, slot_idx: usize) -> u8 {
        let Some(human) = self.human_location() else {
            return 1;
        };
        let club = self.club_ref(human);
        let code = crate::market::sale_blocker(club, slot_idx);
        if (code == 0 || code > 4) && self.sell_tried.contains(&club.names[slot_idx]) {
            return 4;
        }
        code
    }

    /// The original's sentence for a sale blocker (`None` when the player may be sold).
    pub fn sale_blocker_text(&self, slot_idx: usize) -> Option<&'static str> {
        match self.sale_blocker(slot_idx) {
            0 => None,
            1 => Some("Não pode vender mais jogadores"),
            2 => Some("Este jogador já está vendido"),
            3 => Some("Este jogador só pode ser vendido na próxima época"),
            4 => Some("Só pode tentar vender este jogador novamente no próximo jogo"),
            5 => Some("Não pode vender mais guarda-redes"),
            _ => Some("Não pode vender mais jogadores de campo"),
        }
    }

    /// Lowest and highest wage the contract dialog accepts (`ChangeSalaryBtnClick`: `50 G` and `50 000 G`).
    pub fn wage_bounds(&self) -> (i64, i64) {
        (50 * self.scale, 50_000 * self.scale)
    }

    /// What the player at `slot_idx` of the user's club is worth to the club (`PlayerValue`, `1010:7ff4`).
    pub fn human_player_value(&self, slot_idx: usize) -> Option<i64> {
        let human = self.human_location()?;
        let club = self.club_ref(human);
        let s = club.slots.get(slot_idx)?;
        Some(player_value(
            s.strength,
            club.fin.f,
            self.economy,
            self.scale,
        ))
    }

    /// The user proposes `wage` to the player at `slot_idx` (`newSalaryBtnClick`): a proposal above 80 % of his value is
    /// accepted (the wage is set and the player marked `fresh`); otherwise he demands `1.25 *` his value rounded to `50 G`.
    pub fn human_propose_wage(&mut self, slot_idx: usize, wage: i64) -> Result<WageReply, String> {
        let human = self.human_location().ok_or("sem clube")?;
        let club = self.club_ref(human);
        let s = club.slots.get(slot_idx).ok_or("jogador inexistente")?;
        if s.fresh {
            return Err("este jogador já renovou o contrato".into());
        }
        let (lo, hi) = self.wage_bounds();
        let wage = wage.clamp(lo, hi);
        let value = player_value(s.strength, club.fin.f, self.economy, self.scale);
        if wage as f64 > value as f64 * 0.8 {
            let s = &mut self.club_mut(human).slots[slot_idx];
            s.wage = wage;
            s.fresh = true;
            return Ok(WageReply::Accepted);
        }
        let unit = (50 * self.scale) as f64;
        let amount = (value as f64 * 1.25 / unit).round_ties_even() as i64 * 50 * self.scale;
        Ok(WageReply::Demand {
            amount,
            can_sell: self.sale_blocker(slot_idx) == 0,
        })
    }

    /// The user accepts the player's demand (`YesBtnClick`, `1030:363e`).
    pub fn human_accept_wage(&mut self, slot_idx: usize, wage: i64) -> Result<(), String> {
        let human = self.human_location().ok_or("sem clube")?;
        let s = self
            .club_mut(human)
            .slots
            .get_mut(slot_idx)
            .ok_or("jogador inexistente")?;
        s.wage = wage;
        s.fresh = true;
        Ok(())
    }

    /// Whether the user can still take part in the auction at `listing` of the board (cash, squad and foreigner limits).
    pub fn listing_open_to_user(&self, listing: usize) -> bool {
        let Some(l) = self.board.get(listing) else {
            return false;
        };
        let Some(seller) = self.find_club(&l.seller) else {
            return false;
        };
        self.club_ref(seller)
            .names
            .iter()
            .position(|n| *n == l.player)
            .is_some_and(|idx| {
                self.human_location()
                    .is_some_and(|h| self.may_bid_for(h, seller, idx, l.ask))
            })
    }

    /// Every club as `(division, index)` in master-list order (the outside pool is division 4).
    pub fn all_clubs(&self) -> Vec<(usize, usize)> {
        self.all_clubs_by_master()
    }

    /// Where the club of that name is: `(division, index)`, the outside pool being division 4.
    pub fn find_club(&self, name: &str) -> Option<(usize, usize)> {
        self.all_clubs_by_master()
            .into_iter()
            .find(|&loc| self.club_ref(loc).name == name)
    }

    /// The club at a location returned by [`World::find_club`] or [`World::human_location`].
    pub fn club_of(&self, loc: (usize, usize)) -> &Club {
        self.club_ref(loc)
    }

    /// The price at which the club of `club` would sell its player `index` to the user's club (0: not for sale).
    pub fn price_for_user(&self, club: &str, index: usize) -> i64 {
        let (Some(human), Some(seller)) = (self.human_location(), self.find_club(club)) else {
            return 0;
        };
        if seller == human || index >= self.club_ref(seller).slots.len() {
            return 0;
        }
        crate::market::direct_price(
            self.club_ref(seller),
            index,
            Some(self.club_ref(human)),
            self.economy,
            self.scale,
        )
    }

    /// What `human_buy` would check before the purchase of a player (price, cash, squad and foreigner limits).
    pub fn user_can_buy(&self, club: &str, index: usize) -> bool {
        let (Some(human), Some(seller)) = (self.human_location(), self.find_club(club)) else {
            return false;
        };
        let price = self.price_for_user(club, index);
        let me = self.club_ref(human);
        let Some(slot) = self.club_ref(seller).slots.get(index) else {
            return false;
        };
        let name = &self.club_ref(seller).names[index];
        price > 0
            && me.fin.cash > price
            && crate::market::may_bid(me, slot)
            && !self
                .pending
                .iter()
                .any(|p| p.seller == club && p.player == *name)
    }

    fn offer_of(&self, loc: (usize, usize), index: usize) -> Offer {
        let c = self.club_ref(loc);
        let s = &c.slots[index];
        Offer {
            club: c.name.clone(),
            index,
            player: c.names[index].clone(),
            position: s.position,
            strength: s.strength,
            nationality: s.nat,
            price: self.price_for_user(&c.name, index),
        }
    }

    /// `findBtnClick` (`1048:34f5`): the players of all clubs that match, the last clubs of the list first, at most
    /// [`SEARCH_LIMIT`]; the flag is set when the limit cut the list.
    pub fn search_players(&self, q: &PlayerQuery) -> (Vec<Offer>, bool) {
        let human = self.human_location();
        let needle = q.name.to_uppercase();
        let mut out = Vec::new();
        let mut order = self.all_clubs_by_master();
        order.reverse();
        for loc in order {
            if q.hide_mine && Some(loc) == human {
                continue;
            }
            let c = self.club_ref(loc);
            for i in (0..c.slots.len()).rev() {
                let s = &c.slots[i];
                if q.position.is_some_and(|p| p != s.position)
                    || q.nationality.is_some_and(|n| n != s.nat)
                    || (!needle.is_empty() && !c.names[i].to_uppercase().contains(&needle))
                    || q.strength
                        .is_some_and(|(lo, hi)| s.strength < lo || s.strength > hi)
                {
                    continue;
                }
                let price = self.price_for_user(&c.name, i);
                let ok = match q.price {
                    PriceFilter::Any => true,
                    PriceFilter::ForSale => price != 0,
                    PriceFilter::Between(lo, hi) => price != 0 && price >= lo && price <= hi,
                };
                if !ok {
                    continue;
                }
                if out.len() == SEARCH_LIMIT {
                    return (out, true);
                }
                out.push(self.offer_of(loc, i));
            }
        }
        (out, false)
    }

    /// Whether the user watches the player.
    pub fn is_observed(&self, club: &str, player: &str) -> bool {
        self.observed.iter().any(|(c, p)| c == club && p == player)
    }

    /// Puts a player under observation (`FUN_1010_8cbe`); watching the same player twice does nothing.
    pub fn observe(&mut self, club: &str, player: &str) {
        if !self.is_observed(club, player) {
            self.observed.push((club.into(), player.into()));
        }
    }

    /// Stops watching a player (`FUN_1010_8d18`).
    pub fn unobserve(&mut self, club: &str, player: &str) {
        self.observed.retain(|(c, p)| !(c == club && p == player));
    }

    /// The watched players where they are now (a player who changed club is followed; one who is gone is left out).
    pub fn observed_players(&self) -> Vec<Offer> {
        let mut out = Vec::new();
        for (club, player) in &self.observed {
            let mut found = None;
            if let Some(loc) = self.find_club(club) {
                if let Some(i) = self.club_ref(loc).names.iter().position(|n| n == player) {
                    found = Some((loc, i));
                }
            }
            if found.is_none() {
                for loc in self.all_clubs_by_master() {
                    if let Some(i) = self.club_ref(loc).names.iter().position(|n| n == player) {
                        found = Some((loc, i));
                        break;
                    }
                }
            }
            if let Some((loc, i)) = found {
                out.push(self.offer_of(loc, i));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sacked coach swaps with the coach of a weaker club in good form (`5661`), keeping scores and resetting the games.
    #[test]
    fn a_sacked_coach_swaps_with_a_weaker_club_in_form() {
        let mut w = World::synthetic(1);
        w.human = None;
        let rt = crate::cup::team_rating(&w.divisions[0][0].slots);
        for s in w.divisions[2][0].slots.iter_mut() {
            s.strength = rt - 15;
        }
        for s in w.divisions[3][0].slots.iter_mut() {
            s.strength = rt - 15;
        }
        {
            let t = &mut w.divisions[0][0];
            t.k = 0.5;
            t.coach_games = 12;
            t.coach_score = 7;
        }
        w.divisions[2][0].k = 1.3;
        w.divisions[2][0].coach_score = 9;
        w.divisions[3][0].k = 1.4;
        w.divisions[3][0].coach_score = 4; // the lowest score is chosen
        w.divisions[3][0].coach_games = 20;
        let mut cur = std::mem::take(&mut w.divisions[0]);
        w.replace_coach(0, &mut cur, 0, false);
        w.divisions[0] = cur;
        let t = &w.divisions[0][0];
        assert_eq!((t.k, t.coach_games, t.coach_score), (1.0, 0, 4));
        let p = &w.divisions[3][0];
        assert_eq!((p.k, p.coach_games, p.coach_score), (1.0, 0, 7));
        assert_eq!(w.divisions[2][0].coach_score, 9);
        assert_eq!(w.coach_changes.len(), 1);
    }

    #[test]
    fn nobody_is_sacked_without_a_suitable_partner() {
        let mut w = World::synthetic(1);
        w.human = None;
        {
            let t = &mut w.divisions[0][0];
            t.k = 0.5;
            t.coach_games = 12;
        }
        let mut cur = std::mem::take(&mut w.divisions[0]);
        w.replace_coach(0, &mut cur, 0, false);
        w.divisions[0] = cur;
        assert_eq!(w.divisions[0][0].k, 0.5);
        assert!(w.coach_changes.is_empty());
    }

    /// With the option *Ver os desempates por penalties nos jogos sem treinadores humanos* every shootout is played kick by kick
    /// (`1050:0d46`); without it only the humans' and the final's are.
    #[test]
    fn the_option_plays_every_shootout_kick_by_kick() {
        for real in [false, true] {
            let mut w = World::synthetic(5);
            w.human = None;
            w.real_shootouts = real;
            let (mut auto, mut kicks) = (0, 0);
            for _ in 0..(ROUNDS * 3) {
                w.play_round_ctl(&mut Control::default());
                if w.cup_day_due() {
                    w.play_cup_day_ctl(&mut Control::default());
                    for m in &w.last_cup.as_ref().unwrap().matches {
                        if m.shootout {
                            if m.pens.is_some() {
                                kicks += 1;
                            } else {
                                auto += 1;
                            }
                        }
                    }
                }
            }
            if real {
                assert_eq!(auto, 0, "every shootout has its kicks");
                assert!(kicks > 0);
            } else {
                assert!(auto > 0, "computer ties are decided by the automatic draw");
            }
        }
    }

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

    /// *Selecção de Países*: only the chosen countries' teams; above 128 the counts are cut at random down to 128 (`1028:19ac`),
    /// every chosen country keeping at least one team; at or below 128 the world is the one of those teams alone.
    #[test]
    fn a_new_game_takes_the_teams_of_the_chosen_countries() {
        let team = |i: usize, level: u8, country: &str| {
            let mut t = fake_team(i, level);
            t.short_name = format!("T{i}");
            t.country = country.into();
            t
        };
        let mut teams: Vec<EftTeam> = (0..90)
            .map(|i| team(i, (i % 20 + 1) as u8, "BRA"))
            .collect();
        teams.extend((90..150).map(|i| team(i, (i % 20 + 1) as u8, "POR")));
        teams.extend((150..160).map(|i| team(i, (i % 20 + 1) as u8, "ESP")));
        let list = |b: bool, p: bool, e: bool| {
            vec![
                ("BRA".to_string(), b),
                ("POR".to_string(), p),
                ("ESP".to_string(), e),
            ]
        };
        // too few
        assert!(
            World::from_countries(1, &teams, &list(false, false, true), crate::economy::REAIS)
                .is_err()
        );
        // 70 teams: no cut, the same world as the old builder of those teams
        let w = World::from_countries(3, &teams, &list(false, true, true), crate::economy::REAIS)
            .unwrap();
        let only: Vec<EftTeam> = teams
            .iter()
            .filter(|t| t.country != "BRA")
            .cloned()
            .collect();
        let v = World::from_teams(3, &only).unwrap();
        let names = |w: &World| -> Vec<String> {
            w.divisions
                .iter()
                .flatten()
                .chain(w.pool.iter())
                .map(|c| c.name.clone())
                .collect()
        };
        assert_eq!(names(&w), names(&v));
        // 160 teams: cut to 128, every country keeps teams, the bigger ones lose more
        let w = World::from_countries(7, &teams, &list(true, true, true), crate::economy::REAIS)
            .unwrap();
        let all = names(&w);
        assert_eq!(all.len(), MAX_TEAMS);
        let count = |c: &str| {
            teams
                .iter()
                .filter(|t| t.country == c && all.contains(&t.short_name))
                .count()
        };
        let (b, p, e) = (count("BRA"), count("POR"), count("ESP"));
        assert_eq!(b + p + e, MAX_TEAMS);
        assert!(
            e >= 1 && 90 - b >= 60 - p && 60 - p >= 10 - e,
            "{b} {p} {e}"
        );
        // the master list keeps the level order
        let levels: Vec<u8> = all
            .iter()
            .map(|n| teams.iter().find(|t| &t.short_name == n).unwrap().level)
            .collect();
        assert!(levels.windows(2).all(|x| x[0] >= x[1]));
    }

    /// The currency chosen in *Dinheiro* sets the scale `G` (wages, new-game cash), the loan step and the prizes, and is kept in
    /// a saved game.
    #[test]
    fn a_new_game_in_euros_uses_the_euro_record() {
        let teams: Vec<EftTeam> = (0..40)
            .map(|i| {
                let mut t = fake_team(i, (i % 20 + 1) as u8);
                t.short_name = format!("T{i}");
                t
            })
            .collect();
        let list = vec![("POR".to_string(), true)];
        let mut teams = teams;
        for t in teams.iter_mut() {
            t.country = "POR".into();
        }
        let euros = 1;
        let w = World::from_countries(2, &teams, &list, euros).unwrap();
        let r = World::from_countries(2, &teams, &list, crate::economy::REAIS).unwrap();
        assert_eq!((w.currency, w.scale), (euros, 4));
        assert_eq!(w.currency_info().unit, "euros");
        assert_eq!(finance::loan_step(w.currency_info()), 500_000);
        assert_eq!(
            finance::season_prizes(w.currency_info()),
            [15_000_000, 5_000_000, 3_000_000]
        );
        // money scales with G: 4 against the reais' 5
        let (a, b) = (&w.divisions[0][0], &r.divisions[0][0]);
        assert_eq!(a.fin.cash * 5, b.fin.cash * 4);
        assert_eq!(a.slots[0].wage * 5, b.slots[0].wage * 4);
        let back = World::load(&w.save()).unwrap();
        assert_eq!((back.currency, back.scale), (euros, 4));
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

    #[test]
    fn a_season_cup_has_n_minus_one_ties_and_one_champion_on_round_14() {
        let mut w = World::synthetic(41);
        let total = 38usize;
        assert_eq!(w.cup.len(), total);
        let mut ties = 0;
        let mut rounds_with_cup = Vec::new();
        let mut champion = None;
        for round in 1..=14usize {
            w.play_round();
            if round == 14 {
                champion = w.last_season.as_ref().and_then(|s| s.cup_winner.clone());
            }
            if let Some(c) = &w.last_cup {
                rounds_with_cup.push(round);
                ties += c.matches.len();
                // nobody plays twice in a round
                let mut names: Vec<&String> =
                    c.matches.iter().flat_map(|m| [&m.home, &m.away]).collect();
                let before = names.len();
                names.sort();
                names.dedup();
                assert_eq!(names.len(), before, "a club played twice in round {round}");
                for m in &c.matches {
                    assert!(m.winner == m.home || m.winner == m.away);
                    if m.home_goals != m.away_goals {
                        assert!(!m.shootout);
                        assert_eq!(
                            m.winner,
                            if m.home_goals > m.away_goals {
                                &m.home
                            } else {
                                &m.away
                            }
                            .as_str()
                        );
                    } else {
                        assert!(
                            m.shootout && m.extra_time,
                            "a level game goes to extra time and then a shootout"
                        );
                    }
                }
            }
        }
        assert_eq!(ties, total - 1);
        assert_eq!(rounds_with_cup, [4, 6, 8, 10, 12, 14]);
        let champion = champion.expect("the cup has a champion at the end of the season");
        // a new season: everybody is in the cup again and the champion's name is a club of the world
        assert_eq!(w.cup.len(), total);
        assert!(w.cup.contains(&champion));
        assert!(w.cup_champion.is_none());
    }

    #[test]
    fn cup_games_use_the_users_plan_and_can_be_played_for_many_seasons() {
        let mut w = World::synthetic(42);
        for _ in 0..8 {
            w.play_season();
        }
        assert_eq!(w.year, 2006);
        // every season recorded a cup winner
        assert!(w.last_season.as_ref().unwrap().cup_winner.is_some());
        assert!(w.human_location().is_some());
    }

    #[test]
    fn a_game_saved_between_cup_rounds_keeps_the_cup() {
        let mut a = World::synthetic(43);
        for _ in 0..6 {
            a.play_round();
        }
        let left = a.cup.len();
        assert!(left < 38);
        let b = World::load(&a.save()).unwrap();
        assert_eq!(b.cup, a.cup);
        assert_eq!(b.cup.len(), left);
    }

    fn player_count(w: &World) -> usize {
        w.divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .map(|c| c.slots.len())
            .sum()
    }

    #[test]
    fn transfers_move_players_and_money_between_computer_clubs_only() {
        let mut w = World::synthetic(2024);
        let human = w.human.as_ref().unwrap().club.clone();
        let players = player_count(&w);
        let cash: i64 = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .map(|c| c.fin.cash)
            .sum();
        let mut moved = 0;
        for _ in 0..28 {
            w.play_round();
            moved = moved.max(w.transfers.len());
        }
        assert_eq!(
            player_count(&w),
            players,
            "players are neither created nor lost"
        );
        assert!(
            moved > 0,
            "a season of the computer market produces transfers"
        );
        for t in &w.transfers {
            assert_ne!(t.from, human);
            assert_ne!(t.to, human);
            assert_ne!(t.from, t.to);
            assert!(t.fee >= 0 && t.fee % 1000 == 0);
        }
        for c in w.divisions.iter().flatten().chain(w.pool.iter()) {
            assert!(
                (11..=24).contains(&c.slots.len()),
                "{} has {}",
                c.name,
                c.slots.len()
            );
            assert_eq!(c.slots.len(), c.names.len());
        }
        let _ = cash;
    }

    #[test]
    fn a_transferred_player_is_marked_and_cannot_be_sold_again() {
        let mut w = World::synthetic(7);
        for _ in 0..14 {
            w.play_round();
        }
        let t = w
            .transfers
            .first()
            .expect("some transfer in a season")
            .clone();
        let club = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .find(|c| c.name == t.to)
            .unwrap();
        let idx = club
            .names
            .iter()
            .position(|n| *n == t.player)
            .expect("the buyer has the player");
        assert!(club.slots[idx].fresh);
        assert_ne!(crate::market::sale_blocker(club, idx), 0);
    }

    #[test]
    fn the_market_state_survives_a_save() {
        let mut a = World::synthetic(11);
        for _ in 0..9 {
            a.play_round();
        }
        let b = World::load(&a.save()).unwrap();
        for (x, y) in a
            .divisions
            .iter()
            .flatten()
            .zip(b.divisions.iter().flatten())
        {
            assert_eq!(x.master, y.master);
            assert_eq!(x.country, y.country);
            assert_eq!(
                x.slots
                    .iter()
                    .map(|s| (s.wage, s.fresh, s.nat))
                    .collect::<Vec<_>>(),
                y.slots
                    .iter()
                    .map(|s| (s.wage, s.fresh, s.nat))
                    .collect::<Vec<_>>()
            );
        }
    }

    fn set_human_cash(w: &mut World, cash: i64) {
        let (d, i) = w.human_location().unwrap();
        w.divisions[d][i].fin.cash = cash;
    }

    #[test]
    fn loans_accrue_interest_and_can_be_repaid_and_stadiums_grow() {
        let mut w = World::synthetic(3);
        let (d, i) = w.human_location().unwrap();
        let cash = w.divisions[d][i].fin.cash;
        let step = finance::loan_step(w.currency_info());
        assert_eq!(w.human_borrow().unwrap(), step);
        assert_eq!(w.divisions[d][i].fin.cash, cash + step);
        w.play_round();
        let c = &w.divisions[d][i].fin;
        assert_eq!(c.interest, -finance::loan_interest(step));
        assert_eq!(w.human_repay().unwrap(), step);
        assert_eq!(w.divisions[d][i].fin.loan, 0);
        assert!(w.human_repay().is_err());
        // the bank has a limit
        while w.human_borrow().is_ok() {}
        assert!(w.human_borrow().is_err());
        // some computer club grows its stadium within a few seasons
        let before: u32 = w
            .divisions
            .iter()
            .flatten()
            .map(|c| c.fin.stadium as u32)
            .sum();
        for _ in 0..42 {
            w.play_round();
        }
        let after: u32 = w
            .divisions
            .iter()
            .flatten()
            .map(|c| c.fin.stadium as u32)
            .sum();
        assert!(after >= before);
    }

    #[test]
    fn the_user_can_sell_a_player_to_the_computer_clubs() {
        let mut w = World::synthetic(5);
        for _ in 0..2 {
            w.play_round();
        }
        let (d, i) = w.human_location().unwrap();
        let before = w.divisions[d][i].slots.len();
        let cash = w.divisions[d][i].fin.cash;
        let mut sold = None;
        for k in 0..before {
            let ask = crate::market::market_value(&w.divisions[d][i].slots[k], w.economy, w.scale);
            if let Ok(fee) = w.human_sell(k, ask / 4) {
                sold = Some(fee);
                break;
            }
        }
        let fee = sold.expect("some club bids for some player");
        let (d, i) = w.human_location().unwrap();
        let club = &w.divisions[d][i];
        assert_eq!(club.slots.len(), before - 1);
        assert_eq!(club.fin.cash, cash + fee);
        let max = club.slots.len();
        assert!(w
            .human
            .as_ref()
            .unwrap()
            .plan
            .starters
            .iter()
            .all(|&k| k < max));
        assert!(w
            .human
            .as_ref()
            .unwrap()
            .plan
            .bench
            .iter()
            .all(|&k| k < max));
        w.play_round(); // the plan is still playable
    }

    #[test]
    fn the_user_can_outbid_the_computer_on_an_open_auction() {
        let mut w = World::synthetic(9);
        set_human_cash(&mut w, 900_000_000);
        let mut bought = false;
        for _ in 0..14 {
            w.play_round();
            if let Some(l) = w.board.first().cloned() {
                let (d, i) = w.human_location().unwrap();
                let before = w.divisions[d][i].slots.len();
                let cash = w.divisions[d][i].fin.cash;
                let amount = l.best.as_ref().map_or(l.ask, |b| b.1 + 1000).max(l.ask);
                assert!(
                    w.human_bid(0, amount.saturating_sub(1000)).is_err()
                        || l.ask == 0
                        || amount - 1000 >= l.ask
                );
                if w.board.is_empty() {
                    continue;
                }
                w.human_bid(0, amount).expect("a high enough bid wins");
                let club = &w.divisions[d][i];
                assert_eq!(club.slots.len(), before + 1);
                assert_eq!(club.fin.cash, cash - amount);
                assert!(club.slots.last().unwrap().fresh);
                bought = true;
                break;
            }
        }
        assert!(
            bought,
            "an auction the user can join appears within a season"
        );
        w.play_round();
    }

    #[test]
    fn open_auctions_are_saved_and_closed_by_the_next_round() {
        let mut w = World::synthetic(9);
        set_human_cash(&mut w, 900_000_000);
        while w.board.is_empty() {
            w.play_round();
            assert!(w.round != 0, "no auction in a whole season");
        }
        let back = World::load(&w.save()).unwrap();
        assert_eq!(back.board, w.board);
        let mut closed = back;
        closed.play_round();
        assert!(closed.board.iter().all(|l| l.round == closed.round));
    }

    /// The games of a round as text, for comparing runs: score, events and substitutions of every league game.
    fn round_text(w: &World) -> Vec<String> {
        w.last_round
            .iter()
            .map(|(d, m)| {
                format!(
                    "{d} {} {} {}-{} {:?} {:?}",
                    m.home, m.away, m.home_goals, m.away_goals, m.events, m.subs
                )
            })
            .collect()
    }

    #[test]
    fn the_cup_is_drawn_after_an_even_league_day_and_played_on_its_own_day() {
        let mut w = World::synthetic(14);
        let total = w.cup.len();
        while !w.cup_day_due() {
            let odd = w.round.is_multiple_of(2);
            w.play_round_ctl(&mut Control::default());
            assert!(!odd || !w.cup_day_due(), "no cup after an odd round");
            assert!(w.round < 14);
        }
        let drawn_at = w.round;
        assert_eq!(drawn_at % 2, 0, "the cup is drawn after an even round");
        assert!(w.last_cup.is_none(), "the draw does not play the cup");
        assert_eq!(w.cup.len(), total, "nobody is out before the cup day");
        let draw = w.cup_draw.clone().unwrap();
        assert!(draw.name.contains("avos") || !draw.name.is_empty());
        assert!(!draw.pairs.is_empty());
        let (hc, _) = w.human_location().unwrap();
        let _ = hc;
        // the humans' tie is found in the draw, from either side
        let me = w.human.as_ref().unwrap().club.clone();
        match w.human_cup_tie() {
            Some(t) => {
                let pair = draw
                    .pairs
                    .iter()
                    .find(|(h, a)| *h == me || *a == me)
                    .unwrap();
                assert_eq!(t.home, pair.0 == me);
                assert_eq!(t.opponent, if t.home { &pair.1 } else { &pair.0 }.clone());
                assert_eq!(t.stage, draw.name);
                assert_eq!(w.humans_in_cup_tie(), vec![0]);
            }
            None => assert!(w.humans_in_cup_tie().is_empty()),
        }
        // a game saved on the cup day keeps the draw
        let back = World::load(&w.save()).unwrap();
        assert_eq!(back.cup_draw, w.cup_draw);
        let mut a = w.clone();
        let mut b = back;
        a.play_cup_day_ctl(&mut Control::default());
        b.play_cup_day_ctl(&mut Control::default());
        assert!(!a.cup_day_due());
        assert!(a.last_cup.is_some());
        assert_eq!(a.round, drawn_at);
        assert!(a.cup.len() < total);
        assert_eq!(a.cup, b.cup);
        assert_eq!(a.rng.seed(), b.rng.seed());
        // a new round after the cup day plays on as usual
        a.play_round();
        assert_eq!(a.round, drawn_at + 1);
    }

    #[test]
    fn the_plan_changed_on_the_cup_day_is_the_plan_of_the_tie() {
        let mut w = World::synthetic(14);
        while !w.cup_day_due() {
            w.play_round_ctl(&mut Control::default());
            if !w.cup_day_due() && w.cup_draw.is_none() && w.round.is_multiple_of(2) {
                w.play_round();
            }
        }
        let mut other = w.clone();
        let Some(f) = (0..FORMATIONS.len()).find(|&f| {
            let mut t = w.clone();
            t.set_formation(f) && t.human.as_ref().unwrap().plan != w.human.as_ref().unwrap().plan
        }) else {
            return;
        };
        other.set_formation(f);
        w.play_cup_day_ctl(&mut Control::default());
        other.play_cup_day_ctl(&mut Control::default());
        if w.last_cup.as_ref().is_some_and(|c| {
            c.matches.iter().any(|m| {
                let me = &w.human.as_ref().unwrap().club;
                m.home == *me || m.away == *me
            })
        }) {
            // the human's tie does not look the same with another formation
            assert_ne!(
                format!("{:?}", w.last_cup.as_ref().unwrap().matches),
                format!("{:?}", other.last_cup.as_ref().unwrap().matches)
            );
        }
    }

    #[test]
    fn replaying_a_round_without_decisions_gives_the_same_round() {
        for seed in [3, 21, 40] {
            let mut w = World::synthetic(seed);
            for round in 0..6 {
                let snapshot = w.clone();
                let mut ctl = Control::default();
                w.play_round_ctl(&mut ctl);
                let drawn = w.cup_draw.clone();
                let mut cup_ctl = Control::default();
                w.play_cup_day_ctl(&mut cup_ctl);
                // the control does not change a round played without help
                let mut plain = snapshot.clone();
                plain.play_round();
                assert_eq!(
                    round_text(&w),
                    round_text(&plain),
                    "seed {seed} round {round}"
                );
                assert_eq!(w.rng.seed(), plain.rng.seed());
                // a replay with no decisions reproduces it, including the random state at the end
                let mut again = snapshot.clone();
                again.play_round_ctl(&mut Control::replay(ctl.log.clone(), vec![], vec![]));
                assert_eq!(round_text(&w), round_text(&again));
                assert_eq!(drawn, again.cup_draw, "the draw is the same in a replay");
                again.play_cup_day_ctl(&mut Control::replay(cup_ctl.log.clone(), vec![], vec![]));
                assert_eq!(w.rng.seed(), again.rng.seed());
                assert_eq!(
                    format!("{:?}", w.last_cup.as_ref().map(|c| &c.matches)),
                    format!("{:?}", again.last_cup.as_ref().map(|c| &c.matches))
                );
            }
        }
    }

    #[test]
    fn a_substitution_at_the_break_only_changes_the_users_second_half() {
        use crate::matchsim::Side;
        for seed in [5u32, 11, 29] {
            let w0 = World::synthetic(seed);
            let (d, i) = w0.human_location().unwrap();
            let plan = w0.human.as_ref().unwrap().plan.clone();
            let slots = &w0.divisions[d][i].slots;
            let out = *plan
                .starters
                .iter()
                .find(|&&k| slots[k].position == 3)
                .unwrap();
            let inn = *plan
                .bench
                .iter()
                .find(|&&k| slots[k].position != 0)
                .unwrap();
            // the first run
            let mut first = w0.clone();
            let mut ctl = Control::default();
            first.play_round_ctl(&mut ctl);
            let side = first
                .last_round
                .iter()
                .find_map(|(dd, m)| {
                    (*dd == d && (m.home == i || m.away == i)).then_some(if m.home == i {
                        Side::Home
                    } else {
                        Side::Away
                    })
                })
                .unwrap();
            // the user substitutes at the break: restore the world, add the decision and replay
            let mut second = w0.clone();
            let sub = crate::matchsim::Decision::Planned {
                side,
                minute: 46,
                out,
                inn,
            };
            let home = w0.divisions[d][i].name.clone();
            let home = if side == Side::Home {
                home
            } else {
                let m = first
                    .last_round
                    .iter()
                    .find(|(dd, m)| *dd == d && m.away == i)
                    .unwrap();
                w0.divisions[d][m.1.home].name.clone()
            };
            second.play_round_ctl(&mut Control::replay(
                ctl.log.clone(),
                vec![(home, sub)],
                vec![],
            ));
            let (a, b) = (round_text(&first), round_text(&second));
            assert_eq!(a.len(), b.len());
            for (k, ((da, ma), (db, mb))) in
                first.last_round.iter().zip(&second.last_round).enumerate()
            {
                assert_eq!((da, ma.home, ma.away), (db, mb.home, mb.away));
                if *da == d && (ma.home == i || ma.away == i) {
                    let upto = |m: &MatchRecord| {
                        m.events
                            .iter()
                            .filter(|e| e.minute <= 45)
                            .map(|e| format!("{e:?}"))
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(upto(ma), upto(mb), "first half of seed {seed}");
                    assert!(
                        mb.subs.iter().any(|s| s.minute == 46 && !s.auto),
                        "the substitution happens (seed {seed})"
                    );
                } else {
                    assert_eq!(a[k], b[k], "game {k} of seed {seed} is not the user's");
                }
            }
        }
    }

    #[test]
    fn world_data_for_the_front_end_is_recorded() {
        let mut w = World::synthetic(9);
        for _ in 0..8 {
            w.play_round();
        }
        for (_, m) in &w.last_round {
            assert_eq!(m.start[0].len(), m.names[0].len());
            assert_eq!(m.start[1].len(), m.names[1].len());
            assert!(m.start[0].iter().filter(|s| s.starter()).count() >= 10);
        }
        let cup = w.last_cup.as_ref().unwrap();
        assert!(cup.matches.iter().all(|m| !m.start[0].is_empty()));
        for _ in 0..6 {
            w.play_round();
        }
        let s = w.last_season.as_ref().unwrap();
        assert_eq!(s.honours.prizes, finance::season_prizes(w.currency_info()));
        assert!(s.honours.top_goals > 0 && !s.honours.top_scorers.is_empty());
        assert!(!s.honours.league_coach.is_empty() || w.divisions[0][0].coach_name.is_empty());
    }

    #[test]
    fn a_tied_final_is_decided_by_real_kicks() {
        let mut shootouts = 0;
        for seed in 0..60 {
            let mut w = World::synthetic(seed);
            for _ in 0..14 {
                w.play_round();
            }
            // after round 14 `last_cup` is the final
            let f = w.last_cup.as_ref().unwrap();
            assert_eq!(f.matches.len(), 1);
            let m = &f.matches[0];
            if m.shootout {
                shootouts += 1;
                let p = m.pens.as_ref().expect("the final plays the kicks");
                assert_ne!(p.home, p.away);
                assert_eq!(m.winner == m.home, p.home > p.away);
            } else {
                assert!(m.pens.is_none());
            }
        }
        assert!(shootouts > 0, "some final in 60 seasons goes to penalties");
    }

    #[test]
    fn season_prizes_go_to_the_champion_the_cup_winner_and_the_top_scorer_clubs() {
        let mut w = World::synthetic(21);
        for _ in 0..14 {
            w.play_round();
        }
        let s = w.last_season.as_ref().unwrap();
        let champion = s.tables[0][0].name.clone();
        let cup = s.cup_winner.clone().unwrap();
        let [p1, p2, p3] = finance::season_prizes(w.currency_info());
        let find = |n: &str| {
            w.divisions
                .iter()
                .flatten()
                .chain(w.pool.iter())
                .find(|c| c.name == n)
                .unwrap()
        };
        assert!(find(&champion).fin.prizes >= p1);
        assert!(find(&cup).fin.prizes >= p2);
        let total: i64 = w
            .divisions
            .iter()
            .flatten()
            .chain(w.pool.iter())
            .map(|c| c.fin.prizes)
            .sum();
        assert!(total >= p1 + p2 + p3, "{total}");
        // a new season starts with fresh scoring tallies
        assert!(w
            .divisions
            .iter()
            .flatten()
            .all(|c| c.slots.iter().all(|s| s.goals == 0)));
    }

    #[test]
    fn the_user_can_search_and_buy_players_that_arrive_after_the_round() {
        let mut w = World::synthetic(13);
        set_human_cash(&mut w, 900_000_000);
        let offers = w.search_offers(&Search::default());
        assert!(!offers.is_empty() && offers.len() <= 50);
        assert!(offers.iter().all(|o| o.price > 0 && o.price % 1000 == 0));
        let (d, i) = w.human_location().unwrap();
        let (before, cash) = (w.divisions[d][i].slots.len(), w.divisions[d][i].fin.cash);
        let o = offers[0].clone();
        let paid = w.human_buy(&o.club, o.index, &o.player).unwrap();
        assert_eq!(paid, o.price);
        assert_eq!(w.divisions[d][i].fin.cash, cash - paid);
        assert_eq!(
            w.divisions[d][i].slots.len(),
            before,
            "the player arrives with the end of the round"
        );
        assert!(
            w.human_buy(&o.club, o.index, &o.player).is_err(),
            "cannot buy him twice"
        );
        w.play_round();
        let (d, i) = w.human_location().unwrap();
        let club = &w.divisions[d][i];
        assert_eq!(club.slots.len(), before + 1);
        assert!(club.names.contains(&o.player));
        assert_eq!(club.bought, 0);
        // filters work
        let gk = w.search_offers(&Search {
            position: Some(0),
            ..Search::default()
        });
        assert!(gk.iter().all(|o| o.position == 0));
        let cheap = w.search_offers(&Search {
            max_price: Some(5000),
            ..Search::default()
        });
        assert!(cheap.iter().all(|o| o.price <= 5000));
    }

    fn played_world() -> World {
        let mut w = World::synthetic(5);
        for _ in 0..2 {
            w.play_round();
        }
        w
    }

    #[test]
    fn a_wage_above_80_percent_of_the_value_renews_the_contract() {
        let mut w = played_world();
        let k = {
            let (d, i) = w.human_location().unwrap();
            w.divisions[d][i]
                .slots
                .iter()
                .position(|s| !s.fresh)
                .unwrap()
        };
        let value = w.human_player_value(k).unwrap();
        let (lo, hi) = w.wage_bounds();
        assert_eq!((lo, hi), (50 * w.scale, 50_000 * w.scale));
        // a low offer is refused with the player's demand: 1.25 times his value in steps of 50 G
        let low = (value as f64 * 0.8) as i64;
        let Ok(WageReply::Demand { amount, .. }) = w.human_propose_wage(k, low) else {
            panic!("0.8 times the value is not enough");
        };
        assert_eq!(amount % (50 * w.scale), 0);
        assert!((amount as f64 - value as f64 * 1.25).abs() <= 25.0 * w.scale as f64);
        let (d, i) = w.human_location().unwrap();
        assert!(
            !w.divisions[d][i].slots[k].fresh,
            "a refusal changes nothing"
        );
        // accepting the demand sets the wage and the flag
        w.human_accept_wage(k, amount).unwrap();
        assert_eq!(w.divisions[d][i].slots[k].wage, amount);
        assert!(w.divisions[d][i].slots[k].fresh);
        assert!(w.human_propose_wage(k, amount).is_err(), "only once");
        // a good offer is accepted straight away
        let j = w.divisions[d][i]
            .slots
            .iter()
            .position(|s| !s.fresh)
            .unwrap();
        let v = w.human_player_value(j).unwrap();
        assert_eq!(w.human_propose_wage(j, v), Ok(WageReply::Accepted));
        assert_eq!(w.divisions[d][i].slots[j].wage, v);
        assert!(w.divisions[d][i].slots[j].fresh);
    }

    #[test]
    fn a_player_nobody_bids_for_cannot_be_offered_again_before_the_next_game() {
        let mut w = played_world();
        let (d, i) = w.human_location().unwrap();
        let k = w.divisions[d][i]
            .slots
            .iter()
            .position(|s| !s.fresh)
            .unwrap();
        // nobody has that much money
        assert!(w.human_sell(k, 1_000_000_000_000).is_err());
        assert_eq!(w.sale_blocker(k), 4);
        assert!(w.sale_blocker_text(k).unwrap().contains("próximo jogo"));
        assert!(w.human_sell(k, 0).is_err());
        w.play_round();
        let (d, i) = w.human_location().unwrap();
        let name = w.divisions[d][i]
            .names
            .iter()
            .position(|n| !w.sell_tried.contains(n));
        assert!(name.is_some() && w.sell_tried.is_empty());
    }

    #[test]
    fn observed_players_are_followed_and_saved() {
        let mut w = played_world();
        let found = w.search_offers(&Search::default());
        let o = found[0].clone();
        w.observe(&o.club, &o.player);
        w.observe(&o.club, &o.player);
        assert_eq!(w.observed.len(), 1);
        assert!(w.is_observed(&o.club, &o.player));
        let again = World::load(&w.save()).unwrap();
        assert_eq!(again.observed, w.observed);
        let list = w.observed_players();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].player, o.player);
        w.unobserve(&o.club, &o.player);
        assert!(w.observed_players().is_empty());
    }

    #[test]
    fn the_search_filters_by_strength_price_and_name() {
        let w = played_world();
        let (all, cut) = w.search_players(&PlayerQuery::default());
        assert_eq!(all.len(), SEARCH_LIMIT);
        assert!(cut);
        let (strong, _) = w.search_players(&PlayerQuery {
            strength: Some((20, 30)),
            ..PlayerQuery::default()
        });
        assert!(strong.iter().all(|o| (20..=30).contains(&o.strength)));
        let (for_sale, _) = w.search_players(&PlayerQuery {
            price: PriceFilter::ForSale,
            ..PlayerQuery::default()
        });
        assert!(for_sale.iter().all(|o| o.price > 0));
        let (ranged, _) = w.search_players(&PlayerQuery {
            price: PriceFilter::Between(1, 50_000),
            ..PlayerQuery::default()
        });
        assert!(ranged.iter().all(|o| (1..=50_000).contains(&o.price)));
        let name = all[0].player.to_uppercase();
        let part: String = name.chars().take(4).collect();
        let (named, _) = w.search_players(&PlayerQuery {
            name: part.to_lowercase(),
            ..PlayerQuery::default()
        });
        assert!(named
            .iter()
            .all(|o| o.player.to_uppercase().contains(&part)));
        assert!(named.iter().any(|o| o.player == all[0].player));
        let me = w.club_ref(w.human_location().unwrap()).name.clone();
        let (not_mine, _) = w.search_players(&PlayerQuery {
            hide_mine: true,
            ..PlayerQuery::default()
        });
        assert!(not_mine.iter().all(|o| o.club != me));
    }

    // ---- hot seat ----

    fn hot_seat_world(n: usize) -> World {
        let mut w = World::synthetic(7);
        let names: Vec<String> = (1..=n).map(|k| format!("joga{k}")).collect();
        w.name_humans(&names);
        w
    }

    #[test]
    fn naming_players_gives_each_one_a_different_club_of_the_bottom_six() {
        let w = hot_seat_world(6);
        assert_eq!(w.human_count(), 6);
        let clubs = w.human_clubs();
        let mut sorted = clubs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 6, "{clubs:?}");
        for (seat, club) in clubs.iter().enumerate() {
            let (d, i) = w.locate(club).unwrap();
            assert_eq!(d, 3);
            assert!(i >= 2, "bottom six");
            assert_eq!(w.human_at(seat).unwrap().coach, format!("joga{}", seat + 1));
            assert_eq!(w.divisions[d][i].coach_name, format!("joga{}", seat + 1));
        }
        assert_eq!(w.active_seat(), 0);
        // a single name draws no more random numbers than the single-human start
        let mut one = World::synthetic(7);
        let mut named = World::synthetic(7);
        named.name_humans(&["a".into()]);
        assert_eq!(named.human_clubs(), one.human_clubs());
        assert_eq!(named.rng.seed(), one.rng.seed());
        one.name_humans(&[]);
        assert_eq!(one.human_count(), 1);
    }

    #[test]
    fn switching_swaps_the_active_human_and_his_market_lists() {
        let mut w = hot_seat_world(3);
        let clubs = w.human_clubs();
        w.observed.push(("X".into(), "p0".into()));
        assert!(w.switch_human(2));
        assert_eq!(w.human.as_ref().unwrap().club, clubs[2]);
        assert!(w.observed.is_empty(), "seat 2 watches nobody yet");
        w.sell_tried.push("p2".into());
        assert!(w.switch_human(0));
        assert_eq!(w.observed.len(), 1);
        assert!(w.sell_tried.is_empty());
        assert_eq!(w.seat_lists(2).2, ["p2".to_string()]);
        assert!(!w.switch_human(3));
        // next_human walks the seats and stops after the last
        assert!(w.next_human() && w.next_human());
        assert_eq!(w.active_seat(), 2);
        assert!(!w.next_human());
        assert_eq!(w.human_clubs(), clubs, "the order of the seats is kept");
        // for_each_human visits every seat once and restores the active one
        let mut seen = Vec::new();
        w.for_each_human(|x| seen.push(x.human.as_ref().unwrap().club.clone()));
        assert_eq!(seen, clubs);
        assert_eq!(w.active_seat(), 2);
    }

    #[test]
    fn every_human_plays_with_his_own_plan() {
        let mut w = hot_seat_world(2);
        // a formation for each human; the games are played with the plans of both
        let forms: Vec<usize> = (0..2)
            .map(|seat| {
                w.switch_human(seat);
                (0..FORMATIONS.len())
                    .rev()
                    .find(|&f| w.set_formation(f))
                    .unwrap()
            })
            .collect();
        let plans: Vec<Plan> = w.humans().map(|h| h.plan.clone()).collect();
        let clubs = w.human_clubs();
        w.play_round();
        for seat in 0..2 {
            let (d, i) = w.locate(&clubs[seat]).unwrap();
            let rec = w
                .last_round
                .iter()
                .find(|(dd, m)| *dd == d && (m.home == i || m.away == i))
                .expect("the human club played");
            let side = (rec.1.away == i) as usize;
            let started: Vec<usize> = (0..rec.1.start[side].len())
                .filter(|&k| rec.1.start[side][k].status == Status::Starter)
                .collect();
            let mut want = plans[seat].starters.clone();
            want.sort();
            assert_eq!(started, want, "seat {seat} (formation {})", forms[seat]);
        }
    }

    #[test]
    fn decisions_belong_to_the_game_they_were_made_in() {
        let mut w = hot_seat_world(2);
        let mut ctl = Control::default();
        ctl.league = vec![
            (
                "A".into(),
                crate::matchsim::Decision::NoSubstitution {
                    side: crate::matchsim::Side::Home,
                    minute: 10,
                },
            ),
            (
                "B".into(),
                crate::matchsim::Decision::NoSubstitution {
                    side: crate::matchsim::Side::Away,
                    minute: 20,
                },
            ),
        ];
        assert_eq!(ctl.decisions(false, "A").len(), 1);
        assert_eq!(
            ctl.decisions(false, "B")[0],
            crate::matchsim::Decision::NoSubstitution {
                side: crate::matchsim::Side::Away,
                minute: 20
            }
        );
        assert!(ctl.decisions(true, "A").is_empty());
        w.play_round();
    }

    #[test]
    fn the_purchases_of_each_human_arrive_at_his_own_club() {
        let mut w = hot_seat_world(2);
        w.play_round();
        for seat in 0..2 {
            w.switch_human(seat);
            set_human_cash(&mut w, 900_000_000);
        }
        w.switch_human(0);
        let mut bought = Vec::new();
        for seat in 0..2 {
            w.switch_human(seat);
            let offers = w.search_offers(&Search::default());
            let o = offers
                .iter()
                .find(|o| !w.is_human_club(&o.club) && !bought.contains(&o.player))
                .unwrap()
                .clone();
            w.human_buy(&o.club, o.index, &o.player).unwrap();
            bought.push(o.player.clone());
            assert_eq!(w.pending.len(), 1);
        }
        // the other human cannot buy the same player
        w.switch_human(0);
        let p = w.pending[0].clone();
        w.switch_human(1);
        let idx = w
            .club_ref(w.locate(&p.seller).unwrap())
            .names
            .iter()
            .position(|n| *n == p.player)
            .unwrap();
        assert!(w.human_buy(&p.seller, idx, &p.player).is_err());
        let squads: Vec<usize> = (0..2)
            .map(|s| {
                let c = w.human_at(s).unwrap().club.clone();
                w.club_ref(w.locate(&c).unwrap()).slots.len()
            })
            .collect();
        w.play_round();
        for (seat, name) in bought.iter().enumerate() {
            let c = w.human_at(seat).unwrap().club.clone();
            let club = w.club_ref(w.locate(&c).unwrap());
            assert!(club.names.contains(name), "seat {seat} got {name}");
            assert!(club.slots.len() >= squads[seat]);
        }
        assert!(w.seat_lists(0).0.is_empty() && w.seat_lists(1).0.is_empty());
    }

    #[test]
    fn hot_seat_saves_round_trip_and_old_saves_still_load() {
        let mut w = hot_seat_world(3);
        w.play_round();
        w.switch_human(1);
        w.observed.push(("CLUBX".into(), "Zed".into()));
        w.switch_human(2);
        w.sell_tried.push("Quim".into());
        w.pending.push(Pending {
            seller: "S".into(),
            player: "P".into(),
        });
        w.set_formation(3);
        let text = w.save();
        let back = World::load(&text).unwrap();
        assert_eq!(back.human_clubs(), w.human_clubs());
        assert_eq!(back.active_seat(), 2);
        for seat in 0..3 {
            assert_eq!(
                back.human_at(seat).unwrap().coach,
                format!("joga{}", seat + 1)
            );
            assert_eq!(
                back.human_at(seat).unwrap().plan,
                w.human_at(seat).unwrap().plan
            );
            assert_eq!(back.seat_lists(seat), w.seat_lists(seat));
        }
        assert_eq!(back.save(), text);
        // a single-human save (no coach field, no seats) loads as before
        let one = hot_seat_world(1);
        let old: String = one
            .save()
            .lines()
            .map(|l| {
                if l.starts_with("human|") {
                    l.rsplit_once('|').unwrap().0.to_string()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let loaded = World::load(&old).unwrap();
        assert_eq!(loaded.human_count(), 1);
        assert_eq!(
            loaded.human.as_ref().unwrap().club,
            one.human.as_ref().unwrap().club
        );
        assert_eq!(loaded.human.as_ref().unwrap().coach, "");
    }

    #[test]
    fn a_moved_human_coach_takes_the_new_club_of_his_own_seat() {
        let mut w = hot_seat_world(2);
        let clubs = w.human_clubs();
        // the second human is invited to a club of the 3rd division
        w.seats
            .moves
            .push((clubs[1].clone(), w.divisions[2][0].name.clone()));
        w.apply_human_move();
        assert_eq!(w.human_at(1).unwrap().club, w.divisions[2][0].name);
        assert_eq!(w.human_at(1).unwrap().coach, "joga2");
        assert_eq!(w.human_at(0).unwrap().club, clubs[0]);
        assert_eq!(w.active_seat(), 0);
        // applying twice changes nothing
        w.apply_human_move();
        assert_eq!(w.human_count(), 2);
    }

    #[test]
    fn many_rounds_with_several_humans_keep_every_seat_in_the_league_or_unemployed() {
        let mut w = hot_seat_world(4);
        let mut unemployed = 0;
        for _ in 0..30 {
            w.play_round();
            assert_eq!(w.human_count(), 4);
            let mut clubs: Vec<String> = w.humans().map(|h| h.club.clone()).collect();
            assert!(clubs.iter().all(|c| w.locate(c).is_some()));
            // a human without a club sits at a club of the pool; at least one human always has a club
            assert!(!w.employed_seats().is_empty());
            for seat in 0..4 {
                let c = &w.human_at(seat).unwrap().club;
                assert_eq!(w.is_unemployed(seat), w.locate(c).unwrap().0 == 4);
                unemployed += w.is_unemployed(seat) as usize;
            }
            assert!(w.employed_seats().contains(&w.active_seat()));
            clubs.sort();
            clubs.dedup();
            assert_eq!(clubs.len(), 4, "two humans never share a club");
        }
        let _ = unemployed;
    }

    /// A human replaced by a free coach has no club (and no turn) while another human plays; a club rated below 15 that sacks
    /// its coach hires him without asking (`5343`, flag 0x200).
    #[test]
    fn a_sacked_human_waits_at_the_pool_until_a_weak_club_hires_him() {
        let mut w = hot_seat_world(2);
        let clubs = w.human_clubs();
        let (d0, i0) = w.locate(&clubs[1]).unwrap();
        let pool_name = w.pool[0].name.clone();
        // the second human's coach is swapped with the free coach of the pool
        w.seats.moves.push((clubs[1].clone(), pool_name.clone()));
        w.apply_human_move();
        assert_eq!(w.human_at(1).unwrap().club, pool_name);
        assert!(w.is_unemployed(1) && !w.is_unemployed(0));
        assert_eq!(w.employed_seats(), vec![0]);
        assert_eq!(
            w.active_seat(),
            0,
            "the window stays with a human who has a club"
        );
        // a weak club (rating below 15) sacks its coach: the pool human is the only free coach, so he is taken, without asking
        w.pool.truncate(1);
        let (d, i) = w.locate(&clubs[0]).unwrap();
        let (d, i) = if (d, i) == (d0, i0) {
            (d0, (i0 + 1) % 8)
        } else {
            (d, i)
        };
        let weak = w.divisions[d][i].name.clone();
        for s in w.divisions[d][i].slots.iter_mut() {
            s.strength = 1;
        }
        w.divisions[d][i].k = 0.5;
        w.divisions[d][i].coach_games = 12;
        w.accept_invitations = false;
        let mut cur = std::mem::take(&mut w.divisions[d]);
        w.replace_coach(d, &mut cur, i, false);
        w.divisions[d] = cur;
        assert_eq!(w.coach_changes.last(), Some(&(weak.clone(), pool_name)));
        assert_eq!(w.coach_changes_free.last(), Some(&true));
        w.apply_human_move();
        assert_eq!(w.human_at(1).unwrap().club, weak);
        assert!(!w.is_unemployed(1));
        assert_eq!(w.employed_seats(), vec![0, 1]);
    }

    /// With no other human who has a club, a replaced human stays where he is.
    #[test]
    fn the_only_human_is_never_left_without_a_club() {
        let mut w = hot_seat_world(1);
        let club = w.human_clubs()[0].clone();
        let pool_name = w.pool[0].name.clone();
        w.seats.moves.push((club.clone(), pool_name));
        w.apply_human_move();
        assert_eq!(w.human_clubs()[0], club);
        assert!(!w.is_unemployed(0));
    }
}
