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
use elifoot_core::matchsim::{Decision, Event, EventKind, Side, SubEvent};
use elifoot_core::rng::DelphiRng;
use elifoot_core::season::{Club, Control};
use elifoot_core::sector::{Slot, Status};
use elifoot_core::world::World;
use std::collections::{HashMap, VecDeque};

/// The options of the original's *Opções* that act on a match day (defaults of the original's INI).
#[derive(Clone, Debug)]
pub struct Options {
    /// Game time profile 1..=6 (Lento, Médio, Rápido, Super-rápido, Rapidíssimo, Ultrassónico); `Médio` by default.
    pub time_profile: usize,
    /// Substitutions at the break: the dialog of the break shows for the user's games.
    pub halftime_subs: bool,
    /// Watch the draw of the cup: 0 never, 1 when the user's club is drawn (default), 2 always.
    pub cup_draw: u8,
    /// Show the psychological whips (sackings): 0 never, 1 those of the human coaches (default), 2 all.
    pub lashes: u8,
    /// The check boxes of the break dialog: list the red cards and the injuries.
    pub view_red: bool,
    pub view_injuries: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            time_profile: 2,
            halftime_subs: true,
            cup_draw: 1,
            lashes: 1,
            view_red: false,
            view_injuries: false,
        }
    }
}

/// What happens next at the end of a round, once the dialogs open are closed.
pub enum Step {
    Open(Box<Dialog>),
    /// The draw of the cup (if the options say so), then the cup round.
    CupDraw,
    CupView,
    Auctions,
    Main,
}

/// The round being shown, with what is needed to play it again with the user's decisions.
pub struct Session {
    /// The world before the round.
    pub snapshot: Box<World>,
    /// The random state at the checkpoints of the first run.
    pub log: Vec<DelphiRng>,
    pub league: Vec<Decision>,
    pub cup: Vec<Decision>,
    /// Things already asked or shown: `(cup, minute, kind)`.
    pub handled: Vec<(bool, u16, u8)>,
    /// The referee of the user's league game, chosen before the round (as the main window showed it).
    pub referee: String,
    /// The user's coach took an invitation or not (see `World::accept_invitations`).
    pub accept: bool,
    /// The user's club and the coaches of all clubs before the round (names for the sacking list).
    pub coaches: HashMap<String, String>,
}

#[derive(Default)]
pub struct State {
    pub opts: Options,
    pub session: Option<Session>,
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
    /// The squad of `side` at `minute`: for every slot `Starter` (on the pitch), `Sub` (still on the bench) or `Out`. Substitutions
    /// of the minute itself count only when `with_auto` (the automatic reply to an injury or a red card is not yet made while the
    /// user decides). The planned ones of the next minute (the break) are never in.
    pub fn squad_at(&self, side: usize, minute: u16, with_auto: bool) -> Vec<Status> {
        let mut st: Vec<Status> = self.start[side].iter().map(|s| s.status).collect();
        // everything that happened by `minute` in order: the events take players off, the substitutions swap
        let mut items: Vec<(u16, u8, usize)> = Vec::new();
        for (i, e) in self.events.iter().enumerate() {
            if (e.side == Side::Away) as usize == side
                && e.minute <= minute
                && matches!(e.kind, EventKind::RedCard | EventKind::Injury)
            {
                items.push((e.minute, 0, i));
            }
        }
        for (i, s) in self.subs.iter().enumerate() {
            if (s.side == Side::Away) as usize == side
                && (s.minute < minute || (s.minute == minute && (with_auto || !s.auto)))
            {
                items.push((s.minute, 1, i));
            }
        }
        items.sort();
        for (_, kind, i) in items {
            if kind == 0 {
                st[self.events[i].player] = Status::Out;
            } else {
                let s = self.subs[i];
                st[s.out] = Status::Out;
                st[s.inn] = Status::Starter;
            }
        }
        st
    }

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
    Ask(Dialog),
    Over,
}

/// What stops the clock at a minute of the user's game.
enum Pending {
    /// A primary chance: the user's side attacks (`true`: he picks the shooter) or defends.
    Primary { attack: bool, minute: u16 },
    /// The user's goalkeeper was sent off.
    RedKeeper { minute: u16 },
    /// A player of the user was injured.
    Injury { minute: u16, player: usize },
}

