//! Plain C-ABI bindings: `world_new(seed)`, `world_play_round()`, `world_play_season()`, then read the state with
//! `world_json_ptr()` / `world_json_len()` (UTF-8 JSON in WebAssembly memory, valid until the next call).
use rustfoot_core::eft::{self, EftTeam};
use rustfoot_core::matchsim::{Event, EventKind, Side};
use rustfoot_core::season::{standings, Club, MatchRecord, Plan, PlannedSub, FORMATIONS};
use rustfoot_core::world::{World, DIVISION_BASE, DIVISION_NAMES, ROUNDS};
use std::cell::RefCell;
use std::fmt::Write;

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
    static BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static SAVE: RefCell<String> = const { RefCell::new(String::new()) };
    static OFFERS: RefCell<Vec<rustfoot_core::world::Offer>> = const { RefCell::new(Vec::new()) };
    static MSG: RefCell<String> = const { RefCell::new(String::new()) };
    static TEAMS: RefCell<Vec<EftTeam>> = const { RefCell::new(Vec::new()) };
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn mean_strength(c: &Club) -> f64 {
    c.slots.iter().map(|s| s.strength as f64).sum::<f64>() / c.slots.len().max(1) as f64
}

fn event_json(e: &Event, home: &Club, away: &Club) -> String {
    let club = if e.side == Side::Home { home } else { away };
    let kind = match e.kind {
        EventKind::Goal => "goal",
        EventKind::Annulled => "annulled",
        EventKind::RedCard => "red",
        EventKind::Injury => "injury",
    };
    let player = club.names.get(e.player).map(String::as_str).unwrap_or("?");
    format!(
        "{{\"m\":{},\"side\":{},\"kind\":\"{}\",\"player\":{}}}",
        e.minute,
        if e.side == Side::Home { 0 } else { 1 },
        kind,
        esc(player)
    )
}

fn match_json(division: usize, r: &MatchRecord, w: &World) -> String {
    let (home, away) = (
        &w.divisions[division][r.home],
        &w.divisions[division][r.away],
    );
    let events: Vec<String> = r.events.iter().map(|e| event_json(e, home, away)).collect();
    let name = |club: &Club, i: usize| esc(club.names.get(i).map(String::as_str).unwrap_or("?"));
    let subs: Vec<String> = r
        .subs
        .iter()
        .map(|s| {
            let club = if s.side == Side::Home { home } else { away };
            format!(
                "{{\"m\":{},\"side\":{},\"out\":{},\"in\":{},\"auto\":{}}}",
                s.minute,
                if s.side == Side::Home { 0 } else { 1 },
                name(club, s.out),
                name(club, s.inn),
                s.auto
            )
        })
        .collect();
    format!(
        "{{\"div\":{},\"home\":{},\"away\":{},\"hg\":{},\"ag\":{},\"homeIdx\":{},\"awayIdx\":{},\"att\":{},\"gate\":{},\"events\":[{}],\"subs\":[{}]}}",
        division,
        esc(&home.name),
        esc(&away.name),
        r.home_goals,
        r.away_goals,
        r.home,
        r.away,
        r.attendance,
        r.gate,
        events.join(","),
        subs.join(",")
    )
}

