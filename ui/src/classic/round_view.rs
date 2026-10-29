//! The round view (`TJourneyDlg`): the games of a league round in the four divisions, or the ties of a cup round, with their running
//! scores and the latest event of each, while a pie clock runs from kick-off to the final whistle.
//!
//! The clock (`TGauge`, `gkPie`, `MaxValue` 60) turns 6 degrees per minute: blue for the first half (progress 0..45), red for the
//! second (progress 45..90 over the range 45..105), purple for extra time (90..120 over 90..150). The timer starts five ticks before
//! the kick-off (`minute = -5`) and the window closes five ticks after the last minute. The tick interval comes from the game-time
//! profile of the options.
use crate::win31::draw::text;
use crate::win31::fb::{Fb, Rgb, BLACK, GREEN, NAVY, SILVER, WHITE, YELLOW};
use crate::win31::Skin;
use rustfoot_core::matchsim::{EventKind, Side};
use rustfoot_core::season::MatchRecord;
use rustfoot_core::world::{CupMatch, World};

const BLUE: Rgb = Rgb(0, 0, 255);
const RED: Rgb = Rgb(255, 0, 0);
const PURPLE: Rgb = Rgb(128, 0, 128);

#[derive(Clone, Debug)]
pub struct Club {
    pub name: String,
    pub colors: (Rgb, Rgb),
}

#[derive(Clone, Debug)]
pub struct Ev {
    pub minute: u16,
    pub side: usize,
    pub kind: EventKind,
    pub player: String,
    /// The goal came from a primary chance (the original prints "(pen.)").
    pub primary: bool,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub attendance: i64,
    pub clubs: [Club; 2],
    pub events: Vec<Ev>,
    /// A human's club plays in this game.
    pub human: bool,
    /// Which sides of the game (0 home, 1 away) are humans' clubs: both when two humans play each other.
    pub human_sides: [bool; 2],
    /// Index into `world.last_round` (league) or the matches of `world.last_cup`.
    pub src: usize,
    /// Cup only: the tie goes on after 90 minutes (extra time); the others leave the view at minute 90.
    pub extra_time: bool,
}

impl Game {
    /// The score at `minute`.
    pub fn score(&self, minute: i32) -> [i32; 2] {
        let mut s = [0, 0];
        for e in &self.events {
            if e.kind == EventKind::Goal && e.minute as i32 <= minute {
                s[e.side] += 1;
            }
        }
        s
    }

    /// The latest goal, red card, injury or annulled goal up to `minute`.
    pub fn latest(&self, minute: i32) -> Option<&Ev> {
        self.events.iter().rfind(|e| e.minute as i32 <= minute)
    }
}

/// The game-time profiles of the options (`1060:3219`): `(per extra human, interval with a human, interval without)` in ms.
pub const TIME_PROFILES: [(u32, u32, u32); 6] = [
    (200, 1000, 100),
    (150, 800, 100),
    (100, 600, 100),
    (125, 400, 100),
    (50, 250, 100),
    (0, 50, 50),
];

/// Milliseconds per game minute for a profile (1..=6) and the number of human coaches in the games shown (`1048:0ba3`). Windows 3.1
/// fires a timer on the first tick of the 18.2 Hz PC clock after its interval, so the interval grows to whole ticks of 54.925 ms
/// (measured: two humans on *Médio*, 950 ms, ran 1092 ms a minute under js-dos, whose own drawing adds the rest).
pub fn interval(profile: usize, humans: usize) -> u32 {
    let (step, with, without) = TIME_PROFILES[profile.clamp(1, 6) - 1];
    let ms = if humans == 0 {
        without
    } else {
        with + (humans as u32 - 1) * step
    };
    let ticks = (ms * 1000).div_ceil(54925);
    (ticks * 54925 + 500) / 1000
}

