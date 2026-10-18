//! The round view: the games of the four divisions with their running scores and the latest event of each, while a pie clock runs
//! from kick-off to the final whistle.
use crate::win31::draw::text;
use crate::win31::fb::{Fb, Rgb, BLACK, GREEN, NAVY, WHITE, YELLOW};
use crate::win31::Skin;
use elifoot_core::matchsim::{EventKind, Side};
use elifoot_core::world::World;

const BLUE: Rgb = Rgb(0, 0, 255);
const RED: Rgb = Rgb(255, 0, 0);

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
}

#[derive(Clone, Debug)]
pub struct Game {
    pub attendance: i64,
    pub clubs: [Club; 2],
    pub events: Vec<Ev>,
    pub human: bool,
}

impl Game {
    /// The score at `minute`.
    pub fn score(&self, minute: u16) -> [i32; 2] {
        let mut s = [0, 0];
        for e in &self.events {
            if e.kind == EventKind::Goal && e.minute <= minute {
                s[e.side] += 1;
            }
        }
        s
    }

    /// The latest goal, red card or injury up to `minute`.
    pub fn latest(&self, minute: u16) -> Option<&Ev> {
        self.events
            .iter()
            .filter(|e| e.minute <= minute && e.kind != EventKind::Annulled)
            .last()
    }
}

#[derive(Clone, Debug)]
pub struct RoundView {
    pub title: String,
    /// Games of the divisions in table order.
    pub divisions: [Vec<Game>; 4],
    /// Elapsed minutes (0..=90) and the milliseconds toward the next one.
    pub minute: u16,
    pub acc_ms: u32,
}

/// Milliseconds per game minute while the clock runs.
pub const MINUTE_MS: u32 = 90;

impl RoundView {
    /// The view of the round just played (`world.last_round`).
    pub fn of_round(world: &World) -> RoundView {
        let mut divisions: [Vec<Game>; 4] = Default::default();
        let human = world.human_location();
        for (d, m) in &world.last_round {
            let club = |i: usize| {
                let c = &world.divisions[*d][i];
                Club {
                    name: c.name.clone(),
                    colors: (
                        Rgb(c.colors.0[0], c.colors.0[1], c.colors.0[2]),
                        Rgb(c.colors.1[0], c.colors.1[1], c.colors.1[2]),
                    ),
                }
            };
            let events = m
                .events
                .iter()
                .map(|e| {
                    let side = e.side == Side::Away;
                    Ev {
                        minute: e.minute,
                        side: side as usize,
                        kind: e.kind,
                        player: m.names[side as usize]
                            .get(e.player)
                            .cloned()
                            .unwrap_or_default(),
                    }
                })
                .collect();
            divisions[*d].push(Game {
                attendance: m.attendance,
                clubs: [club(m.home), club(m.away)],
                events,
                human: human == Some((*d, m.home)) || human == Some((*d, m.away)),
            });
        }
        RoundView {
            title: format!("{}ª Jornada - {}", world.round, world.year),
            divisions,
            minute: 0,
            acc_ms: 0,
        }
    }

    /// Advances the clock; false once the full time is reached.
    pub fn tick(&mut self, ms: u32) -> bool {
        self.acc_ms += ms;
        while self.acc_ms >= MINUTE_MS && self.minute < 90 {
            self.acc_ms -= MINUTE_MS;
            self.minute += 1;
        }
        self.minute < 90
    }
}

fn grouped(n: i64) -> String {
    n.to_string()
}

/// Draws the whole screen.
pub fn draw(fb: &mut Fb, skin: &Skin, v: &RoundView) {
    fb.fill(0, 0, 800, 600, GREEN);
    fb.fill(0, 0, 800, 1, BLACK);
    fb.fill(0, 0, 1, 600, BLACK);
    fb.fill(1, 1, 799, 18, NAVY);
    fb.fill(0, 19, 800, 1, BLACK);
    if let Some(f) = skin.font("system", 13, false, false) {
        let w = f.width(&v.title);
        text(fb, f, (800 - w) / 2 + 2, 2, &v.title, WHITE, false);
    }
    let (Some(reg), Some(bold)) = (
        skin.font("arial", 13, false, false),
        skin.font("arial", 13, true, false),
    ) else {
        return;
    };
    // the frame of the four divisions
    let navy = NAVY;
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
            row(fb, reg, bold, 92 + 110 * d as i32 + 22 * r as i32, g, v.minute);
        }
    }
    clock(fb, v.minute);
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
    minute: u16,
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
        text(fb, reg, x, y + 1, &format!("{} {}'", e.player, e.minute), YELLOW, false);
    }
}

/// The disc of the pie clock: `(first, last)` x offsets of every row of the 45x45 circle.
const DISC: [(i32, i32); 45] = [
    (18, 26), (15, 29), (12, 32), (11, 33), (9, 35), (8, 36), (7, 37), (6, 38), (5, 39), (4, 40), (4, 40), (3, 41), (2, 42),
    (2, 42), (2, 42), (1, 43), (1, 43), (1, 43), (0, 44), (0, 44), (0, 44), (0, 44), (0, 44), (0, 44), (0, 44), (0, 44),
    (0, 44), (1, 43), (1, 43), (1, 43), (2, 42), (2, 42), (2, 42), (3, 41), (4, 40), (4, 40), (5, 39), (6, 38), (7, 37),
    (8, 36), (9, 35), (11, 33), (12, 32), (15, 29), (18, 26),
];

fn clock(fb: &mut Fb, minute: u16) {
    let (ox, oy) = (751, 25);
    let angle = minute as f64 * 4.0;
    let inside = |x: i32, y: i32| {
        DISC.get(y as usize)
            .is_some_and(|&(a, b)| (a..=b).contains(&x))
    };
    for (y, &(a, b)) in DISC.iter().enumerate() {
        for x in a..=b {
            let (dx, dy) = (x as f64 - 22.0, y as f64 - 22.0);
            let mut t = dx.atan2(-dy).to_degrees();
            if t < 0.0 {
                t += 360.0;
            }
            let edge = !(inside(x - 1, y as i32)
                && inside(x + 1, y as i32)
                && inside(x, y as i32 - 1)
                && inside(x, y as i32 + 1));
            let blue = edge || t < angle;
            fb.set(ox + x, oy + y as i32, if blue { BLUE } else { WHITE });
        }
    }
}