fn human_json(w: &World) -> String {
    let (Some(h), Some((d, i))) = (w.human.as_ref(), w.human_location()) else {
        return "null".to_string();
    };
    let club = &w.divisions[d][i];
    let squad: Vec<String> = club
        .slots
        .iter()
        .enumerate()
        .map(|(k, p)| {
            format!(
                "{{\"i\":{},\"name\":{},\"pos\":{},\"str\":{},\"star\":{},\"ban\":{},\"inj\":{},\"wage\":{},\"ask\":{},\"sell\":{}}}",
                k,
                esc(club.names.get(k).map(String::as_str).unwrap_or("?")),
                p.position,
                p.strength,
                p.star(),
                p.ban,
                p.injured,
                p.wage,
                rustfoot_core::market::floor_1000(rustfoot_core::market::market_value(p, w.economy, w.scale)),
                rustfoot_core::market::sale_blocker(club, k) == 0
            )
        })
        .collect();
    let list = |v: &[usize]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    let subs: Vec<String> = h
        .plan
        .subs
        .iter()
        .map(|s| format!("[{},{},{}]", s.minute, s.out, s.inn))
        .collect();
    let formations: Vec<String> = FORMATIONS
        .iter()
        .map(|f| format!("[{},{},{}]", f.0, f.1, f.2))
        .collect();
    let fin = &club.fin;
    let money = w.money_ctx(d);
    let wage = rustfoot_core::finance::wage_bill(&club.slots);
    let finance = format!(
        "{{\"cash\":{},\"loan\":{},\"f\":{:.4},\"stadium\":{},\"capacity\":{},\"gate\":{},\"sales\":{},\"purchases\":{},\"wages\":{},\"interest\":{},\"wageBill\":{},\"price\":{},\"scale\":{},\"step\":{},\"maxLoan\":{},\"fill\":{:.3},\"canExpand\":{},\"expandCost\":{}}}",
        fin.cash,
        fin.loan,
        fin.f,
        fin.stadium,
        fin.capacity(),
        fin.gate,
        fin.sales,
        fin.purchases,
        fin.wages,
        fin.interest,
        wage,
        money.price,
        w.scale,
        rustfoot_core::finance::loan_step(w.scale),
        rustfoot_core::finance::max_loan(rustfoot_core::finance::rating(&club.slots), club.k, w.economy, w.scale),
        fin.fill,
        rustfoot_core::finance::wants_expansion(fin),
        rustfoot_core::finance::expansion_cost(fin, w.economy, w.scale)
    );
    format!(
        "{{\"club\":{},\"finance\":{},\"div\":{},\"idx\":{},\"formation\":{},\"formations\":[{}],\"squad\":[{}],\"plan\":{{\"starters\":[{}],\"bench\":[{}],\"subs\":[{}]}},\"subsLeft\":3}}",
        esc(&h.club),
        finance,
        d,
        i,
        h.formation,
        formations.join(","),
        squad.join(","),
        list(&h.plan.starters),
        list(&h.plan.bench),
        subs.join(",")
    )
}

fn cup_json(w: &World) -> String {
    let Some(c) = &w.last_cup else {
        return "null".to_string();
    };
    let ms: Vec<String> = c
        .matches
        .iter()
        .map(|m| {
            let pname = |side: Side, i: usize| {
                let v = if side == Side::Home { &m.home_players } else { &m.away_players };
                esc(v.get(i).map(String::as_str).unwrap_or("?"))
            };
            let events: Vec<String> = m
                .events
                .iter()
                .map(|e| {
                    let kind = match e.kind {
                        EventKind::Goal => "goal",
                        EventKind::Annulled => "annulled",
                        EventKind::RedCard => "red",
                        EventKind::Injury => "injury",
                    };
                    format!(
                        "{{\"m\":{},\"side\":{},\"kind\":\"{}\",\"player\":{}}}",
                        e.minute,
                        if e.side == Side::Home { 0 } else { 1 },
                        kind,
                        pname(e.side, e.player)
                    )
                })
                .collect();
            let subs: Vec<String> = m
                .subs
                .iter()
                .map(|s| {
                    format!(
                        "{{\"m\":{},\"side\":{},\"out\":{},\"in\":{},\"auto\":{}}}",
                        s.minute,
                        if s.side == Side::Home { 0 } else { 1 },
                        pname(s.side, s.out),
                        pname(s.side, s.inn),
                        s.auto
                    )
                })
                .collect();
            let pens = match &m.pens {
                None => "null".to_string(),
                Some(p) => {
                    let ks: Vec<String> = p
                        .kicks
                        .iter()
                        .map(|k| {
                            let side = if k.side == 0 { Side::Home } else { Side::Away };
                            format!(
                                "{{\"side\":{},\"player\":{},\"out\":\"{}\",\"goal\":{}}}",
                                k.side,
                                pname(side, k.player),
                                k.outcome.text(),
                                k.outcome == rustfoot_core::outcome::Outcome::Goal
                            )
                        })
                        .collect();
                    format!("{{\"h\":{},\"a\":{},\"kicks\":[{}]}}", p.home, p.away, ks.join(","))
                }
            };
            format!(
                "{{\"home\":{},\"away\":{},\"hg\":{},\"ag\":{},\"pens\":{},\"extraTime\":{},\"shootout\":{},\"att\":{},\"winner\":{},\"events\":[{}],\"subs\":[{}]}}",
                esc(&m.home),
                esc(&m.away),
                m.home_goals,
                m.away_goals,
                pens,
                m.extra_time,
                m.shootout,
                m.attendance,
                esc(&m.winner),
                events.join(","),
                subs.join(",")
            )
        })
        .collect();
    format!(
        "{{\"name\":{},\"left\":{},\"matches\":[{}]}}",
        esc(&c.name),
        c.left,
        ms.join(",")
    )
}