#[derive(Clone, Debug)]
pub struct RoundView {
    pub title: String,
    /// A cup round: one list of ties, 20 to a page.
    pub cup: bool,
    /// Games of the divisions in table order (league).
    pub divisions: [Vec<Game>; 4],
    /// The ties (cup).
    pub games: Vec<Game>,
    /// Page of the cup list.
    pub page: usize,
    /// Elapsed minutes: -5 before the kick-off, 90 at the end (120 with extra time).
    pub minute: i32,
    pub acc_ms: u32,
    /// Milliseconds per minute.
    pub interval: u32,
    /// The last minute: 90, 120 once the extra time has begun.
    pub max: i32,
    /// The pause of the first half has been shown.
    pub halftime_seen: bool,
    /// The pause before the extra time has been shown.
    pub extra_seen: bool,
    /// Cup only: minute 90 passed and the games that ended have left the list.
    pub cut: bool,
    /// The last minute whose events (primary chances, injuries...) were already put to the user.
    pub checked: i32,
    /// Backdrop only: the green application window between the dialogs of the end of a round.
    pub backdrop: bool,
}

/// Players' short names of a record.
fn ev_of(e: &rustfoot_core::matchsim::Event, names: &[Vec<String>; 2]) -> Ev {
    let side = (e.side == Side::Away) as usize;
    Ev {
        minute: e.minute,
        side,
        kind: e.kind,
        player: names[side].get(e.player).cloned().unwrap_or_default(),
        primary: e.primary,
    }
}

fn club_of(world: &World, name: &str) -> Club {
    let c = world
        .divisions
        .iter()
        .flatten()
        .chain(world.pool.iter())
        .find(|c| c.name == name);
    match c {
        Some(c) => Club {
            name: c.name.clone(),
            // the form's colour is the EFT foreground, the lettering the EFT background
            colors: (
                Rgb(c.colors.1[0], c.colors.1[1], c.colors.1[2]),
                Rgb(c.colors.0[0], c.colors.0[1], c.colors.0[2]),
            ),
        },
        None => Club {
            name: name.to_string(),
            colors: (BLACK, WHITE),
        },
    }
}

impl RoundView {
    pub fn blank(title: String, cup: bool) -> RoundView {
        RoundView {
            title,
            cup,
            divisions: Default::default(),
            games: Vec::new(),
            page: 0,
            minute: -5,
            acc_ms: 0,
            interval: 800,
            max: 90,
            halftime_seen: false,
            extra_seen: false,
            cut: false,
            checked: 0,
            backdrop: false,
        }
    }

    /// The green window of the application while the dialogs of the end of a round show.
    pub fn backdrop() -> RoundView {
        let mut v = RoundView::blank(String::new(), false);
        v.backdrop = true;
        v
    }

    /// The view of the round just played (`world.last_round`).
    pub fn of_round(world: &World) -> RoundView {
        let mut v = RoundView::blank(format!("{}ª Jornada - {}", world.round, world.year), false);
        let humans = world.human_locations();
        for (k, (d, m)) in world.last_round.iter().enumerate() {
            let club = |i: usize| {
                let c = &world.divisions[*d][i];
                Club {
                    name: c.name.clone(),
                    colors: (
                        Rgb(c.colors.1[0], c.colors.1[1], c.colors.1[2]),
                        Rgb(c.colors.0[0], c.colors.0[1], c.colors.0[2]),
                    ),
                }
            };
            let human_sides = [
                humans.contains(&(*d, m.home)),
                humans.contains(&(*d, m.away)),
            ];
            v.divisions[*d].push(Game {
                attendance: m.attendance,
                clubs: [club(m.home), club(m.away)],
                events: m.events.iter().map(|e| ev_of(e, &m.names)).collect(),
                human: human_sides.iter().any(|&h| h),
                human_sides,
                src: k,
                extra_time: false,
            });
        }
        v
    }

