//! Save and load a [`World`] as a small versioned text format (one record per line, fields separated by `|`).
//!
//! This is *our* format, not the original `.e98` (whose layout is only partly decoded, see `docs/findings.md`). It stores
//! everything needed to continue a game identically: the clubs with their squads and standings, the outside pool, the year,
//! round and economy index, the user's plan, and the RNG seed. The results of the last round and the last season summary
//! are not stored.
use crate::economy::{DEFAULT_SCALE, INITIAL_INDEX};
use crate::finance::Finance;
use crate::rng::DelphiRng;
use crate::season::{Club, Plan, PlannedSub};
use crate::sector::{Slot, Status};
use crate::world::{Human, World};

const MAGIC: &str = "ELIFOOT98-RS 1";

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
        if let Some(h) = &self.human {
            let subs: Vec<String> = h
                .plan
                .subs
                .iter()
                .map(|s| format!("{}:{}:{}", s.minute, s.out, s.inn))
                .collect();
            out.push_str(&format!(
                "human|{}|{}|{}|{}|{}\n",
                clean(&h.club),
                h.formation,
                list(&h.plan.starters),
                list(&h.plan.bench),
                subs.join(",")
            ));
        }
        for name in &self.cup {
            out.push_str(&format!("cup|{}\n", clean(name)));
        }
        if let Some(c) = &self.cup_champion {
            out.push_str(&format!("cupchampion|{}\n", clean(c)));
        }
        let mut write_club = |division: usize, c: &Club| {
            out.push_str(&format!(
                "club|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}\n",
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
                c.fin.interest
            ));
            for (slot, name) in c.slots.iter().zip(&c.names) {
                out.push_str(&format!(
                    "p|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                    clean(name),
                    slot.position,
                    slot.strength,
                    slot.same_country as u8,
                    slot.ban,
                    slot.injured,
                    slot.rating,
                    status_code(slot.status),
                    0
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
        let mut human: Option<Human> = None;
        let mut cup: Vec<String> = Vec::new();
        let mut cup_champion: Option<String> = None;
        let mut has_cup = false;
        let mut has_finance = false;
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
                    human = Some(Human {
                        club: f.get(1).ok_or("human without club")?.to_string(),
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
                            gate: int(15),
                            sales: int(16),
                            purchases: int(17),
                            wages: int(18),
                            interest: int(19),
                        };
                        has_finance = true;
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
            human,
            cup,
            last_cup: None,
            cup_champion,
        };
        if !has_cup {
            w.reset_cup();
        }
        if !has_finance {
            w.give_new_game_finances();
        }
        if w.human.is_none() || w.human_location().is_none() {
            w.pick_human();
        }
        Ok(w)
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