fn world_json(w: &World) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\"year\":{},\"round\":{},\"rounds\":{},\"economy\":{:.3},\"divisions\":[",
        w.year, w.round, ROUNDS, w.economy
    );
    for (d, div) in w.divisions.iter().enumerate() {
        if d > 0 {
            out.push(',');
        }
        let rows: Vec<String> = standings(div)
            .into_iter()
            .map(|i| {
                let c = &div[i];
                format!(
                    "{{\"name\":{},\"pts\":{},\"w\":{},\"d\":{},\"l\":{},\"gf\":{},\"ga\":{},\"k\":{:.2},\"str\":{:.1}}}",
                    esc(&c.name),
                    c.points,
                    c.wins,
                    c.draws,
                    c.losses,
                    c.goals_for,
                    c.goals_against,
                    c.k,
                    mean_strength(c)
                )
            })
            .collect();
        let _ = write!(
            out,
            "{{\"name\":{},\"base\":{},\"table\":[{}]}}",
            esc(DIVISION_NAMES[d]),
            DIVISION_BASE[d],
            rows.join(",")
        );
    }
    let _ = write!(out, "],\"human\":{},\"lastRound\":[", human_json(w));
    let ms: Vec<String> = w
        .last_round
        .iter()
        .map(|(d, r)| match_json(*d, r, w))
        .collect();
    out.push_str(&ms.join(","));
    let transfers: Vec<String> = w
        .transfers
        .iter()
        .rev()
        .take(40)
        .map(|t| {
            format!(
                "{{\"year\":{},\"round\":{},\"player\":{},\"pos\":{},\"str\":{},\"from\":{},\"to\":{},\"fee\":{}}}",
                t.year,
                t.round,
                esc(&t.player),
                t.position,
                t.strength,
                esc(&t.from),
                esc(&t.to),
                t.fee
            )
        })
        .collect();
    let board: Vec<String> = w
        .board
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let seller = w
                .divisions
                .iter()
                .flatten()
                .chain(w.pool.iter())
                .find(|c| c.name == l.seller);
            let (pos, st) = seller
                .and_then(|c| c.names.iter().position(|n| *n == l.player).map(|k| c.slots[k]))
                .map(|sl| (sl.position, sl.strength))
                .unwrap_or((0, 0));
            let (bc, bf) = l.best.clone().unwrap_or_default();
            format!(
                "{{\"i\":{},\"seller\":{},\"player\":{},\"pos\":{},\"str\":{},\"ask\":{},\"best\":{},\"bestClub\":{}}}",
                i,
                esc(&l.seller),
                esc(&l.player),
                pos,
                st,
                l.ask,
                bf,
                esc(&bc)
            )
        })
        .collect();
    let offers: Vec<String> = OFFERS.with(|o| {
        o.borrow()
            .iter()
            .enumerate()
            .map(|(i, f)| {
                format!(
                    "{{\"i\":{},\"club\":{},\"player\":{},\"pos\":{},\"str\":{},\"nat\":{},\"price\":{}}}",
                    i,
                    esc(&f.club),
                    esc(&f.player),
                    f.position,
                    f.strength,
                    esc(&String::from_utf8_lossy(&f.nationality)),
                    f.price
                )
            })
            .collect()
    });
    let msg = MSG.with(|m| std::mem::take(&mut *m.borrow_mut()));
    let _ = write!(
        out,
        "],\"coachChanges\":[{}],\"humanMoved\":{},\"transfers\":[{}],\"board\":[{}],\"offers\":[{}],\"msg\":{}",
        w.coach_changes
            .iter()
            .map(|(a, b)| format!("[{},{}]", esc(a), esc(b)))
            .collect::<Vec<_>>()
            .join(","),
        match w.human_moves().first() {
            Some((a, b)) => format!("[{},{}]", esc(a), esc(b)),
            None => "null".into(),
        },
        transfers.join(","),
        board.join(","),
        offers.join(","),
        esc(&msg)
    );
    let _ = write!(
        out,
        ",\"cup\":{},\"cupLeft\":{},\"lastCup\":{},\"lastSeason\":",
        w.cup
            .iter()
            .any(|n| Some(n.as_str()) == w.human.as_ref().map(|h| h.club.as_str())),
        w.cup.len(),
        cup_json(w)
    );
    match &w.last_season {
        None => out.push_str("null"),
        Some(s) => {
            let tables: Vec<String> = s
                .tables
                .iter()
                .map(|t| {
                    let rows: Vec<String> = t
                        .iter()
                        .map(|r| format!("{{\"name\":{},\"pts\":{},\"w\":{},\"d\":{},\"l\":{},\"gf\":{},\"ga\":{}}}", esc(&r.name), r.points, r.wins, r.draws, r.losses, r.goals_for, r.goals_against))
                        .collect();
                    format!("[{}]", rows.join(","))
                })
                .collect();
            let moves: Vec<String> = s
                .moves
                .iter()
                .map(|m| {
                    format!(
                        "{{\"name\":{},\"from\":{},\"to\":{}}}",
                        esc(&m.name),
                        m.from,
                        m.to
                    )
                })
                .collect();
            let _ = write!(
                out,
                "{{\"year\":{},\"tables\":[{}],\"moves\":[{}],\"cupWinner\":{}}}",
                s.year,
                tables.join(","),
                moves.join(","),
                s.cup_winner
                    .as_deref()
                    .map(esc)
                    .unwrap_or_else(|| "null".to_string())
            );
        }
    }
    out.push('}');
    out
}