    /// The view of the cup round just played (`world.last_cup`): the title is "Taça - round - year".
    pub fn of_cup(world: &World) -> Option<RoundView> {
        let cup = world.last_cup.as_ref().filter(|c| !c.matches.is_empty())?;
        let mut v = RoundView::blank(
            format!(
                "Taça - {} - {}",
                cup.name,
                world.year - (world.round == 0) as u16
            ),
            true,
        );
        let humans = world.human_clubs();
        for (k, m) in cup.matches.iter().enumerate() {
            let names = [m.home_players.clone(), m.away_players.clone()];
            let human_sides = [humans.contains(&m.home), humans.contains(&m.away)];
            v.games.push(Game {
                attendance: m.attendance,
                clubs: [club_of(world, &m.home), club_of(world, &m.away)],
                events: m
                    .events
                    .iter()
                    .map(|e| ev_of(e, &[names[0].clone(), names[1].clone()]))
                    .collect(),
                human: human_sides.iter().any(|&h| h),
                human_sides,
                src: k,
                extra_time: m.extra_time,
            });
        }
        Some(v)
    }

    /// Whether the user's club plays in this view.
    pub fn human_plays(&self) -> bool {
        self.shown().any(|g| g.human)
    }

    /// The games on the screen: all of them, or in a cup view after minute 90 only the ones in extra time.
    pub fn shown(&self) -> impl Iterator<Item = &Game> {
        let cut = self.cup && self.cut;
        self.divisions
            .iter()
            .flatten()
            .chain(self.games.iter())
            .filter(move |g| !cut || g.extra_time)
    }

    /// Number of human coaches in the games on the screen (the original counts both sides of every game).
    pub fn humans(&self) -> usize {
        self.shown()
            .map(|g| g.human_sides.iter().filter(|&&h| h).count())
            .sum()
    }

    /// Pages of 20 rows of a cup view minus one.
    pub fn last_page(&self) -> usize {
        let n = self.shown().count();
        n.saturating_sub(1) / 20
    }

    /// Time passes: the clock moves on by one minute when the interval of the game timer has elapsed (at most one per call, so
    /// that nothing the user is asked at a minute is skipped); returns whether it did.
    pub fn advance(&mut self, ms: u32) -> bool {
        self.acc_ms += ms;
        let step = self.interval.max(1);
        if self.acc_ms >= step && self.minute <= self.max + 5 {
            self.acc_ms -= step;
            self.minute += 1;
            return true;
        }
        false
    }

    /// The window closes five minutes after the end.
    pub fn over(&self) -> bool {
        self.minute > self.max + 5
    }
}

fn grouped(n: i64) -> String {
    n.to_string()
}

/// Colour of the pie and the progress in minutes of the phase (the gauge's `Progress`, over its range of 60).
fn clock_state(v: &RoundView) -> (Rgb, i32) {
    let m = v.minute.max(0);
    if v.extra_seen {
        (PURPLE, m - 90)
    } else if v.halftime_seen {
        (RED, m - 45)
    } else {
        (BLUE, m)
    }
}

