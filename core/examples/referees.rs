//! The day's games of an original save (`270a`) with their referee (`2706`) and stored bonus values (`+0x4c` / `+0x4e`) next to
//! `c95c` (`matchsim::fixture_value`) of the referee's name and country against the clubs' short names and countries:
//! `cargo run --release -p rustfoot-core --example referees -- save.e98 ...`.
use rustfoot_core::e98;
use rustfoot_core::matchsim::{referee_value, TeamMemory};

fn main() -> Result<(), String> {
    let (mut n, mut ok) = (0, 0);
    let (mut batch, mut picked) = (0, 0);
    for path in std::env::args().skip(1) {
        let s = e98::parse(&std::fs::read(&path).map_err(|e| e.to_string())?)?;
        // the last batch of games (`0875` shuffles the referees before each division and before a cup day): game k takes the
        // first referee in list order of the right country that is free now or refereeing game k or a later one of the batch
        let last = s.games.iter().rposition(|g| g[0] != 0).map_or(0, |i| i + 1);
        let tail: Vec<&Vec<u8>> = s.games[last..].iter().collect();
        let rid = |g: &Vec<u8>| u16::from_le_bytes([g[60], g[61]]);
        let cty = |id: u16| {
            s.teams
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.country.clone())
                .unwrap_or_default()
        };
        for (k, g) in tail.iter().enumerate() {
            let (h, a) = (
                cty(u16::from_le_bytes([g[42], g[43]])),
                cty(u16::from_le_bytes([g[44], g[45]])),
            );
            let later: Vec<u16> = tail[k..].iter().map(|g| rid(g)).collect();
            let avail = |r: &&e98::E98Referee| r.free || later.contains(&r.id);
            let pick = s
                .referees
                .iter()
                .filter(avail)
                .find(|r| r.country == h && r.country == a)
                .or_else(|| {
                    s.referees
                        .iter()
                        .filter(avail)
                        .find(|r| r.country != h && r.country != a)
                });
            batch += 1;
            picked += (pick.map(|r| r.id) == Some(rid(g))) as usize;
        }
        for g in s.games.iter().filter(|g| g[0] == 0) {
            let w = |k: usize| u16::from_le_bytes([g[k], g[k + 1]]);
            let (home, away, rid, hb, ab) = (w(42), w(44), w(60), w(62) as i32, w(64) as i32);
            let team = |id: u16| s.teams.iter().find(|t| t.id == id);
            let (Some(h), Some(a), Some(r)) = (
                team(home),
                team(away),
                s.referees.iter().find(|r| r.id == rid),
            ) else {
                println!("{path}: game {home} - {away} referee {rid}: not found");
                continue;
            };
            let bytes = |s: &str| s.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>();
            let v = |t: &e98::E98Team| {
                let (bg, fg) = t.colors;
                let short = bytes(&t.short_name);
                let country = bytes(&t.country);
                let mem = TeamMemory {
                    short_name: &short,
                    colors: [bg[0], bg[1], bg[2], 0, fg[0], fg[1], fg[2], 0],
                    country: &country,
                    level: t.level,
                };
                referee_value(&bytes(&r.name), &bytes(&r.country), &mem)
            };
            let (ch, ca) = (v(h) + 2, v(a));
            n += 1;
            ok += (ch == hb && ca == ab) as usize;
            println!(
                "{:<16} {} - {:<16} {}  {:<22} {}  stored {hb:>2} {ab:>2}  c95c {ch:>2} {ca:>2}",
                h.short_name, h.country, a.short_name, a.country, r.name, r.country
            );
        }
    }
    println!("{ok} of {n} games match");
    println!("{picked} of {batch} games of the last batch have the referee the rule picks");
    Ok(())
}
