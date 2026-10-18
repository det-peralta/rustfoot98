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
use crate::market::{code, country_class};
use crate::matchsim::{Event, SubEvent};
use crate::rng::DelphiRng;
use crate::season::{apply_plan, auto_lineup, fixtures, play_game, play_round_with};
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
}

#[derive(Clone, Debug)]
pub struct CupRound {
    pub name: String,
    pub matches: Vec<CupMatch>,
    /// Clubs still in the cup after this round.
    pub left: usize,
}

#[derive(Clone, Debug)]
pub struct SeasonSummary {
    pub year: u16,
    pub tables: [Vec<TableRow>; 4],
    pub moves: Vec<Move>,
    pub cup_winner: Option<String>,
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
    /// Currency scale `G` (`10c8:2d87`).
    pub scale: i64,
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
    /// Coach changes of the last round played, as `(sacked club, club that gave its coach)`; the second is the club the
    /// coach came from. The user's own moves are in `human_moved`.
    pub coach_changes: Vec<(String, String)>,
    /// Set when the user's coach was moved to another club by a swap (`(from, to)`).
    pub human_moved: Option<(String, String)>,
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
        slot.wage = player_value(strength, 1.0, INITIAL_INDEX, DEFAULT_SCALE);
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
    club.fin = Finance::new_game(base, DEFAULT_SCALE);
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
                pos,
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
            scale: DEFAULT_SCALE,
            cash_ref: [0; 4],
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            coach_changes: Vec::new(),
            human_moved: None,
        };
        w.reset_cup();
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
            cash_ref: [0; 4],
            round,
            rng: DelphiRng::new(seed),
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            coach_changes: Vec::new(),
            human_moved: None,
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
            cash_ref: [0; 4],
            round: 0,
            rng,
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup: Vec::new(),
            last_cup: None,
            cup_champion: None,
            transfers: Vec::new(),
            board: Vec::new(),
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            coach_changes: Vec::new(),
            human_moved: None,
        };
        w.reset_cup();
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
        self.human = Some(Human {
            club: name,
            formation,
            plan,
        });
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
        self.human
            .as_ref()
            .is_some_and(|h| h.club == self.club_ref(loc).name)
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
        let Some(h) = self.human_location() else {
            return false;
        };
        let c = self.club_ref(h);
        h != seller
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
        if self.is_human(seller) {
            self.remap_plan_after_removal(slot_idx);
        }
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
        if self.is_human(seller) || self.is_human(buyer_loc) {
            self.refresh_plan();
        }
    }

    /// Keeps the user's plan consistent after the player at `removed` left the squad.
    fn remap_plan_after_removal(&mut self, removed: usize) {
        let Some(h) = self.human.as_mut() else { return };
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
        if !crate::market::may_bid(me, &slot) {
            return Err("o plantel está cheio ou já tens estrangeiros a mais".into());
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
            if self.is_human(seller) {
                self.remap_plan_after_removal(idx);
            }
        }
        self.refresh_plan();
    }

    /// The user takes one loan step (`1010:665e`), within the bank's limit (`1040:3899`).
    pub fn human_borrow(&mut self) -> Result<i64, String> {
        let loc = self.human_location().ok_or("sem clube")?;
        let (e, g) = (self.economy, self.scale);
        let c = self.club_mut(loc);
        let step = finance::loan_step(g);
        let max = finance::max_loan(finance::rating(&c.slots), c.k, e, g);
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
        let step = finance::loan_step(self.scale);
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
        if !finance::wants_expansion(&c.fin) {
            return Err("o estádio ainda não está cheio o suficiente (80 %)".into());
        }
        let cost = finance::expansion_cost(&c.fin, e, g);
        if c.fin.cash < cost {
            return Err(format!("as obras custam {cost}"));
        }
        finance::expand(&mut c.fin);
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
        if !crate::market::may_bid(hc, &self.club_ref(seller).slots[idx]) {
            return Err("o plantel está cheio ou já tens estrangeiros a mais".into());
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
    fn renegotiate_wages(&mut self, loc: (usize, usize)) {
        let (e, g) = (self.economy, self.scale);
        let club = self.club_mut(loc);
        let f = club.fin.f;
        let (mut cut, mut raise) = (None, None);
        let (mut cut_best, mut raise_best) = (1.2f64, 0.8f64);
        for (i, s) in club.slots.iter().enumerate() {
            if s.fresh {
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
            // the user's wages are renegotiated by the computer (the coach options of the original ask whether to do so)
            if round >= 2 && self.club_ref(loc).coach_games > 1 {
                self.renegotiate_wages(loc);
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
            self.renegotiate_wages(loc);
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
        let human = self.human.as_ref().map(|h| h.club.clone());
        for d in 0..4 {
            let ai: i64 = self.divisions[d]
                .iter()
                .filter(|c| Some(&c.name) != human.as_ref())
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
        let human_name = self.human.as_ref().map(|h| h.club.clone());
        let is_human = |c: &Club| human_name.as_deref() == Some(c.name.as_str());
        let exclude_humans = no_humans;
        // coach list order: computer coaches by club order, the user's coach last
        let mut cands: Vec<((usize, usize), Club)> = Vec::new();
        for l in locs {
            if l == me {
                continue;
            }
            let c = get(self, cur, l);
            let diff = rating_t - crate::cup::team_rating(&c.slots) as i32;
            if c.k >= 1.25 && (11..20).contains(&diff) && !(exclude_humans && is_human(&c)) {
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
        let Some(&pick) = v
            .iter()
            .find(|&&k| !is_human(&cands[k].1) || self.accept_invitations)
        else {
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
        {
            let pc = if ploc.0 == d {
                &mut cur[ploc.1]
            } else {
                self.club_mut(ploc)
            };
            pc.k = 1.0;
            pc.coach_score = t_score;
            pc.coach_games = t_games;
        }
        self.coach_changes.push((t.name.clone(), p.name.clone()));
        if t_human {
            self.human_moved = Some((t.name.clone(), p.name.clone()));
        } else if p_human {
            self.human_moved = Some((p.name.clone(), t.name.clone()));
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
        let Some((_, to)) = self.human_moved.clone() else {
            return;
        };
        if self.human.as_ref().is_some_and(|h| h.club != to) {
            if let Some((d, i)) = self.locate(&to).filter(|l| l.0 < 4) {
                self.set_human(d, i);
            }
        }
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

    /// The cup of `round` (even), played after that round's league games; see `cup.rs` for the rules.
    fn play_cup_round(&mut self, round: usize) {
        let total = self.divisions.iter().map(Vec::len).sum::<usize>() + self.pool.len();
        if total <= cup::threshold(round) || self.cup.len() < 2 {
            return;
        }
        cup::shuffle(&mut self.rng, &mut self.cup);
        let n = self.cup.len();
        let (k, pairs) = cup::round_size(n);
        let entrants: Vec<String> = self.cup.iter().take(pairs * 2).cloned().collect();
        let plan = self
            .human
            .as_ref()
            .map(|h| (h.club.clone(), h.plan.clone()));
        let mut matches = Vec::new();
        let mut losers = Vec::new();
        for pair in entrants.chunks(2) {
            let (hn, an) = (pair[0].clone(), pair[1].clone());
            let (Some(hl), Some(al)) = (self.locate(&hn), self.locate(&an)) else {
                continue;
            };
            let money = self.money_ctx(hl.0.min(3));
            let placeholder = || Club::new("", Vec::new());
            let mut home = std::mem::replace(self.club_at(hl), placeholder());
            let mut away = std::mem::replace(self.club_at(al), placeholder());
            let rng = &mut self.rng;
            let mut plan_of = |c: &mut Club| -> Option<Plan> {
                match &plan {
                    Some((club, p)) if *club == c.name => {
                        apply_plan(&mut c.slots, p);
                        Some(p.clone())
                    }
                    _ => {
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
            let g = play_game(
                &mut self.rng,
                &mut home,
                &mut away,
                hb,
                ab,
                hp.as_ref(),
                ap.as_ref(),
                true,
            );
            let shootout = g.home_goals == g.away_goals;
            let human_in_tie = self
                .human
                .as_ref()
                .is_some_and(|h| h.club == home.name || h.club == away.name);
            let mut pens = None;
            let home_out = if shootout {
                if human_in_tie || round == ROUNDS {
                    // the dialog with real kicks (always in the final, or when a human plays)
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
            name: cup::round_name(k),
            matches,
            left: self.cup.len(),
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
        self.close_board();
        self.sell_tried.clear();
        self.last_round.clear();
        self.coach_changes.clear();
        self.human_moved = None;
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
            if let (Some((hd, hi)), Some(h)) = (self.human_location(), self.human.as_ref()) {
                if hd == d {
                    plans.insert(hi, h.plan.clone());
                }
            }
            let money = self.money_ctx(d);
            // the division is taken out so that a sacking can reach the other divisions (the coach swap of `4f40`)
            let mut cur = std::mem::take(&mut self.divisions[d]);
            let mut rng = std::mem::replace(&mut self.rng, DelphiRng::new(0));
            let records = play_round_with(
                &mut rng,
                &mut cur,
                round,
                &mut names,
                &plans,
                Some(money),
                &mut |cur, i| self.replace_coach(d, cur, i, round == ROUNDS),
            );
            self.rng = rng;
            self.divisions[d] = cur;
            self.apply_human_move();
            self.last_round.extend(records.into_iter().map(|r| (d, r)));
        }
        // order of `1020:0c41`: games, economic factors (`6e48`), clubs sell and renegotiate (`3e9c`), then the evolution
        self.update_factors();
        self.market_round(round);
        self.deliver_purchases();
        for (d, base) in DIVISION_BASE.iter().enumerate() {
            end_of_round(&mut self.rng, &mut self.divisions[d], *base);
        }
        self.economy = walk_index(&mut self.rng, self.economy, self.year);
        self.round = round;
        self.last_cup = None;
        if round.is_multiple_of(2) {
            self.play_cup_round(round);
            self.apply_human_move();
        }
        if let Some(h) = self.human.as_mut() {
            h.plan.subs.clear(); // planned substitutions are for one round only (league game and cup tie)
        }
        self.refresh_plan();
        if round == ROUNDS {
            self.finish_season();
        }
    }

    /// `1020:2ae6`: the champion of the first division, the cup winner and the club of every top scorer get their prizes
    /// (category 3); the scorers' tallies then start again.
    fn pay_prizes(&mut self, cup_winner: Option<&str>) {
        let [p1, p2, p3] = finance::season_prizes(self.scale);
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
        self.pay_prizes(cup_winner.as_deref());
        end_of_season(
            &mut self.rng,
            &mut self.divisions,
            &mut self.pool,
            cup_winner.as_deref(),
        );
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
            cup_winner,
        });
        self.year += 1;
        self.round = 0;
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
            .is_some_and(|idx| self.human_may_bid(seller, idx, l.ask))
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
        let step = finance::loan_step(w.scale);
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
        let [p1, p2, p3] = finance::season_prizes(w.scale);
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
}