impl Classic {
    fn referee_of_next_game(&self) -> String {
        let Some((d, j, home)) = main_win::next_opponent(self) else {
            return String::new();
        };
        let Some((_, me)) = main_win::human_club(self) else {
            return String::new();
        };
        let opp = &self.world.divisions[d][j];
        if home {
            main_win::referee(self, &me, opp)
        } else {
            main_win::referee(self, opp, &me)
        }
    }

    /// Plays the next round of the league (and cup) and shows it.
    pub fn play_round(&mut self) {
        self.main.menu = None;
        self.md.todo.clear();
        let referee = self.referee_of_next_game();
        self.world.accept_invitations = true;
        let snapshot = Box::new(self.world.clone());
        let coaches = self
            .world
            .divisions
            .iter()
            .flatten()
            .chain(self.world.pool.iter())
            .map(|c| (c.name.clone(), c.coach_name.clone()))
            .collect();
        let mut ctl = Control::default();
        self.world.play_round_ctl(&mut ctl);
        self.md.session = Some(Session {
            snapshot,
            log: std::mem::take(&mut ctl.log),
            league: Vec::new(),
            cup: Vec::new(),
            handled: Vec::new(),
            referee,
            accept: true,
            coaches,
        });
        self.start_round_view();
    }

    /// Shows the league round just played: the clock runs from -5 to 90.
    pub fn start_round_view(&mut self) {
        let mut v = RoundView::of_round(&self.world);
        v.interval = round_view::interval(self.md.opts.time_profile, v.humans());
        self.screen = Screen::Round(Box::new(v));
        self.dirty = true;
    }

