//! The screens: league, squad, finance and cup. Each one is a plain function from the game state to drawing commands.
use crate::canvas::{Canvas, Rect, Rgb};
use crate::{theme, App, HEIGHT, WIDTH};
use rustfoot_core::season::standings;
use rustfoot_core::sector::Status;

pub const PLAY_ROUND: Rect = Rect::new(560, 548, 110, 34);
pub const PLAY_SEASON: Rect = Rect::new(676, 548, 112, 34);

pub(crate) const POS: [&str; 4] = ["GR", "DEF", "MED", "AVA"];

pub(crate) fn money(v: i64) -> String {
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

pub(crate) fn panel(c: &mut dyn Canvas, r: Rect, title: &str) {
    c.fill(r, theme::panel());
    c.outline(r, theme::line());
    c.text(r.x + 10, r.y + 6, title, theme::accent(), 14);
}

pub(crate) fn footer(app: &mut App, c: &mut dyn Canvas) {
    c.text(12, 556, &app.message, theme::muted(), 13);
    c.text(
        12,
        576,
        "Espaço: jogar jornada · S: época · Tab: mudar ecrã · 1-4: divisão · roda: deslocar",
        theme::line(),
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
        if app.tab(c, r, name, app.division == d) {
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
        c.text(table.x + x, table.y + 28, h, theme::muted(), 12);
    }
    for (rank, i) in standings(clubs).into_iter().enumerate() {
        let k = &clubs[i];
        let y = table.y + 48 + rank as i32 * 24;
        let me = mine.as_deref() == Some(k.name.as_str());
        if me {
            c.fill(Rect::new(table.x + 4, y - 2, table.w - 8, 22), theme::sel());
        }
        let col = if rank < 2 {
            theme::good()
        } else if rank >= 6 {
            theme::bad()
        } else {
            theme::text()
        };
        c.text(table.x + 8, y, &format!("{}", rank + 1), theme::muted(), 13);
        c.text(table.x + 28, y, &k.name, col, 13);
        c.text(
            table.x + 250,
            y,
            &(k.wins + k.draws + k.losses).to_string(),
            theme::text(),
            13,
        );
        c.text(table.x + 280, y, &k.wins.to_string(), theme::text(), 13);
        c.text(table.x + 310, y, &k.draws.to_string(), theme::text(), 13);
        c.text(table.x + 340, y, &k.losses.to_string(), theme::text(), 13);
        c.text(
            table.x + 372,
            y,
            &format!("{}-{}", k.goals_for, k.goals_against),
            theme::text(),
            13,
        );
        c.text(table.x + 436, y, &k.points.to_string(), theme::accent(), 13);
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
            theme::text(),
            13,
        );
        c.text(
            results.x + 10,
            y + 16,
            &format!("{} espectadores", money(m.attendance)),
            theme::muted(),
            11,
        );
        y += 46;
    }
    if app.world.last_round.is_empty() {
        c.text(
            results.x + 10,
            y,
            "Ainda não foi jogada nenhuma jornada.",
            theme::muted(),
            13,
        );
    }
    referee_aid(app, c);
    let news = Rect::new(12, 352, 520, 186);
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
            theme::text(),
            12,
        );
        y += 20;
    }
    if app.world.transfers.is_empty() {
        c.text(
            news.x + 10,
            y,
            "Sem transferências ainda.",
            theme::muted(),
            13,
        );
    }
    footer(app, c);
}

/// `TRefereeAidDlg`: how much the referee helps each side in the user's next league game (the fixture bonuses of
/// `1010:c95c`: a higher bonus keeps the game calmer and makes a red card or an annulled goal less likely).
fn referee_aid(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(544, 352, 244, 186);
    panel(c, r, "Ajuda do árbitro");
    let next = app.world.round + 1;
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let fixture = (next <= 14)
        .then(|| rustfoot_core::season::fixtures(next))
        .and_then(|f| f.into_iter().find(|&(h, a)| h == i || a == i));
    let Some((h, a)) = fixture else {
        c.text(
            r.x + 10,
            r.y + 36,
            "Sem jogo na próxima jornada.",
            theme::muted(),
            13,
        );
        return;
    };
    let division = rustfoot_core::world::DIVISION_NAMES[d].as_bytes();
    let clubs = &app.world.divisions[d];
    let bonus_h =
        rustfoot_core::matchsim::fixture_value(division, clubs[h].name.as_bytes(), true) + 2;
    let bonus_a = rustfoot_core::matchsim::fixture_value(division, clubs[a].name.as_bytes(), true);
    for (n, (club, bonus)) in [(&clubs[h].name, bonus_h), (&clubs[a].name, bonus_a)]
        .into_iter()
        .enumerate()
    {
        let y = r.y + 40 + n as i32 * 62;
        let mine = if n == 0 { h == i } else { a == i };
        c.text(
            r.x + 10,
            y,
            &short(club),
            if mine { theme::accent() } else { theme::text() },
            13,
        );
        let gauge = Rect::new(r.x + 10, y + 22, r.w - 20, 16);
        c.fill(gauge, theme::bg());
        c.outline(gauge, theme::line());
        let w = (gauge.w - 2) * bonus.clamp(0, 14) / 14;
        c.fill(
            Rect::new(gauge.x + 1, gauge.y + 1, w, gauge.h - 2),
            if bonus >= 9 {
                theme::good()
            } else {
                theme::accent()
            },
        );
        c.text(r.x + r.w - 40, y, &bonus.to_string(), theme::muted(), 13);
    }
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
    c.text(r.x + 160, r.y + 6, &club.name, theme::text(), 14);
    for (h, x) in [
        ("Pos", 12),
        ("Jogador", 60),
        ("F", 330),
        ("Salário", 380),
        ("Papel", 480),
        ("Estado", 580),
    ] {
        c.text(r.x + x, r.y + 30, h, theme::muted(), 12);
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
            c.fill(Rect::new(r.x + 4, y - 2, r.w - 8, 22), theme::sel());
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
        c.text(r.x + 12, y, POS[s.position as usize], theme::muted(), 13);
        c.text(r.x + 60, y, &club.names[k], theme::text(), 13);
        c.text(r.x + 330, y, &s.strength.to_string(), theme::accent(), 13);
        c.text(r.x + 380, y, &money(s.wage), theme::text(), 13);
        c.text(r.x + 480, y, role, theme::text(), 13);
        c.text(r.x + 580, y, &state, theme::bad(), 13);
    }
    footer(app, c);
}

