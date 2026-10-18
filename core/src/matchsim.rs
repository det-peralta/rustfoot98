//! Minute-by-minute match (v1), following `1048:069b` -> `a6eb`, `aa11`, `ac72`, `aea7`.
//!
//! Implemented: primary bonus rolls (`Random(6000)`/`Random(7000)`), the main goal generator with the
//! weighted scorer roulette (bench weights dilute it, as in the original), penalties after a goalkeeper or
//! defender is sent off, red cards (`aa11`), injuries (`ac72`), automatic substitutions (`67f7`) and goal
//! annulment (`aea7`). **Not yet modelled:** half-time/extra time, the post-match `K` update, human-controlled
//! interaction, the foul roulette of the interactive path.
use crate::chance::{self, SideState};
use crate::outcome::{self, Outcome};
use crate::rng::DelphiRng;
use crate::sector::{self, Sectors, Slot, Status};

/// Weight of the old value in the morale moving average (`1068:10d1` call sites in `1010:9396`).
pub const MORALE_WEIGHT: f64 = 0.85;

/// `1068:10d1`: `K = 0.85 * K + 0.15 * result`.
pub fn update_morale(k: f64, result: f64) -> f64 {
    MORALE_WEIGHT * k + (1.0 - MORALE_WEIGHT) * result
}

/// Morale result values `(home, away)` for a finished game (`1010:9396`): home win `2.0/0.0`, away win `0.0/2.5`,
/// draw `0.8/1.3`.
pub fn morale_results(home_goals: i32, away_goals: i32) -> (f64, f64) {
    match home_goals.cmp(&away_goals) {
        std::cmp::Ordering::Greater => (2.0, 0.0),
        std::cmp::Ordering::Less => (0.0, 2.5),
        std::cmp::Ordering::Equal => (0.8, 1.3),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Home,
    Away,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Home => Side::Away,
            Side::Away => Side::Home,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Goal,
    Annulled,
    /// Event kind 4 (`aa11`).
    RedCard,
    /// Event kind 5 (`ac72`).
    Injury,
}

/// A substitution (planned by the user or automatic after an injury/red card): `inn` replaces `out`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubEvent {
    pub minute: u16,
    pub side: Side,
    pub out: usize,
    pub inn: usize,
    pub auto: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub minute: u16,
    pub side: Side,
    pub kind: EventKind,
    /// Index into that team's slots (scorer, sent-off or injured player).
    pub player: usize,
    /// Position of the player (the original stores it in the event, `+0x0c`).
    pub position: u8,
    /// Goal came from the rare bonus/penalty path (event kind 2).
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
    /// `team+0x4d`: substitutions left, `min(3, fit substitutes)` (`1010:4b73`).
    pub subs_left: u8,
}