/// Draws the whole screen. `active`: the window is the active one (no dialog on top), so the caption is blue.
pub fn draw(fb: &mut Fb, skin: &Skin, v: &RoundView, active: bool) {
    fb.fill(0, 0, 800, 600, GREEN);
    if v.backdrop {
        // the hidden application window: a silver caption bar across the screen (as behind the main window)
        fb.fill(0, 0, 800, 18, SILVER);
        fb.fill(0, 18, 800, 1, BLACK);
        if let Some(f) = skin.font("system", 13, false, false) {
            let w = f.width("Elifoot 98");
            text(fb, f, (800 - w) / 2 + 1, 1, "Elifoot 98", BLACK, false);
        }
        return;
    }
    fb.fill(0, 0, 800, 1, BLACK);
    fb.fill(0, 0, 1, 600, BLACK);
    let (bar, ink) = if active {
        (NAVY, WHITE)
    } else {
        (SILVER, BLACK)
    };
    fb.fill(1, 1, 799, 18, bar);
    fb.fill(0, 19, 800, 1, BLACK);
    if let Some(f) = skin.font("system", 13, false, false) {
        let w = f.width(&v.title);
        text(fb, f, (800 - w) / 2 + 2, 2, &v.title, ink, false);
    }
    let (Some(reg), Some(bold)) = (
        skin.font("arial", 13, false, false),
        skin.font("arial", 13, true, false),
    ) else {
        return;
    };
    let minute = v.minute.max(0);
    let navy = NAVY;
    if v.cup {
        let shown: Vec<&Game> = v.shown().collect();
        let first = v.page * 20;
        let page: Vec<&Game> = shown.iter().skip(first).take(20).copied().collect();
        // the frame of the list: from y = 79 to the last row
        let bottom = 79 + 22 * page.len() as i32 + 11;
        fb.fill(13, 79, 777, 1, navy);
        fb.fill(13, 79, 1, bottom - 79 + 1, navy);
        fb.fill(789, 79, 1, bottom - 79 + 1, navy);
        fb.fill(13, bottom, 777, 1, navy);
        for (r, g) in page.iter().enumerate() {
            row(fb, reg, bold, 92 + 22 * r as i32, g, minute);
        }
        if v.last_page() > 0 {
            page_buttons(fb, skin, v);
        }
    } else {
        // the frame of the four divisions
        for (d, top) in [79, 189, 299, 409].into_iter().enumerate() {
            fb.fill(13, top, 777, 1, navy);
            let bottom = if d == 3 { 520 } else { top + 110 };
            fb.fill(13, top, 1, bottom - top + 1, navy);
            fb.fill(789, top, 1, bottom - top + 1, navy);
            if d == 3 {
                fb.fill(13, 520, 777, 1, navy);
            }
            let title = format!("{}ª Divisão", d + 1);
            let tw = bold.width(&title);
            fb.fill(65, top, tw, 1, GREEN);
            text(fb, bold, 65, top - 8, &title, YELLOW, false);
            for (r, g) in v.divisions[d].iter().enumerate() {
                row(
                    fb,
                    reg,
                    bold,
                    92 + 110 * d as i32 + 22 * r as i32,
                    g,
                    minute,
                );
            }
        }
    }
    let (color, progress) = clock_state(v);
    clock(fb, color, progress);
}

/// The `<` and `>` buttons at the bottom right (visible only when there is more than one page).
fn page_buttons(fb: &mut Fb, skin: &Skin, v: &RoundView) {
    let Some(f) = skin.font("arial", 27, true, false) else {
        return;
    };
    for (k, (label, x, enabled)) in [
        ("<", 800 - 64 - 5, v.page > 0),
        (">", 800 - 32 - 5, v.page < v.last_page()),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = k;
        let y = 600 - 19 - 32 - 5;
        fb.fill(x, y, 32, 32, SILVER);
        fb.fill(x, y, 32, 1, WHITE);
        fb.fill(x, y, 1, 32, WHITE);
        fb.fill(x + 31, y, 1, 32, BLACK);
        fb.fill(x, y + 31, 32, 1, BLACK);
        let w = f.width(label);
        text(
            fb,
            f,
            x + (32 - w) / 2,
            y + 3,
            label,
            if enabled { BLACK } else { Rgb(128, 128, 128) },
            false,
        );
    }
}

/// Hit areas of the page buttons in screen coordinates: `(name, x, y)` (32x32).
pub fn button_at(v: &RoundView, x: i32, y: i32) -> Option<&'static str> {
    if !v.cup || v.last_page() == 0 {
        return None;
    }
    let top = 600 - 19 - 32 - 5;
    if !(top..top + 32).contains(&y) {
        return None;
    }
    if (800 - 64 - 5..800 - 32 - 5).contains(&x) {
        Some("previousGamePageBtn")
    } else if (800 - 32 - 5..800 - 5).contains(&x) {
        Some("nextGamePageBtn")
    } else {
        None
    }
}

