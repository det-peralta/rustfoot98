//! The dialogs of the original that are not plain tables: line-up and substitutions (`Táctica`), the transfer market
//! (auctions, player search, selling) and the options. Same rules as the DOM page: every action goes through `World`.
use crate::canvas::{Canvas, Rect};
use crate::screens::{footer, money, panel, POS};
use crate::{theme, App};
use elifoot_core::economy::player_value;
use elifoot_core::season::{Plan, PlannedSub, FORMATIONS};

/// Where a player stands in a plan: 0 not selected, 1 starter, 2 bench.
fn role(plan: &Plan, i: usize) -> u8 {
    if plan.starters.contains(&i) {
        1
    } else if plan.bench.contains(&i) {
        2
    } else {
        0
    }
}

/// Swaps the roles of two squad members (starter, substitute or out); planned substitutions that no longer make sense
/// are dropped.
pub(crate) fn swap_roles(plan: &mut Plan, a: usize, b: usize) {
    let (ra, rb) = (role(plan, a), role(plan, b));
    let slot = |v: &mut Vec<usize>, from: usize, to: usize| {
        if let Some(p) = v.iter().position(|&x| x == from) {
            v[p] = to;
        }
    };
    match (ra, rb) {
        (1, 2) => {
            slot(&mut plan.starters, a, b);
            slot(&mut plan.bench, b, a);
        }
        (2, 1) => {
            slot(&mut plan.starters, b, a);
            slot(&mut plan.bench, a, b);
        }
        (1, 0) => slot(&mut plan.starters, a, b),
        (0, 1) => slot(&mut plan.starters, b, a),
        (2, 0) => slot(&mut plan.bench, a, b),
        (0, 2) => slot(&mut plan.bench, b, a),
        _ => {}
    }
    let (starters, bench) = (plan.starters.clone(), plan.bench.clone());
    plan.subs
        .retain(|s| starters.contains(&s.out) && bench.contains(&s.inn));
}

fn next_in(list: &[usize], current: usize) -> usize {
    match list.iter().position(|&x| x == current) {
        Some(p) => list[(p + 1) % list.len()],
        None => list[0],
    }
}