impl TeamState {
    pub fn new(slots: Vec<Slot>) -> Self {
        let fit = slots
            .iter()
            .filter(|s| s.status == Status::Sub && s.available())
            .count();
        let sectors = sector::compute(&slots);
        Self {
            slots,
            sectors,
            goals: 0,
            annulled: 0,
            k: 1.0,
            subs_left: fit.min(3) as u8,
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

    fn recompute(&mut self) {
        self.sectors = sector::compute(&self.slots);
    }
}

/// `1010:c95c`: the per-fixture value behind `match+0x4c` / `+0x4e`.
///
/// `a` is the Pascal string at `division+7 -> +4`, `b` the one at `team data+0x2d`, `domestic` is true when the team's
/// country code (`team data+0x4a`) equals the string at `division+7 -> +0x2d`. Bytes past a string's length count as 0.
/// The match stores `fixture_value(home) + 2` for the home side and `fixture_value(away)` for the away side.
pub fn fixture_value(a: &[u8], b: &[u8], domestic: bool) -> i32 {
    let n = a.len().max(b.len());
    let at = |s: &[u8], i: usize| s.get(i).copied().unwrap_or(0) as i32;
    let sum: i32 = (0..n).map(|i| at(a, i) - at(b, i)).sum();
    (sum % 11).abs() + if domestic { 3 } else { 0 }
}

/// `1010:0f2e` weights for the roulette `1010:100b`: kind 1 strength, 2 `pos^2*strength*g` (scorers),
/// 3 aggression (red cards), 4 injury proneness.
fn weight(p: &Slot, kind: u8) -> u32 {
    match kind {
        1 => p.strength as u32,
        2 => {
            ((p.position as u16 * p.position as u16)
                .wrapping_mul(p.strength)
                .wrapping_mul(p.g as u16)) as u32
        }
        3 => p.aggression as u32,
        4 => p.proneness as u32,
        _ => 0,
    }
}

/// `1010:100b(kind, mode 1, squad)`: the total includes **every** slot, the pick only walks the starters,
/// so a roll that lands in the bench's share selects nobody.
pub fn roulette(rng: &mut DelphiRng, slots: &[Slot], kind: u8) -> Option<usize> {
    let total: u32 = slots
        .iter()
        .map(|p| weight(p, kind))
        .fold(0u16, |a, w| a.wrapping_add(w as u16)) as u32;
    let r = rng.random(total as u16) as u32;
    let mut acc = 0u32;
    for (i, p) in slots.iter().enumerate() {
        if p.starter() {
            acc += weight(p, kind);
            if r < acc {
                return Some(i);
            }
        }
    }
    None
}

/// A primary chance (`1038:39b6`): at `minute` the side `side` shoots at the opponent's best goalkeeper. The shooter is the
/// best attacker unless the user chose one; `outcome` is the shot duel (`81ed`). Goals and misses alike are recorded, because the
/// original shows its penalty dialog for every primary chance of a game with a human coach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Primary {
    pub minute: u16,
    pub side: Side,
    pub shooter: usize,
    pub keeper: usize,
    pub outcome: Outcome,
}

/// A decision of the user while the game runs (what the original asks in dialogs). The decisions never change what happened
/// before their `minute`, so a game can be replayed with them from the same random state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The shooter of the primary chance at `minute`.
    Shooter {
        side: Side,
        minute: u16,
        player: usize,
    },
    /// The substitution after an injury, or the sending-off of the goalkeeper, at `minute`: `out` (`None`: the injured player)
    /// makes way for `inn`.
    Substitute {
        side: Side,
        minute: u16,
        out: Option<usize>,
        inn: usize,
    },
    /// The user closed the dialog of `minute` without substituting (the automatic substitution is skipped).
    NoSubstitution { side: Side, minute: u16 },
    /// A substitution made in the break (`minute` 46, or 91 before extra time): like a planned one, but for this game only.
    Planned {
        side: Side,
        minute: u16,
        out: usize,
        inn: usize,
    },
}

#[derive(Clone, Debug)]
pub struct Match {
    pub home: TeamState,
    pub away: TeamState,
    pub minute: u16,
    pub events: Vec<Event>,
    /// Substitutions made so far (kept apart from `events`: a substitution must not hide the last event of the minute).
    pub subs: Vec<SubEvent>,
    /// Substitutions planned by the user: `(side, minute, out, in)`.
    pub planned: Vec<(Side, u16, usize, usize)>,
    /// Match object `+0x4c` / `+0x4e`: per-match bonus values (`c95c` + 2 for the home side).
    pub home_bonus: i32,
    pub away_bonus: i32,
    /// Every primary chance of the game.
    pub primaries: Vec<Primary>,
    /// What the user decided (see [`Decision`]); empty for a game played without interaction.
    pub decisions: Vec<Decision>,
}

impl Match {
    pub fn new(home: Vec<Slot>, away: Vec<Slot>, home_bonus: i32, away_bonus: i32) -> Self {
        Self {
            home: TeamState::new(home),
            away: TeamState::new(away),
            minute: 0,
            events: Vec::new(),
            subs: Vec::new(),
            planned: Vec::new(),
            home_bonus,
            away_bonus,
            primaries: Vec::new(),
            decisions: Vec::new(),
        }
    }

    pub fn team(&self, side: Side) -> &TeamState {
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
        let (att, def) = (self.team(side), self.team(side.other()));
        let minute = self.minute;
        let chosen = self.decisions.iter().find_map(|d| match *d {
            Decision::Shooter {
                side: s,
                minute: m,
                player,
            } if s == side && m == minute && att.slots.get(player).is_some_and(Slot::starter) => {
                Some(player)
            }
            _ => None,
        });
        let shooter = match chosen {
            Some(p) => p,
            None => sector::best_attacker(&att.slots)?,
        };
        let keeper = sector::best_defender(&def.slots)?;
        let force = att.slots[shooter].attack_force();
        let strength = def.slots[keeper].strength;
        let outcome = outcome::shot(rng, force, strength);
        self.primaries.push(Primary {
            minute,
            side,
            shooter,
            keeper,
            outcome,
        });
        (outcome == Outcome::Goal).then_some(shooter)
    }