pub fn finance(app: &mut App, c: &mut dyn Canvas) {
    balance(app, c);
    let r = Rect::new(12, 54, 500, 484);
    panel(c, r, "Finanças");
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let f = &app.world.divisions[d][i].fin;
    let wage = rustfoot_core::finance::wage_bill(&app.world.divisions[d][i].slots);
    let rows: Vec<(&str, String, Rgb)> = vec![
        ("Caixa", money(f.cash), theme::text()),
        ("Bilhetes (época)", money(f.gate), theme::good()),
        ("Salários (época)", money(f.wages), theme::bad()),
        ("Salários por jogo", money(-wage), theme::bad()),
        ("Prémios", money(f.prizes), theme::good()),
        ("Vendas", money(f.sales), theme::good()),
        ("Compras", money(f.purchases), theme::bad()),
        ("Empréstimo", money(f.loan), theme::text()),
        (
            "Estádio",
            format!(
                "{} lugares (nível {}), {}% cheio",
                money(f.capacity()),
                f.stadium,
                (f.fill * 100.0).round()
            ),
            theme::text(),
        ),
    ];
    for (n, (k, v, col)) in rows.iter().enumerate() {
        let y = r.y + 36 + n as i32 * 30;
        c.text(r.x + 14, y, k, theme::muted(), 14);
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

/// The balance sheet of the season (`TBalanceDlg`): income, expenses and the result, from the club's ledger.
fn balance(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(524, 54, 264, 484);
    panel(c, r, "Balanço da época");
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let f = &app.world.divisions[d][i].fin;
    let income = [
        ("Bilheteira", f.gate),
        ("Vendas de jogadores", f.sales),
        ("Prémios", f.prizes),
    ];
    let spent = [
        ("Salários", f.wages),
        ("Compras de jogadores", f.purchases),
        ("Juros", f.interest),
        ("Obras no estádio", f.works),
    ];
    let mut y = r.y + 34;
    let mut total = 0;
    for (title, rows) in [("Receitas", &income[..]), ("Despesas", &spent[..])] {
        c.text(r.x + 10, y, title, theme::accent(), 14);
        y += 24;
        for (k, v) in rows {
            c.text(r.x + 14, y, k, theme::muted(), 13);
            let w = c.text_width(&money(*v), 13);
            c.text(r.x + r.w - 14 - w, y, &money(*v), theme::text(), 13);
            total += v;
            y += 22;
        }
        y += 10;
    }
    c.fill(Rect::new(r.x + 10, y, r.w - 20, 1), theme::line());
    y += 8;
    let col = if total >= 0 {
        theme::good()
    } else {
        theme::bad()
    };
    c.text(r.x + 14, y, "Resultado", theme::text(), 14);
    let w = c.text_width(&money(total), 14);
    c.text(r.x + r.w - 14 - w, y, &money(total), col, 14);
    c.text(
        r.x + 14,
        y + 28,
        &format!("Caixa: {}", money(f.cash)),
        theme::muted(),
        13,
    );
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
                theme::muted(),
                14,
            );
        }
        Some(cup) => {
            c.text(
                r.x + 160,
                r.y + 6,
                &format!("{} — restam {} clubes", cup.name, cup.left),
                theme::text(),
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
                    theme::text(),
                    13,
                );
            }
        }
    }
    footer(app, c);
}

#[allow(dead_code)]
const _: (i32, i32) = (WIDTH, HEIGHT);
