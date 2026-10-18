//! Reader for the original game's `.e98` saves (`JOGOS\*.e98`), and the import into a [`World`].
//!
//! The whole file is walked with the structure of the original loader (`1020:1d8d`; layout in `docs/e98-format.md`) and
//! must be consumed to the last byte, which was checked on real saves. Imported: rosters (status, goals, bans, injuries,
//! wages), standings, morale `K`, finances, the cup entrants and the division lists. Skipped (read but unused): the
//! coach/event tables, the played-round records and the news lists.
use crate::cipher::read_pstr_n;
use crate::economy::DEFAULT_SCALE;
use crate::finance::Finance;
use crate::history::{CoachOptions, Fixture, History, Winner};
use crate::market::{code, country_class};
use crate::season::Club;
use crate::sector::{Slot, Status};
use crate::world::base_strength;
use crate::world::{Transfer, World};
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
    /// Aggression (`+0x2d`), injury proneness (`+0x2f`) and talent `g` (`+0x2b`).
    pub aggression: u16,
    pub proneness: u16,
    pub g: u16,
    /// Career history (`+0x17..0x26`): games, goals, red cards, injuries.
    pub history: [u32; 4],
}

/// A history record of a club or a coach (7 bytes: `u8 kind`, `i32 year`, `u16` coach id for a club, team id for a coach).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E98Event {
    pub kind: u8,
    pub year: u32,
    pub other: u16,
}

/// A coach record (`1010:8612`).
#[derive(Clone, Debug)]
pub struct E98Coach {
    pub id: u16,
    pub name: String,
    pub human: bool,
    /// `+6`: a user's coach on holiday (the computer plays for him).
    pub holiday: bool,
    /// Titles won as coach (`+0x16` league, `+0x18` cup): the ranking sorts by them first.
    pub titles: (u16, u16),
    /// The five profile options (`+0x1a`): computer manages the wages of domestic, Bosman and other foreign players, auction
    /// offers mode (0 all, 1 not the weakest, 2 none), encrypt offers.
    pub options: [u8; 5],
    pub events: Vec<E98Event>,
}

/// One line of the champions list (`1010:ca77`, 10 bytes): year and the ids of champion, its coach, cup winner, its coach.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E98Winner {
    pub year: u16,
    pub champion: u16,
    pub champion_coach: u16,
    pub cup_winner: u16,
    pub cup_coach: u16,
}

/// One line of the last transfers list (`1010:cf1f`, 12 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E98Transfer {
    /// `+4`: involves a user's club (shown in blue).
    pub human: bool,
    pub player: u16,
    pub strength: u16,
    pub fee: i64,
    /// Team id shown in the last column (`+0xa`).
    pub club: u16,
}

/// One entry of a club's fixture list (`1010:2031`, the original's `TCalendarEntry`, 16 bytes in the save: `u16 +8, u16 +0xe,
/// u16 +0xa, u16 +0xc, u32 +0x10, u8 +7, u8 +6, u16 +4`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct E98Fixture {
    /// Team id of the opponent (`+8`).
    pub opponent: u16,
    /// Venue (`+0xe`; letters `C`, `F`, `N` of the calendar list): 0 home, 1 away, 2 neutral ground.
    pub venue: u16,
    /// The two scores (`+0xa`, `+0xc`): the club's own goals first; -1 while the game is unplayed.
    pub goals_a: i16,
    pub goals_b: i16,
    /// Attendance (`+0x10`).
    pub gate: i64,
    /// `+7`: a cup tie (the list shows cup rows in red, league rows in blue).
    pub cup: bool,
    /// `+6`: 0 normal time, 1 extra time, 2 won on penalties, 3 lost on penalties.
    pub result: u8,
    /// Round number (`+4`; below 1 shows blank).
    pub round: u16,
}

#[derive(Clone, Debug)]
pub struct E98Team {
    pub id: u16,
    pub full_name: String,
    pub short_name: String,
    pub country: String,
    pub level: u8,
    /// Background and foreground colours of the `.EFT` object (RGB).
    pub colors: ([u8; 3], [u8; 3]),
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
    pub coach_name: String,
    /// Team ids of the opponents in the fixture list (each twice); empty for clubs outside the league.
    pub opponents: Vec<u16>,
    /// Titles and places of the club (`TTeamEvent`).
    pub events: Vec<E98Event>,
    /// The whole fixture list with results.
    pub calendar: Vec<E98Fixture>,
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
    pub coaches: Vec<E98Coach>,
    pub winners: Vec<E98Winner>,
    pub transfers: Vec<E98Transfer>,
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

fn event(c: &mut Cur) -> Result<E98Event, String> {
    let e = c.raw(7)?;
    Ok(E98Event {
        kind: e[0],
        year: long(e, 1) as u32,
        other: u16::from_le_bytes([e[5], e[6]]),
    })
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
        aggression: u16::from_le_bytes([tail[17], tail[18]]),
        proneness: u16::from_le_bytes([tail[21], tail[22]]),
        g: u16::from_le_bytes([tail[23], tail[24]]),
        history: [0, 1, 2, 3].map(|k| long(tail, 32 + 4 * k) as u32),
    })
}