    /// Main generator for one side; the scorer comes from the roulette (kind 2) and may be nobody.
    fn secondary_goal(&mut self, rng: &mut DelphiRng, side: Side) -> Option<usize> {
        let own = self.team(side).side_state();
        let opp = self.team(side.other()).sectors;
        if chance::chance(rng, &own, &opp, side == Side::Home, self.minute) {
            roulette(rng, &self.team(side).slots, 2)
        } else {
            None
        }
    }

    /// Penalty after a red card in the previous minute (`a6eb`): goalkeeper always, defender 1 in 3.
    fn penalty_for(&self, rng: &mut DelphiRng, side: Side) -> bool {
        let Some(e) = self.events.last() else {
            return false;
        };
        if e.minute + 1 != self.minute
            || e.kind != EventKind::RedCard
            || e.side != side.other()
            || self.minute % 45 <= 1
        {
            return false;
        }
        e.position == 0 || (e.position == 1 && rng.random(3) == 0)
    }

    /// `1010:a6eb`. Returns true when a goal was scored this minute.
    fn chances(&mut self, rng: &mut DelphiRng) -> bool {
        let m = self.minute;
        let home_pen = self.penalty_for(rng, Side::Home);
        let r6000 = rng.random(6000) as i32;
        let mut home = None;
        if (self.home_bonus >= 0 && r6000 < self.home_bonus) || home_pen {
            home = self.primary_goal(rng, Side::Home).map(|s| (s, true));
        }
        let home_block = home_pen || home.is_some();
        let away_pen = self.penalty_for(rng, Side::Away);
        let r7000 = rng.random(7000) as i32;
        let mut away = None;
        if ((self.away_bonus >= 0 && r7000 < self.away_bonus) || away_pen) && home.is_none() {
            away = self.primary_goal(rng, Side::Away).map(|s| (s, true));
        }
        let away_block = away_pen || away.is_some();
        if !home_block && !away_block {
            home = self.secondary_goal(rng, Side::Home).map(|s| (s, false));
            if home.is_none() {
                away = self.secondary_goal(rng, Side::Away).map(|s| (s, false));
            }
        }
        for (side, hit) in [(Side::Home, home), (Side::Away, away)] {
            if let Some((player, primary)) = hit {
                let position = self.team(side).slots[player].position;
                self.team_mut(side).goals += 1;
                self.team_mut(side).slots[player].goals += 1;
                self.events.push(Event {
                    minute: m,
                    side,
                    kind: EventKind::Goal,
                    player,
                    position,
                    primary,
                });
                return true;
            }
        }
        false
    }

    /// `1010:67f7`: automatic substitution. Mode 0 (injury): first designated substitute with the same
    /// goalkeeper-ness. Mode 1 (red card): only for a sent-off goalkeeper; the last starter makes way.
    fn auto_sub(&mut self, side: Side, out: usize, mode: u8) {
        let minute = self.minute;
        let decision = self.decisions.iter().find_map(|d| match *d {
            Decision::Substitute {
                side: s,
                minute: m,
                out,
                inn,
            } if s == side && m == minute => Some(Some((out, inn))),
            Decision::NoSubstitution { side: s, minute: m } if s == side && m == minute => {
                Some(None)
            }
            _ => None,
        });
        let t = self.team_mut(side);
        if t.subs_left == 0 {
            return;
        }
        let is_gk = t.slots[out].position == 0;
        let (leaving, coming) = match (decision, mode) {
            (Some(None), _) => (None, None),
            (Some(Some((o, inn))), _) => {
                let o = o.unwrap_or(out);
                let ok = o < t.slots.len()
                    && inn < t.slots.len()
                    && (t.slots[o].starter() || o == out)
                    && t.slots[inn].status == Status::Sub
                    && t.slots[inn].available();
                if ok {
                    (Some(o), Some(inn))
                } else {
                    (None, None)
                }
            }
            (None, mode) => Self::automatic(t, out, is_gk, mode),
        };
        if let (Some(l), Some(c)) = (leaving, coming) {
            t.slots[l].status = Status::Out;
            t.slots[c].status = Status::Starter;
            t.subs_left -= 1;
            t.recompute();
            self.subs.push(SubEvent {
                minute,
                side,
                out: l,
                inn: c,
                auto: decision.is_none(),
            });
        }
    }

