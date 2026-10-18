//! The screens: league, squad, finance and cup. Each one is a plain function from the game state to drawing commands.
use crate::canvas::{Canvas, Rect, Rgb};
use crate::{theme, App, HEIGHT, WIDTH};
use elifoot_core::season::standings;
use elifoot_core::sector::Status;

pub const PLAY_ROUND: Rect = Rect::new(560, 548, 110, 34);
pub const PLAY_SEASON: Rect = Rect::new(676, 548, 112, 34);

const POS: [&str; 4] = ["GR", "DEF", "MED", "AVA"];

fn money(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(ch);
    }
    if v < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn panel(c: &mut dyn Canvas, r: Rect, title: &str) {
    c.fill(r, theme::PANEL);
    c.outline(r, theme::LINE);
    c.text(r.x + 10, r.y + 6, title, theme::ACCENT, 14);
}

fn footer(app: &mut App, c: &mut dyn Canvas) {
    c.text(12, 556, &app.message, theme::MUTED, 13);
    c.text(
        12,
        576,
        "Espaço: jogar jornada · S: época · Tab: mudar ecrã · 1-4: divisão · roda: deslocar",
        theme::LINE,
        11,
    );
    if app.button(c, PLAY_ROUND, "Jogar jornada") {
        app.play_round();
    }
    if app.button(c, PLAY_SEASON, "Jogar época") {
        app.play_season();
    }
}

pub fn league(app: &mut App, c: &mut dyn Canvas) {
    let names = ["1ª Divisão", "2ª Divisão", "3ª Divisão", "4ª Divisão"];
    for (d, name) in names.iter().enumerate() {
        let r = Rect::new(12 + d as i32 * 108, 54, 102, 26);
        c.fill(
            r,
            if app.division == d {
                theme::ACCENT
            } else if app.hot(r) {
                theme::BUTTON_HOT
            } else {
                theme::BUTTON
            },
        );
        let col = if app.division == d {
            theme::BG
        } else {
            theme::TEXT
        };
        c.text(r.x + 8, r.y + 6, name, col, 13);
        if app.hot(r) && app.released_now() {
            app.division = d;
        }
    }
    let division = app.division.min(3);
    let table = Rect::new(12, 90, 470, 250);
    panel(c, table, "Classificação");
    let mine = app.world.human.as_ref().map(|h| h.club.clone());
    let clubs = &app.world.divisions[division];
    let heads = [
        ("Clube", 20),
        ("J", 250),
        ("V", 280),
        ("E", 310),
        ("D", 340),
        ("Golos", 372),
        ("Pts", 436),
    ];
    for (h, x) in heads {
        c.text(table.x + x, table.y + 28, h, theme::MUTED, 12);
    }
    for (rank, i) in standings(clubs).into_iter().enumerate() {
        let k = &clubs[i];
        let y = table.y + 48 + rank as i32 * 24;
        let me = mine.as_deref() == Some(k.name.as_str());
        if me {
            c.fill(
                Rect::new(table.x + 4, y - 2, table.w - 8, 22),
                theme::BUTTON,
            );
        }
        let col = if rank < 2 {
            theme::GOOD
        } else if rank >= 6 {
            theme::BAD
        } else {
            theme::TEXT
        };
        c.text(table.x + 8, y, &format!("{}", rank + 1), theme::MUTED, 13);
        c.text(table.x + 28, y, &k.name, col, 13);
        c.text(
            table.x + 250,
            y,
            &(k.wins + k.draws + k.losses).to_string(),
            theme::TEXT,
            13,
        );
        c.text(table.x + 280, y, &k.wins.to_string(), theme::TEXT, 13);
        c.text(table.x + 310, y, &k.draws.to_string(), theme::TEXT, 13);
        c.text(table.x + 340, y, &k.losses.to_string(), theme::TEXT, 13);
        c.text(
            table.x + 372,
            y,
            &format!("{}-{}", k.goals_for, k.goals_against),
            theme::TEXT,
            13,
        );
        c.text(table.x + 436, y, &k.points.to_string(), theme::ACCENT, 13);
    }
    let results = Rect::new(494, 90, 294, 250);
    panel(c, results, "Última jornada");
    let mut y = results.y + 32;
    for (d, m) in app.world.last_round.iter().filter(|(d, _)| *d == division) {
        let (h, a) = (
            &app.world.divisions[*d][m.home],
            &app.world.divisions[*d][m.away],
        );
        c.text(
            results.x + 10,
            y,
            &format!(
                "{} {}-{} {}",
                short(&h.name),
                m.home_goals,
                m.away_goals,
                short(&a.name)
            ),
            theme::TEXT,
            13,
        );
        c.text(
            results.x + 10,
            y + 16,
            &format!("{} espectadores", money(m.attendance)),
            theme::MUTED,
            11,
        );
        y += 46;
    }
    if app.world.last_round.is_empty() {
        c.text(
            results.x + 10,
            y,
            "Ainda não foi jogada nenhuma jornada.",
            theme::MUTED,
            13,
        );
    }
    let news = Rect::new(12, 352, 776, 186);
    panel(c, news, "Transferências recentes");
    let mut y = news.y + 30;
    for t in app.world.transfers.iter().rev().take(7) {
        c.text(
            news.x + 10,
            y,
            &format!(
                "{} · J{}  {} ({}) {} → {}  {}",
                t.year,
                t.round,
                t.player,
                t.strength,
                t.from,
                t.to,
                money(t.fee)
            ),
            theme::TEXT,
            12,
        );
        y += 20;
    }
    if app.world.transfers.is_empty() {
        c.text(
            news.x + 10,
            y,
            "Sem transferências ainda.",
            theme::MUTED,
            13,
        );
    }
    footer(app, c);
}

