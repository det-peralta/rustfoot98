//! Save and load a [`World`] as a small versioned text format (one record per line, fields separated by `|`).
//!
//! This is *our* format, not the original `.e98` (whose layout is only partly decoded, see `docs/findings.md`). It stores
//! everything needed to continue a game identically: the clubs with their squads and standings, the outside pool, the year,
//! round and economy index, the user's plan, and the RNG seed. The results of the last round and the last season summary
//! are not stored.
use crate::economy::{DEFAULT_SCALE, INITIAL_INDEX};
use crate::finance::Finance;
use crate::history::{ClubEvent, CoachEvent, Fixture, History, Winner};
use crate::market::{code, country_class};
use crate::rng::DelphiRng;
use crate::season::{Club, Plan, PlannedSub};
use crate::sector::{Slot, Status};
use crate::world::{Human, Listing, Pending, Transfer, World};

const MAGIC: &str = "ELIFOOT98-RS 1";

fn seat_suffix(seat: usize) -> String {
    if seat == 0 {
        String::new()
    } else {
        format!("|{seat}")
    }
}

fn clean(s: &str) -> String {
    s.replace(['|', '\n', '\r'], " ")
}

fn list(v: &[usize]) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn status_code(s: Status) -> u8 {
    s as u8
}

impl World {
    /// Serialises the world.
    pub fn save(&self) -> String {
        let mut out = String::new();
        out.push_str(MAGIC);
        out.push('\n');
        out.push_str(&format!(
            "year|{}\neconomy|{:?}\nround|{}\nseed|{}\nscale|{}\ncashref|{}|{}|{}|{}\n",
            self.year,
            self.economy,
            self.round,
            self.rng.seed(),
            self.scale,
            self.cash_ref[0],
            self.cash_ref[1],
            self.cash_ref[2],
            self.cash_ref[3]
        ));
        // the humans in seat order, then the seat of the active one (the single-human game writes one `human` line)
        for h in self.humans() {
            let subs: Vec<String> = h
                .plan
                .subs
                .iter()
                .map(|s| format!("{}:{}:{}", s.minute, s.out, s.inn))
                .collect();
            out.push_str(&format!(
                "human|{}|{}|{}|{}|{}|{}\n",
                clean(&h.club),
                h.formation,
                list(&h.plan.starters),
                list(&h.plan.bench),
                subs.join(","),
                clean(&h.coach)
            ));
        }
        if self.human_count() > 1 {
            out.push_str(&format!("activeseat|{}\n", self.seats.active));
        }
        for name in &self.cup {
            out.push_str(&format!("cup|{}\n", clean(name)));
        }
        // (the lists of the humans after the first carry their seat as a last field)
        for seat in 0..self.human_count().max(1) {
            let (pending, _, _) = self.seat_lists(seat);
            for p in pending {
                out.push_str(&format!(
                    "pending|{}|{}{}\n",
                    clean(&p.seller),
                    clean(&p.player),
                    seat_suffix(seat)
                ));
            }
        }
        for l in &self.board {
            let (b, fee) = l
                .best
                .as_ref()
                .map(|(b, f)| (clean(b), *f))
                .unwrap_or_default();
            out.push_str(&format!(
                "listing|{}|{}|{}|{}|{}|{}\n",
                clean(&l.seller),
                clean(&l.player),
                l.ask,
                l.round,
                b,
                fee
            ));
        }
        for seat in 0..self.human_count().max(1) {
            let (_, observed, sell_tried) = self.seat_lists(seat);
            for (club, player) in observed {
                out.push_str(&format!(
                    "observed|{}|{}{}\n",
                    clean(club),
                    clean(player),
                    seat_suffix(seat)
                ));
            }
            for player in sell_tried {
                out.push_str(&format!(
                    "selltried|{}{}\n",
                    clean(player),
                    seat_suffix(seat)
                ));
            }
        }
        if let Some(c) = &self.cup_champion {
            out.push_str(&format!("cupchampion|{}\n", clean(c)));
        }
        // a cup round drawn but not yet played (its own match day)
        if let Some(d) = &self.cup_draw {
            out.push_str(&format!("cupdraw|{}|{}\n", d.round, clean(&d.name)));
            for (h, a) in &d.pairs {
                out.push_str(&format!("cuptie|{}|{}\n", clean(h), clean(a)));
            }
        }
        for (d, order) in self.tie_order.iter().enumerate() {
            let list: Vec<String> = order.iter().map(|i| i.to_string()).collect();
            out.push_str(&format!("tieorder|{d}|{}\n", list.join(",")));
        }
        self.save_history(&mut out);
        let mut write_club = |division: usize, c: &Club| {
            out.push_str(&format!(
                "club|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}\n",
                division,
                clean(&c.name),
                c.k,
                c.points,
                c.wins,
                c.draws,
                c.losses,
                c.goals_for,
                c.goals_against,
                c.coach_games,
                c.fin.cash,
                c.fin.loan,
                c.fin.f,
                c.fin.stadium,
                c.fin.gate,
                c.fin.sales,
                c.fin.purchases,
                c.fin.wages,
                c.fin.interest,
                String::from_utf8_lossy(&c.country),
                c.master,
                c.bought,
                c.bought_foreign,
                c.fin.fill,
                c.fin.works,
                c.fin.prizes,
                c.coach_score,
                c.colors
                    .0
                    .iter()
                    .chain(c.colors.1.iter())
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                clean(&c.full_name),
                clean(&c.coach_name)
            ));
            for (slot, name) in c.slots.iter().zip(&c.names) {
                out.push_str(&format!(
                    "p|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                    clean(name),
                    slot.position,
                    slot.strength,
                    slot.same_country as u8,
                    slot.ban,
                    slot.injured,
                    slot.rating,
                    status_code(slot.status),
                    0,
                    String::from_utf8_lossy(&slot.nat),
                    slot.wage,
                    slot.fresh as u8,
                    slot.goals,
                    slot.history.map(|h| h.to_string()).join(" ")
                ));
            }
        };
        for (d, division) in self.divisions.iter().enumerate() {
            for c in division {
                write_club(d, c);
            }
        }
        for c in &self.pool {
            write_club(4, c);
        }
        out
    }

