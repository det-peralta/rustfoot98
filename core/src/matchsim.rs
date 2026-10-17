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
        let shooter = sector::best_attacker(&att.slots)?;
        let keeper = sector::best_defender(&def.slots)?;
        let force = att.slots[shooter].attack_force();
        let strength = def.slots[keeper].strength;
        (outcome::shot(rng, force, strength) == Outcome::Goal).then_some(shooter)
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
        let t = self.team_mut(side);
        if t.subs_left == 0 {
            return;
        }
        let is_gk = t.slots[out].position == 0;
        let (leaving, coming) = match mode {
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
        };
        if let (Some(l), Some(c)) = (leaving, coming) {
            t.slots[l].status = Status::Out;
            t.slots[c].status = Status::Starter;
            t.subs_left -= 1;
            t.recompute();
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

    /// `1020:038e`: after a processed game the user's team gains +1.0 morale (`K`) and its opponent +0.5.
    /// (`K` resets to 1.0 when a coach takes over, `1010:4f40`.)
    pub fn apply_morale(user: &mut TeamState, opponent: &mut TeamState) {
        user.k += 1.0;
        opponent.k += 0.5;
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
    fn morale_grows_after_games() {
        let mut rng = DelphiRng::new(11);
        let (mut a, mut b) = (
            TeamState::new(squad(6, &mut rng)),
            TeamState::new(squad(6, &mut rng)),
        );
        Match::apply_morale(&mut a, &mut b);
        Match::apply_morale(&mut a, &mut b);
        assert_eq!((a.k, b.k), (3.0, 2.0));
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
}