pub fn tactics(app: &mut App, c: &mut dyn Canvas) {
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let club = app.world.divisions[d][i].clone();
    let Some(human) = app.world.human.clone() else {
        return;
    };
    let mut plan = human.plan.clone();
    let mut changed = false;

    let left = Rect::new(12, 54, 440, 484);
    panel(c, left, "Plantel · clica em dois jogadores para os trocar");
    let mut idx: Vec<usize> = (0..club.slots.len()).collect();
    idx.sort_by_key(|&k| {
        (
            club.slots[k].position,
            std::cmp::Reverse(club.slots[k].strength),
        )
    });
    let rows = 17usize;
    let first = (app.scroll / 24).min(idx.len().saturating_sub(rows) as i32) as usize;
    for (n, &k) in idx.iter().skip(first).take(rows).enumerate() {
        let s = &club.slots[k];
        let y = left.y + 34 + n as i32 * 26;
        let row = Rect::new(left.x + 4, y - 2, left.w - 8, 24);
        let fill = if app.selected == Some(k) {
            theme::ACCENT
        } else if role(&plan, k) == 1 {
            theme::BUTTON
        } else if app.hot(row) {
            theme::BUTTON_HOT
        } else {
            theme::PANEL
        };
        c.fill(row, fill);
        let ink = if app.selected == Some(k) {
            theme::BG
        } else {
            theme::TEXT
        };
        let label = match role(&plan, k) {
            1 => "Titular",
            2 => "Banco",
            _ => "Fora",
        };
        let state = if s.injured > 0 {
            format!("lesionado {} j", s.injured)
        } else if s.ban > 0 {
            format!("suspenso {} j", s.ban)
        } else {
            String::new()
        };
        c.text(left.x + 12, y, POS[s.position as usize], theme::MUTED, 13);
        c.text(left.x + 50, y, &club.names[k], ink, 13);
        c.text(left.x + 250, y, &s.strength.to_string(), theme::ACCENT, 13);
        c.text(left.x + 290, y, label, ink, 13);
        c.text(left.x + 350, y, &state, theme::BAD, 12);
        if app.hot(row) && app.released_now() {
            match app.selected {
                None => app.selected = Some(k),
                Some(a) if a == k => app.selected = None,
                Some(a) => {
                    let becoming_active =
                        |from: usize, to: usize| role(&plan, from) == 0 && role(&plan, to) != 0;
                    let blocked = (becoming_active(k, a) && !club.slots[k].available())
                        || (becoming_active(a, k) && !club.slots[a].available());
                    if blocked {
                        app.message = "Um jogador lesionado ou suspenso não pode jogar.".into();
                    } else {
                        swap_roles(&mut plan, a, k);
                        changed = true;
                    }
                    app.selected = None;
                }
            }
        }
    }

    let right = Rect::new(464, 54, 324, 190);
    panel(c, right, "Esquema");
    for (n, f) in FORMATIONS.iter().enumerate() {
        let r = Rect::new(
            right.x + 10 + (n % 3) as i32 * 104,
            right.y + 32 + (n / 3) as i32 * 36,
            98,
            30,
        );
        let label = format!("{}-{}-{}", f.0, f.1, f.2);
        let active = human.formation == n;
        if active {
            c.fill(Rect::new(r.x - 2, r.y - 2, r.w + 4, r.h + 4), theme::ACCENT);
        }
        if app.button(c, r, &label) {
            if app.world.set_formation(n) {
                app.message = format!("Esquema {label}.");
                return;
            }
            app.message = "O plantel não permite esse esquema.".into();
        }
    }

    let subs = Rect::new(464, 256, 324, 282);
    panel(c, subs, "Substituições (máx. 3)");
    let name = |k: usize| club.names.get(k).cloned().unwrap_or_default();
    let mut remove = None;
    for n in 0..plan.subs.len() {
        let y = subs.y + 34 + n as i32 * 62;
        let s = plan.subs[n];
        c.text(subs.x + 10, y, "Minuto", theme::MUTED, 12);
        if app.button(c, Rect::new(subs.x + 64, y - 4, 26, 24), "-") {
            plan.subs[n].minute = s.minute.saturating_sub(5).max(1);
            changed = true;
        }
        c.text(subs.x + 98, y, &s.minute.to_string(), theme::TEXT, 13);
        if app.button(c, Rect::new(subs.x + 124, y - 4, 26, 24), "+") {
            plan.subs[n].minute = (s.minute + 5).min(90);
            changed = true;
        }
        if app.button(c, Rect::new(subs.x + 286, y - 4, 28, 24), "x") {
            remove = Some(n);
        }
        if app.button(
            c,
            Rect::new(subs.x + 10, y + 22, 148, 24),
            &format!("Sai {}", name(s.out)),
        ) && !plan.starters.is_empty()
        {
            plan.subs[n].out = next_in(&plan.starters, s.out);
            changed = true;
        }
        if app.button(
            c,
            Rect::new(subs.x + 166, y + 22, 148, 24),
            &format!("Entra {}", name(s.inn)),
        ) && !plan.bench.is_empty()
        {
            plan.subs[n].inn = next_in(&plan.bench, s.inn);
            changed = true;
        }
    }
    if let Some(n) = remove {
        plan.subs.remove(n);
        changed = true;
    }
    if plan.subs.len() < 3
        && !plan.bench.is_empty()
        && !plan.starters.is_empty()
        && app.button(
            c,
            Rect::new(subs.x + 10, subs.y + subs.h - 40, 180, 28),
            "Adicionar substituição",
        )
    {
        plan.subs.push(PlannedSub {
            minute: 60,
            out: plan.starters[plan.starters.len() - 1],
            inn: plan.bench[0],
        });
        changed = true;
    }
    let counts = [0u8, 1, 2, 3].map(|p| {
        plan.starters
            .iter()
            .filter(|&&k| club.slots[k].position == p)
            .count()
    });
    c.text(
        subs.x + 200,
        subs.y + subs.h - 34,
        &format!("{}GR {}-{}-{}", counts[0], counts[1], counts[2], counts[3]),
        theme::MUTED,
        13,
    );
    if changed {
        app.world.set_plan(plan);
    }
    footer(app, c);
}