fn publish() {
    let json = WORLD
        .with(|w| w.borrow().as_ref().map(world_json))
        .unwrap_or_else(|| "null".to_string());
    OUT.with(|o| *o.borrow_mut() = json);
}

#[no_mangle]
pub extern "C" fn world_new(seed: u32) {
    WORLD.with(|w| *w.borrow_mut() = Some(World::synthetic(seed)));
    publish();
}

/// Resizes the shared input buffer and returns its address; JavaScript writes a `.EFT` file there.
#[no_mangle]
pub extern "C" fn buf_resize(len: usize) -> *mut u8 {
    BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.resize(len, 0);
        b.as_mut_ptr()
    })
}

/// Forgets the team files added so far.
#[no_mangle]
pub extern "C" fn eft_clear() {
    TEAMS.with(|t| t.borrow_mut().clear());
}

/// Parses the first `len` bytes of the buffer as a `.EFT` team; returns 1 when it was valid and added.
#[no_mangle]
pub extern "C" fn eft_add(len: usize) -> i32 {
    let parsed = BUF.with(|b| eft::parse(&b.borrow()[..len.min(b.borrow().len())]).ok());
    match parsed {
        Some(team) => {
            TEAMS.with(|t| t.borrow_mut().push(team));
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn eft_count() -> usize {
    TEAMS.with(|t| t.borrow().len())
}

/// Builds the world from the added team files; returns 1 on success, 0 when it failed (the JSON is then `{"error":...}`).
#[no_mangle]
pub extern "C" fn world_new_from_efts(seed: u32) -> i32 {
    let result = TEAMS.with(|t| World::from_teams(seed, &t.borrow()));
    match result {
        Ok(w) => {
            WORLD.with(|cell| *cell.borrow_mut() = Some(w));
            publish();
            1
        }
        Err(e) => {
            OUT.with(|o| *o.borrow_mut() = format!("{{\"error\":{}}}", esc(&e)));
            0
        }
    }
}

/// Serialises the game; read the text with `save_ptr()` / `save_len()`.
#[no_mangle]
pub extern "C" fn world_save() {
    let text = WORLD
        .with(|w| w.borrow().as_ref().map(World::save))
        .unwrap_or_default();
    SAVE.with(|s| *s.borrow_mut() = text);
}

#[no_mangle]
pub extern "C" fn save_ptr() -> *const u8 {
    SAVE.with(|s| s.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn save_len() -> usize {
    SAVE.with(|s| s.borrow().len())
}

/// Loads a game from the first `len` bytes of the buffer; returns 1 on success, 0 on failure (JSON is then `{"error":...}`).
#[no_mangle]
pub extern "C" fn world_load(len: usize) -> i32 {
    let text =
        BUF.with(|b| String::from_utf8_lossy(&b.borrow()[..len.min(b.borrow().len())]).to_string());
    match World::load(&text) {
        Ok(w) => {
            WORLD.with(|cell| *cell.borrow_mut() = Some(w));
            publish();
            1
        }
        Err(e) => {
            OUT.with(|o| *o.borrow_mut() = format!("{{\"error\":{}}}", esc(&e)));
            0
        }
    }
}

/// Imports an original `.e98` save from the first `len` bytes of the buffer; returns 1 on success, 0 on failure
/// (the JSON is then `{"error":...}`).
#[no_mangle]
pub extern "C" fn world_import_e98(len: usize, seed: u32) -> i32 {
    let bytes = BUF.with(|b| b.borrow()[..len.min(b.borrow().len())].to_vec());
    match rustfoot_core::e98::parse(&bytes).and_then(|s| World::from_e98(seed, &s)) {
        Ok(w) => {
            WORLD.with(|cell| *cell.borrow_mut() = Some(w));
            publish();
            1
        }
        Err(e) => {
            OUT.with(|o| *o.borrow_mut() = format!("{{\"error\":{}}}", esc(&e)));
            0
        }
    }
}

/// The user bids `amount` for open auction `listing`; returns 1 on success, 0 with the reason in `msg`.
#[no_mangle]
pub extern "C" fn market_bid(listing: u32, amount: f64) -> i32 {
    let r = WORLD.with(|w| {
        w.borrow_mut()
            .as_mut()
            .map(|w| w.human_bid(listing as usize, amount as i64))
    });
    finish_market(
        r.unwrap_or(Err("sem jogo".into()))
            .map(|_| "Licitação vencedora: o jogador é teu.".to_string()),
    )
}

/// The user offers squad player `slot` for sale at `ask`; returns 1 on success, 0 with the reason in `msg`.
#[no_mangle]
pub extern "C" fn market_sell(slot: u32, ask: f64) -> i32 {
    let r = WORLD.with(|w| {
        w.borrow_mut()
            .as_mut()
            .map(|w| w.human_sell(slot as usize, ask as i64))
    });
    finish_market(
        r.unwrap_or(Err("sem jogo".into()))
            .map(|fee| format!("Vendido por {fee}.")),
    )
}

/// Player search (`Procurar jogadores`): the name filter is in the buffer (`name_len` bytes), `nat` is a three-letter code packed
/// as `a<<16|b<<8|c` (0 = any), `position` -1 = any, `max_price` 0 = any. The results come back as `offers` in the JSON.
#[no_mangle]
pub extern "C" fn buy_search(
    name_len: usize,
    nat: u32,
    min_strength: u32,
    position: i32,
    max_price: f64,
    hide_mine: i32,
) {
    let name = BUF.with(|b| {
        String::from_utf8_lossy(&b.borrow()[..name_len.min(b.borrow().len())]).to_string()
    });
    let q = rustfoot_core::world::Search {
        name,
        nationality: (nat != 0).then_some([(nat >> 16) as u8, (nat >> 8) as u8, nat as u8]),
        min_strength: min_strength as u16,
        position: (position >= 0).then_some(position as u8),
        max_price: (max_price > 0.0).then_some(max_price as i64),
        hide_mine: hide_mine != 0,
    };
    let found = WORLD.with(|w| {
        w.borrow()
            .as_ref()
            .map(|w| w.search_offers(&q))
            .unwrap_or_default()
    });
    OFFERS.with(|o| *o.borrow_mut() = found);
    publish();
}

/// Buys offer number `i` of the last search at its asking price; returns 1 on success, 0 with the reason in `msg`.
#[no_mangle]
pub extern "C" fn buy_player(i: u32) -> i32 {
    let offer = OFFERS.with(|o| o.borrow().get(i as usize).cloned());
    let r = match offer {
        None => Err("pesquisa desactualizada".to_string()),
        Some(f) => WORLD
            .with(|w| {
                w.borrow_mut()
                    .as_mut()
                    .map(|w| w.human_buy(&f.club, f.index, &f.player))
            })
            .unwrap_or(Err("sem jogo".into()))
            .map(|p| format!("{} comprado por {}; chega no fim da jornada.", f.player, p)),
    };
    if r.is_ok() {
        OFFERS.with(|o| {
            let mut o = o.borrow_mut();
            if (i as usize) < o.len() {
                o.remove(i as usize);
            }
        });
    }
    finish_market(r)
}

/// Loan step taken / repaid and stadium works, with the answer in `msg`.
#[no_mangle]
pub extern "C" fn loan_borrow() -> i32 {
    let r = WORLD.with(|w| w.borrow_mut().as_mut().map(|w| w.human_borrow()));
    finish_market(
        r.unwrap_or(Err("sem jogo".into()))
            .map(|v| format!("Empréstimo de {v} concedido.")),
    )
}

#[no_mangle]
pub extern "C" fn loan_repay() -> i32 {
    let r = WORLD.with(|w| w.borrow_mut().as_mut().map(|w| w.human_repay()));
    finish_market(
        r.unwrap_or(Err("sem jogo".into()))
            .map(|v| format!("Amortizaste {v}.")),
    )
}

#[no_mangle]
pub extern "C" fn stadium_expand() -> i32 {
    let r = WORLD.with(|w| w.borrow_mut().as_mut().map(|w| w.human_expand()));
    finish_market(
        r.unwrap_or(Err("sem jogo".into()))
            .map(|v| format!("Obras concluídas por {v}: mais 5000 lugares.")),
    )
}

fn finish_market(r: Result<String, String>) -> i32 {
    let ok = r.is_ok();
    MSG.with(|m| *m.borrow_mut() = r.unwrap_or_else(|e| e));
    publish();
    ok as i32
}

/// Makes club `club` of division `division` the user's club.
#[no_mangle]
pub extern "C" fn human_set(division: u32, club: u32) {
    WORLD.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            if (division as usize) < 4 && (club as usize) < w.divisions[division as usize].len() {
                w.set_human(division as usize, club as usize);
            }
        }
    });
    publish();
}

/// Position in the division list of the club whose name is in the buffer (`len` bytes), or -1.
#[no_mangle]
pub extern "C" fn club_index(division: u32, len: usize) -> i32 {
    let name =
        BUF.with(|b| String::from_utf8_lossy(&b.borrow()[..len.min(b.borrow().len())]).to_string());
    WORLD.with(|w| {
        w.borrow()
            .as_ref()
            .and_then(|w| {
                w.divisions
                    .get(division as usize)
                    .and_then(|d| d.iter().position(|c| c.name == name))
            })
            .map_or(-1, |i| i as i32)
    })
}

/// Picks the best line-up of formation number `k` (index into the twelve formations); returns 1 when it can be fielded.
#[no_mangle]
pub extern "C" fn human_formation(k: u32) -> i32 {
    let ok = WORLD.with(|w| {
        w.borrow_mut()
            .as_mut()
            .is_some_and(|w| w.set_formation(k as usize))
    });
    publish();
    ok as i32
}

/// Sets the user's plan from the buffer: `[n, starters.., m, bench.., k, (minute, out, in)..]`, one byte per value.
#[no_mangle]
pub extern "C" fn human_plan(len: usize) -> i32 {
    let bytes: Vec<u8> = BUF.with(|b| b.borrow()[..len.min(b.borrow().len())].to_vec());
    let mut it = bytes.into_iter();
    let mut take = |n: usize| -> Vec<u8> { (&mut it).take(n).collect() };
    let n = take(1).first().copied().unwrap_or(0) as usize;
    let starters: Vec<usize> = take(n).into_iter().map(usize::from).collect();
    let m = take(1).first().copied().unwrap_or(0) as usize;
    let bench: Vec<usize> = take(m).into_iter().map(usize::from).collect();
    let k = take(1).first().copied().unwrap_or(0) as usize;
    let raw = take(3 * k);
    let subs: Vec<PlannedSub> = raw
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| PlannedSub {
            minute: c[0] as u16,
            out: c[1] as usize,
            inn: c[2] as usize,
        })
        .collect();
    WORLD.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            w.set_plan(Plan {
                starters,
                bench,
                subs,
            });
        }
    });
    publish();
    1
}

