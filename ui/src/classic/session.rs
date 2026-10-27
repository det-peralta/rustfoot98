//! The flow of a match day: `Jogar` plays the round, the round view replays it with the pie clock, the user is asked at the
//! moments the original asks (the break, an injury, a sent-off goalkeeper, a primary chance), and the dialogs of the end of a round
//! follow in the original's order (coach invitation, sackings, classification, cup draw and cup round, shootouts, end of season,
//! auctions).
//!
//! The core computes a whole round in one go. To let the user act at a minute of his own game, the round is played on a copy of the
//! world, the view shows it, and whenever the user decides something the round is played again from the copy with the decision added
//! (`Control::replay`): the random state of every game is restored at the same checkpoints, so only the user's own game changes, and
//! only after the minute of the decision. A round nobody touches is exactly the round `World::play_round` plays.
use super::dialog::{Dialog, Kind};
use super::round_view::{self, RoundView};
use super::{dialogs, main_win, Classic, Screen};
use rustfoot_core::matchsim::{Decision, Event, EventKind, Side, SubEvent};
use rustfoot_core::rng::DelphiRng;
use rustfoot_core::season::{Club, Control};
use rustfoot_core::sector::{Slot, Status};
use rustfoot_core::world::World;
use std::collections::{HashMap, VecDeque};

/// What happens next at the end of a round, once the dialogs open are closed.
pub enum Step {
    Open(Box<Dialog>),
    /// The draw of the cup (if the options say so).
    CupDraw,
    /// The league day is over and a cup round has been drawn: the main window comes back for the humans who play a tie (the
    /// cup is played on its own day when they press *Jogar*); when no human plays, the cup day follows at once.
    CupDay,
    /// The invitations to other clubs that humans have not answered yet, one dialog at a time (a refusal replays the round, so
    /// what is still to come is looked up when its turn comes).
    Invitations,
    Auctions,
    Main,
}

/// The round being shown, with what is needed to play it again with the user's decisions.
pub struct Session {
    /// A cup day (the session of the ties, not of a league round).
    pub cup_day: bool,
    /// The world before the round.
    pub snapshot: Box<World>,
    /// The random state at the checkpoints of the first run.
    pub log: Vec<DelphiRng>,
    /// The decisions of the humans in their league games and cup ties, each with the home club of the game.
    pub league: Vec<(String, Decision)>,
    pub cup: Vec<(String, Decision)>,
    /// Things already asked or shown: `(cup, game, minute, kind)`; the side of a question is in the high bits of `kind`.
    pub handled: Vec<(bool, usize, u16, u8)>,
    /// The referee of each human's league game, chosen before the round (as the main window showed it), by his club.
    pub referees: HashMap<String, String>,
    /// Clubs of the humans who declined the invitation they got (see `World::decline_invitation`).
    pub declined: Vec<String>,
    /// Clubs of the humans whose invitation was answered (yes or no).
    pub answered: Vec<String>,
    /// The user's club and the coaches of all clubs before the round (names for the sacking list).
    pub coaches: HashMap<String, String>,
}

#[derive(Default)]
pub struct State {
    /// The check boxes of the break dialog: list the red cards and the injuries.
    pub view_red: bool,
    pub view_injuries: bool,
    pub session: Option<Session>,
    /// The `A gravar o jogo...` gauge of the save after a round is open (the save is requested when it is full).
    pub saving: bool,
    /// The seat that took the first turn of the day (hot seat).
    pub first_turn: usize,
    pub todo: VecDeque<Step>,
    /// Titles won in this run of the program: `club -> (league, cup)` (the end-of-season window counts them).
    pub titles: HashMap<String, (u16, u16)>,
    pub coach_titles: HashMap<String, (u16, u16)>,
}

/// A game on the screen as the dialogs need it: names, squads at the kick-off, events and substitutions.
pub struct Info<'a> {
    pub clubs: [String; 2],
    pub names: [&'a [String]; 2],
    pub start: [&'a [Slot]; 2],
    pub events: &'a [Event],
    pub subs: &'a [SubEvent],
    pub bonus: (i32, i32),
}

pub fn info(world: &World, cup: bool, src: usize) -> Option<Info<'_>> {
    if cup {
        let m = world.last_cup.as_ref()?.matches.get(src)?;
        Some(Info {
            clubs: [m.home.clone(), m.away.clone()],
            names: [&m.home_players, &m.away_players],
            start: [&m.start[0], &m.start[1]],
            events: &m.events,
            subs: &m.subs,
            bonus: m.bonus,
        })
    } else {
        let (d, m) = world.last_round.get(src)?;
        Some(Info {
            clubs: [
                world.divisions[*d][m.home].name.clone(),
                world.divisions[*d][m.away].name.clone(),
            ],
            names: [&m.names[0], &m.names[1]],
            start: [&m.start[0], &m.start[1]],
            events: &m.events,
            subs: &m.subs,
            bonus: m.bonus,
        })
    }
}

pub fn find_club<'a>(world: &'a World, name: &str) -> Option<&'a Club> {
    world
        .divisions
        .iter()
        .flatten()
        .chain(world.pool.iter())
        .find(|c| c.name == name)
}

impl Info<'_> {
    /// Substitutions the club of `side` can still make at `minute` (before the user's own decisions): `min(3, fit bench)` at the
    /// kick-off minus the ones made.
    pub fn subs_left(&self, side: usize, minute: u16, with_auto: bool) -> usize {
        let bench = self.start[side]
            .iter()
            .filter(|s| s.status == Status::Sub && s.available())
            .count()
            .min(3);
        let made = self
            .subs
            .iter()
            .filter(|s| {
                (s.side == Side::Away) as usize == side
                    && (s.minute < minute || (s.minute == minute && (with_auto || !s.auto)))
            })
            .count();
        bench.saturating_sub(made)
    }
}

/// The kinds of things the original asks while a game runs.
const PRIMARY: u8 = 2;
const RED: u8 = 4;
const INJURY: u8 = 5;

/// What a tick of the round view came to.
enum Tick {
    Run,
    /// Dialogs to show one after the other (the break of a game with two humans asks both).
    Ask(Vec<Dialog>),
    Over,
}

/// What stops the clock at a minute of the user's game.
enum Pending {
    /// A primary chance: the human's side `side` attacks (`true`: he picks the shooter) or defends.
    Primary {
        attack: bool,
        minute: u16,
        side: usize,
    },
    /// The goalkeeper of the human's side `side` was sent off.
    RedKeeper { minute: u16, side: usize },
    /// A player of the human's side `side` was injured.
    Injury {
        minute: u16,
        side: usize,
        player: usize,
    },
}

impl Classic {
    fn referee_of_next_game(&self) -> String {
        let Some((d, j, home)) = main_win::next_opponent(self) else {
            return String::new();
        };
        let Some((_, me)) = main_win::human_club(self) else {
            return String::new();
        };
        let opp = self.world.club_of((d, j));
        if home {
            main_win::referee(self, &me, opp)
        } else {
            main_win::referee(self, opp, &me)
        }
    }