/// The game and the side under a click at `(x, y)` (screen coordinates): the original opens the squad dialog of that club
/// (`FormMouseDown`: x within the home chip or the away chip of the row).
pub fn game_at(v: &RoundView, x: i32, y: i32) -> Option<(&Game, usize)> {
    let side = if (61..241).contains(&x) {
        0
    } else if (274..454).contains(&x) {
        1
    } else {
        return None;
    };
    if v.cup {
        let first = v.page * 20;
        for (r, g) in v.shown().skip(first).take(20).enumerate() {
            let top = 92 + 22 * r as i32;
            if (top..top + 20).contains(&y) {
                return Some((g, side));
            }
        }
    } else {
        for (d, base) in [92, 202, 312, 422].into_iter().enumerate() {
            for (r, g) in v.divisions[d].iter().enumerate() {
                let top = base + 22 * r as i32;
                if (top..top + 20).contains(&y) {
                    return Some((g, side));
                }
            }
        }
    }
    None
}

fn chip(fb: &mut Fb, font: &crate::win31::skin::Font, x: i32, y: i32, c: &Club) {
    let (bg, fg) = c.colors;
    if bg == GREEN {
        fb.fill(x, y, 180, 1, fg);
        fb.fill(x, y + 19, 180, 1, fg);
        fb.fill(x, y, 1, 20, fg);
        fb.fill(x + 179, y, 1, 20, fg);
    } else {
        fb.fill(x, y, 180, 20, bg);
    }
    text(fb, font, x + 4, y + 1, &c.name, fg, false);
}

fn row(
    fb: &mut Fb,
    reg: &crate::win31::skin::Font,
    bold: &crate::win31::skin::Font,
    y: i32,
    g: &Game,
    minute: i32,
) {
    let a = grouped(g.attendance);
    text(fb, reg, 58 - reg.width(&a), y, &a, YELLOW, false);
    chip(fb, reg, 61, y, &g.clubs[0]);
    chip(fb, reg, 274, y, &g.clubs[1]);
    let s = g.score(minute);
    text(fb, bold, 246, y + 1, &s[0].to_string(), YELLOW, false);
    text(fb, bold, 262, y + 1, &s[1].to_string(), YELLOW, false);
    if let Some(e) = g.latest(minute) {
        let mut x = 458;
        match e.kind {
            EventKind::RedCard => {
                fb.fill(457, y + 3, 14, 14, RED);
                x = 478;
            }
            EventKind::Injury => {
                fb.fill(457, y + 3, 14, 14, WHITE);
                fb.fill(462, y + 4, 3, 13, RED);
                fb.fill(458, y + 8, 13, 3, RED);
                x = 478;
            }
            _ => {}
        }
        let suffix = match e.kind {
            EventKind::Goal if e.primary => "' (pen.)",
            EventKind::Annulled => "' - Anulado",
            _ => "'",
        };
        text(
            fb,
            reg,
            x,
            y + 1,
            &format!("{} {}{}", e.player, e.minute, suffix),
            YELLOW,
            false,
        );
    }
}

/// The disc of the pie clock: `(first, last)` x offsets of every row of the 45x45 circle.
const DISC: [(i32, i32); 45] = [
    (18, 26),
    (15, 29),
    (12, 32),
    (11, 33),
    (9, 35),
    (8, 36),
    (7, 37),
    (6, 38),
    (5, 39),
    (4, 40),
    (4, 40),
    (3, 41),
    (2, 42),
    (2, 42),
    (2, 42),
    (1, 43),
    (1, 43),
    (1, 43),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (0, 44),
    (1, 43),
    (1, 43),
    (1, 43),
    (2, 42),
    (2, 42),
    (2, 42),
    (3, 41),
    (4, 40),
    (4, 40),
    (5, 39),
    (6, 38),
    (7, 37),
    (8, 36),
    (9, 35),
    (11, 33),
    (12, 32),
    (15, 29),
    (18, 26),
];