/// Whether the user accepts being invited to take over a club whose coach was sacked (`1` yes, `0` no).
#[no_mangle]
pub extern "C" fn world_set_accept_invitations(on: i32) {
    WORLD.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            w.accept_invitations = on != 0;
        }
    });
}

#[no_mangle]
pub extern "C" fn world_play_round() {
    WORLD.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            w.play_round();
        }
    });
    publish();
}

#[no_mangle]
pub extern "C" fn world_play_season() {
    WORLD.with(|w| {
        if let Some(w) = w.borrow_mut().as_mut() {
            w.play_season();
        }
    });
    publish();
}

#[no_mangle]
pub extern "C" fn world_json_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn world_json_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json() -> String {
        OUT.with(|o| o.borrow().clone())
    }

    #[test]
    fn json_is_well_formed_and_updates() {
        world_new(7);
        let first = json();
        assert!(first.starts_with("{\"year\":1998,\"round\":0"));
        assert!(first.contains("\"economy\":2.000"));
        world_play_round();
        let second = json();
        assert!(second.contains("\"round\":1"));
        assert!(second.contains("\"lastRound\":[{"));
        world_play_season();
        assert!(json().contains("\"lastSeason\":{\"year\":1998"));
        // brace and bracket balance outside strings
        let (mut depth, mut in_str, mut esc_next) = (0i32, false, false);
        for c in json().chars() {
            if in_str {
                match (esc_next, c) {
                    (true, _) => esc_next = false,
                    (false, '\\') => esc_next = true,
                    (false, '"') => in_str = false,
                    _ => {}
                }
            } else {
                match c {
                    '"' => in_str = true,
                    '{' | '[' => depth += 1,
                    '}' | ']' => depth -= 1,
                    _ => {}
                }
            }
            assert!(depth >= 0);
        }
        assert_eq!(depth, 0);
    }

    fn eft_bytes(i: usize) -> Vec<u8> {
        use rustfoot_core::cipher::encode;
        let mut d = vec![0u8; 0x32];
        d[..3].copy_from_slice(b"EFa");
        d.extend(encode(format!("CLUBE NUMERO {i}").as_bytes()));
        d.extend(encode(format!("CLUBE {i}").as_bytes()));
        d.extend([0xff, 0xff, 0xff, 0, 0, 0, 0, 0]);
        d.extend(encode(b"POR"));
        let squad: Vec<(u8, usize)> = vec![(0, 2), (1, 6), (2, 6), (3, 5)];
        d.extend([
            1 + (i % 15) as u8,
            squad.iter().map(|s| s.1).sum::<usize>() as u8,
            0,
        ]);
        for (pos, n) in squad {
            for k in 0..n {
                d.extend(encode(b"POR"));
                d.extend(encode(format!("Jogador {i} {pos} {k}").as_bytes()));
                d.extend([pos, 0]);
            }
        }
        d.extend(encode(b"Mister"));
        d
    }

    fn add(bytes: &[u8]) -> i32 {
        let ptr = buf_resize(bytes.len());
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        eft_add(bytes.len())
    }

    #[test]
    fn real_team_files_can_be_loaded_through_the_abi() {
        eft_clear();
        assert_eq!(add(b"not a team"), 0);
        for i in 0..20 {
            assert_eq!(add(&eft_bytes(i)), 1);
        }
        assert_eq!(eft_count(), 20);
        assert_eq!(world_new_from_efts(1), 0);
        assert!(json().starts_with("{\"error\":\"need at least"));
        for i in 20..40 {
            assert_eq!(add(&eft_bytes(i)), 1);
        }
        assert_eq!(world_new_from_efts(1), 1);
        assert!(json().starts_with("{\"year\":1998"));
        world_play_season();
        assert!(json().contains("\"lastSeason\":{"));
    }

    #[test]
    fn escapes_special_characters() {
        assert_eq!(esc("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    }

    fn send_plan(bytes: &[u8]) {
        let ptr = buf_resize(bytes.len());
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        human_plan(bytes.len());
    }

    /// Extracts the integer list after `"key":[` in the human block.
    fn list_after(json: &str, key: &str) -> Vec<u32> {
        let i = json.find(key).expect(key) + key.len();
        let j = json[i..].find(']').unwrap();
        json[i..i + j]
            .split(',')
            .filter(|x| !x.is_empty())
            .map(|x| x.parse().unwrap())
            .collect()
    }

    #[test]
    fn the_users_club_can_be_controlled_through_the_abi() {
        world_new(7);
        let first = json();
        assert!(first.contains("\"human\":{\"club\":"));
        assert!(first.contains("\"formations\":[[4,4,2],[3,3,4]"));
        // change formation to 3-4-3 (index 2): 11 starters
        assert_eq!(human_formation(2), 1);
        let starters = list_after(&json(), "\"starters\":[");
        assert_eq!(starters.len(), 11);
        assert_eq!(human_formation(99), 0);
        // send a plan: the same starters and bench, one substitution at minute 46 (first outfield starter -> first bench player)
        let j = json();
        let bench = list_after(&j, "\"bench\":[");
        let (out, inn) = (starters[1] as u8, bench[1] as u8);
        let mut bytes = vec![starters.len() as u8];
        bytes.extend(starters.iter().map(|&x| x as u8));
        bytes.push(bench.len() as u8);
        bytes.extend(bench.iter().map(|&x| x as u8));
        bytes.extend([1, 46, out, inn]);
        send_plan(&bytes);
        assert!(json().contains("\"subs\":[[46,"));
        world_play_round();
        let after = json();
        assert!(after.contains("\"subs\":[]}")); // planned subs are consumed
        assert!(after.contains("\"homeIdx\":"));
        // garbage input must not panic
        send_plan(&[200, 1, 2]);
        send_plan(&[]);
        assert!(json().contains("\"human\":{"));
    }

    #[test]
    fn a_game_can_be_saved_and_loaded_through_the_abi() {
        world_new(5);
        world_play_season();
        world_play_round();
        world_save();
        let text = SAVE.with(|s| s.borrow().clone());
        assert!(text.starts_with("ELIFOOT98-RS 1\n"));
        let before = json();
        world_new(99); // a different game
        let ptr = buf_resize(text.len());
        unsafe { std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len()) };
        assert_eq!(world_load(text.len()), 1);
        // same table, year, round and club as before the save (the last round's matches are not stored)
        let strip = |j: &str| j[..j.find("\"human\":").unwrap()].to_string();
        assert_eq!(strip(&json()), strip(&before));
        // garbage is refused with a message and leaves the game usable
        let bad = b"nope";
        let ptr = buf_resize(bad.len());
        unsafe { std::ptr::copy_nonoverlapping(bad.as_ptr(), ptr, bad.len()) };
        assert_eq!(world_load(bad.len()), 0);
        assert!(json().starts_with("{\"error\":"));
    }
}