fn short(name: &str) -> String {
    name.chars().take(14).collect()
}

pub fn squad(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(12, 54, 776, 484);
    panel(c, r, "O meu plantel");
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let club = &app.world.divisions[d][i];
    c.text(r.x + 160, r.y + 6, &club.name, theme::TEXT, 14);
    for (h, x) in [
        ("Pos", 12),
        ("Jogador", 60),
        ("F", 330),
        ("Salário", 380),
        ("Papel", 480),
        ("Estado", 580),
    ] {
        c.text(r.x + x, r.y + 30, h, theme::MUTED, 12);
    }
    let mut idx: Vec<usize> = (0..club.slots.len()).collect();
    idx.sort_by_key(|&k| {
        (
            club.slots[k].position,
            std::cmp::Reverse(club.slots[k].strength),
        )
    });
    let rows = 17;
    let first = (app.scroll / 24).min(idx.len().saturating_sub(rows as usize) as i32) as usize;
    for (n, &k) in idx.iter().skip(first).take(rows as usize).enumerate() {
        let s = &club.slots[k];
        let y = r.y + 50 + n as i32 * 24;
        if s.status == Status::Starter {
            c.fill(Rect::new(r.x + 4, y - 2, r.w - 8, 22), theme::BUTTON);
        }
        let role = match s.status {
            Status::Starter => "Titular",
            Status::Sub => "Banco",
            Status::Out => "Fora",
        };
        let state = if s.injured > 0 {
            format!("Lesionado {} j", s.injured)
        } else if s.ban > 0 {
            format!("Suspenso {} j", s.ban)
        } else {
            String::new()
        };
        c.text(r.x + 12, y, POS[s.position as usize], theme::MUTED, 13);
        c.text(r.x + 60, y, &club.names[k], theme::TEXT, 13);
        c.text(r.x + 330, y, &s.strength.to_string(), theme::ACCENT, 13);
        c.text(r.x + 380, y, &money(s.wage), theme::TEXT, 13);
        c.text(r.x + 480, y, role, theme::TEXT, 13);
        c.text(r.x + 580, y, &state, theme::BAD, 13);
    }
    footer(app, c);
}

pub fn finance(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(12, 54, 500, 484);
    panel(c, r, "Finanças");
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let f = &app.world.divisions[d][i].fin;
    let wage = elifoot_core::finance::wage_bill(&app.world.divisions[d][i].slots);
    let rows: Vec<(&str, String, Rgb)> = vec![
        ("Caixa", money(f.cash), theme::TEXT),
        ("Bilhetes (época)", money(f.gate), theme::GOOD),
        ("Salários (época)", money(f.wages), theme::BAD),
        ("Salários por jogo", money(-wage), theme::BAD),
        ("Prémios", money(f.prizes), theme::GOOD),
        ("Vendas", money(f.sales), theme::GOOD),
        ("Compras", money(f.purchases), theme::BAD),
        ("Empréstimo", money(f.loan), theme::TEXT),
        (
            "Estádio",
            format!(
                "{} lugares (nível {}), {}% cheio",
                money(f.capacity()),
                f.stadium,
                (f.fill * 100.0).round()
            ),
            theme::TEXT,
        ),
    ];
    for (n, (k, v, col)) in rows.iter().enumerate() {
        let y = r.y + 36 + n as i32 * 30;
        c.text(r.x + 14, y, k, theme::MUTED, 14);
        c.text(r.x + 210, y, v, *col, 14);
    }
    let b = Rect::new(r.x + 14, r.y + 36 + rows.len() as i32 * 30 + 12, 150, 30);
    if app.button(c, b, "Pedir empréstimo") {
        app.message = match app.world.human_borrow() {
            Ok(v) => format!("Empréstimo de {} concedido.", money(v)),
            Err(e) => e,
        };
    }
    let b2 = Rect::new(b.x + 160, b.y, 150, 30);
    if app.button(c, b2, "Amortizar") {
        app.message = match app.world.human_repay() {
            Ok(v) => format!("Amortizaste {}.", money(v)),
            Err(e) => e,
        };
    }
    let b3 = Rect::new(b2.x + 160, b.y, 160, 30);
    if app.button(c, b3, "Ampliar estádio") {
        app.message = match app.world.human_expand() {
            Ok(v) => format!("Obras por {}.", money(v)),
            Err(e) => e,
        };
    }
    footer(app, c);
}

pub fn cup(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(12, 54, 776, 484);
    panel(c, r, "Taça");
    match &app.world.last_cup {
        None => {
            c.text(
                r.x + 14,
                r.y + 40,
                "Ainda não houve sorteio nesta época.",
                theme::MUTED,
                14,
            );
        }
        Some(cup) => {
            c.text(
                r.x + 160,
                r.y + 6,
                &format!("{} — restam {} clubes", cup.name, cup.left),
                theme::TEXT,
                14,
            );
            for (n, m) in cup.matches.iter().take(18).enumerate() {
                let y = r.y + 36 + n as i32 * 24;
                let how = match (&m.pens, m.shootout, m.extra_time) {
                    (Some(p), _, _) => format!(" (g.p. {}-{})", p.home, p.away),
                    (None, true, _) => " (g.p.)".to_string(),
                    (None, false, true) => " (a.p.)".to_string(),
                    _ => String::new(),
                };
                c.text(
                    r.x + 14,
                    y,
                    &format!(
                        "{} {}-{} {}{}",
                        m.home, m.home_goals, m.away_goals, m.away, how
                    ),
                    theme::TEXT,
                    13,
                );
            }
        }
    }
    footer(app, c);
}

#[allow(dead_code)]
const _: (i32, i32) = (WIDTH, HEIGHT);