/// The line of Bresenham as the display driver draws it (ties go to the minor axis).
fn line(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    let (mut x, mut y) = (x0, y0);
    let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut pts = Vec::new();
    loop {
        pts.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 > dy {
            err += dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
    pts
}

/// The pie (`TGauge`, `gkPie`): the ring in the colour of the phase, and when the progress is more than zero a sector from 12 o'clock
/// clockwise. The gauge truncates the progress to whole per cent, takes the end of the sector at `Round(22 * (1 - cos/sin))` of the
/// angle `pi * (percent / 50 + 0.5)` and has GDI draw the pie: the pen draws the two radial lines in the colour.
fn clock(fb: &mut Fb, color: Rgb, progress: i32) {
    let (ox, oy) = (751, 25);
    let percent = (100 * progress.max(0)) / 60;
    let inside = |x: i32, y: i32| {
        y >= 0
            && DISC
                .get(y as usize)
                .is_some_and(|&(a, b)| (a..=b).contains(&x))
    };
    let mut on = [[false; 45]; 45];
    for (y, &(a, b)) in DISC.iter().enumerate() {
        for x in a..=b {
            let edge = !(inside(x - 1, y as i32)
                && inside(x + 1, y as i32)
                && inside(x, y as i32 - 1)
                && inside(x, y as i32 + 1));
            on[y][x as usize] = edge;
        }
    }
    if percent > 0 {
        let a = std::f64::consts::PI * (percent as f64 / 50.0 + 0.5);
        let sx = (22.0 * (1.0 - a.cos()) + 0.5).floor() as i32;
        let sy = (22.0 * (1.0 - a.sin()) + 0.5).floor() as i32;
        let mut th = ((sx - 22) as f64).atan2((22 - sy) as f64);
        if th < 0.0 {
            th += 2.0 * std::f64::consts::PI;
        }
        for (y, &(a0, b0)) in DISC.iter().enumerate() {
            for x in a0..=b0 {
                let mut t = ((x - 22) as f64).atan2(-((y as i32 - 22) as f64));
                if t < 0.0 {
                    t += 2.0 * std::f64::consts::PI;
                }
                if t <= th + 1e-9 {
                    on[y][x as usize] = true;
                }
            }
        }
        for (x, y) in line(22, 22, sx, sy).into_iter().chain(line(22, 22, 22, 0)) {
            if inside(x, y) {
                on[y as usize][x as usize] = true;
            }
        }
    }
    for (y, row) in on.iter().enumerate() {
        for (x, &c) in row.iter().enumerate() {
            if inside(x as i32, y as i32) {
                fb.set(ox + x as i32, oy + y as i32, if c { color } else { WHITE });
            }
        }
    }
}

/// Used by tests and the session: the record of a league game of the view.
pub fn record<'a>(world: &'a World, g: &Game) -> Option<&'a MatchRecord> {
    world.last_round.get(g.src).map(|(_, m)| m)
}