    /// The choice of `67f7` when nobody decides.
    fn automatic(
        t: &TeamState,
        out: usize,
        is_gk: bool,
        mode: u8,
    ) -> (Option<usize>, Option<usize>) {
        match mode {
            0 => (
                Some(out),
                t.slots
                    .iter()
                    .position(|s| s.status == Status::Sub && (s.position == 0) == is_gk),
            ),
            _ if is_gk => {
                let sub = t
                    .slots
                    .iter()
                    .position(|s| s.status == Status::Sub && s.position == 0);
                let last_starter = (1..t.slots.len()).rev().find(|&i| t.slots[i].starter());
                (last_starter, sub)
            }
            _ => (None, None),
        }
    }

    /// A user-planned substitution at the current minute: the player must be a starter, the replacement a fit
    /// substitute, and the team must have substitutions left. Invalid plans are ignored.
    fn planned_sub(&mut self, side: Side, out: usize, inn: usize) {
        let minute = self.minute;
        let t = self.team_mut(side);
        let valid = t.subs_left > 0
            && out < t.slots.len()
            && inn < t.slots.len()
            && t.slots[out].starter()
            && t.slots[inn].status == Status::Sub
            && t.slots[inn].available()
            // a goalkeeper may only be replaced by a goalkeeper (and the reverse), as in the automatic rule
            && (t.slots[out].position == 0) == (t.slots[inn].position == 0);
        if valid {
            t.slots[out].status = Status::Out;
            t.slots[inn].status = Status::Starter;
            t.subs_left -= 1;
            t.recompute();
            self.subs.push(SubEvent {
                minute,
                side,
                out,
                inn,
                auto: false,
            });
        }
    }

    /// `1010:aa11` (red card). Returns true when a player was sent off.
    fn red_card(&mut self, rng: &mut DelphiRng) -> bool {
        let r = rng.random(1000) as i32;
        let side = if r < 10 && self.home_bonus < r {
            Side::Home
        } else if r < 10 && self.away_bonus < r {
            Side::Away
        } else {
            return false;
        };
        let Some(i) = roulette(rng, &self.team(side).slots, 3) else {
            return false;
        };
        if self.team(side).slots[i].aggression < 1 {
            return false;
        }
        let ban = rng.random(4) as u8 + 1;
        let position = {
            let p = &mut self.team_mut(side).slots[i];
            p.status = Status::Out;
            p.ban = ban;
            p.position
        };
        self.events.push(Event {
            minute: self.minute,
            side,
            kind: EventKind::RedCard,
            player: i,
            position,
            primary: false,
        });
        self.auto_sub(side, i, 1);
        self.home.recompute();
        self.away.recompute();
        true
    }