    /// Parses a saved world. Errors describe the offending line.
    pub fn load(text: &str) -> Result<World, String> {
        let mut lines = text.lines().enumerate();
        match lines.next() {
            Some((_, l)) if l.trim() == MAGIC => {}
            _ => return Err("not an Elifoot 98 (Rust) save file".into()),
        }
        let (mut year, mut economy, mut round, mut seed) = (None, INITIAL_INDEX, 0usize, None);
        let mut humans: Vec<Human> = Vec::new();
        let mut active_seat = 0usize;
        type Lists = (Vec<Pending>, Vec<(String, String)>, Vec<String>);
        let mut lists: Vec<Lists> = vec![Default::default()];
        let seat_of =
            |f: &[&str], at: usize| f.get(at).and_then(|x| x.parse::<usize>().ok()).unwrap_or(0);
        let mut cup: Vec<String> = Vec::new();
        let mut cup_draw: Option<crate::world::CupDraw> = None;
        let mut cup_champion: Option<String> = None;
        let mut has_cup = false;
        let mut has_finance = false;
        let mut board: Vec<Listing> = Vec::new();
        let mut history = History::default();
        let mut tie_order: Vec<Vec<usize>> = Vec::new();
        let mut transfers: Vec<Transfer> = Vec::new();
        let mut has_market = false;
        let mut scale = DEFAULT_SCALE;
        let mut cash_ref = [0i64; 4];
        let mut divisions: [Vec<Club>; 4] = Default::default();
        let mut pool: Vec<Club> = Vec::new();
        let mut current: Option<(usize, Club)> = None;
        let flush = |cur: &mut Option<(usize, Club)>,
                     divisions: &mut [Vec<Club>; 4],
                     pool: &mut Vec<Club>| {
            if let Some((d, c)) = cur.take() {
                if d < 4 {
                    divisions[d].push(c);
                } else {
                    pool.push(c);
                }
            }
        };
        let num = |n: usize, f: &[&str], i: usize| -> Result<i64, String> {
            f.get(i)
                .and_then(|x| x.parse::<i64>().ok())
                .ok_or_else(|| format!("line {}: bad number in field {}", n + 1, i))
        };
        let nums = |n: usize, s: &str| -> Result<Vec<usize>, String> {
            s.split(',')
                .filter(|x| !x.is_empty())
                .map(|x| {
                    x.parse::<usize>()
                        .map_err(|_| format!("line {}: bad list", n + 1))
                })
                .collect()
        };
        for (n, line) in lines {
            if line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('|').collect();
            match f[0] {
                "year" => year = Some(num(n, &f, 1)? as u16),
                "economy" => {
                    economy = f
                        .get(1)
                        .and_then(|x| x.parse().ok())
                        .ok_or(format!("line {}: bad economy", n + 1))?
                }
                "round" => round = num(n, &f, 1)? as usize,
                "pending" | "observed" | "selltried" => {
                    // the seat of the human they belong to is the last field (absent: the first human)
                    let at = if f[0] == "selltried" { 2 } else { 3 };
                    let seat = seat_of(&f, at);
                    if lists.len() <= seat {
                        lists.resize(seat + 1, Default::default());
                    }
                    let get = |i: usize, what: &str| {
                        f.get(i)
                            .map(|x| x.to_string())
                            .ok_or(format!("{} without field {i} ({what})", f[0]))
                    };
                    match f[0] {
                        "pending" => lists[seat].0.push(Pending {
                            seller: get(1, "seller")?,
                            player: get(2, "player")?,
                        }),
                        "observed" => lists[seat].1.push((get(1, "club")?, get(2, "player")?)),
                        _ => lists[seat].2.push(get(1, "player")?),
                    }
                }
                "activeseat" => active_seat = num(n, &f, 1)? as usize,
                "listing" => {
                    let best = match (f.get(5), f.get(6).and_then(|x| x.parse::<i64>().ok())) {
                        (Some(b), Some(fee)) if !b.is_empty() => Some((b.to_string(), fee)),
                        _ => None,
                    };
                    board.push(Listing {
                        seller: f.get(1).ok_or("listing without seller")?.to_string(),
                        player: f.get(2).ok_or("listing without player")?.to_string(),
                        ask: num(n, &f, 3)?,
                        round: num(n, &f, 4)? as usize,
                        best,
                    });
                }
                "tieorder" => {
                    let d = num(n, &f, 1)? as usize;
                    if d < 4 {
                        tie_order.resize(4, Vec::new());
                        tie_order[d] = f
                            .get(2)
                            .unwrap_or(&"")
                            .split(',')
                            .filter_map(|v| v.parse().ok())
                            .collect();
                    }
                }
                "winner" => history.winners.push(Winner {
                    year: num(n, &f, 1)? as u16,
                    champion: f.get(2).unwrap_or(&"").to_string(),
                    champion_coach: f.get(3).unwrap_or(&"").to_string(),
                    cup_winner: f.get(4).unwrap_or(&"").to_string(),
                    cup_coach: f.get(5).unwrap_or(&"").to_string(),
                }),
                "fixture" => history
                    .calendars
                    .entry(f.get(1).unwrap_or(&"").to_string())
                    .or_default()
                    .push(Fixture {
                        opponent: f.get(2).unwrap_or(&"").to_string(),
                        venue: num(n, &f, 3)? as u8,
                        goals: (num(n, &f, 4)? as i16, num(n, &f, 5)? as i16),
                        gate: num(n, &f, 6)?,
                        cup: num(n, &f, 7)? != 0,
                        result: num(n, &f, 8)? as u8,
                        round: num(n, &f, 9)? as u16,
                    }),
                "clubevent" => history
                    .club_events
                    .entry(f.get(1).unwrap_or(&"").to_string())
                    .or_default()
                    .push(ClubEvent {
                        kind: num(n, &f, 2)? as u8,
                        year: num(n, &f, 3)? as u32,
                        coach: f.get(4).unwrap_or(&"").to_string(),
                    }),
                "coachevent" => history
                    .coach_events
                    .entry(f.get(1).unwrap_or(&"").to_string())
                    .or_default()
                    .push(CoachEvent {
                        kind: num(n, &f, 2)? as u8,
                        year: num(n, &f, 3)? as u32,
                        club: f.get(4).unwrap_or(&"").to_string(),
                    }),
                "coachtitles" => {
                    history.coach_titles.insert(
                        f.get(1).unwrap_or(&"").to_string(),
                        (num(n, &f, 2)? as u16, num(n, &f, 3)? as u16),
                    );
                }
                "coachopts" => {
                    history.options.salary = [
                        num(n, &f, 1)? != 0,
                        num(n, &f, 2)? != 0,
                        num(n, &f, 3)? != 0,
                    ];
                    history.options.auction = num(n, &f, 4)?.clamp(0, 2) as u8;
                    history.options.encrypt = num(n, &f, 5)? != 0;
                    history.holiday = num(n, &f, 6)? != 0;
                }
                "transfer" => transfers.push(Transfer {
                    round: num(n, &f, 1)? as usize,
                    year: num(n, &f, 2)? as u16,
                    player: f.get(3).unwrap_or(&"").to_string(),
                    position: num(n, &f, 4)? as u8,
                    strength: num(n, &f, 5)? as u16,
                    from: f.get(6).unwrap_or(&"").to_string(),
                    to: f.get(7).unwrap_or(&"").to_string(),
                    fee: num(n, &f, 8)?,
                }),
                "scale" => scale = num(n, &f, 1)?.max(1),
                "cashref" => {
                    for (i, r) in cash_ref.iter_mut().enumerate() {
                        *r = num(n, &f, 1 + i)?;
                    }
                }
                "seed" => seed = Some(num(n, &f, 1)? as u32),
                "cup" => {
                    has_cup = true;
                    cup.push(f.get(1).ok_or("cup entry without name")?.to_string());
                }
                "cupdraw" => {
                    cup_draw = Some(crate::world::CupDraw {
                        round: num(n, &f, 1)? as usize,
                        name: f.get(2).unwrap_or(&"").to_string(),
                        pairs: Vec::new(),
                    })
                }
                "cuptie" => {
                    if let (Some(d), Some(h), Some(a)) = (cup_draw.as_mut(), f.get(1), f.get(2)) {
                        d.pairs.push((h.to_string(), a.to_string()));
                    }
                }
                "cupchampion" => {
                    cup_champion = Some(f.get(1).ok_or("cupchampion without name")?.to_string())
                }
                "human" => {
                    let subs = f
                        .get(5)
                        .unwrap_or(&"")
                        .split(',')
                        .filter(|x| !x.is_empty())
                        .map(|x| {
                            let p: Vec<usize> =
                                x.split(':').filter_map(|v| v.parse().ok()).collect();
                            (p.len() == 3)
                                .then(|| PlannedSub {
                                    minute: p[0] as u16,
                                    out: p[1],
                                    inn: p[2],
                                })
                                .ok_or(format!("line {}: bad substitution", n + 1))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    humans.push(Human {
                        club: f.get(1).ok_or("human without club")?.to_string(),
                        coach: f.get(6).unwrap_or(&"").to_string(),
                        formation: num(n, &f, 2)? as usize,
                        plan: Plan {
                            starters: nums(n, f.get(3).unwrap_or(&""))?,
                            bench: nums(n, f.get(4).unwrap_or(&""))?,
                            subs,
                        },
                    });
                }
                "club" => {
                    flush(&mut current, &mut divisions, &mut pool);
                    let mut c = Club::new(f.get(2).ok_or("club without name")?, Vec::new());
                    c.k = f
                        .get(3)
                        .and_then(|x| x.parse().ok())
                        .ok_or(format!("line {}: bad morale", n + 1))?;
                    c.points = num(n, &f, 4)? as u16;
                    c.wins = num(n, &f, 5)? as u16;
                    c.draws = num(n, &f, 6)? as u16;
                    c.losses = num(n, &f, 7)? as u16;
                    c.goals_for = num(n, &f, 8)? as i32;
                    c.goals_against = num(n, &f, 9)? as i32;
                    // older saves have no coach counter
                    c.coach_games = f.get(10).and_then(|x| x.parse().ok()).unwrap_or(0);
                    // older saves carry no finances: they keep the defaults and are given a new-game state below
                    if f.len() >= 20 {
                        let int = |i: usize| f[i].parse::<i64>().unwrap_or(0);
                        c.fin = Finance {
                            cash: int(11),
                            loan: int(12),
                            f: f[13].parse().unwrap_or(1.0),
                            stadium: int(14).clamp(1, 24) as u8,
                            fill: f.get(24).and_then(|x| x.parse().ok()).unwrap_or(0.6),
                            gate: int(15),
                            sales: int(16),
                            purchases: int(17),
                            wages: int(18),
                            interest: int(19),
                            works: f.get(25).and_then(|x| x.parse().ok()).unwrap_or(0),
                            prizes: f.get(26).and_then(|x| x.parse().ok()).unwrap_or(0),
                        };
                        has_finance = true;
                    }
                    if let Some(cc) = f.get(20) {
                        c.country = code(cc);
                        c.master = f.get(21).and_then(|x| x.parse().ok()).unwrap_or(0);
                        c.bought = f.get(22).and_then(|x| x.parse().ok()).unwrap_or(0);
                        c.bought_foreign = f.get(23).and_then(|x| x.parse().ok()).unwrap_or(0);
                        has_market = true;
                    }
                    c.coach_score = f.get(27).and_then(|x| x.parse().ok()).unwrap_or(0);
                    if let Some(v) = f.get(29) {
                        c.full_name = v.to_string();
                    }
                    if let Some(v) = f.get(30) {
                        c.coach_name = v.to_string();
                    }
                    if let Some(v) = f.get(28) {
                        let n: Vec<u8> = v.split(',').filter_map(|t| t.parse().ok()).collect();
                        if n.len() == 6 {
                            c.colors = ([n[0], n[1], n[2]], [n[3], n[4], n[5]]);
                        }
                    }
                    c.names.clear();
                    current = Some((num(n, &f, 1)? as usize, c));
                }
                "p" => {
                    let (_, c) = current
                        .as_mut()
                        .ok_or(format!("line {}: player before any club", n + 1))?;
                    let status = match num(n, &f, 8)? {
                        0 => Status::Out,
                        1 => Status::Starter,
                        2 => Status::Sub,
                        _ => return Err(format!("line {}: bad status", n + 1)),
                    };
                    let name = f.get(1).ok_or("player without name")?;
                    let mut slot = Slot::new(
                        name,
                        num(n, &f, 2)? as u8,
                        num(n, &f, 3)? as u16,
                        num(n, &f, 4)? == 1,
                        status,
                    );
                    slot.ban = num(n, &f, 5)? as u8;
                    slot.injured = num(n, &f, 6)? as u8;
                    slot.rating = num(n, &f, 7)? as i32;
                    if let Some(nat) = f.get(10) {
                        slot.nat = code(nat);
                        slot.wage = f.get(11).and_then(|x| x.parse().ok()).unwrap_or(0);
                        slot.fresh = f.get(12).is_some_and(|x| *x == "1");
                        slot.goals = f.get(13).and_then(|x| x.parse().ok()).unwrap_or(0);
                        if let Some(h) = f.get(14) {
                            for (k, v) in h.split(' ').take(4).enumerate() {
                                slot.history[k] = v.parse().unwrap_or(0);
                            }
                        }
                        slot.foreign = country_class(c.country, slot.nat);
                    }
                    c.slots.push(slot);
                    c.names.push(name.to_string());
                }
                other => return Err(format!("line {}: unknown record '{other}'", n + 1)),
            }
        }
        flush(&mut current, &mut divisions, &mut pool);
        if divisions.iter().any(|d| d.len() != 8) {
            return Err("every division must have 8 clubs".into());
        }
        if pool.len() < 2 {
            return Err("the outside pool needs at least 2 clubs".into());
        }
        let mut w = World {
            divisions,
            pool,
            year: year.ok_or("missing year")?,
            economy,
            scale,
            cash_ref,
            round,
            rng: DelphiRng::new(seed.ok_or("missing seed")?),
            last_round: Vec::new(),
            last_season: None,
            human: None,
            cup,
            last_cup: None,
            cup_draw,
            cup_champion,
            transfers,
            board,
            pending: Vec::new(),
            observed: Vec::new(),
            sell_tried: Vec::new(),
            accept_invitations: false,
            real_shootouts: false,
            coach_changes: Vec::new(),
            coach_changes_free: Vec::new(),
            seats: Default::default(),
            history,
            tie_order,
        };
        if !has_cup {
            w.reset_cup();
        }
        if !has_finance {
            w.give_new_game_finances();
        }
        if !has_market {
            w.give_market_defaults();
        }
        // the humans take their seats; the active one is installed last
        let count = humans.len();
        for (seat, h) in humans.into_iter().enumerate() {
            let (pending, observed, sell_tried) = lists.get(seat).cloned().unwrap_or_default();
            w.seats.benched.push(crate::world::Bench {
                seat,
                human: h,
                pending,
                observed,
                sell_tried,
            });
        }
        if count > 0 {
            let active = if active_seat < count { active_seat } else { 0 };
            w.seats.active = usize::MAX; // nobody is active yet: `switch_human` swaps nobody out
            w.switch_human(active);
        }
        if w.human.is_none() {
            w.pick_human();
        }
        for seat in 0..w.human_count() {
            w.switch_human(seat);
            if w.human_location().is_none() {
                w.pick_human();
            }
        }
        w.switch_human(if active_seat < w.human_count() {
            active_seat
        } else {
            0
        });
        Ok(w)
    }
}

impl World {
    /// The records of `history.rs` and the transfers list.
    fn save_history(&self, out: &mut String) {
        let h = &self.history;
        for w in &h.winners {
            out.push_str(&format!(
                "winner|{}|{}|{}|{}|{}\n",
                w.year,
                clean(&w.champion),
                clean(&w.champion_coach),
                clean(&w.cup_winner),
                clean(&w.cup_coach)
            ));
        }
        for (club, list) in &h.calendars {
            for f in list {
                out.push_str(&format!(
                    "fixture|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                    clean(club),
                    clean(&f.opponent),
                    f.venue,
                    f.goals.0,
                    f.goals.1,
                    f.gate,
                    f.cup as u8,
                    f.result,
                    f.round
                ));
            }
        }
        for (club, list) in &h.club_events {
            for e in list {
                out.push_str(&format!(
                    "clubevent|{}|{}|{}|{}\n",
                    clean(club),
                    e.kind,
                    e.year,
                    clean(&e.coach)
                ));
            }
        }
        for (coach, list) in &h.coach_events {
            for e in list {
                out.push_str(&format!(
                    "coachevent|{}|{}|{}|{}\n",
                    clean(coach),
                    e.kind,
                    e.year,
                    clean(&e.club)
                ));
            }
        }
        for (coach, t) in &h.coach_titles {
            out.push_str(&format!("coachtitles|{}|{}|{}\n", clean(coach), t.0, t.1));
        }
        let o = &h.options;
        out.push_str(&format!(
            "coachopts|{}|{}|{}|{}|{}|{}\n",
            o.salary[0] as u8,
            o.salary[1] as u8,
            o.salary[2] as u8,
            o.auction,
            o.encrypt as u8,
            h.holiday as u8
        ));
        for t in &self.transfers {
            out.push_str(&format!(
                "transfer|{}|{}|{}|{}|{}|{}|{}|{}\n",
                t.round,
                t.year,
                clean(&t.player),
                t.position,
                t.strength,
                clean(&t.from),
                clean(&t.to),
                t.fee
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_game_continues_identically() {
        let mut a = World::synthetic(31);
        a.play_season();
        a.play_round();
        a.play_round();
        assert!(a.set_formation(3));
        let text = a.save();
        let mut b = World::load(&text).unwrap();
        // the saved text is stable ...
        assert_eq!(b.save(), text);
        // ... and the two worlds evolve identically from here on
        for _ in 0..30 {
            a.play_round();
            b.play_round();
            assert_eq!(a.save(), b.save());
        }
    }

    #[test]
    fn saves_survive_odd_names_and_the_plan() {
        let mut w = World::synthetic(32);
        w.divisions[0][0].name = "Sporting | \"Clube\"\nLisboa".into();
        w.divisions[0][0].names[0] = "João | Pé\r".into();
        let text = w.save();
        assert_eq!(text.lines().filter(|l| l.starts_with("club|")).count(), 38);
        let back = World::load(&text).unwrap();
        assert_eq!(back.divisions[0][0].name, "Sporting   \"Clube\" Lisboa");
        assert_eq!(
            back.human.as_ref().unwrap().plan,
            w.human.as_ref().unwrap().plan
        );
        assert_eq!(back.year, 1998);
        assert_eq!(back.economy, INITIAL_INDEX);
    }

    #[test]
    fn loading_rejects_garbage_with_a_message() {
        assert!(World::load("hello").unwrap_err().contains("not an Elifoot"));
        let good = World::synthetic(33).save();
        assert!(World::load(&good.replace("year|1998", "year|abc")).is_err());
        assert!(World::load(&good.replace("p|", "q|"))
            .unwrap_err()
            .contains("unknown record"));
        let truncated: String = good.lines().take(40).collect::<Vec<_>>().join("\n");
        assert!(World::load(&truncated).unwrap_err().contains("8 clubs"));
        assert!(World::load(&good.replacen("seed|", "sead|", 1)).is_err());
    }

    #[test]
    fn finances_survive_a_save_and_old_saves_get_new_game_money() {
        let mut w = World::synthetic(77);
        for _ in 0..3 {
            w.play_round();
        }
        let back = World::load(&w.save()).unwrap();
        for d in 0..4 {
            for (a, b) in w.divisions[d].iter().zip(&back.divisions[d]) {
                assert_eq!(a.fin, b.fin);
            }
        }
        assert_eq!(back.cash_ref, w.cash_ref);
        assert!(w.divisions[0]
            .iter()
            .all(|c| c.fin.gate >= 0 && c.fin.wages < 0));
        // a save from before finances: club lines have at most 11 fields
        let old: String = w
            .save()
            .lines()
            .map(|l| {
                if l.starts_with("club|") {
                    l.split('|').take(11).collect::<Vec<_>>().join("|")
                } else {
                    l.to_string()
                }
            })
            .filter(|l| !l.starts_with("scale|") && !l.starts_with("cashref|"))
            .collect::<Vec<_>>()
            .join("\n");
        let loaded = World::load(&old).unwrap();
        assert_eq!(loaded.divisions[0][0].fin.cash, 49 * 50_000 * 5);
        assert_eq!(loaded.divisions[0][0].fin.stadium, 15);
    }
}
