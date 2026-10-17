//! Minute-by-minute match, following `1048:069b` -> `1010:a6eb`/`46df`/`aea7`.
//!
//! Implemented: primary bonus rolls (`Random(6000)`/`Random(7000)`), the main goal generator, scorer
//! selection and goal annulment. **Not yet modelled** (decoded elsewhere in the original): red cards and
//! injuries (`aa11`, `ac72`), penalties, substitutions, half-time/extra-time handling, human interaction.
//! `scorer` for the main generator uses an approximation of the roulette in `1010:100b` (weights unknown).
use crate::chance::{self, SideState};
use crate::outcome::{self, Outcome};
use crate::rng::DelphiRng;
use crate::sector::{self, Sectors, Slot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Goal,
    Annulled,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub minute: u16,
    pub side: Side,
    pub kind: EventKind,
    /// Index into the scoring team's slots.
    pub scorer: usize,
    /// Came from the rare `Random(6000)/(7000)` path (event kind 2 in the original).
    pub primary: bool,
}

#[derive(Clone, Debug)]
pub struct TeamState {
    pub slots: Vec<Slot>,
    pub sectors: Sectors,
    pub goals: i32,
    pub annulled: i32,
    /// `team+0x5b` Real48 multiplier (1.0 at the start).
    pub k: f64,
}

impl TeamState {
    pub fn new(slots: Vec<Slot>) -> Self {
        let sectors = sector::compute(&slots);
        Self {
            slots,
            sectors,
            goals: 0,
            annulled: 0,
            k: 1.0,
        }
    }