fn team(c: &mut Cur) -> Result<E98Team, String> {
    let id = c.u16()?;
    // the .EFT object: 50-byte anchor, full and short name, colours, country, level
    c.raw(50)?;
    let full_name = c.pstr()?;
    let short_name = c.pstr()?;
    let col = c.raw(8)?;
    let colors = ([col[0], col[1], col[2]], [col[4], col[5], col[6]]);
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
    let events = c.list(event)?;
    c.list(|c| c.raw(8).map(|_| ()))?;
    let calendar = c.list(|c| {
        let e = c.raw(16)?;
        let w = |k: usize| u16::from_le_bytes([e[k], e[k + 1]]);
        Ok(E98Fixture {
            opponent: w(0),
            venue: w(2),
            goals_a: w(4) as i16,
            goals_b: w(6) as i16,
            gate: long(e, 8),
            cup: e[12] != 0,
            result: e[13],
            round: w(14),
        })
    })?;
    let opponents = calendar.iter().map(|f| f.opponent).collect();
    c.raw(1 + 6)?; // stadium level and fill
    Ok(E98Team {
        id,
        full_name,
        short_name,
        country,
        level,
        colors,
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
        coach_name: String::new(),
        opponents,
        events,
        calendar,
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
    let coach_records = c.list(|c| {
        let id = c.u16()?;
        let name = c.pstr()?;
        let human = c.u8()? != 0;
        let holiday = c.u8()? != 0;
        let games = c.u16()?;
        let rest = c.raw(4 + 4 + 5)?;
        let score = long(rest, 0);
        let events = c.list(event)?;
        Ok((
            E98Coach {
                id,
                name,
                human,
                holiday,
                titles: (
                    u16::from_le_bytes([rest[4], rest[5]]),
                    u16::from_le_bytes([rest[6], rest[7]]),
                ),
                options: [rest[8], rest[9], rest[10], rest[11], rest[12]],
                events,
            },
            games,
            score,
        ))
    })?;
    let coaches: Vec<(u16, bool, u16, i64, String)> = coach_records
        .iter()
        .map(|(k, g, s)| (k.id, k.human, *g, *s, k.name.clone()))
        .collect();
    c.list(|c| {
        c.raw(3)?;
        c.pstr()?;
        c.pstr()?;
        Ok(())
    })?;
    let mut teams = c.list(team)?;
    for t in &mut teams {
        if let Some((_, human, games, score, name)) = coaches.iter().find(|k| k.0 == t.coach) {
            t.coach_human = *human;
            t.coach_games = *games;
            t.coach_score = *score;
            t.coach_name = name.clone();
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
    let winners = c.list(|c| {
        let e = c.raw(10)?;
        let w = |k: usize| u16::from_le_bytes([e[k], e[k + 1]]);
        Ok(E98Winner {
            year: w(0),
            champion: w(2),
            champion_coach: w(4),
            cup_winner: w(6),
            cup_coach: w(8),
        })
    })?;
    let transfers = c.list(|c| {
        // read order of `1010:cf1f`: +6 player, +0xa club, +0xc fee, +8 strength, +5, +4 flag
        let e = c.raw(12)?;
        let w = |k: usize| u16::from_le_bytes([e[k], e[k + 1]]);
        Ok(E98Transfer {
            player: w(0),
            club: w(2),
            fee: long(e, 4),
            strength: w(8),
            human: e[11] != 0,
        })
    })?;
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
        coaches: coach_records.into_iter().map(|(k, _, _)| k).collect(),
        winners,
        transfers,
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
            s.aggression = p.aggression.min(255) as u8;
            s.proneness = p.proneness.min(255) as u8;
            s.g = p.g.min(255) as u8;
            s.goals = p.goals;
            s.history = p.history;
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
    c.colors = t.colors;
    c.full_name = t.full_name.clone();
    c.coach_name = t.coach_name.clone();
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

/// The records of `history.rs` from a save: calendars, champions, club and coach histories, titles and the user's coach profile.
fn history_of(save: &E98, club: &HashMap<u16, String>) -> History {
    let team = |id: u16| club.get(&id).cloned().unwrap_or_default();
    let coach = |id: u16| {
        save.coaches
            .iter()
            .find(|k| k.id == id)
            .map(|k| k.name.clone())
            .unwrap_or_default()
    };
    let mut h = History::default();
    for w in &save.winners {
        h.winners.push(Winner {
            year: w.year,
            champion: team(w.champion),
            champion_coach: coach(w.champion_coach),
            cup_winner: team(w.cup_winner),
            cup_coach: coach(w.cup_coach),
        });
    }
    for t in &save.teams {
        let name = team(t.id);
        if !t.calendar.is_empty() {
            h.calendars.insert(
                name.clone(),
                t.calendar
                    .iter()
                    .map(|f| Fixture {
                        opponent: team(f.opponent),
                        venue: f.venue.min(2) as u8,
                        goals: (f.goals_a, f.goals_b),
                        gate: f.gate,
                        cup: f.cup,
                        result: f.result,
                        round: f.round,
                    })
                    .collect(),
            );
        }
        for e in &t.events {
            h.add_club_event(&name, e.kind, e.year, &coach(e.other));
        }
    }
    for k in &save.coaches {
        for e in &k.events {
            h.add_coach_event(&k.name, e.kind, e.year, &team(e.other));
        }
        if k.titles != (0, 0) {
            h.coach_titles.insert(k.name.clone(), k.titles);
        }
    }
    if let Some(k) = save.coaches.iter().find(|k| {
        k.human
            && save
                .teams
                .iter()
                .any(|t| t.id == save.human_id && t.coach == k.id)
    }) {
        h.options = CoachOptions {
            salary: [k.options[0] != 0, k.options[1] != 0, k.options[2] != 0],
            auction: k.options[3].min(2),
            encrypt: k.options[4] != 0,
        };
        h.holiday = k.holiday;
    }
    h
}

/// The last transfers of the save: the player is looked up by id among the squads, the club is the buyer.
fn transfers_of(save: &E98, club: &HashMap<u16, String>) -> Vec<Transfer> {
    save.transfers
        .iter()
        .map(|t| {
            let p = save
                .teams
                .iter()
                .flat_map(|tm| tm.players.iter())
                .find(|p| p.id == t.player);
            Transfer {
                round: 0,
                year: save.year,
                player: p.map(|p| p.name.clone()).unwrap_or_default(),
                position: p.map_or(0, |p| p.position),
                strength: t.strength,
                from: String::new(),
                to: club.get(&t.club).cloned().unwrap_or_default(),
                fee: t.fee,
            }
        })
        .collect()
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
        let mut club_names: HashMap<u16, String> = HashMap::new();
        for (d, idx) in divs.iter().enumerate() {
            for &i in idx {
                in_league.insert(i);
                let name = name_of(&save.teams[i]);
                club_names.insert(save.teams[i].id, name.clone());
                divisions[d].push(club_of(&save.teams[i], name, base_of(i)));
            }
        }
        let mut rest: Vec<usize> = (0..save.teams.len())
            .filter(|i| !in_league.contains(i))
            .collect();
        rest.sort_by_key(|&i| std::cmp::Reverse(save.teams[i].level));
        let pool: Vec<Club> = rest
            .iter()
            .map(|&i| {
                let name = name_of(&save.teams[i]);
                club_names.insert(save.teams[i].id, name.clone());
                club_of(&save.teams[i], name, base_of(i))
            })
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
        w.history = history_of(save, &club_names);
        w.transfers = transfers_of(save, &club_names);
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
            if name == "snap2.e98" {
                // the lists of the information dialogs: the three trades, the coaches' first events, the calendars of two rounds
                assert_eq!(save.transfers.len(), 3);
                assert_eq!(w.transfers.len(), 3);
                assert_eq!(w.transfers[0].fee, 5_171_000);
                assert!(save.winners.is_empty());
                assert_eq!(
                    w.history
                        .coach_events
                        .values()
                        .filter(|v| v[0].kind == 5)
                        .count(),
                    50 // 54 coaches, four names are shared by two coaches
                );
                let me = &w.human.as_ref().unwrap().club;
                let cal = w.calendar(me);
                assert_eq!(cal.len(), 14);
                assert_eq!(cal.iter().filter(|f| f.played()).count(), 2);
                assert_eq!(
                    (cal[0].goals, cal[0].gate, cal[0].venue),
                    ((1, 2), 10730, 1)
                );
                assert_eq!(w.history.options.auction, 1);
                assert!(
                    !w.history.options.salary[0],
                    "the original's new-game profile"
                );
            }
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