    /// `1010:ac72` (injury). Returns true when a player was injured.
    fn injury(&mut self, rng: &mut DelphiRng) -> bool {
        let side = match rng.random(500) {
            0 => Side::Home,
            1 => Side::Away,
            _ => return false,
        };
        let Some(i) = roulette(rng, &self.team(side).slots, 4) else {
            return false;
        };
        if self.team(side).slots[i].proneness < 1 {
            return false;
        }
        let upper = rng.random(20);
        let games = rng.random(upper) as u8 + 1;
        let position = {
            let p = &mut self.team_mut(side).slots[i];
            p.status = Status::Out;
            p.injured = games;
            p.strength = p.strength.saturating_sub(10).max(1);
            p.position
        };
        self.events.push(Event {
            minute: self.minute,
            side,
            kind: EventKind::Injury,
            player: i,
            position,
            primary: false,
        });
        self.auto_sub(side, i, 0);
        self.home.recompute();
        self.away.recompute();
        true
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
                t.slots[last.player].goals = t.slots[last.player].goals.saturating_sub(1);
                self.events.last_mut().unwrap().kind = EventKind::Annulled;
            }
        }
    }

    /// `1010:afff` -> `49e2` for both teams: one random squad member per team changes its season rating
    /// (a starter loses 1, anyone else gains 1). Home first, then away; always runs, goal or not.
    fn tick_ratings(&mut self, rng: &mut DelphiRng) {
        for side in [Side::Home, Side::Away] {
            let t = self.team_mut(side);
            let i = rng.random(t.slots.len() as u16) as usize;
            if let Some(p) = t.slots.get_mut(i) {
                p.rating += if p.starter() { -1 } else { 1 };
            }
        }
    }

    /// One minute of play (`1048:069b` body for a single game).
    pub fn step(&mut self, rng: &mut DelphiRng) {
        self.minute += 1;
        for (side, minute, out, inn) in self.planned.clone() {
            if minute == self.minute {
                self.planned_sub(side, out, inn);
            }
        }
        for d in self.decisions.clone() {
            if let Decision::Planned {
                side,
                minute,
                out,
                inn,
            } = d
            {
                if minute == self.minute {
                    self.planned_sub(side, out, inn);
                }
            }
        }
        let happened = self.chances(rng) || self.red_card(rng) || self.injury(rng);
        if !happened && self.minute != 46 && self.minute != 91 {
            self.annul(rng);
        }
        self.tick_ratings(rng);
    }

    pub fn play(&mut self, rng: &mut DelphiRng) {
        while self.minute < 90 {
            self.step(rng);
        }
    }

    /// Extra time (`1048:069b`): the clock goes on from 90 to 120 for a drawn cup game; minute 91 skips the annulment check
    /// like minute 46, as in the normal loop.
    pub fn play_extra_time(&mut self, rng: &mut DelphiRng) {
        while self.minute < 120 {
            self.step(rng);
        }
    }

    pub fn score(&self) -> (i32, i32) {
        (self.home.goals, self.away.goals)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    fn name(rng: &mut DelphiRng) -> String {
        (0..5 + rng.random(8))
            .map(|_| (b'a' + rng.random(26) as u8) as char)
            .collect()
    }

    /// 4-4-2 starters with `strength`, plus a bench of 1 GK + 4 outfield players.
    pub fn squad(strength: u16, rng: &mut DelphiRng) -> Vec<Slot> {
        let mut v = Vec::new();
        for &pos in &[0u8, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3] {
            v.push(Slot::new(&name(rng), pos, strength, true, Status::Starter));
        }
        for &pos in &[0u8, 1, 2, 3, 3] {
            v.push(Slot::new(
                &name(rng),
                pos,
                strength.saturating_sub(1).max(1),
                true,
                Status::Sub,
            ));
        }
        v
    }

    /// Bonus values through the real `c95c` formula with random strings (domestic league match).
    fn approx_bonus(rng: &mut DelphiRng) -> (i32, i32) {
        let a = name(rng);
        let (h, v) = (name(rng), name(rng));
        (
            fixture_value(a.as_bytes(), h.as_bytes(), true) + 2,
            fixture_value(a.as_bytes(), v.as_bytes(), true),
        )
    }

    fn run(n: u32, hs: u16, as_: u16, seed: u32) -> (f64, f64, f64, f64, f64, f64) {
        let mut rng = DelphiRng::new(seed);
        let (mut hg, mut ag, mut hw, mut aw, mut reds, mut inj) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for _ in 0..n {
            let (hb, ab) = approx_bonus(&mut rng);
            let mut m = Match::new(squad(hs, &mut rng), squad(as_, &mut rng), hb, ab);
            m.play(&mut rng);
            let (x, y) = m.score();
            hg += x as f64;
            ag += y as f64;
            hw += (x > y) as u32 as f64;
            aw += (y > x) as u32 as f64;
            reds += m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::RedCard)
                .count() as f64;
            inj += m
                .events
                .iter()
                .filter(|e| e.kind == EventKind::Injury)
                .count() as f64;
        }
        let n = n as f64;
        (hg / n, ag / n, hw / n, aw / n, reds / n, inj / n)
    }

    #[test]
    fn fixture_value_matches_c95c() {
        // identical strings: sum 0 -> 0, domestic adds 3
        assert_eq!(fixture_value(b"abc", b"abc", false), 0);
        assert_eq!(fixture_value(b"abc", b"abc", true), 3);
        // sum = (b-a)+(c-b)+(d-c) = 3 over equal length strings
        assert_eq!(fixture_value(b"bcd", b"abc", false), 3);
        // a longer second string subtracts its extra bytes: -(10 + 'a'(97)) mod 11 -> |-107 % 11| = |-8| = 8
        assert_eq!(fixture_value(b"", b"\x0aa", false), 8);
        // always 0..=10 (+3)
        for i in 0..200u8 {
            let v = fixture_value(&[i, i.wrapping_mul(7)], &[i.wrapping_mul(3)], i % 2 == 0);
            assert!((0..=13).contains(&v));
        }
    }

    #[test]
    fn goals_per_game_are_in_a_football_range() {
        let (h, a, ..) = run(20_000, 6, 6, 1);
        assert!(
            (0.6..2.5).contains(&h) && (0.4..2.2).contains(&a),
            "home {h:.2} away {a:.2}"
        );
    }

    #[test]
    fn home_side_has_an_advantage() {
        let (h, a, hw, aw, ..) = run(20_000, 6, 6, 2);
        assert!(h > a && hw > aw, "{h:.2}-{a:.2} wins {hw:.2}/{aw:.2}");
    }

    #[test]
    fn stronger_team_wins_more() {
        let (_, _, hw, aw, ..) = run(20_000, 9, 4, 3);
        assert!(hw > aw + 0.2, "{hw:.2} vs {aw:.2}");
        let (_, _, hw2, aw2, ..) = run(20_000, 4, 9, 4);
        assert!(aw2 > hw2, "{hw2:.2} vs {aw2:.2}");
    }

    #[test]
    fn cards_and_injuries_happen_at_plausible_rates() {
        let (.., reds, inj) = run(20_000, 6, 6, 5);
        assert!((0.1..1.5).contains(&reds), "reds/game {reds:.3}");
        assert!((0.05..0.6).contains(&inj), "injuries/game {inj:.3}");
    }

    #[test]
    fn goals_equal_events_minus_annulled() {
        let mut rng = DelphiRng::new(6);
        let mut annulled = 0;
        for _ in 0..5_000 {
            let (hb, ab) = approx_bonus(&mut rng);
            let mut m = Match::new(squad(8, &mut rng), squad(8, &mut rng), hb, ab);
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

    #[test]
    fn sent_off_player_leaves_and_goalkeeper_is_replaced() {
        let mut rng = DelphiRng::new(7);
        let mut m = Match::new(squad(6, &mut rng), squad(6, &mut rng), 2, 0);
        let gk = m
            .home
            .slots
            .iter()
            .position(|s| s.starter() && s.position == 0)
            .unwrap();
        m.home.slots[gk].status = Status::Out; // as aa11 does
        m.auto_sub(Side::Home, gk, 1);
        assert!(
            m.home
                .slots
                .iter()
                .filter(|s| s.starter() && s.position == 0)
                .count()
                == 1
        );
        assert_eq!(m.home.subs_left, 2);
        // the last starter made way, so the team now plays with 10
        assert_eq!(m.home.slots.iter().filter(|s| s.starter()).count(), 10);
    }

    #[test]
    fn injury_substitute_has_the_same_kind() {
        let mut rng = DelphiRng::new(8);
        let mut m = Match::new(squad(6, &mut rng), squad(6, &mut rng), 2, 0);
        let fw = m
            .home
            .slots
            .iter()
            .position(|s| s.starter() && s.position == 3)
            .unwrap();
        m.home.slots[fw].status = Status::Out;
        m.auto_sub(Side::Home, fw, 0);
        assert_eq!(m.home.slots.iter().filter(|s| s.starter()).count(), 11);
        assert_eq!(
            m.home
                .slots
                .iter()
                .filter(|s| s.starter() && s.position == 0)
                .count(),
            1
        );
    }

    #[test]
    fn season_ratings_make_starters_decline_and_reserves_improve() {
        use crate::evolution::{evolve_squad, level_for_rank};
        let mut rng = DelphiRng::new(10);
        let mut team = squad(8, &mut rng);
        let before_starters: i32 = team
            .iter()
            .filter(|p| p.starter())
            .map(|p| p.strength as i32)
            .sum();
        let before_bench: i32 = team
            .iter()
            .filter(|p| !p.starter())
            .map(|p| p.strength as i32)
            .sum();
        for _ in 0..38 {
            let mut m = Match::new(team.clone(), squad(8, &mut rng), 8, 6);
            m.play(&mut rng);
            for (slot, played) in team.iter_mut().zip(m.home.slots.iter()) {
                slot.rating = played.rating; // injuries/cards of that match are not carried over
            }
        }
        assert!(team.iter().filter(|p| p.starter()).all(|p| p.rating < 0));
        assert!(team.iter().filter(|p| !p.starter()).all(|p| p.rating > 0));
        evolve_squad(&mut rng, &mut team, level_for_rank(8, 0));
        let after_starters: i32 = team
            .iter()
            .filter(|p| p.starter())
            .map(|p| p.strength as i32)
            .sum();
        let after_bench: i32 = team
            .iter()
            .filter(|p| !p.starter())
            .map(|p| p.strength as i32)
            .sum();
        assert!(
            after_starters < before_starters,
            "{before_starters} -> {after_starters}"
        );
        assert!(
            after_bench > before_bench,
            "{before_bench} -> {after_bench}"
        );
    }

    #[test]
    fn morale_is_a_moving_average_of_results() {
        assert_eq!(morale_results(2, 0), (2.0, 0.0));
        assert_eq!(morale_results(0, 1), (0.0, 2.5));
        assert_eq!(morale_results(1, 1), (0.8, 1.3));
        // one home win from the initial 1.0: 0.85 + 0.15 * 2.0
        assert!((update_morale(1.0, 2.0) - 1.15).abs() < 1e-12);
        // a long unbeaten home run converges to 2.0, a losing run to 0
        let (mut up, mut down) = (1.0, 1.0);
        for _ in 0..200 {
            up = update_morale(up, 2.0);
            down = update_morale(down, 0.0);
        }
        assert!((up - 2.0).abs() < 1e-6 && down < 1e-6);
    }

    #[test]
    fn bench_weight_dilutes_the_scorer_roulette() {
        // same starters, but a bench full of strong attackers: the roulette lands on the bench more often
        let mut rng = DelphiRng::new(9);
        let mut lean = squad(6, &mut rng);
        let mut stacked = lean.clone();
        for p in stacked.iter_mut().filter(|p| p.status == Status::Sub) {
            p.strength = 30;
            p.position = 3;
            p.g = 10;
        }
        for p in lean.iter_mut().filter(|p| p.status == Status::Sub) {
            p.strength = 1;
        }
        let nobody = |slots: &[Slot], rng: &mut DelphiRng| {
            (0..50_000)
                .filter(|_| roulette(rng, slots, 2).is_none())
                .count()
        };
        assert!(nobody(&stacked, &mut rng) > 5 * nobody(&lean, &mut rng).max(1));
    }

    #[test]
    fn planned_substitutions_run_at_their_minute_and_invalid_ones_are_ignored() {
        let mut rng = DelphiRng::new(13);
        let mut m = Match::new(squad(6, &mut rng), squad(6, &mut rng), 8, 6);
        let starter = m
            .home
            .slots
            .iter()
            .position(|s| s.starter() && s.position == 3)
            .unwrap();
        let bench = m
            .home
            .slots
            .iter()
            .position(|s| s.status == Status::Sub && s.position == 3)
            .unwrap();
        let keeper = m
            .home
            .slots
            .iter()
            .position(|s| s.starter() && s.position == 0)
            .unwrap();
        m.planned = vec![
            (Side::Home, 30, starter, bench),
            (Side::Home, 40, keeper, bench), // bench is already on the pitch by then: ignored
            (Side::Home, 50, 999, bench),    // bogus index: ignored
            (Side::Away, 60, starter, bench), // the away squad has its own numbering but this pair is also legal
        ];
        m.play(&mut rng);
        let planned: Vec<&SubEvent> = m.subs.iter().filter(|s| !s.auto).collect();
        assert!(planned
            .iter()
            .any(|s| s.side == Side::Home && s.minute == 30 && s.out == starter && s.inn == bench));
        assert!(!planned.iter().any(|s| s.minute == 40 || s.minute == 50));
        assert!(m.home.slots[bench].starter() || m.home.slots[bench].status == Status::Out);
        assert!(m.home.subs_left <= 2);
    }

    /// Plays the same game again with the user's `decisions`: everything before their minute stays as it was.
    fn replay(seed: u32, decisions: &[Decision]) -> Match {
        let mut rng = DelphiRng::new(seed);
        let mut m = Match::new(squad(6, &mut rng), squad(6, &mut rng), 8, 6);
        m.decisions = decisions.to_vec();
        m.play(&mut rng);
        m
    }

    /// The squads of `replay(seed, ..)` before kick-off.
    fn start(seed: u32) -> Vec<Slot> {
        let mut rng = DelphiRng::new(seed);
        squad(6, &mut rng)
    }

    #[test]
    fn an_injured_player_is_replaced_by_the_one_the_user_chooses() {
        let mut checked = 0;
        for seed in 1..4000 {
            let base = replay(seed, &[]);
            let Some(inj) = base
                .events
                .iter()
                .find(|e| e.kind == EventKind::Injury && e.side == Side::Home)
            else {
                continue;
            };
            let Some(auto) = base
                .subs
                .iter()
                .find(|s| s.side == Side::Home && s.minute == inj.minute)
            else {
                continue;
            };
            // another fit substitute of the same kind than the automatic pick
            let squad = start(seed);
            let Some(other) = (0..squad.len()).find(|&i| {
                i != auto.inn
                    && squad[i].status == Status::Sub
                    && squad[i].position == squad[inj.player].position
                    && base.subs.iter().all(|s| s.inn != i)
            }) else {
                continue;
            };
            let d = Decision::Substitute {
                side: Side::Home,
                minute: inj.minute,
                out: None,
                inn: other,
            };
            let m = replay(seed, &[d]);
            let sub = m
                .subs
                .iter()
                .find(|s| s.side == Side::Home && s.minute == inj.minute)
                .unwrap();
            assert_eq!((sub.out, sub.inn, sub.auto), (inj.player, other, false));
            // nothing before the substitution changed
            let before = |m: &Match| {
                m.events
                    .iter()
                    .filter(|e| e.minute <= inj.minute)
                    .map(|e| format!("{e:?}"))
                    .collect::<Vec<_>>()
            };
            assert_eq!(before(&m), before(&base));
            // declining leaves the team a man down
            let none = replay(
                seed,
                &[Decision::NoSubstitution {
                    side: Side::Home,
                    minute: inj.minute,
                }],
            );
            assert!(!none
                .subs
                .iter()
                .any(|s| s.side == Side::Home && s.minute == inj.minute));
            checked += 1;
            if checked == 5 {
                break;
            }
        }
        assert!(checked > 0, "no injury with a spare substitute found");
    }

    #[test]
    fn the_user_picks_the_shooter_of_a_primary_chance() {
        let mut checked = 0;
        for seed in 1..3000 {
            let base = replay(seed, &[]);
            let Some(p) = base.primaries.iter().find(|p| p.side == Side::Home) else {
                continue;
            };
            let start = start(seed);
            // another starter who is still on the pitch at that minute
            let Some(other) = (0..start.len()).find(|&i| {
                i != p.shooter
                    && start[i].starter()
                    && start[i].position == 2
                    && base
                        .events
                        .iter()
                        .all(|e| e.player != i || e.side != Side::Home || e.minute >= p.minute)
            }) else {
                continue;
            };
            let m = replay(
                seed,
                &[Decision::Shooter {
                    side: Side::Home,
                    minute: p.minute,
                    player: other,
                }],
            );
            let q = m
                .primaries
                .iter()
                .find(|q| q.side == Side::Home && q.minute == p.minute)
                .unwrap();
            assert_eq!(q.shooter, other);
            // the chances before it are the same
            let before = |m: &Match| {
                m.primaries
                    .iter()
                    .filter(|q| q.minute < p.minute)
                    .copied()
                    .collect::<Vec<_>>()
            };
            assert_eq!(before(&m), before(&base));
            checked += 1;
            if checked == 20 {
                break;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn decisions_that_change_nothing_leave_the_game_untouched() {
        for seed in 1..60 {
            let base = replay(seed, &[]);
            let noop = replay(
                seed,
                &[
                    Decision::Shooter {
                        side: Side::Away,
                        minute: 200,
                        player: 3,
                    },
                    Decision::NoSubstitution {
                        side: Side::Home,
                        minute: 200,
                    },
                ],
            );
            assert_eq!(format!("{:?}", base.events), format!("{:?}", noop.events));
            assert_eq!(base.score(), noop.score());
            assert_eq!(format!("{:?}", base.subs), format!("{:?}", noop.subs));
        }
    }
}
