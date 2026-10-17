//! Plain C-ABI bindings: `world_new(seed)`, `world_play_round()`, `world_play_season()`, then read the state with
//! `world_json_ptr()` / `world_json_len()` (UTF-8 JSON in WebAssembly memory, valid until the next call).
use elifoot_core::eft::{self, EftTeam};
use elifoot_core::matchsim::{Event, EventKind, Side};
use elifoot_core::season::{standings, Club, MatchRecord};
use elifoot_core::world::{World, DIVISION_BASE, DIVISION_NAMES, ROUNDS};
use std::cell::RefCell;
use std::fmt::Write;

thread_local! {
    static WORLD: RefCell<Option<World>> = const { RefCell::new(None) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
    static BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
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
    format!(
        "{{\"div\":{},\"home\":{},\"away\":{},\"hg\":{},\"ag\":{},\"events\":[{}]}}",
        division,
        esc(&home.name),
        esc(&away.name),
        r.home_goals,
        r.away_goals,
        events.join(",")
    )
}

fn world_json(w: &World) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "{{\"year\":{},\"round\":{},\"rounds\":{},\"divisions\":[",
        w.year, w.round, ROUNDS
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
    out.push_str("],\"lastRound\":[");
    let ms: Vec<String> = w
        .last_round
        .iter()
        .map(|(d, r)| match_json(*d, r, w))
        .collect();
    out.push_str(&ms.join(","));
    out.push_str("],\"lastSeason\":");
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
                "{{\"year\":{},\"tables\":[{}],\"moves\":[{}]}}",
                s.year,
                tables.join(","),
                moves.join(",")
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
        use elifoot_core::cipher::encode;
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
}