    /// Plays the round again from the world before it with the user's decisions and shows the result from the same minute.
    pub fn replay_round(&mut self, league: Vec<Decision>, cup: Vec<Decision>) {
        let Some(sess) = self.md.session.as_mut() else {
            return;
        };
        sess.league = league;
        sess.cup = cup;
        let mut w = (*sess.snapshot).clone();
        w.accept_invitations = sess.accept;
        let mut ctl = Control::replay(sess.log.clone(), sess.league.clone(), sess.cup.clone());
        w.play_round_ctl(&mut ctl);
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

    /// The user decided something at a minute of his game (`cup`: the cup tie, otherwise the league game).
    pub fn decide(&mut self, cup: bool, ds: Vec<Decision>) {
        let Some(sess) = self.md.session.as_ref() else {
            return;
        };
        let (mut league, mut cups) = (sess.league.clone(), sess.cup.clone());
        if cup {
            cups.extend(ds);
        } else {
            league.extend(ds);
        }
        self.replay_round(league, cups);
    }

    fn mark(&mut self, cup: bool, minute: u16, kind: u8) {
        if let Some(s) = self.md.session.as_mut() {
            s.handled.push((cup, minute, kind));
        }
    }

    fn was_asked(&self, cup: bool, minute: u16, kind: u8) -> bool {
        self.md
            .session
            .as_ref()
            .is_some_and(|s| s.handled.contains(&(cup, minute, kind)))
    }

    /// The first thing the original would ask at `minute` of the user's game that was not asked yet (the engine's order: primary
    /// chance, red card, injury).
    fn pending_at(&self, rv: &RoundView, minute: u16) -> Option<(usize, Pending)> {
        let g = rv.shown().find(|g| g.human)?;
        let side = g.human_side?;
        let info = info(&self.world, rv.cup, g.src)?;
        // primary chances
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
        for p in prim.iter().filter(|p| p.minute == minute) {
            if !self.was_asked(rv.cup, minute, PRIMARY) {
                return Some((
                    g.src,
                    Pending::Primary {
                        attack: (p.side == Side::Away) as usize == side,
                        minute,
                    },
                ));
            }
        }
        for e in info.events.iter().filter(|e| e.minute == minute) {
            if (e.side == Side::Away) as usize != side {
                continue;
            }
            match e.kind {
                EventKind::RedCard if e.position == 0 && !self.was_asked(rv.cup, minute, RED) => {
                    return Some((g.src, Pending::RedKeeper { minute }));
                }
                EventKind::Injury
                    if !self.was_asked(rv.cup, minute, INJURY)
                        && info.subs_left(side, minute, false) > 0 =>
                {
                    return Some((
                        g.src,
                        Pending::Injury {
                            minute,
                            player: e.player,
                        },
                    ));
                }
                _ => {}
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
            Tick::Ask(d) => self.open(d),
            Tick::Over => self.finish_view(),
        }
    }

    /// One tick of the game timer on the view: first what the user is asked at the minute on the clock, then the clock.
    fn round_step(&mut self, rv: &mut RoundView, ms: u32) -> Tick {
        if rv.minute >= 1 && rv.checked < rv.minute {
            if let Some((src, p)) = self.pending_at(rv, rv.minute as u16) {
                let cup = rv.cup;
                let d = match p {
                    Pending::Primary { attack, minute } => {
                        self.mark(cup, minute, PRIMARY);
                        dialogs::matchday::penalty(self, cup, src, minute, attack)
                    }
                    Pending::RedKeeper { minute } => {
                        self.mark(cup, minute, RED);
                        dialogs::matchday::substitution(self, cup, src, minute, 3)
                    }
                    Pending::Injury { minute, player } => {
                        self.mark(cup, minute, INJURY);
                        dialogs::matchday::forced(self, cup, src, minute, player)
                    }
                };
                return Tick::Ask(d);
            }
            rv.checked = rv.minute;
            if rv.minute == 45 && !rv.halftime_seen {
                if let Some(d) = self.halftime_dialog(rv, 46) {
                    return Tick::Ask(d);
                }
                rv.halftime_seen = true;
            }
            if rv.minute == 90 && rv.cup && !rv.cut {
                // the ties that were not drawn leave the list; the others go on
                rv.cut = true;
                rv.page = 0;
                if rv.shown().next().is_some() {
                    rv.max = 120;
                    rv.interval = round_view::interval(self.md.opts.time_profile, rv.humans());
                    if let Some(d) = self.halftime_dialog(rv, 91) {
                        return Tick::Ask(d);
                    }
                    rv.extra_seen = true;
                }
            }
        }
        rv.advance(ms);
        if rv.over() {
            Tick::Over
        } else {
            Tick::Run
        }
    }

    /// The dialog of the break for the user's game (`next`: the minute the substitutions made now take place, 46 or 91).
    fn halftime_dialog(&self, rv: &RoundView, next: u16) -> Option<Dialog> {
        if !self.md.opts.halftime_subs {
            return None;
        }
        let g = rv.shown().find(|g| g.human)?;
        let side = g.human_side?;
        Some(dialogs::matchday::halftime(self, rv.cup, g.src, side, next))
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
            Step::CupView => {
                if let Some(mut v) = RoundView::of_cup(&self.world) {
                    v.interval = round_view::interval(self.md.opts.time_profile, v.humans());
                    self.screen = Screen::Round(Box::new(v));
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

    pub fn is_invitation(&self) -> Option<(String, String)> {
        let (from, to) = self.world.human_moved.clone()?;
        let sess = self.md.session.as_ref()?;
        let mine = sess.snapshot.human.as_ref()?.club.clone();
        let invited = from == mine
            && self
                .world
                .coach_changes
                .iter()
                .any(|(sacked, gave)| *sacked == to && *gave == from);
        invited.then_some((from, to))
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
            c.md.opts.halftime_subs = false;
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
        use elifoot_core::sector::Status;
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
        let st = dialog_status_for_test(&info, side, 45);
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

    /// The status of the squad at the break, for the tests.
    fn dialog_status_for_test(info: &Info, side: usize, minute: u16) -> Vec<Status> {
        info.squad_at(side, minute, true)
    }

    #[test]
    fn many_rounds_with_a_user_who_answers_everything_stay_consistent() {
        let mut c = classic(2);
        let start = c.world.year;
        for _ in 0..16 {
            c.play_round();
            let seen = play_through(&mut c);
            assert!(seen.contains(&Kind::Halftime));
            assert!(matches!(c.screen, Screen::Main));
            assert!(c.md.todo.is_empty());
        }
        // 16 rounds: a season and two rounds; the end of the season was shown once
        assert!(c.world.year > start);
    }
}