    /// The referee of the next league game of every human, by his club (as the main window of each shows it).
    fn referees_of_next_games(&mut self) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let back = self.world.active_seat();
        for seat in self.world.employed_seats() {
            self.world.switch_human(seat);
            if let Some(h) = self.world.human.as_ref() {
                out.insert(h.club.clone(), self.referee_of_next_game());
            }
        }
        self.world.switch_human(back);
        out
    }

    /// The humans who take a turn on the main window before the next day is played, in the original's order (`World::turn_order`:
    /// the day's games one after the other, the home club first; on a cup day only the humans who play a tie, `1010:3e9c` opens the
    /// main window only for the games of the day).
    fn turn_seats(&self) -> Vec<usize> {
        self.world.turn_order()
    }

    /// The round button (hot seat): the main window goes to the next human of the day's order; when every human has pressed it
    /// since the last round, the day is played. On a cup day it plays the cup day.
    pub fn round_button(&mut self) {
        self.turns += 1;
        let order = self.turn_seats();
        if self.turns < order.len() {
            self.main.menu = None;
            // the human who has the window takes the first turn (the first day starts with the first player), the others follow
            // in the day's order
            let first = if self.turns == 1 {
                self.world.active_seat()
            } else {
                self.md.first_turn
            };
            self.md.first_turn = first;
            let seq: Vec<usize> = std::iter::once(first)
                .chain(order.iter().copied().filter(|&s| s != first))
                .collect();
            if let Some(&next) = seq.get(self.turns) {
                self.switch_human(next);
            }
            return;
        }
        self.turns = 0;
        self.play_round();
    }

    /// The coaches of all clubs now (names for the sacking list).
    fn coach_names(&self) -> HashMap<String, String> {
        self.world
            .divisions
            .iter()
            .flatten()
            .chain(self.world.pool.iter())
            .map(|c| (c.name.clone(), c.coach_name.clone()))
            .collect()
    }

    /// *Jogar*: plays the next match day and shows it: the cup day when a cup round has been drawn, otherwise the next league round
    /// (the cup of an even round is drawn at its end and played on its own day).
    pub fn play_round(&mut self) {
        // the game is saved when the day starts as the options say (the original shows the gauge `A gravar o jogo...` on the bare
        // desktop right after *Jogar*, then the round view); the day is played when the gauge is full
        if !self.md.saving
            && self.started
            && !self.game_name.is_empty()
            && self.opts.saves_after(self.world.round)
        {
            self.md.saving = true;
            self.main.menu = None;
            self.screen = Screen::Round(Box::new(RoundView::backdrop()));
            self.open(dialogs::newgame::open_progress("save"));
            self.dirty = true;
            return;
        }
        self.md.saving = false;
        if self.world.cup_day_due() {
            self.play_cup_day();
        } else {
            self.play_league_day();
        }
    }

    fn play_league_day(&mut self) {
        self.main.menu = None;
        self.md.todo.clear();
        let referees = self.referees_of_next_games();
        self.world.accept_invitations = true;
        self.world.seats.declined.clear();
        let snapshot = Box::new(self.world.clone());
        let coaches = self.coach_names();
        let mut ctl = Control::default();
        self.world.play_round_ctl(&mut ctl);
        self.md.session = Some(Session {
            cup_day: false,
            snapshot,
            log: std::mem::take(&mut ctl.log),
            league: Vec::new(),
            cup: Vec::new(),
            handled: Vec::new(),
            referees,
            declined: Vec::new(),
            answered: Vec::new(),
            coaches,
        });
        self.start_round_view();
    }

    /// Plays the cup day: the ties drawn at the end of the league day, with the humans' plans as they are now.
    pub fn play_cup_day(&mut self) {
        self.main.menu = None;
        self.md.todo.clear();
        self.world.seats.declined.clear();
        self.world.real_shootouts = self.opts.view_computer_penalties;
        let snapshot = Box::new(self.world.clone());
        let coaches = self.coach_names();
        let mut ctl = Control::default();
        self.world.play_cup_day_ctl(&mut ctl);
        self.md.session = Some(Session {
            cup_day: true,
            snapshot,
            log: std::mem::take(&mut ctl.log),
            league: Vec::new(),
            cup: Vec::new(),
            handled: Vec::new(),
            referees: HashMap::new(),
            declined: Vec::new(),
            answered: Vec::new(),
            coaches,
        });
        self.start_cup_view();
    }

    /// Shows the cup round just played (`Taça - <stage> - <year>`); without any tie the day is over.
    pub fn start_cup_view(&mut self) {
        match RoundView::of_cup(&self.world) {
            Some(mut v) => {
                v.interval = round_view::interval(self.opts.profile as usize + 1, v.humans());
                self.screen = Screen::Round(Box::new(v));
            }
            None => {
                self.screen = Screen::Round(Box::new(RoundView::backdrop()));
                dialogs::matchday::end_of_cup(self);
            }
        }
        self.dirty = true;
    }

    /// Shows the league round just played: the clock runs from -5 to 90.
    pub fn start_round_view(&mut self) {
        let mut v = RoundView::of_round(&self.world);
        // (after round 14 the world is already in the next season)
        if let Some(s) = &self.md.session {
            v.title = format!("{}ª Jornada - {}", s.snapshot.round + 1, s.snapshot.year);
        }
        v.interval = round_view::interval(self.opts.profile as usize + 1, v.humans());
        self.screen = Screen::Round(Box::new(v));
        self.dirty = true;
    }

    /// Plays the round again from the world before it with the user's decisions and shows the result from the same minute.
    pub fn replay_round(&mut self, league: Vec<(String, Decision)>, cup: Vec<(String, Decision)>) {
        let Some(sess) = self.md.session.as_mut() else {
            return;
        };
        sess.league = league;
        sess.cup = cup;
        let mut w = (*sess.snapshot).clone();
        w.accept_invitations = true;
        w.real_shootouts = self.opts.view_computer_penalties;
        w.seats.declined = sess.declined.clone();
        let mut ctl = Control::replay(sess.log.clone(), sess.league.clone(), sess.cup.clone());
        if sess.cup_day {
            w.play_cup_day_ctl(&mut ctl);
        } else {
            w.play_round_ctl(&mut ctl);
        }
        self.world = w;
        // the view of the same round with what the replay produced, at the minute it was
        if let Screen::Round(rv) = &self.screen {
            if rv.backdrop {
                return;
            }
            let old = rv.clone();
            let fresh = if old.cup {
                RoundView::of_cup(&self.world)
            } else {
                Some(RoundView::of_round(&self.world))
            };
            if let Some(mut v) = fresh {
                v.title = old.title.clone();
                v.page = old.page;
                v.minute = old.minute;
                v.acc_ms = old.acc_ms;
                v.interval = old.interval;
                v.max = old.max;
                v.halftime_seen = old.halftime_seen;
                v.extra_seen = old.extra_seen;
                v.cut = old.cut;
                v.checked = old.checked;
                self.screen = Screen::Round(Box::new(v));
            }
        }
        self.dirty = true;
    }

    /// A human decided something at a minute of his game `src` (`cup`: the cup tie, otherwise the league game).
    pub fn decide(&mut self, cup: bool, src: usize, ds: Vec<Decision>) {
        let Some(sess) = self.md.session.as_ref() else {
            return;
        };
        let Some(home) = info(&self.world, cup, src).map(|i| i.clubs[0].clone()) else {
            return;
        };
        let (mut league, mut cups) = (sess.league.clone(), sess.cup.clone());
        let list = if cup { &mut cups } else { &mut league };
        list.extend(ds.into_iter().map(|d| (home.clone(), d)));
        self.replay_round(league, cups);
    }

    fn mark(&mut self, cup: bool, src: usize, minute: u16, kind: u8) {
        if let Some(s) = self.md.session.as_mut() {
            s.handled.push((cup, src, minute, kind));
        }
    }

    fn was_asked(&self, cup: bool, src: usize, minute: u16, kind: u8) -> bool {
        self.md
            .session
            .as_ref()
            .is_some_and(|s| s.handled.contains(&(cup, src, minute, kind)))
    }

    /// The first thing the original would ask at `minute` of the games of the humans that was not asked yet (the engine's order:
    /// primary chance, red card, injury), game after game.
    fn pending_at(&self, rv: &RoundView, minute: u16) -> Option<(usize, Pending)> {
        for g in rv.shown().filter(|g| g.human) {
            let Some(info) = info(&self.world, rv.cup, g.src) else {
                continue;
            };
            let prim = if rv.cup {
                self.world
                    .last_cup
                    .as_ref()?
                    .matches
                    .get(g.src)?
                    .primaries
                    .clone()
            } else {
                self.world.last_round.get(g.src)?.1.primaries.clone()
            };
            // primary chances: the attacker decides when he is a human, otherwise the defender is told
            for p in prim.iter().filter(|p| p.minute == minute) {
                let att = (p.side == Side::Away) as usize;
                let me = if g.human_sides[att] { att } else { 1 - att };
                if !self.was_asked(rv.cup, g.src, minute, PRIMARY) {
                    return Some((
                        g.src,
                        Pending::Primary {
                            attack: me == att,
                            minute,
                            side: me,
                        },
                    ));
                }
            }
            for e in info.events.iter().filter(|e| e.minute == minute) {
                let side = (e.side == Side::Away) as usize;
                if !g.human_sides[side] {
                    continue;
                }
                let key = |kind: u8| kind | (side as u8) << 4;
                match e.kind {
                    EventKind::RedCard
                        if e.position == 0 && !self.was_asked(rv.cup, g.src, minute, key(RED)) =>
                    {
                        return Some((g.src, Pending::RedKeeper { minute, side }));
                    }
                    EventKind::Injury
                        if !self.was_asked(rv.cup, g.src, minute, key(INJURY))
                            && info.subs_left(side, minute, false) > 0 =>
                    {
                        return Some((
                            g.src,
                            Pending::Injury {
                                minute,
                                side,
                                player: e.player,
                            },
                        ));
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Time passes on the round view.
    pub fn round_tick(&mut self, ms: u32) {
        let Screen::Round(mut rv) = std::mem::replace(&mut self.screen, Screen::Main) else {
            return;
        };
        self.dirty = true;
        if rv.backdrop {
            self.screen = Screen::Round(rv);
            self.flow();
            return;
        }
        let out = self.round_step(&mut rv, ms);
        self.screen = Screen::Round(rv);
        match out {
            Tick::Run => {}
            Tick::Ask(ds) => {
                let mut ds = ds.into_iter();
                if let Some(d) = ds.next() {
                    self.open(d);
                }
                self.queue.extend(ds);
            }
            Tick::Over => self.finish_view(),
        }
    }

    /// One tick of the game timer on the view: first what the user is asked at the minute on the clock, then the clock.
    fn round_step(&mut self, rv: &mut RoundView, ms: u32) -> Tick {
        if rv.minute >= 1 && rv.checked < rv.minute {
            if let Some((src, p)) = self.pending_at(rv, rv.minute as u16) {
                let cup = rv.cup;
                let d = match p {
                    Pending::Primary {
                        attack,
                        minute,
                        side,
                    } => {
                        self.mark(cup, src, minute, PRIMARY);
                        // (`1038:328b`: the penalty dialog beeps when it opens)
                        self.beep();
                        dialogs::matchday::penalty(self, cup, src, minute, side, attack)
                    }
                    Pending::RedKeeper { minute, side } => {
                        self.mark(cup, src, minute, RED | (side as u8) << 4);
                        dialogs::matchday::substitution(self, cup, src, minute, side, 3)
                    }
                    Pending::Injury {
                        minute,
                        side,
                        player,
                    } => {
                        self.mark(cup, src, minute, INJURY | (side as u8) << 4);
                        dialogs::matchday::forced(self, cup, src, minute, side, player)
                    }
                };
                return Tick::Ask(vec![d]);
            }
            rv.checked = rv.minute;
            if rv.minute == 45 && !rv.halftime_seen {
                let ds = self.halftime_dialogs(rv, 46);
                if !ds.is_empty() {
                    return Tick::Ask(ds);
                }
                rv.halftime_seen = true;
            }
            if rv.minute == 90 && rv.cup && !rv.cut {
                // the ties that were not drawn leave the list; the others go on
                rv.cut = true;
                rv.page = 0;
                if rv.shown().next().is_some() {
                    rv.max = 120;
                    rv.interval = round_view::interval(self.opts.profile as usize + 1, rv.humans());
                    let ds = self.halftime_dialogs(rv, 91);
                    if !ds.is_empty() {
                        return Tick::Ask(ds);
                    }
                    rv.extra_seen = true;
                }
            }
        }
        if rv.advance(ms) {
            // a goal, a red card, an injury or an annulled goal in a game of a human's club beeps (`46df`, `aa11`, `ac72`, `aea7`)
            let minute = rv.minute;
            let n = rv
                .shown()
                .filter(|g| g.human)
                .flat_map(|g| g.events.iter())
                .filter(|e| e.minute as i32 == minute)
                .count();
            for _ in 0..n {
                self.beep();
            }
        }
        if rv.over() {
            Tick::Over
        } else {
            Tick::Run
        }
    }

    /// The dialogs of the break, one for every human side of the games on the view (`next`: the minute the substitutions made now
    /// take place, 46 or 91 before the extra time).
    fn halftime_dialogs(&self, rv: &RoundView, next: u16) -> Vec<Dialog> {
        if !self.opts.half_time_subs {
            return Vec::new();
        }
        let mut out: Vec<(usize, Dialog)> = Vec::new();
        for g in rv.shown().filter(|g| g.human) {
            for side in (0..2).filter(|&s| g.human_sides[s]) {
                let seat = self
                    .world
                    .seat_of(&g.clubs[side].name)
                    .unwrap_or(usize::MAX);
                out.push((
                    seat,
                    dialogs::matchday::halftime(self, rv.cup, g.src, side, next),
                ));
            }
        }
        // (the humans in turn, in the order of the players)
        out.sort_by_key(|(seat, _)| *seat);
        out.into_iter().map(|(_, d)| d).collect()
    }

    /// The last minute of the view is over: the window closes and the end-of-round dialogs follow.
    fn finish_view(&mut self) {
        let Screen::Round(rv) = &self.screen else {
            return;
        };
        let cup = rv.cup;
        self.screen = Screen::Round(Box::new(RoundView::backdrop()));
        self.dirty = true;
        if cup {
            dialogs::matchday::end_of_cup(self);
        } else {
            dialogs::matchday::end_of_round(self);
        }
    }

    /// The final whistle of the round view: kept for the hosts (the flow itself is `finish_view`).
    pub fn finish_round(&mut self) {
        self.screen = Screen::Main;
        self.md.session = None;
        // the next day starts with the main window of the first human of its games
        if let Some(&first) = self.world.turn_order().first() {
            self.switch_human(first);
        }
        self.turns = 0;
        self.dirty = true;
    }

    /// Runs the next step of the end of a round when no dialog is open.
    pub fn flow(&mut self) {
        if !self.dialogs.is_empty() || !self.queue.is_empty() {
            return;
        }
        let Some(step) = self.md.todo.pop_front() else {
            self.finish_round();
            return;
        };
        self.dirty = true;
        match step {
            Step::Open(d) => self.open(*d),
            Step::CupDraw => {
                if let Some(d) = dialogs::matchday::cup_draw(self) {
                    self.open(d);
                }
            }
            Step::CupDay => {
                let seats = self.world.humans_in_cup_tie();
                if seats.is_empty() {
                    // no human plays a tie: no main window, the cup day follows the draw
                    self.play_cup_day();
                } else {
                    if !seats.contains(&self.world.active_seat()) {
                        self.switch_human(seats[0]);
                    }
                    self.turns = 0;
                    self.finish_round();
                }
            }
            Step::Invitations => {
                if let Some((from, to)) = self.next_invitation() {
                    // (the answer to the dialog marks it; the next human's invitation comes after it)
                    self.md.todo.push_front(Step::Invitations);
                    let d = dialogs::matchday::invitation(self, &from, &to);
                    self.open(d);
                }
            }
            Step::Auctions => dialogs::market::queue_auctions(self),
            Step::Main => self.finish_round(),
        }
    }

    /// A click on the round view: a club of a game opens its squad dialog; the page buttons turn the pages of a cup list.
    pub fn round_click(&mut self, x: i32, y: i32) {
        let Screen::Round(rv) = &mut self.screen else {
            return;
        };
        if rv.backdrop {
            return;
        }
        if let Some(b) = round_view::button_at(rv, x, y) {
            if b == "nextGamePageBtn" && rv.page < rv.last_page() {
                rv.page += 1;
            } else if b == "previousGamePageBtn" && rv.page > 0 {
                rv.page -= 1;
            }
            self.dirty = true;
            return;
        }
        let hit = round_view::game_at(rv, x, y).map(|(g, side)| (rv.cup, g.src, side, rv.minute));
        if let Some((cup, src, side, minute)) = hit {
            if minute >= 1 {
                let d = dialogs::matchday::substitution_view(self, cup, src, side, minute as u16);
                self.open(d);
            }
        }
    }

    /// The first invitation of this round that a human has not answered: `(his club before, the club that wants him)`.
    pub fn next_invitation(&self) -> Option<(String, String)> {
        let sess = self.md.session.as_ref()?;
        self.world
            .human_moves()
            .iter()
            .find(|(from, to)| {
                !sess.answered.contains(from)
                    && sess.snapshot.is_human_club(from)
                    // (a human without a club, at the pool, is hired without being asked)
                    && !self.world.pool.iter().any(|c| c.name == *from)
                    && self
                        .world
                        .coach_changes
                        .iter()
                        .any(|(sacked, gave)| sacked == to && gave == from)
            })
            .cloned()
    }
}

/// Kind of a dialog that belongs to this module.
pub fn is_matchday(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Halftime
            | Kind::ForcedSubstitution
            | Kind::Penalty
            | Kind::Shootout
            | Kind::CupDraw
            | Kind::EndOfSeason
            | Kind::RefereeAid
            | Kind::Invitation
            | Kind::Lashes
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classic::Data;
    use crate::win31::{Forms, Skin};

    fn classic(seed: u32) -> Classic {
        Classic::new(
            World::synthetic(seed),
            Skin::default(),
            Forms::default(),
            Data::default(),
        )
    }

    /// Lets the clock run until a dialog opens or the main window is back.
    fn run(c: &mut Classic) {
        for _ in 0..200_000 {
            if !c.dialogs.is_empty() || matches!(c.screen, Screen::Main) {
                return;
            }
            c.tick(20);
        }
        panic!("the round did not end");
    }

    /// Presses a control of the dialog on top (what a click on it would do).
    fn press(c: &mut Classic, name: &str) {
        let mut d = c.dialogs.pop().expect("a dialog is open");
        let out = dialogs::click(c, &mut d, name);
        c.apply(d, out);
    }

    fn top_kind(c: &Classic) -> Option<Kind> {
        c.dialogs.last().map(|d| d.kind.clone())
    }

    /// Plays rounds like a user who answers every dialog by closing it; the injuries are answered with the first player of the bench.
    fn play_through(c: &mut Classic) -> Vec<Kind> {
        let mut seen = Vec::new();
        for _ in 0..2000 {
            run(c);
            let Some(kind) = top_kind(c) else {
                return seen;
            };
            seen.push(kind.clone());
            match kind {
                Kind::ForcedSubstitution => {
                    press(c, "ListBoxOfSubstitutePlayers#0");
                    press(c, "OKBtn");
                }
                Kind::Penalty => {
                    let attack = c.dialogs.last().unwrap().num("attack") != 0;
                    if attack {
                        press(c, "ListBoxOfPlayers#0");
                        press(c, "shootBtn");
                    }
                    // the dialog closes by itself after five seconds
                    for _ in 0..300 {
                        c.tick(20);
                    }
                    c.dialogs.clear();
                }
                Kind::Halftime | Kind::Invitation => press(c, "OKBtn"),
                _ => {
                    c.dialogs.pop();
                }
            }
        }
        panic!("too many dialogs");
    }

    fn text(w: &World) -> Vec<String> {
        w.last_round
            .iter()
            .map(|(d, m)| {
                format!(
                    "{d} {} {} {}-{} {:?} {:?}",
                    m.home, m.away, m.home_goals, m.away_goals, m.events, m.subs
                )
            })
            .collect()
    }

    #[test]
    fn a_round_nobody_touches_is_the_round_the_core_plays() {
        let mut compared = 0;
        for seed in [3, 8, 21, 22, 23, 24, 25, 26] {
            let mut c = classic(seed);
            let mut plain = c.world.clone();
            plain.play_round();
            c.opts.half_time_subs = false;
            c.play_round();
            // no decision is taken: the dialogs that ask something are not answered, the replay never happens
            run(&mut c);
            while let Some(k) = top_kind(&c) {
                match k {
                    Kind::Penalty if c.dialogs.last().unwrap().num("attack") == 0 => {
                        c.dialogs.pop();
                    }
                    Kind::Penalty | Kind::ForcedSubstitution => break,
                    _ => {
                        c.dialogs.pop();
                    }
                }
                run(&mut c);
            }
            if matches!(c.screen, Screen::Main) {
                assert_eq!(text(&c.world), text(&plain), "seed {seed}");
                assert_eq!(c.world.rng.seed(), plain.rng.seed());
                compared += 1;
            }
        }
        assert!(
            compared >= 2,
            "only {compared} seeds ran without a question"
        );
    }

    #[test]
    fn the_clock_pauses_for_the_break_and_the_dialog_shows_the_score() {
        let mut c = classic(5);
        c.play_round();
        run(&mut c);
        // the first thing that stops the clock may be a primary chance or an injury before the break; skip those
        let mut guard = 0;
        while top_kind(&c) != Some(Kind::Halftime) && guard < 40 {
            match top_kind(&c) {
                Some(Kind::ForcedSubstitution) => {
                    press(&mut c, "ListBoxOfSubstitutePlayers#0");
                    press(&mut c, "OKBtn");
                }
                Some(_) => c.dialogs.clear(),
                None => {}
            }
            run(&mut c);
            guard += 1;
        }
        assert_eq!(top_kind(&c), Some(Kind::Halftime));
        let Screen::Round(rv) = &c.screen else {
            panic!("the round view is showing");
        };
        assert_eq!(rv.minute, 45);
        assert!(!rv.halftime_seen, "the clock turns red after the break");
        press(&mut c, "OKBtn");
        let Screen::Round(rv) = &c.screen else {
            panic!("the round view is showing");
        };
        assert!(rv.halftime_seen);
    }

    #[test]
    fn a_substitution_made_at_the_break_is_played_in_the_second_half_only() {
        use rustfoot_core::sector::Status;
        let mut c = classic(11);
        let (d, i) = c.world.human_location().unwrap();
        let before = c.world.clone();
        c.play_round();
        let mut guard = 0;
        // answer what comes before the break, up to the dialog of the break
        while top_kind(&c) != Some(Kind::Halftime) && guard < 40 {
            run(&mut c);
            match top_kind(&c) {
                Some(Kind::ForcedSubstitution) => {
                    press(&mut c, "ListBoxOfSubstitutePlayers#0");
                    press(&mut c, "OKBtn");
                }
                Some(Kind::Halftime) | None => {}
                Some(_) => c.dialogs.clear(),
            }
            guard += 1;
        }
        let Some(Kind::Halftime) = top_kind(&c) else {
            panic!("no break dialog");
        };
        // pick an outfield starter and a bench player of the user's squad as the dialog lists them
        let (src, side) = {
            let dlg = c.dialogs.last().unwrap();
            (dlg.num("src") as usize, dlg.num("side") as usize)
        };
        let info = info(&c.world, false, src).unwrap();
        let st = dialogs::matchday::squad_after(&info, side, 45);
        let out = (0..st.len())
            .find(|&k| st[k] == Status::Starter && info.start[side][k].position == 3)
            .unwrap();
        let inn = (0..st.len())
            .find(|&k| st[k] == Status::Sub && info.start[side][k].position != 0)
            .unwrap();
        let first_half: Vec<String> = info
            .events
            .iter()
            .filter(|e| e.minute <= 45)
            .map(|e| format!("{e:?}"))
            .collect();
        let others: Vec<String> = text(&c.world);
        {
            let dlg = c.dialogs.last_mut().unwrap();
            dlg.st.insert("pend".into(), format!("{out}:{inn}"));
        }
        press(&mut c, "OKBtn");
        let info = super::info(&c.world, false, src).unwrap();
        assert!(info
            .subs
            .iter()
            .any(|s| s.minute == 46 && !s.auto && s.out == out && s.inn == inn));
        let again: Vec<String> = info
            .events
            .iter()
            .filter(|e| e.minute <= 45)
            .map(|e| format!("{e:?}"))
            .collect();
        assert_eq!(first_half, again, "the first half does not change");
        // the other games of the round are the ones the user saw
        let now = text(&c.world);
        for (k, (a, b)) in others.iter().zip(&now).enumerate() {
            if k != src {
                assert_eq!(a, b, "game {k}");
            }
        }
        let _ = (d, i, before);
        play_through(&mut c);
    }

    /// With "Habilitar som" on, the events of the user's game beep as the clock reaches them; with it off nothing does.
    #[test]
    fn the_events_of_a_humans_game_beep_only_with_the_sound_on() {
        for on in [true, false] {
            let mut c = classic(2);
            c.opts.sound_off = !on;
            let mut beeps = 0;
            let mut events = 0;
            for _ in 0..4 {
                c.play_round();
                if let Screen::Round(rv) = &c.screen {
                    events += rv
                        .shown()
                        .filter(|g| g.human)
                        .map(|g| g.events.len())
                        .sum::<usize>();
                }
                play_through(&mut c);
                beeps += c
                    .requests
                    .iter()
                    .filter(|r| **r == crate::classic::Request::Beep)
                    .count();
                c.requests.clear();
            }
            if on {
                assert!(events > 0, "the user's games had events");
                assert!(beeps >= events, "{beeps} beeps for {events} events");
            } else {
                assert_eq!(beeps, 0);
            }
        }
    }

    /// The cup draw names a team every 1500 ms while humans' clubs are left to be drawn (each beeps), then every 400 ms (`1020:3232`).
    #[test]
    fn the_cup_draw_slows_down_for_the_humans_clubs_and_beeps_for_them() {
        let mut c = classic(2);
        c.opts.sound_off = false;
        for _ in 0..30 {
            if c.world.cup_draw.is_some() && !c.world.humans_in_cup_tie().is_empty() {
                break;
            }
            c.play_round();
            play_through(&mut c);
        }
        let pairs = c
            .world
            .cup_draw
            .as_ref()
            .expect("a cup round was drawn")
            .pairs
            .clone();
        let humans = pairs
            .iter()
            .flat_map(|(h, a)| [h, a])
            .filter(|n| c.world.is_human_club(n))
            .count();
        assert!(humans > 0);
        c.requests.clear();
        let mut d = dialogs::matchday::cup_draw(&c).unwrap();
        let mut gaps = Vec::new();
        let (mut ms, mut last, mut ticks) = (0, 0, 0);
        loop {
            let out = dialogs::tick(&mut c, &mut d, 100);
            ms += 100;
            if d.num("ticks") != ticks {
                ticks = d.num("ticks");
                gaps.push((ms - last, d.num("left")));
                last = ms;
            }
            if matches!(out, crate::classic::dialog::Outcome::Close) {
                break;
            }
        }
        let beeps = c
            .requests
            .iter()
            .filter(|r| **r == crate::classic::Request::Beep)
            .count();
        assert_eq!(beeps, humans, "one beep per human club named");
        assert_eq!(gaps.len(), pairs.len() * 2 + 5);
        let first_fast = gaps.iter().position(|&(_, left)| left == 0).unwrap();
        assert!(gaps[..=first_fast].iter().all(|&(g, _)| g == 1500));
        assert!(gaps[first_fast + 1..].iter().all(|&(g, _)| g == 400));
    }

    /// The referee's name on the break dialog opens the referee aid only in a registered copy (`1008:2427`).
    #[test]
    fn the_referee_aid_belongs_to_the_registered_copy() {
        for registered in [false, true] {
            let mut c = classic(2);
            c.main.market.registered = registered;
            c.play_round();
            run(&mut c);
            assert_eq!(top_kind(&c), Some(Kind::Halftime));
            press(&mut c, "textReferee");
            let aid = top_kind(&c) == Some(Kind::RefereeAid);
            assert_eq!(aid, registered);
        }
    }

    #[test]
    fn many_rounds_with_a_user_who_answers_everything_stay_consistent() {
        let mut c = classic(2);
        let start = c.world.year;
        for _ in 0..24 {
            c.play_round();
            let seen = play_through(&mut c);
            assert!(
                seen.contains(&Kind::Halftime)
                    || c.world.cup_day_due()
                    || seen.contains(&Kind::CupDraw)
            );
            assert!(matches!(c.screen, Screen::Main));
            assert!(c.md.todo.is_empty());
        }
        // 14 league days and the cup days of a season and more; the end of the season was shown once
        assert!(c.world.year > start);
    }

    #[test]
    fn a_tie_in_extra_time_stays_on_the_screen_with_its_own_break_at_90() {
        let mut found = 0;
        for seed in 0..600 {
            let mut w = World::synthetic(seed);
            for _ in 0..7 {
                w.play_round();
            }
            // only worlds in which the user's cup tie of the next round goes to extra time (the user's own decisions do not change
            // that before the break: the league game is separate)
            let mut probe = w.clone();
            probe.play_round();
            let me = probe.human.as_ref().unwrap().club.clone();
            let in_et = probe.last_cup.as_ref().is_some_and(|r| {
                r.matches
                    .iter()
                    .any(|m| (m.home == me || m.away == me) && m.extra_time)
            });
            if !in_et {
                continue;
            }
            let mut c = Classic::new(w, Skin::default(), Forms::default(), Data::default());
            c.play_round();
            // the league round: answer everything until the main window of the cup day is back
            let mut guard = 0;
            loop {
                run(&mut c);
                if matches!(&c.screen, Screen::Main) {
                    break;
                }
                match top_kind(&c) {
                    Some(Kind::ForcedSubstitution) => {
                        press(&mut c, "ListBoxOfSubstitutePlayers#0");
                        press(&mut c, "OKBtn");
                    }
                    Some(Kind::Penalty) if c.dialogs.last().unwrap().num("attack") != 0 => {
                        press(&mut c, "ListBoxOfPlayers#0");
                        press(&mut c, "shootBtn");
                        c.dialogs.clear();
                    }
                    Some(Kind::Halftime) => press(&mut c, "OKBtn"),
                    Some(_) => {
                        c.dialogs.pop();
                    }
                    None => c.tick(20),
                }
                guard += 1;
                assert!(guard < 4000);
            }
            if !c.world.cup_day_due() || c.world.humans_in_cup_tie().is_empty() {
                continue;
            }
            // Jogar on the cup day
            c.play_round();
            run(&mut c);
            let Screen::Round(rv) = &c.screen else {
                continue;
            };
            let extra: Vec<bool> = rv.games.iter().map(|g| g.extra_time).collect();
            if !rv.games.iter().any(|g| g.human && g.extra_time) {
                continue;
            }
            // run the clock to minute 90 answering the questions of the user's tie
            for _ in 0..4000 {
                match top_kind(&c) {
                    Some(Kind::ForcedSubstitution) => {
                        press(&mut c, "ListBoxOfSubstitutePlayers#0");
                        press(&mut c, "OKBtn");
                    }
                    Some(Kind::Penalty) if c.dialogs.last().unwrap().num("attack") != 0 => {
                        press(&mut c, "ListBoxOfPlayers#0");
                        press(&mut c, "shootBtn");
                        c.dialogs.clear();
                    }
                    Some(Kind::Halftime) if c.dialogs.last().unwrap().num("next") == 91 => break,
                    Some(Kind::Halftime) => press(&mut c, "OKBtn"),
                    Some(_) => {
                        c.dialogs.pop();
                    }
                    None => c.tick(20),
                }
            }
            let d = c.dialogs.last().expect("the break before the extra time");
            assert_eq!(
                (d.kind.clone(), d.num("next"), d.num("minute")),
                (Kind::Halftime, 91, 90)
            );
            let Screen::Round(rv) = &c.screen else {
                panic!("cup view")
            };
            assert!(rv.cut && rv.max == 120 && !rv.extra_seen);
            assert_eq!(rv.shown().count(), extra.iter().filter(|&&e| e).count());
            assert!(rv.shown().all(|g| g.extra_time));
            press(&mut c, "OKBtn");
            let Screen::Round(rv) = &c.screen else {
                panic!("cup view")
            };
            assert!(rv.extra_seen, "the clock turns purple after the break");
            // the view goes on to minute 120 and closes five ticks later
            let mut ticks = 0;
            while matches!(&c.screen, Screen::Round(r) if r.cup && !r.backdrop)
                && c.dialogs.is_empty()
            {
                c.tick(20);
                ticks += 1;
                assert!(ticks < 100_000);
            }
            found += 1;
            if found == 2 {
                break;
            }
        }
        assert!(found >= 1, "no seed had the user's tie in extra time");
    }

    /// A world in which the club (0, 0) is about to sack its coach and the user's coach is the one the game picks (a lot weaker and in
    /// form), so that the round brings an invitation when the club loses or draws.
    fn invitation_world(seed: u32) -> World {
        let mut w = World::synthetic(seed);
        {
            let t = &mut w.divisions[0][0];
            t.k = 0.4;
            t.coach_games = 12;
            t.coach_score = 7;
            for s in t.slots.iter_mut() {
                s.strength = 30;
            }
        }
        let (d, i) = w.human_location().unwrap();
        let h = &mut w.divisions[d][i];
        h.k = 1.5;
        for s in h.slots.iter_mut() {
            s.strength = 15;
        }
        w
    }

    #[test]
    fn declining_an_invitation_plays_the_round_again_without_it() {
        let mut found = 0;
        for seed in 0..200 {
            let w = invitation_world(seed);
            let mut probe = w.clone();
            probe.accept_invitations = true;
            probe.play_round();
            let invited = probe
                .human_moves()
                .first()
                .is_some_and(|(_, to)| probe.coach_changes.iter().any(|(sacked, _)| sacked == to));
            if !invited {
                continue;
            }
            let mut c = Classic::new(w, Skin::default(), Forms::default(), Data::default());
            let mine = c.world.human.as_ref().unwrap().club.clone();
            c.play_round();
            let seen = play_until(&mut c, Kind::Invitation);
            assert!(
                seen,
                "the invitation dialog shows after the round view (seed {seed})"
            );
            let d = c.dialogs.last().unwrap();
            assert_eq!(d.kind, Kind::Invitation);
            let (_, to) = c.world.human_moves().first().cloned().unwrap();
            assert_eq!(d.st.get("team"), Some(&to));
            // the user accepted by default: he already manages the other club
            assert_eq!(c.world.human.as_ref().unwrap().club, to);
            press(&mut c, "noBtn");
            assert_eq!(
                c.world.human.as_ref().unwrap().club,
                mine,
                "declined: nothing moved"
            );
            assert!(c.world.human_moves().is_empty());
            found += 1;
            if found == 2 {
                break;
            }
        }
        assert!(found >= 1, "no seed produced an invitation");
    }

    fn hot_seat(n: usize, seed: u32) -> Classic {
        let mut w = World::synthetic(seed);
        w.name_humans(&(1..=n).map(|k| format!("joga{k}")).collect::<Vec<_>>());
        let mut c = Classic::new(w, Skin::default(), Forms::default(), Data::default());
        c.enter_game();
        c
    }

    #[test]
    fn the_round_button_hands_the_window_to_each_human_and_the_last_one_starts_the_round() {
        let mut c = hot_seat(3, 4);
        assert_eq!((c.world.active_seat(), c.coach.as_str()), (0, "joga1"));
        let order = c.world.turn_order();
        assert_eq!(order.len(), 3);
        // the first player starts, then the others in the order of the day's games (`13e1`)
        c.round_button();
        let rest: Vec<usize> = order.iter().copied().filter(|&s| s != 0).collect();
        let second = rest[0];
        assert_eq!(c.world.active_seat(), second);
        assert_eq!(c.coach, format!("joga{}", second + 1));
        assert!(matches!(c.screen, Screen::Main) && c.md.session.is_none());
        c.round_button();
        assert_eq!(c.world.active_seat(), rest[1]);
        assert!(c.md.session.is_none());
        c.round_button();
        assert!(c.md.session.is_some(), "the third Enter plays the round");
        assert!(matches!(c.screen, Screen::Round(_)));
        play_through(&mut c);
        assert!(matches!(c.screen, Screen::Main));
        assert_eq!(c.world.round, 1);
        // the next day opens on the first human of its games, and the turns go along that day's order
        let order = c.world.turn_order();
        assert_eq!(c.world.active_seat(), order[0]);
        c.round_button();
        assert_eq!(c.world.active_seat(), order[1]);
    }

    /// Plays league days like a user who answers everything until the main window is back with a cup day pending, in which a
    /// human plays a tie; `None` when the season ended first.
    fn until_cup_day(c: &mut Classic) -> Option<()> {
        for _ in 0..40 {
            if c.world.cup_day_due() && !c.world.humans_in_cup_tie().is_empty() {
                return Some(());
            }
            c.play_round();
            play_through(c);
            assert!(matches!(c.screen, Screen::Main));
        }
        None
    }

    #[test]
    fn the_cup_is_played_on_its_own_day_with_the_main_window_in_between() {
        let mut found = 0;
        for seed in [2, 7, 12, 30] {
            let mut c = classic(seed);
            let Some(()) = until_cup_day(&mut c) else {
                continue;
            };
            found += 1;
            let draw = c.world.cup_draw.clone().unwrap();
            let round = c.world.round;
            assert!(matches!(c.screen, Screen::Main) && c.md.session.is_none());
            assert!(c.world.last_cup.is_none(), "the ties are not played yet");
            // the main window shows the tie: the opponent with its division, the stage, no standings
            let tie = c.world.human_cup_tie().unwrap();
            let ((_, _), club) = main_win::human_club(&c).unwrap();
            let v = main_win::view(&c, &club, crate::classic::Page::Game);
            let opp = &v.texts["textOpponent"];
            assert!(opp.contains(&tie.opponent), "{opp}");
            assert!(
                opp.contains(&format!(
                    "{} TAÇA - {}",
                    if tie.home { "CASA" } else { "FORA" },
                    draw.name
                )),
                "{opp}"
            );
            assert!(
                opp.contains("Divisão)") || opp.contains("(Distrital)"),
                "{opp}"
            );
            assert!(v.lists["ListBoxClassification"].is_empty());
            // the plan changed now counts: Jogar plays the cup day
            c.round_button();
            assert!(c.md.session.as_ref().is_some_and(|s| s.cup_day));
            let Screen::Round(rv) = &c.screen else {
                panic!("the cup view");
            };
            assert!(rv.cup && rv.title.starts_with("Taça - "), "{}", rv.title);
            play_through(&mut c);
            assert!(matches!(c.screen, Screen::Main));
            assert!(!c.world.cup_day_due());
            assert!(c.world.last_cup.is_some());
            assert!(c.world.round == round || c.world.round == 0);
            assert!(c.md.todo.is_empty());
        }
        assert!(found >= 2, "no cup day with a human tie in {found} seeds");
    }

    #[test]
    fn a_human_out_of_the_cup_gets_no_main_window_on_the_cup_day() {
        let mut shown = 0;
        for seed in 0..60 {
            let mut c = classic(seed);
            for _ in 0..30 {
                c.play_round();
                run(&mut c);
                let mut guard = 0;
                // answer the dialogs of the league round; stop when the main window is back or the cup view runs
                while !matches!(c.screen, Screen::Main) && guard < 40_000 {
                    match top_kind(&c) {
                        Some(Kind::ForcedSubstitution) => {
                            press(&mut c, "ListBoxOfSubstitutePlayers#0");
                            press(&mut c, "OKBtn");
                        }
                        Some(Kind::Halftime) => press(&mut c, "OKBtn"),
                        Some(_) => {
                            c.dialogs.pop();
                        }
                        None => c.tick(20),
                    }
                    guard += 1;
                }
                // a cup day that the user does not play is never waiting on the main window
                assert!(
                    !matches!(c.screen, Screen::Main)
                        || !c.world.cup_day_due()
                        || !c.world.humans_in_cup_tie().is_empty(),
                    "the main window came back for a cup day without a human tie"
                );
                if c.world.round.is_multiple_of(2)
                    && c.world.last_cup.is_some()
                    && !c.world.cup_day_due()
                {
                    shown += 1;
                }
            }
        }
        assert!(shown > 0);
    }

    #[test]
    fn on_a_cup_day_only_the_humans_with_a_tie_take_turns() {
        let mut found = 0;
        for seed in [4, 9, 13, 17] {
            let mut c = hot_seat(3, seed);
            if until_cup_day(&mut c).is_none() {
                continue;
            }
            let seats = c.world.humans_in_cup_tie();
            assert!(!seats.is_empty());
            assert!(
                seats.contains(&c.world.active_seat()),
                "the window is with a human who plays"
            );
            for k in 1..seats.len() {
                let before = c.world.active_seat();
                c.round_button();
                assert!(
                    c.md.session.is_none(),
                    "turn {k}: the cup day is not played yet"
                );
                assert!(seats.contains(&c.world.active_seat()));
                assert_ne!(c.world.active_seat(), before);
            }
            c.round_button();
            assert!(c.md.session.as_ref().is_some_and(|s| s.cup_day));
            play_through(&mut c);
            assert!(!c.world.cup_day_due());
            found += 1;
        }
        assert!(found >= 2);
    }

    #[test]
    fn every_human_gets_his_own_break_dialog_and_the_questions_of_his_game() {
        let mut c = hot_seat(2, 6);
        c.round_button();
        c.round_button();
        let humans = c.world.human_clubs();
        let mut breaks = Vec::new();
        for _ in 0..4000 {
            run(&mut c);
            match top_kind(&c) {
                Some(Kind::Halftime) => {
                    let d = c.dialogs.last().unwrap();
                    if d.num("mode") == 4 {
                        let info = info(&c.world, false, d.num("src") as usize).unwrap();
                        breaks.push(info.clubs[d.num("side") as usize].clone());
                    }
                    press(&mut c, "OKBtn");
                }
                Some(Kind::ForcedSubstitution) => {
                    press(&mut c, "ListBoxOfSubstitutePlayers#0");
                    press(&mut c, "OKBtn");
                }
                Some(Kind::Penalty) => {
                    if c.dialogs.last().unwrap().num("attack") != 0 {
                        press(&mut c, "ListBoxOfPlayers#0");
                        press(&mut c, "shootBtn");
                    }
                    c.dialogs.clear();
                }
                Some(_) => {
                    c.dialogs.pop();
                }
                None if matches!(c.screen, Screen::Main) => break,
                None => c.tick(20),
            }
        }
        assert_eq!(breaks, humans, "one break per human, in seat order");
    }

    #[test]
    fn several_rounds_and_a_season_with_four_humans_stay_consistent() {
        let mut c = hot_seat(4, 9);
        for _ in 0..16 {
            for _ in 0..4 {
                c.round_button();
            }
            play_through(&mut c);
            assert!(matches!(c.screen, Screen::Main));
            assert!(c.md.todo.is_empty());
            assert_eq!(c.world.human_count(), 4);
        }
        assert!(c.world.year > 1998);
    }

    /// Plays the round view answering what comes, until a dialog of `kind` is open.
    fn play_until(c: &mut Classic, kind: Kind) -> bool {
        for _ in 0..6000 {
            run(c);
            match top_kind(c) {
                Some(k) if k == kind => return true,
                Some(Kind::ForcedSubstitution) => {
                    press(c, "ListBoxOfSubstitutePlayers#0");
                    press(c, "OKBtn");
                }
                Some(Kind::Penalty) if c.dialogs.last().unwrap().num("attack") != 0 => {
                    press(c, "ListBoxOfPlayers#0");
                    press(c, "shootBtn");
                    c.dialogs.clear();
                }
                Some(Kind::Halftime) => press(c, "OKBtn"),
                Some(_) => {
                    c.dialogs.pop();
                }
                None if matches!(c.screen, Screen::Main) => return false,
                None => c.tick(20),
            }
        }
        false
    }

    /// A hot-seat game played for many days: a human who was replaced by a free coach has no turn, and the main window never
    /// rests on him.
    #[test]
    fn a_human_without_a_club_takes_no_turn() {
        let mut unemployed_days = 0;
        for seed in 1..=6 {
            let mut c = hot_seat(2, seed);
            for _ in 0..70 {
                // every human with a club presses the round button, then the day is played with its dialogs
                let turns = c.turn_seats().len().max(1);
                for _ in 0..turns {
                    c.round_button();
                }
                for _ in 0..6000 {
                    run(&mut c);
                    match top_kind(&c) {
                        Some(Kind::Halftime) => press(&mut c, "OKBtn"),
                        Some(Kind::Invitation) => press(&mut c, "yesBtn"),
                        Some(Kind::ForcedSubstitution) => {
                            press(&mut c, "ListBoxOfSubstitutePlayers#0");
                            press(&mut c, "OKBtn");
                        }
                        Some(Kind::Penalty) => {
                            if c.dialogs.last().unwrap().num("attack") != 0 {
                                press(&mut c, "ListBoxOfPlayers#0");
                                press(&mut c, "shootBtn");
                            }
                            c.dialogs.clear();
                        }
                        Some(_) => {
                            c.dialogs.pop();
                        }
                        None if matches!(c.screen, Screen::Main) => break,
                        None => c.tick(20),
                    }
                }
                assert!(
                    matches!(c.screen, Screen::Main),
                    "the day ended on the main window (seed {seed}, round {}, dialogs {}, top {:?}, todo {}, queue {}, cup due {}, employed {:?})",
                    c.world.round,
                    c.dialogs.len(),
                    top_kind(&c),
                    c.md.todo.len(),
                    c.queue.len(),
                    c.world.cup_day_due(),
                    c.world.employed_seats()
                );
                let active = c.world.active_seat();
                assert!(
                    !c.world.is_unemployed(active) || c.world.employed_seats().is_empty(),
                    "seed {seed}: the window is on a human without a club (active {active}, unemployed {:?}, clubs {:?}, round {}, moves {:?})", (0..2).map(|s| c.world.is_unemployed(s)).collect::<Vec<_>>(), c.world.human_clubs(), c.world.round, c.world.seats.moves
                );
                unemployed_days += (0..2).filter(|&s| c.world.is_unemployed(s)).count();
            }
        }
        eprintln!("days with an unemployed human: {unemployed_days}");
    }
}