fn parse_amount(s: &str) -> Option<i64> {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

pub fn market(app: &mut App, c: &mut dyn Canvas) {
    let Some((d, i)) = app.world.human_location() else {
        return;
    };
    let club = app.world.divisions[d][i].clone();

    // auctions of computer-managed clubs' players
    let board = Rect::new(12, 54, 380, 224);
    panel(c, board, "Leilões abertos");
    let listings = app.world.board.clone();
    for (n, l) in listings.iter().take(5).enumerate() {
        let y = board.y + 32 + n as i32 * 38;
        c.text(
            board.x + 10,
            y,
            &format!("{} ({})", l.player, l.seller),
            theme::TEXT,
            13,
        );
        let best = l
            .best
            .as_ref()
            .map(|(who, fee)| format!("melhor {} ({})", money(*fee), who))
            .unwrap_or_else(|| "sem licitações".into());
        c.text(
            board.x + 10,
            y + 16,
            &format!("pedido {} · {}", money(l.ask), best),
            theme::MUTED,
            11,
        );
        let amount = l
            .best
            .as_ref()
            .map_or(l.ask, |(_, b)| (b + 1000).max(l.ask));
        if app.button(
            c,
            Rect::new(board.x + board.w - 120, y, 110, 28),
            &format!("Licitar {}", money(amount)),
        ) {
            app.message = match app.world.human_bid(n, amount) {
                Ok(()) => format!("Licitação de {} aceite.", money(amount)),
                Err(e) => e,
            };
            break;
        }
    }
    if listings.is_empty() {
        c.text(
            board.x + 10,
            board.y + 34,
            "Nenhum leilão aberto. Joga uma jornada.",
            theme::MUTED,
            13,
        );
    }

    // selling
    let sell = Rect::new(12, 290, 380, 248);
    panel(c, sell, "Vender jogadores");
    app.text_field(
        c,
        "ask",
        Rect::new(sell.x + 150, sell.y + 4, 220, 24),
        "preço pedido (vazio = valor)",
    );
    let asked = app.fields.get("ask").and_then(|s| parse_amount(s));
    let mut idx: Vec<usize> = (0..club.slots.len()).collect();
    idx.sort_by_key(|&k| std::cmp::Reverse(club.slots[k].strength));
    let rows = 7usize;
    let first = (app.scroll2 / 28).min(idx.len().saturating_sub(rows) as i32) as usize;
    for (n, &k) in idx.iter().skip(first).take(rows).enumerate() {
        let s = &club.slots[k];
        let y = sell.y + 38 + n as i32 * 28;
        c.text(
            sell.x + 10,
            y + 4,
            POS[s.position as usize],
            theme::MUTED,
            12,
        );
        c.text(sell.x + 42, y + 4, &club.names[k], theme::TEXT, 13);
        c.text(
            sell.x + 220,
            y + 4,
            &s.strength.to_string(),
            theme::ACCENT,
            13,
        );
        let value = player_value(s.strength, club.fin.f, app.world.economy, app.world.scale);
        if app.button(c, Rect::new(sell.x + 264, y, 106, 24), "Vender") {
            let ask = asked.unwrap_or(value);
            app.message = match app.world.human_sell(k, ask) {
                Ok(fee) => format!("{} vendido por {}.", club.names[k], money(fee)),
                Err(e) => e,
            };
            break;
        }
    }

    // search and direct purchase
    let find = Rect::new(404, 54, 384, 484);
    panel(c, find, "Procurar jogadores");
    app.text_field(
        c,
        "name",
        Rect::new(find.x + 10, find.y + 32, 200, 26),
        "nome",
    );
    let pos_label = match app.query.position {
        None => "Qualquer posição".to_string(),
        Some(p) => POS[p as usize].to_string(),
    };
    if app.button(c, Rect::new(find.x + 220, find.y + 32, 154, 26), &pos_label) {
        app.query.position = match app.query.position {
            None => Some(0),
            Some(3) => None,
            Some(p) => Some(p + 1),
        };
    }
    c.text(find.x + 10, find.y + 70, "Força mínima", theme::MUTED, 13);
    if app.button(c, Rect::new(find.x + 120, find.y + 64, 28, 26), "-") {
        app.query.min_strength = app.query.min_strength.saturating_sub(1);
    }
    c.text(
        find.x + 156,
        find.y + 70,
        &app.query.min_strength.to_string(),
        theme::TEXT,
        13,
    );
    if app.button(c, Rect::new(find.x + 188, find.y + 64, 28, 26), "+") {
        app.query.min_strength = (app.query.min_strength + 1).min(60);
    }
    app.text_field(
        c,
        "maxprice",
        Rect::new(find.x + 226, find.y + 64, 148, 26),
        "preço máx.",
    );
    if app.button(c, Rect::new(find.x + 10, find.y + 98, 120, 28), "Procurar") {
        app.query.name = app.fields.get("name").cloned().unwrap_or_default();
        app.query.max_price = app.fields.get("maxprice").and_then(|s| parse_amount(s));
        app.offers = app.world.search_offers(&app.query);
        app.scroll = 0;
        app.message = format!("{} jogadores encontrados.", app.offers.len());
    }
    c.text(
        find.x + 140,
        find.y + 104,
        &format!("{} compras a chegar", app.world.pending.len()),
        theme::MUTED,
        12,
    );
    let rows = 9usize;
    let first = (app.scroll / 24).min(app.offers.len().saturating_sub(rows) as i32) as usize;
    let shown: Vec<_> = app.offers.iter().skip(first).take(rows).cloned().collect();
    for (n, o) in shown.iter().enumerate() {
        let y = find.y + 140 + n as i32 * 36;
        c.text(
            find.x + 10,
            y,
            &format!("{} · {} {}", o.player, POS[o.position as usize], o.strength),
            theme::TEXT,
            13,
        );
        c.text(
            find.x + 10,
            y + 16,
            &format!("{} · {}", o.club, String::from_utf8_lossy(&o.nationality)),
            theme::MUTED,
            11,
        );
        if app.button(
            c,
            Rect::new(find.x + find.w - 130, y, 120, 28),
            &money(o.price),
        ) {
            app.message = match app.world.human_buy(&o.club, o.index, &o.player) {
                Ok(fee) => format!(
                    "{} comprado por {}; chega no fim da jornada.",
                    o.player,
                    money(fee)
                ),
                Err(e) => e,
            };
            app.offers = app.world.search_offers(&app.query);
            break;
        }
    }
    if app.offers.is_empty() {
        c.text(
            find.x + 10,
            find.y + 146,
            "Define os critérios e carrega em Procurar.",
            theme::MUTED,
            13,
        );
    }
    footer(app, c);
}

pub fn options(app: &mut App, c: &mut dyn Canvas) {
    let r = Rect::new(12, 54, 776, 484);
    panel(c, r, "Opções e controlo");
    let on = app.world.accept_invitations;
    let label = format!(
        "Aceitar convites de treinador: {}",
        if on { "sim" } else { "não" }
    );
    if app.button(c, Rect::new(r.x + 14, r.y + 34, 330, 30), &label) {
        app.world.accept_invitations = !on;
    }
    c.text(
        r.x + 360,
        r.y + 42,
        &format!("Índice de inflação: {:.4}", app.world.economy),
        theme::MUTED,
        13,
    );

    let mut loans: Vec<(&str, i64)> = Vec::new();
    let mut debts: Vec<(&str, i64)> = Vec::new();
    for club in app.world.divisions.iter().flatten() {
        if club.fin.loan > 0 {
            loans.push((club.name.as_str(), club.fin.loan));
        }
        if club.fin.cash < 0 {
            debts.push((club.name.as_str(), club.fin.cash));
        }
    }
    loans.sort_by_key(|&(_, v)| std::cmp::Reverse(v));
    debts.sort_by_key(|&(_, v)| v);
    for (col, (title, list)) in [
        ("Equipas com empréstimos", &loans),
        ("Equipas endividadas", &debts),
    ]
    .iter()
    .enumerate()
    {
        let x = r.x + 14 + col as i32 * 380;
        c.text(x, r.y + 84, title, theme::ACCENT, 14);
        for (n, (name, v)) in list.iter().take(8).enumerate() {
            c.text(x, r.y + 110 + n as i32 * 22, name, theme::TEXT, 13);
            c.text(
                x + 230,
                r.y + 110 + n as i32 * 22,
                &money(*v),
                theme::MUTED,
                13,
            );
        }
        if list.is_empty() {
            c.text(x, r.y + 110, "Nenhuma.", theme::MUTED, 13);
        }
    }
    c.text(
        r.x + 14,
        r.y + 304,
        "Treinadores (última jornada)",
        theme::ACCENT,
        14,
    );
    let mut y = r.y + 330;
    if let Some((from, to)) = &app.world.human_moved {
        c.text(
            r.x + 14,
            y,
            &format!("O teu treinador passou de {from} para {to}."),
            theme::GOOD,
            13,
        );
        y += 22;
    }
    for (a, b) in app.world.coach_changes.iter().take(6) {
        c.text(
            r.x + 14,
            y,
            &format!("{a} trocou de treinador com {b}."),
            theme::TEXT,
            13,
        );
        y += 22;
    }
    if app.world.coach_changes.is_empty() && app.world.human_moved.is_none() {
        c.text(r.x + 14, y, "Sem mudanças de treinador.", theme::MUTED, 13);
    }
    footer(app, c);
}