    fn side_state(&self) -> SideState {
        SideState {
            sectors: self.sectors,
            goals: self.goals,
            annulled: self.annulled,
            k: self.k,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Match {
    pub home: TeamState,
    pub away: TeamState,
    pub minute: u16,
    pub events: Vec<Event>,
    /// Match object `+0x4c` / `+0x4e`: per-match bonus values (`c95c` + 2 for the home side).
    pub home_bonus: i32,
    pub away_bonus: i32,
}

impl Match {
    pub fn new(home: Vec<Slot>, away: Vec<Slot>, home_bonus: i32, away_bonus: i32) -> Self {
        Self {
            home: TeamState::new(home),
            away: TeamState::new(away),
            minute: 0,
            events: Vec::new(),
            home_bonus,
            away_bonus,
        }
    }

    fn team(&self, side: Side) -> &TeamState {
        match side {
            Side::Home => &self.home,
            Side::Away => &self.away,
        }
    }

    fn team_mut(&mut self, side: Side) -> &mut TeamState {
        match side {
            Side::Home => &mut self.home,
            Side::Away => &mut self.away,
        }
    }

    /// AI-resolved chance (`1038:39b6`): best attacker against the best goalkeeper; a goal needs outcome `Goal`.
    fn primary_goal(&mut self, rng: &mut DelphiRng, side: Side) -> Option<usize> {
        let (att, def) = match side {
            Side::Home => (&self.home, &self.away),
            Side::Away => (&self.away, &self.home),
        };
        let shooter = sector::best_attacker(&att.slots)?;
        let keeper = sector::best_defender(&def.slots)?;
        let o = outcome::shot(
            rng,
            att.slots[shooter].attack_force(),
            def.slots[keeper].strength,
        );
        (o == Outcome::Goal).then_some(shooter)
    }

    /// Approximation of the `100b` roulette: weight = attack force of starters (outfield players weigh at least 1).
    fn roulette_scorer(&self, rng: &mut DelphiRng, side: Side) -> Option<usize> {
        let slots = &self.team(side).slots;
        let w = |p: &Slot| {
            if p.starter {
                p.attack_force().max(1) as u32
            } else {
                0
            }
        };
        let total: u32 = slots.iter().map(w).sum();
        if total == 0 {
            return None;
        }
        let r = rng.random(total.min(u16::MAX as u32) as u16) as u32;
        let mut acc = 0;
        slots.iter().position(|p| {
            acc += w(p);
            w(p) > 0 && r < acc
        })
    }

    /// Main generator for one side; returns the scorer when a goal happens.
    fn secondary_goal(&mut self, rng: &mut DelphiRng, side: Side) -> Option<usize> {
        let own = self.team(side).side_state();
        let opp = self
            .team(match side {
                Side::Home => Side::Away,
                Side::Away => Side::Home,
            })
            .sectors;
        if chance::chance(rng, &own, &opp, side == Side::Home, self.minute) {
            self.roulette_scorer(rng, side)
        } else {
            None
        }
    }

    /// `1010:a6eb` without penalties. Returns true when a goal was scored this minute.
    fn chances(&mut self, rng: &mut DelphiRng) -> bool {
        let m = self.minute;
        let r6000 = rng.random(6000) as i32;
        let mut home = if self.home_bonus >= 0 && r6000 < self.home_bonus {
            self.primary_goal(rng, Side::Home).map(|s| (s, true))
        } else {
            None
        };
        let r7000 = rng.random(7000) as i32;
        let mut away = if home.is_none() && self.away_bonus >= 0 && r7000 < self.away_bonus {
            self.primary_goal(rng, Side::Away).map(|s| (s, true))
        } else {
            None
        };
        if home.is_none() && away.is_none() {
            home = self.secondary_goal(rng, Side::Home).map(|s| (s, false));
        }
        if home.is_none() && away.is_none() {
            away = self.secondary_goal(rng, Side::Away).map(|s| (s, false));
        }
        for (side, hit) in [(Side::Home, home), (Side::Away, away)] {
            if let Some((scorer, primary)) = hit {
                self.team_mut(side).goals += 1;
                self.events.push(Event {
                    minute: m,
                    side,
                    kind: EventKind::Goal,
                    scorer,
                    primary,
                });
                return true;
            }
        }
        false
    }

    /// `1010:aea7`: the goal scored in the previous minute may be annulled.
    fn annul(&mut self, rng: &mut DelphiRng) {
        if self.events.is_empty() {
            return;
        }
        let r = rng.random(200) as i32;
        if r < 10 && self.home_bonus < r {
            let last = *self.events.last().unwrap();
            if last.kind == EventKind::Goal && last.minute + 1 == self.minute {
                let t = self.team_mut(last.side);
                t.annulled += 1;
                t.goals -= 1;
                self.events.last_mut().unwrap().kind = EventKind::Annulled;
            }
        }
    }

    /// One minute of play (`1048:069b` body for a single game).
    pub fn step(&mut self, rng: &mut DelphiRng) {
        self.minute += 1;
        let goal = self.chances(rng);
        // aa11 (Random(1000)) and ac72 (Random(500)) would run here when no goal happened: not modelled yet.
        if !goal && self.minute != 46 && self.minute != 91 {
            self.annul(rng);
        }
    }

    pub fn play(&mut self, rng: &mut DelphiRng) {
        while self.minute < 90 {
            self.step(rng);
        }
    }

    pub fn score(&self) -> (i32, i32) {
        (self.home.goals, self.away.goals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4-4-2 with `strength` everywhere, domestic players, `g` drawn from 1..=10.
    pub fn squad(strength: u16, rng: &mut DelphiRng) -> Vec<Slot> {
        let shape = [0u8, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3];
        shape
            .iter()
            .map(|&position| Slot {
                starter: true,
                position,
                strength,
                g: rng.random(10) as u8 + 1,
                same_country: true,
            })
            .collect()
    }

    fn avg(n: u32, home_s: u16, away_s: u16, seed: u32) -> (f64, f64, f64, f64) {
        let mut rng = DelphiRng::new(seed);
        let (mut hg, mut ag, mut hw, mut aw) = (0.0, 0.0, 0.0, 0.0);
        for _ in 0..n {
            let (h, a) = (squad(home_s, &mut rng), squad(away_s, &mut rng));
            let mut m = Match::new(h, a, 2, 0);
            m.play(&mut rng);
            let (x, y) = m.score();
            hg += x as f64;
            ag += y as f64;
            hw += (x > y) as u32 as f64;
            aw += (y > x) as u32 as f64;
        }
        let n = n as f64;
        (hg / n, ag / n, hw / n, aw / n)
    }

    #[test]
    fn goals_per_game_are_in_a_football_range() {
        let (h, a, _, _) = avg(20_000, 6, 6, 1);
        assert!(
            (0.8..2.5).contains(&h) && (0.6..2.2).contains(&a),
            "home {h:.2} away {a:.2}"
        );
    }

    #[test]
    fn home_side_has_an_advantage() {
        let (h, a, hw, aw) = avg(20_000, 6, 6, 2);
        assert!(h > a && hw > aw, "{h:.2}-{a:.2} wins {hw:.2}/{aw:.2}");
    }

    #[test]
    fn stronger_team_wins_more() {
        let (_, _, hw, aw) = avg(20_000, 9, 4, 3);
        assert!(hw > aw + 0.2, "{hw:.2} vs {aw:.2}");
        let (_, _, hw2, aw2) = avg(20_000, 4, 9, 4);
        assert!(aw2 > hw2, "{hw2:.2} vs {aw2:.2}");
    }

    #[test]
    fn annulled_goals_are_removed_from_the_score() {
        let mut rng = DelphiRng::new(5);
        let mut annulled = 0;
        for _ in 0..5_000 {
            let (h, a) = (squad(8, &mut rng), squad(8, &mut rng));
            let mut m = Match::new(h, a, 2, 0);
            m.play(&mut rng);
            let goals = m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Goal)
                .count() as i32;
            assert_eq!(m.home.goals + m.away.goals, goals);
            annulled += m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Annulled)
                .count();
        }
        assert!(annulled > 0);
    }
}