/// The cup tie of a game of a cup view.
pub fn tie<'a>(world: &'a World, g: &Game) -> Option<&'a CupMatch> {
    world.last_cup.as_ref().and_then(|c| c.matches.get(g.src))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(events: Vec<Ev>) -> Game {
        let club = |n: &str| Club {
            name: n.into(),
            colors: (BLACK, WHITE),
        };
        Game {
            attendance: 1000,
            clubs: [club("A"), club("B")],
            events,
            human: false,
            human_sides: [false; 2],
            src: 0,
            extra_time: false,
        }
    }

    fn ev(minute: u16, side: usize, kind: EventKind) -> Ev {
        Ev {
            minute,
            side,
            kind,
            player: "X".into(),
            primary: false,
        }
    }

    #[test]
    fn the_timer_interval_follows_the_profile_and_the_humans() {
        // Médio: 100 ms without a human, 800 ms with one, 150 ms more for every further human coach, each rounded up to whole
        // ticks of the PC clock
        assert_eq!(interval(2, 0), 110);
        assert_eq!(interval(2, 1), 824);
        assert_eq!(interval(2, 2), 989);
        assert_eq!(interval(1, 1), 1044);
        assert_eq!(interval(6, 1), 55);
        assert_eq!(interval(6, 0), 55);
        // out-of-range profiles fall back to the nearest one
        assert_eq!(interval(0, 1), 1044);
        assert_eq!(interval(9, 1), 55);
    }

    #[test]
    fn the_clock_starts_five_ticks_early_and_the_window_closes_five_ticks_late() {
        let mut v = RoundView::blank("t".into(), false);
        v.interval = 100;
        assert_eq!(v.minute, -5);
        for _ in 0..95 {
            v.advance(100);
        }
        assert_eq!(v.minute, 90);
        assert!(!v.over());
        for _ in 0..5 {
            v.advance(100);
            assert!(!v.over());
        }
        v.advance(100);
        assert!(v.over(), "minute {}", v.minute);
        // a tick shorter than the interval does not move the clock
        let mut w = RoundView::blank("t".into(), false);
        w.interval = 100;
        w.advance(60);
        assert_eq!(w.minute, -5);
        w.advance(60);
        assert_eq!(w.minute, -4);
    }

    #[test]
    fn the_pie_turns_six_degrees_a_minute_blue_red_and_purple() {
        let mut v = RoundView::blank("t".into(), false);
        v.minute = 30;
        assert_eq!(clock_state(&v), (BLUE, 30));
        v.minute = 45;
        assert_eq!(clock_state(&v), (BLUE, 45));
        v.halftime_seen = true;
        assert_eq!(clock_state(&v), (RED, 0));
        v.minute = 90;
        assert_eq!(clock_state(&v), (RED, 45));
        v.extra_seen = true;
        v.minute = 105;
        assert_eq!(clock_state(&v), (PURPLE, 15));
    }

    #[test]
    fn the_score_and_the_latest_event_follow_the_clock() {
        let g = game(vec![
            ev(10, 0, EventKind::Goal),
            ev(20, 1, EventKind::RedCard),
            ev(30, 1, EventKind::Goal),
            ev(31, 1, EventKind::Annulled),
        ]);
        assert_eq!(g.score(9), [0, 0]);
        assert_eq!(g.score(25), [1, 0]);
        // a goal that was annulled no longer counts
        assert_eq!(g.score(40), [1, 1]);
        assert_eq!(g.latest(5).map(|e| e.minute), None);
        assert_eq!(g.latest(25).map(|e| e.kind), Some(EventKind::RedCard));
        assert_eq!(g.latest(40).map(|e| e.kind), Some(EventKind::Annulled));
    }

    #[test]
    fn clicks_find_the_club_of_a_game_and_the_page_buttons() {
        let mut v = RoundView::blank("t".into(), false);
        for d in 0..4 {
            for _ in 0..4 {
                v.divisions[d].push(game(vec![]));
            }
        }
        // the first game of the first division: home chip, away chip, between them nothing
        assert!(matches!(game_at(&v, 100, 100), Some((_, 0))));
        assert!(matches!(game_at(&v, 300, 100), Some((_, 1))));
        assert!(game_at(&v, 250, 100).is_none());
        assert!(game_at(&v, 100, 85).is_none());
        // 4th division, last row
        assert!(game_at(&v, 100, 422 + 66 + 5).is_some());
        // a cup list of 45 ties has three pages and the buttons answer only then
        let mut c = RoundView::blank("t".into(), true);
        assert_eq!(button_at(&c, 770, 550), None);
        for _ in 0..45 {
            c.games.push(game(vec![]));
        }
        assert_eq!(c.last_page(), 2);
        assert_eq!(button_at(&c, 770, 550), Some("nextGamePageBtn"));
        assert_eq!(button_at(&c, 740, 550), Some("previousGamePageBtn"));
        c.page = 2;
        assert!(matches!(game_at(&c, 100, 92 + 22 * 4), Some((_, 0))));
        assert!(
            game_at(&c, 100, 92 + 22 * 5).is_none(),
            "five ties on the last page"
        );
    }

    #[test]
    fn after_minute_90_a_cup_list_keeps_only_the_ties_in_extra_time() {
        let mut v = RoundView::blank("t".into(), true);
        for k in 0..6 {
            let mut g = game(vec![]);
            g.extra_time = k % 3 == 0;
            v.games.push(g);
        }
        assert_eq!(v.shown().count(), 6);
        v.cut = true;
        assert_eq!(v.shown().count(), 2);
    }
}
