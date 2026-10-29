//! Buying, selling and contracts: the Vender / Contrato / Mensagem pages of the main window, the auction dialog
//! (`TSellPlayerDlg`), the salary demand (`TAskMoreSalaryDlg`), the player search (`TCheckPlayerDlg`), the squad viewer
//! (`TPlantelDlg`) and the list of watched players (`TObservedPlayersDlg`).
//!
//! The rules are `rustfoot-core`'s (`World::human_sell`, `human_bid`, `human_buy`, `human_propose_wage`, `search_players`...); this
//! module only moves their data into the forms' texts and lists and answers clicks and keys. The behaviour of the original comes
//! from the decompiled handlers (`1018:3cf0 Vender1Click`, `453c sellBtnClick`, `46ee newSalaryBtnClick`, `1008:2fec`
//! `AuctionDialogInit`, `1048:29f0` / `34f5` / `3223` for the search, `1048:13fa` for the squad viewer); see the market section of
//! `docs/classic-dialogs.md`.
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::{main_win, text, Classic, Key, Page};
use crate::win31::fb::Rgb;
use rustfoot_core::season::Club;
use rustfoot_core::world::{Offer, PlayerQuery, PriceFilter, WageReply, SEARCH_LIMIT};
use std::collections::BTreeMap;

/// Name of the `Kind::Other` dialog that asks whether the user accepts the salary a player demands.
pub const ASK_MORE_SALARY: &str = "AskMoreSalary";
/// Players the user can watch at the same time (`1010:8cbe`).
pub const MAX_OBSERVED: usize = 15;
/// How long the result of an auction stays on the screen when nobody closes it (milliseconds).
const RESULT_MS: i64 = 3500;

/// State of the three notebook pages of the main window that this module drives.
#[derive(Default)]
pub struct Pages {
    /// Text of `textSellPrice` (thousands of the currency, digits only).
    pub sell_price: String,
    /// Text of `spinNewSalary`.
    pub spin: String,
    /// Text of `textMessage`.
    pub message: String,
    /// A copy of the original's registration state: the squad viewer button of the opponent page is only there in registered copies
    /// (feature `0x10` of `1050:30bc`). The goldens come from an unregistered copy, so the default is `false`.
    pub registered: bool,
    /// The player chosen in a dialog that was opened from the squad viewer (club, player): the viewer shows him when it is on top again.
    pub pick: Option<(String, String)>,
}

fn rgb_s(c: [u8; 3]) -> String {
    format!("{},{},{}", c[0], c[1], c[2])
}

fn rgb_of(c: [u8; 3]) -> Rgb {
    Rgb(c[0], c[1], c[2])
}

/// Digits of `s` as a number (the originals read edits with `Val`; anything else is an error).
pub fn parse_digits(s: &str) -> Option<i64> {
    let t = s.trim();
    (!t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
        .then(|| t.parse().ok())
        .flatten()
}

/// `sellBtnClick`: the text of the price field in units (the field is in thousands; empty means no minimum). `None` for text that
/// is not a number.
pub fn sell_price_of(text: &str) -> Option<i64> {
    if text.trim().is_empty() {
        return Some(0);
    }
    parse_digits(text).map(|v| v.saturating_mul(1000))
}

/// The ranges of the strength combo box of the search (`1048:29f0`).
pub const SKILL_RANGES: [(u16, u16); 6] =
    [(1, 10), (5, 15), (10, 20), (20, 30), (30, 40), (40, 50)];

/// The price ranges of the search, by combo item `1..=7` (item 0 is `Qualquer`): `(lo, hi)` in units, the last without a bound.
/// The limits are `50 000, 200 000, 500 000, 1M, 2.5M, 5M` times the currency scale.
pub fn price_ranges(scale: i64) -> [(i64, i64); 7] {
    let k = scale;
    [
        (1, 50_000 * k),
        (50_000 * k, 200_000 * k),
        (200_000 * k, 500_000 * k),
        (500_000 * k, 1_000_000 * k),
        (1_000_000 * k, 2_500_000 * k),
        (2_500_000 * k, 5_000_000 * k),
        (5_000_000 * k, i64::MAX),
    ]
}

/// The texts of the price combo box, in the original's words.
pub fn price_items(scale: i64) -> Vec<String> {
    let r = price_ranges(scale);
    let mut v = vec!["Qualquer".to_string()];
    v.push(format!(" Até {}", text::int(r[0].1)));
    for x in &r[1..6] {
        v.push(format!(" De {} até {}", text::int(x.0), text::int(x.1)));
    }
    v.push(format!(" Mais de {}", text::int(r[6].0)));
    v
}

/// The price filter of the combo item `i`.
pub fn price_filter(scale: i64, item: Option<usize>) -> PriceFilter {
    match item {
        None => PriceFilter::Any,
        Some(0) => PriceFilter::ForSale,
        Some(i) => {
            let (lo, hi) = price_ranges(scale)[(i - 1).min(6)];
            PriceFilter::Between(lo, hi)
        }
    }
}

fn skill_items() -> Vec<String> {
    SKILL_RANGES
        .iter()
        .map(|(a, b)| format!("{a} a {b}"))
        .collect()
}

fn country_name(c: &Classic, code: &[u8; 3]) -> String {
    let k = String::from_utf8_lossy(code).to_string();
    c.data.countries.get(&k).cloned().unwrap_or(k)
}

/// An amount in words in the game's currency.
fn money(c: &Classic, v: i64) -> String {
    text::money_words(v, c.world.currency_info().unit)
}

fn suspended_text(n: u8) -> String {
    match n {
        0 => String::new(),
        1 => "Suspenso por 1 jogo".into(),
        n => format!("Suspenso por {n} jogos"),
    }
}

fn injured_text(n: u8) -> String {
    match n {
        0 => String::new(),
        1 => "Lesionado por 1 jogo".into(),
        n => format!("Lesionado por {n} jogos"),
    }
}

/// The selected row of the squad list of the main window as an index into the user's squad.
fn selected_slot(c: &Classic) -> Option<usize> {
    let (_, club) = main_win::human_club(c)?;
    let row = c.main.selected?;
    main_win::roster_order(&club).get(row).copied()
}

/// The sentence the auction dialog shows for the sale of a player: the buyer and the fee, or why nobody bought him.
fn sale_sentence(c: &Classic, sold: Result<i64, String>) -> String {
    match sold {
        Ok(fee) => {
            let to = c
                .world
                .transfers
                .last()
                .map(|t| t.to.clone())
                .unwrap_or_default();
            format!("Vendido ao {to} por {}", money(c, fee))
        }
        Err(e) => e,
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// The notebook pages of the main window
// ---------------------------------------------------------------------------------------------------------------------------

/// Shows `text` on the Mensagem page (`textMessage`).
pub fn show_message_page(c: &mut Classic, text: &str) {
    c.main.market.message = text.to_string();
    c.main.page = Some(Page::Message);
    c.dirty = true;
}

/// `Vender1Click` (`1018:3cf0`): the menu entry Jogador > Vender for the selected player. A player who cannot be sold gets the
/// reason on the Mensagem page, otherwise the Vender page opens with an empty price.
pub fn vender(c: &mut Classic) {
    let Some(slot) = selected_slot(c) else {
        return;
    };
    match c.world.sale_blocker_text(slot) {
        None => {
            c.main.market.sell_price.clear();
            c.main.page = Some(Page::Sell);
        }
        Some(why) => show_message_page(c, why),
    }
}

/// `ChangeSalaryBtnClick` (`1018:3f69`): the Contrato page for the selected player, with his wage in the spin edit. Nothing happens
/// for a player who already has a fresh contract.
fn change_salary(c: &mut Classic) {
    let Some(slot) = selected_slot(c) else {
        return;
    };
    let Some((_, club)) = main_win::human_club(c) else {
        return;
    };
    let s = &club.slots[slot];
    if s.fresh {
        return;
    }
    let (lo, hi) = c.world.wage_bounds();
    c.main.market.spin = s.wage.clamp(lo, hi).to_string();
    c.main.page = Some(Page::Contract);
}

/// Texts the pages add to the main window's view.
pub fn page_texts(c: &Classic, page: Page, t: &mut BTreeMap<String, String>) {
    t.insert(
        "opponentPlantelBtn.visible".into(),
        (c.main.market.registered as u8).to_string(),
    );
    let m = &c.main.market;
    match page {
        Page::Sell => {
            t.insert("textSellPrice".into(), m.sell_price.clone());
            t.insert(
                "textSellPrice.caret".into(),
                m.sell_price.chars().count().to_string(),
            );
            t.insert("@focus".into(), "textSellPrice".into());
        }
        Page::Contract => {
            t.insert("spinNewSalary.value".into(), m.spin.clone());
            t.insert("@focus".into(), "spinNewSalary".into());
        }
        Page::Message => {
            t.insert("textMessage".into(), m.message.clone());
        }
        _ => {}
    }
}

/// A control of the main window was pressed; true when this module handled it. Called before the main window's own handling.
pub fn main_click(c: &mut Classic, name: &str) -> bool {
    let page = c.main.page.unwrap_or(Page::Game);
    if page == Page::Message && !name.starts_with("TabSet#") {
        // the message goes away with the next click
        c.main.page = Some(Page::Player);
        return true;
    }
    match name {
        "ChangeSalaryBtn" => change_salary(c),
        "cancelSellBtn" | "cancelNewSalaryBtn" => c.main.page = Some(Page::Player),
        "sellBtn" => sell_clicked(c),
        "newSalaryBtn" => propose_clicked(c),
        "spinNewSalary" => spin_clicked(c),
        _ => return false,
    }
    true
}

/// The arrows of the spin edit: the right-hand strip, upper half up, lower half down (one unit a click, like the VCL's default).
fn spin_clicked(c: &mut Classic) {
    let (mx, my) = c.mouse;
    let Some(h) = c.hits.iter().rev().find(|h| h.name == "spinNewSalary") else {
        return;
    };
    if mx < h.x + h.w - 18 {
        return;
    }
    let up = my < h.y + h.h / 2;
    let (lo, hi) = c.world.wage_bounds();
    let v = parse_digits(&c.main.market.spin).unwrap_or(lo);
    let v = if up { v + 1 } else { v - 1 };
    c.main.market.spin = v.clamp(lo, hi).to_string();
}

/// A key while one of the pages is up; true when it was used.
pub fn main_key(c: &mut Classic, k: Key) -> bool {
    let page = c.main.page.unwrap_or(Page::Game);
    match page {
        Page::Message => {
            if matches!(k, Key::Enter | Key::Escape | Key::Char(' ')) {
                c.main.page = Some(Page::Player);
                return true;
            }
            false
        }
        Page::Sell => {
            match k {
                Key::Enter => sell_clicked(c),
                Key::Escape => c.main.page = Some(Page::Player),
                Key::Backspace => {
                    c.main.market.sell_price.pop();
                }
                Key::Char(ch) if ch.is_ascii_digit() && c.main.market.sell_price.len() < 6 => {
                    c.main.market.sell_price.push(ch)
                }
                _ => return false,
            }
            true
        }
        Page::Contract => {
            let (lo, hi) = c.world.wage_bounds();
            match k {
                Key::Enter => propose_clicked(c),
                Key::Escape => c.main.page = Some(Page::Player),
                Key::Backspace => {
                    c.main.market.spin.pop();
                }
                Key::Up | Key::Down => {
                    let v = parse_digits(&c.main.market.spin).unwrap_or(lo);
                    let v = if k == Key::Up { v + 1 } else { v - 1 };
                    c.main.market.spin = v.clamp(lo, hi).to_string();
                }
                Key::Char(ch) if ch.is_ascii_digit() && c.main.market.spin.len() < 6 => {
                    c.main.market.spin.push(ch)
                }
                _ => return false,
            }
            true
        }
        _ => false,
    }
}

/// `sellBtnClick` (`1018:453c`): offers the selected player at the price of the field (0: whatever the best bid is).
fn sell_clicked(c: &mut Classic) {
    let Some(slot) = selected_slot(c) else {
        return;
    };
    let Some(price) = sell_price_of(&c.main.market.sell_price) else {
        return;
    };
    if let Some(d) = sell_player(c, slot, price) {
        c.open(d);
    }
}

/// Puts the user's player up for sale and returns the auction dialog with its outcome; the first page comes back.
fn sell_player(c: &mut Classic, slot: usize, price: i64) -> Option<Dialog> {
    let (_, club) = main_win::human_club(c)?;
    let (club_name, player) = (club.name.clone(), club.names[slot].clone());
    let mut d = auction_dialog(c, &club_name, &player, price)?;
    let sold = c.world.human_sell(slot, price);
    let result = sale_sentence(c, sold);
    show_result(&mut d, &result);
    c.main.page = Some(Page::Game);
    c.main.selected = None;
    Some(d)
}

/// `newSalaryBtnClick` (`1018:46ee`): the user proposes the wage of the spin edit.
fn propose_clicked(c: &mut Classic) {
    let Some(slot) = selected_slot(c) else {
        return;
    };
    let (lo, _) = c.world.wage_bounds();
    let wage = parse_digits(&c.main.market.spin).unwrap_or(lo);
    match c.world.human_propose_wage(slot, wage) {
        Ok(WageReply::Accepted) => c.main.page = Some(Page::Player),
        Ok(WageReply::Demand { amount, can_sell }) => {
            if let Some(d) = ask_more_salary(c, slot, amount, can_sell) {
                c.open(d);
            }
        }
        Err(e) => show_message_page(c, &e),
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// The salary demand (TAskMoreSalaryDlg, 1030:330e)
// ---------------------------------------------------------------------------------------------------------------------------

/// The dialog in which a player refuses the proposed wage and demands `amount`: mode 2 (accept, or he is put up for auction) or
/// mode 3 (OK only: the club may not sell him).
pub fn ask_more_salary(c: &Classic, slot: usize, amount: i64, can_sell: bool) -> Option<Dialog> {
    let (_, club) = main_win::human_club(c)?;
    let mut d = Dialog::new(Kind::Other(ASK_MORE_SALARY.into()), "TASKMORESALARYDLG");
    d.set_num("slot", slot as i64);
    d.set_num("amount", amount);
    d.set_num("can_sell", can_sell as i64);
    d.bg = Some(rgb_of(club.colors.1));
    let fg = rgb_s(club.colors.0);
    d.text("textCoachName", c.coach.clone());
    d.text("textCoachName.color", fg.clone());
    d.text("textMessage.color", fg);
    let unit = c.world.currency_info().unit;
    let body = if can_sell {
        format!(
            "Nem pensar! Exijo {amount} {unit}!\nSe não aceitar o aumento, o jogador será posto à venda em leilão.\n\nAceita o aumento?"
        )
    } else {
        format!(
            "Nem pensar! Exijo {amount} {unit}!\n\nUma vez que este jogador não pode ser vendido, tem de aceitar o aumento."
        )
    };
    d.text("textMessage", body);
    d.text("YesBtn.visible", (can_sell as u8).to_string());
    d.text("NoBtn.visible", (can_sell as u8).to_string());
    d.text("okBtn.visible", (!can_sell as u8).to_string());
    d.focus = if can_sell { "YesBtn" } else { "okBtn" }.into();
    Some(d)
}

fn ask_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    let slot = d.num("slot") as usize;
    match name {
        "YesBtn" | "okBtn" => {
            let _ = c.world.human_accept_wage(slot, d.num("amount"));
            c.main.page = Some(Page::Player);
            Outcome::Close
        }
        "NoBtn" => {
            // refused: he goes to auction at any price, back to the first page
            match sell_player(c, slot, 0) {
                Some(r) => Outcome::Replace(Box::new(r)),
                None => Outcome::Close,
            }
        }
        _ => Outcome::Keep,
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// The auction dialog (TSellPlayerDlg, 1008:2fec)
// ---------------------------------------------------------------------------------------------------------------------------

/// The auction form filled with the player of `club` named `player`: his club's colours, position, strength, wage he asks, the base
/// `price`, behaviour and history, and the user's own panel (coach, cash, offer edit). The panel is a bidding panel until
/// [`show_result`] swaps it for the sentence that closes an auction.
pub fn auction_dialog(c: &Classic, club: &str, player: &str, base: i64) -> Option<Dialog> {
    let loc = c.world.find_club(club)?;
    let seller = c.world.club_of(loc);
    let idx = seller.names.iter().position(|n| n == player)?;
    let s = &seller.slots[idx];
    let value = rustfoot_core::economy::player_value(
        s.strength,
        seller.fin.f,
        c.world.economy,
        c.world.scale,
    );
    let mut d = Dialog::new(Kind::Auction, "TSELLPLAYERDLG");
    d.st.insert("mode".into(), "bid".into());
    d.st.insert("seller".into(), club.into());
    d.st.insert("player".into(), player.into());
    d.set_num("base", base);
    d.text("textTeamName", seller.name.clone());
    d.text("textTeamName.color", rgb_s(seller.colors.0));
    d.text("textTeamName.bgcolor", rgb_s(seller.colors.1));
    d.text("textTeamName.autosize", "1");
    // the form of this dialog is shown with a client area exactly as declared
    d.text("@client", "580x340");
    d.text("textPlayerName", player);
    d.text("textPlayerPosition", text::POSITIONS[s.position as usize]);
    d.text("textPlayerSkill", s.strength.to_string());
    d.text(
        "textDesiredSalary",
        format!("{value} {}", c.world.currency_info().unit),
    );
    d.text("textBasePrice", money(c, base));
    d.text("textPlayerCountry", country_name(c, &s.nat));
    d.text(
        "bitmapTeamCountry.image",
        format!("FLAGS/{}", String::from_utf8_lossy(&s.nat)),
    );
    d.text("LabelSuspended", suspended_text(s.ban));
    d.text("labelInjured", injured_text(s.injured));
    d.text("LabelSuspended.visible", ((s.ban > 0) as u8).to_string());
    d.text("labelInjured.visible", ((s.injured > 0) as u8).to_string());
    d.text("textPlayerBehaviour", text::behaviour(s.aggression));
    d.text("textSeasonGoals", s.goals.to_string());
    d.text("textHistoryGamesPlayed", s.history[0].to_string());
    d.text("textHistoryGoals", s.history[1].to_string());
    d.text("textHistoryRedCards", s.history[2].to_string());
    d.text("textHistoryInjuries", s.history[3].to_string());
    d.text("textTransferred", "");
    d.text("textHumanOffer", "");
    d.text("Panel1", "");
    d.text(
        "Label7",
        format!("Oferta (milhares de {}):", c.world.currency_info().unit),
    );
    d.text("textCoachName", c.coach.clone());
    if let Some((_, mine)) = main_win::human_club(c) {
        d.text(
            "textCash",
            format!("Dinheiro em caixa: {}", money(c, mine.fin.cash)),
        );
        d.text("PanelHumanOffer.bgcolor", rgb_s(mine.colors.1));
        d.text("PanelHumanOffer.color", rgb_s(mine.colors.0));
        d.text("textCoachName.color", rgb_s(mine.colors.0));
    }
    d.focus = "textHumanOffer".into();
    Some(d)
}

/// Ends the auction in the dialog: the bidding panel goes away and `result` is shown.
fn show_result(d: &mut Dialog, result: &str) {
    d.st.insert("mode".into(), "result".into());
    d.set_num("t", 0);
    d.text("textTransferred", result);
    d.text("PanelHumanOffer.visible", "0");
}

/// One auction dialog for every open auction of the board the user can still take part in, in the order the clubs offered them.
/// The match-day code calls this at the end of a round; the dialogs are shown one after the other.
pub fn queue_auctions(c: &mut Classic) {
    // an auction is offered to every human who can take part, the first player first, before the next auction comes (as the
    // original does); the main window goes to each human when his dialog shows
    let back = c.world.active_seat();
    let n = c.world.human_count();
    let mut wanted: Vec<(usize, String, String, i64, usize)> = Vec::new();
    for seat in 0..n {
        c.world.switch_human(seat);
        for i in 0..c.world.board.len() {
            if c.world.listing_open_to_user(i) {
                let l = &c.world.board[i];
                wanted.push((i, l.seller.clone(), l.player.clone(), l.ask, seat));
            }
        }
    }
    wanted.sort_by_key(|w| (w.0, w.4));
    for (_, club, player, ask, seat) in wanted {
        c.world.switch_human(seat);
        if let Some(mut d) = auction_dialog(c, &club, &player, ask) {
            d.st.insert("seat".into(), seat.to_string());
            c.queue.push_back(d);
        }
    }
    c.world.switch_human(back);
}

fn auction_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if name != "OKBtn" {
        return Outcome::Keep;
    }
    if d.st.get("mode").map(String::as_str) != Some("bid") {
        return Outcome::Close;
    }
    // `OKBtnClick` (1008:3b91): the offer is in thousands
    let offer = d.texts.get("textHumanOffer").cloned().unwrap_or_default();
    let Some(v) = parse_digits(&offer) else {
        // nothing typed: the user lets the auction go; nonsense is ignored
        return if offer.trim().is_empty() {
            Outcome::Close
        } else {
            Outcome::Keep
        };
    };
    let amount = v.saturating_mul(1000);
    let seller = d.st.get("seller").cloned().unwrap_or_default();
    let player = d.st.get("player").cloned().unwrap_or_default();
    let Some(i) = c
        .world
        .board
        .iter()
        .position(|l| l.seller == seller && l.player == player)
    else {
        return Outcome::Close;
    };
    match c.world.human_bid(i, amount) {
        Ok(()) => {
            let me = main_win::human_club(c)
                .map(|(_, m)| m.name)
                .unwrap_or_default();
            show_result(d, &format!("Vendido ao {me} por {}", money(c, amount)));
            Outcome::Keep
        }
        Err(e) => {
            c.message("Elifoot 98", &e);
            Outcome::Keep
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// The player search (TCheckPlayerDlg, 1048:29f0 / 34f5 / 3223)
// ---------------------------------------------------------------------------------------------------------------------------

/// Jogador > Procurar...: the search dialog, with no filter chosen.
pub fn open_search(c: &Classic, from_plantel: bool) -> Dialog {
    let _ = c;
    let mut d = Dialog::new(Kind::Search, "TCHECKPLAYERDLG");
    d.focus = "textName".into();
    d.text("textName", "");
    d.text("checkBoxNotMyTeam.checked", "1");
    if from_plantel {
        d.st.insert("from_plantel".into(), "1".into());
    }
    d
}

const COMBOS: [&str; 4] = ["comboSkill", "comboPosition", "comboPrice", "comboCountry"];

fn sel_of(d: &Dialog, combo: &str) -> Option<usize> {
    d.st.get(combo).and_then(|v| v.parse().ok())
}

fn set_sel(d: &mut Dialog, combo: &str, v: Option<usize>) {
    match v {
        Some(i) => d.st.insert(combo.into(), i.to_string()),
        None => d.st.remove(combo),
    };
    d.set_num("changed", 1);
}

/// The countries of the country combo box, by name.
fn countries(c: &Classic) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = c
        .data
        .countries
        .iter()
        .map(|(k, n)| (n.clone(), k.clone()))
        .collect();
    v.sort_by_key(|(n, _)| n.to_lowercase());
    v
}

fn combo_items(c: &Classic, combo: &str) -> Vec<String> {
    match combo {
        "comboSkill" => skill_items(),
        "comboPosition" => text::POSITIONS.iter().map(|s| s.to_string()).collect(),
        "comboPrice" => price_items(c.world.scale),
        _ => countries(c).into_iter().map(|(n, _)| n).collect(),
    }
}

/// The player row of the lists of the search and the observed players: position letter, name, club, strength, price, nationality
/// and behaviour.
fn offer_row(c: &Classic, o: &Offer) -> String {
    let aggression = c
        .world
        .find_club(&o.club)
        .and_then(|loc| {
            c.world
                .club_of(loc)
                .slots
                .get(o.index)
                .map(|s| s.aggression)
        })
        .unwrap_or(0);
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        ["G", "D", "M", "A"][o.position as usize],
        o.player,
        o.club,
        o.strength,
        if o.price > 0 {
            text::int(o.price)
        } else {
            String::new()
        },
        String::from_utf8_lossy(&o.nationality),
        ["Fair", "Cor", "Cav", "Can", "Cac", "Sar"][aggression.min(5) as usize]
    )
}

fn found_of(d: &Dialog) -> Vec<(String, String)> {
    d.st.get("found")
        .map(|s| {
            s.lines()
                .filter_map(|l| l.split_once('\u{1}'))
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn query_of(c: &Classic, d: &Dialog) -> PlayerQuery {
    let nationality = sel_of(d, "comboCountry")
        .and_then(|i| countries(c).get(i).map(|(_, k)| k.clone()))
        .map(|k| rustfoot_core::market::code(&k));
    PlayerQuery {
        name: d.texts.get("textName").cloned().unwrap_or_default(),
        nationality,
        strength: sel_of(d, "comboSkill").and_then(|i| SKILL_RANGES.get(i).copied()),
        position: sel_of(d, "comboPosition").map(|i| i as u8),
        price: price_filter(c.world.scale, sel_of(d, "comboPrice")),
        hide_mine: d
            .texts
            .get("checkBoxNotMyTeam.checked")
            .is_none_or(|v| v != "0"),
    }
}

fn offer_at(c: &Classic, club: &str, index: usize) -> Option<Offer> {
    let loc = c.world.find_club(club)?;
    let cl = c.world.club_of(loc);
    let s = cl.slots.get(index)?;
    Some(Offer {
        club: cl.name.clone(),
        index,
        player: cl.names[index].clone(),
        position: s.position,
        strength: s.strength,
        nationality: s.nat,
        price: c.world.price_for_user(club, index),
    })
}

fn search_bind(c: &Classic, d: &mut Dialog) {
    // CharCase = ecUpperCase
    if let Some(t) = d.texts.get_mut("textName") {
        *t = t.to_uppercase();
    }
    let name = d.texts.get("textName").cloned().unwrap_or_default();
    d.texts
        .insert("textName.caret".into(), name.chars().count().to_string());
    if d.st.get("lastname") != Some(&name) {
        d.st.insert("lastname".into(), name.clone());
        d.set_num("changed", 1);
    }
    if !d.st.contains_key("changed") || d.num("changed") != 0 {
        d.texts.insert("findBtn.enabled".into(), "1".into());
    }
    for combo in COMBOS {
        let items = combo_items(c, combo);
        let sel = sel_of(d, combo);
        d.texts.insert(
            format!("{combo}.text"),
            sel.and_then(|i| items.get(i).cloned()).unwrap_or_default(),
        );
        d.texts
            .insert(format!("{combo}.sel"), sel.unwrap_or(0).to_string());
        d.lists.insert(combo.to_string(), items);
    }
    if let Some((_, me)) = main_win::human_club(c) {
        d.texts.insert("textMoney".into(), money(c, me.fin.cash));
    }
    let found = found_of(d);
    let rows: Vec<String> = found
        .iter()
        .filter_map(|(club, player)| {
            let loc = c.world.find_club(club)?;
            let i = c
                .world
                .club_of(loc)
                .names
                .iter()
                .position(|n| n == player)?;
            Some(offer_row(c, &offer_at(c, club, i)?))
        })
        .collect();
    d.lists.insert("ListBoxOfPlayers".into(), rows);
    d.texts.insert(
        "ListBoxOfPlayers.cols".into(),
        "4,24,164,300,356,390,430".into(),
    );
    d.texts
        .insert("ListBoxOfPlayers.align".into(), "llllrll".into());
    let has_sel = d.texts.contains_key("ListBoxOfPlayers.sel");
    d.texts
        .insert("teamBtn.enabled".into(), (has_sel as u8).to_string());
    d.texts.insert(
        "labelListedOnlyFirst.visible".into(),
        ((found.len() >= SEARCH_LIMIT) as u8).to_string(),
    );
}

fn search_find(c: &Classic, d: &mut Dialog) {
    let q = query_of(c, d);
    let (mut found, _) = c.world.search_players(&q);
    // the list box is sorted
    found.sort_by_key(|o| (o.player.to_uppercase(), o.club.clone()));
    d.st.insert(
        "found".into(),
        found
            .iter()
            .map(|o| format!("{}\u{1}{}", o.club, o.player))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    d.texts.remove("ListBoxOfPlayers.sel");
    d.texts.remove("ListBoxOfPlayers.top");
    d.texts.insert("findBtn.enabled".into(), "0".into());
    d.set_num("changed", 0);
}

fn close_combos(d: &mut Dialog) {
    for combo in COMBOS {
        d.texts.remove(&format!("{combo}.open"));
    }
}

/// What a click on a combo box or one of its rows did.
fn combo_click(d: &mut Dialog, name: &str) -> bool {
    if let Some((combo, row)) = name.split_once('#') {
        if COMBOS.contains(&combo) {
            if let Ok(i) = row.parse::<usize>() {
                set_sel(d, combo, Some(i));
            }
            close_combos(d);
            return true;
        }
        return false;
    }
    if COMBOS.contains(&name) {
        let open = d
            .texts
            .get(&format!("{name}.open"))
            .is_some_and(|v| v == "1");
        close_combos(d);
        if !open {
            d.texts.insert(format!("{name}.open"), "1".into());
        }
        return true;
    }
    false
}

fn search_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if combo_click(d, name) {
        return Outcome::Keep;
    }
    close_combos(d);
    match name {
        "OKBtn" => return Outcome::Close,
        "findBtn" => search_find(c, d),
        "clearNameBtn" => {
            d.texts.insert("textName".into(), String::new());
            d.focus = "textName".into();
        }
        "clearSkillBtn" => set_sel(d, "comboSkill", None),
        "clearPositionBtn" => set_sel(d, "comboPosition", None),
        "clearCountryBtn" => set_sel(d, "comboCountry", None),
        // the price button toggles between "any" and "no price chosen"
        "clearPriceBtn" => {
            let v = sel_of(d, "comboPrice").map_or(Some(0), |_| None);
            set_sel(d, "comboPrice", v)
        }
        "myCountryBtn" => {
            if let Some((_, me)) = main_win::human_club(c) {
                let code = String::from_utf8_lossy(&me.country).to_string();
                let i = countries(c).iter().position(|(_, k)| *k == code);
                if i.is_some() {
                    set_sel(d, "comboCountry", i);
                }
            }
            d.set_num("changed", 1);
        }
        "checkBoxNotMyTeam" => {
            let on = d
                .texts
                .get("checkBoxNotMyTeam.checked")
                .is_none_or(|v| v != "0");
            d.texts
                .insert("checkBoxNotMyTeam.checked".into(), (!on as u8).to_string());
            d.set_num("changed", 1);
        }
        "teamBtn" => {
            let Some(row) = d
                .texts
                .get("ListBoxOfPlayers.sel")
                .and_then(|v| v.parse::<usize>().ok())
            else {
                return Outcome::Keep;
            };
            let Some((club, player)) = found_of(d).get(row).cloned() else {
                return Outcome::Keep;
            };
            return pick_player(c, d, club, player);
        }
        _ => {
            if let Some(row) = name
                .strip_prefix("ListBoxOfPlayers#")
                .and_then(|r| r.parse::<usize>().ok())
            {
                d.texts
                    .insert("ListBoxOfPlayers.sel".into(), row.to_string());
            }
        }
    }
    Outcome::Keep
}

/// The user chose a player in the search or the observed list: from the menu the squad viewer takes the place of the dialog, from the
/// squad viewer the choice goes back to it.
fn pick_player(c: &mut Classic, d: &Dialog, club: String, player: String) -> Outcome {
    if d.st.contains_key("from_plantel") {
        c.main.market.pick = Some((club, player));
        Outcome::Close
    } else {
        Outcome::Replace(Box::new(open_plantel(c, &club, Some(&player))))
    }
}

fn step_row(cur: Option<usize>, n: usize, key: Key) -> usize {
    match (cur, key) {
        (None, _) => 0,
        (Some(i), Key::Down) => (i + 1).min(n.saturating_sub(1)),
        (Some(i), _) => i.saturating_sub(1),
    }
}

fn search_key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    if !matches!(key, Key::Up | Key::Down) {
        return None;
    }
    // arrows change the chosen item of a combo box that has the focus, otherwise the row of the list
    let focus = d.focus.clone();
    if COMBOS.contains(&focus.as_str()) {
        let n = combo_items(c, &focus).len();
        let next = step_row(sel_of(d, &focus), n, key);
        set_sel(d, &focus, Some(next));
        return Some(Outcome::Keep);
    }
    let n = found_of(d).len();
    if n > 0 {
        let cur = d
            .texts
            .get("ListBoxOfPlayers.sel")
            .and_then(|v| v.parse::<usize>().ok());
        d.texts.insert(
            "ListBoxOfPlayers.sel".into(),
            step_row(cur, n, key).to_string(),
        );
        return Some(Outcome::Keep);
    }
    None
}

// ---------------------------------------------------------------------------------------------------------------------------
// The squad viewer (TPlantelDlg, 1048:129e / 13fa / 1d66)
// ---------------------------------------------------------------------------------------------------------------------------

/// The squad viewer on `club`, with `player` selected when given.
pub fn open_plantel(c: &Classic, club: &str, player: Option<&str>) -> Dialog {
    let mut d = Dialog::new(Kind::Plantel, "TPLANTELDLG");
    d.st.insert("club".into(), club.into());
    d.focus = "ListBoxOfPlayers".into();
    select_player(c, &mut d, player);
    d
}

/// The squad viewer on the next opponent (the button of the opponent page).
pub fn open_plantel_of_opponent(c: &Classic) -> Option<Dialog> {
    let (d, j, _) = main_win::next_opponent(c)?;
    Some(open_plantel(c, &c.world.club_of((d, j)).name, None))
}

fn plantel_club<'a>(c: &'a Classic, d: &Dialog) -> Option<&'a Club> {
    let name = d.st.get("club")?;
    c.world.find_club(name).map(|loc| c.world.club_of(loc))
}

fn select_player(c: &Classic, d: &mut Dialog, player: Option<&str>) {
    d.texts.remove("ListBoxOfPlayers.sel");
    d.texts.remove("ListBoxOfPlayers.top");
    let Some(p) = player else {
        return;
    };
    if let Some(club) = plantel_club(c, d) {
        let order = main_win::roster_order(club);
        if let Some(row) = order.iter().position(|&i| club.names[i] == p) {
            d.texts
                .insert("ListBoxOfPlayers.sel".into(), row.to_string());
        }
    }
}

fn consume_pick(c: &mut Classic, d: &mut Dialog) {
    if let Some((club, player)) = c.main.market.pick.take() {
        d.st.insert("club".into(), club);
        select_player(c, d, Some(&player));
    }
}

/// The club names of the combo box, sorted.
fn all_club_names(c: &Classic) -> Vec<String> {
    let mut v: Vec<String> = (0..4)
        .flat_map(|dv| c.world.divisions[dv].iter().map(|cl| cl.name.clone()))
        .chain(c.world.pool.iter().map(|cl| cl.name.clone()))
        .collect();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

fn selected_of_plantel(c: &Classic, d: &Dialog) -> Option<(usize, String)> {
    let club = plantel_club(c, d)?;
    let row = d.texts.get("ListBoxOfPlayers.sel")?.parse::<usize>().ok()?;
    let i = *main_win::roster_order(club).get(row)?;
    Some((i, club.names[i].clone()))
}

fn plantel_bind(c: &Classic, d: &mut Dialog) {
    let Some(club) = plantel_club(c, d) else {
        return;
    };
    let mine = c
        .world
        .human_location()
        .map(|l| c.world.club_of(l).name.clone());
    let own = mine.as_deref() == Some(club.name.as_str());
    let country = country_name(c, &club.country);
    d.text("@caption", format!("{} ({})", club.name, country));
    let names = all_club_names(c);
    let cur = names.iter().position(|n| *n == club.name).unwrap_or(0);
    d.texts
        .insert("ComboCurrentTeam.text".into(), club.name.clone());
    d.texts
        .insert("ComboCurrentTeam.sel".into(), cur.to_string());
    d.lists.insert("ComboCurrentTeam".into(), names);
    let division = (0..4).find(|&dv| c.world.divisions[dv].iter().any(|x| x.name == club.name));
    d.text(
        "textTeamDivision",
        division.map_or(String::new(), |dv| format!("{}ª Divisão", dv + 1)),
    );
    d.text(
        "gaugeMoral.value",
        ((club.k * 10.0).round() as i32).clamp(0, 20).to_string(),
    );
    d.text("gaugeMoral.barcolor", "248,252,0");
    // the squad, grouped by position like the main window's list
    let order = main_win::roster_order(club);
    let rows: Vec<String> = order
        .iter()
        .map(|&i| {
            let s = &club.slots[i];
            let price = if own {
                0
            } else {
                c.world.price_for_user(&club.name, i)
            };
            format!(
                "{}|{}{}|{}|{}|{}|{}",
                ["G", "D", "M", "A"][s.position as usize],
                club.names[i],
                if s.star() { "*" } else { "" }, // the name as `1058:13fe` writes it
                s.strength,
                if s.ban > 0 { "S" } else { "" },
                if s.injured > 0 { "L" } else { "" },
                if price > 0 {
                    text::int(price)
                } else {
                    String::new()
                }
            )
        })
        .collect();
    d.lists.insert("ListBoxOfPlayers".into(), rows);
    d.texts.insert(
        "ListBoxOfPlayers.cols".into(),
        "4,24,230,262,278,340".into(),
    );
    d.texts
        .insert("ListBoxOfPlayers.align".into(), "llrllr".into());
    let (bg, fg) = (club.colors.1, club.colors.0);
    d.bg = Some(rgb_of(bg));
    for label in [
        "textTeamDivision",
        "textDesiredSalary",
        "Label1",
        "Label2",
        "Label3",
        "Label4",
        "Label5",
        "Jogos",
        "textPlayerBehaviour",
        "textSeasonGoals",
        "textHistoryGoals",
        "textHistoryRedCards",
        "textHistoryInjuries",
        "textHistoryGamesPlayed",
        "LabelInjured",
        "LabelSuspended",
        "GroupBox1",
    ] {
        d.texts.insert(format!("{label}.color"), rgb_s(fg));
    }
    // the chosen player
    let sel = selected_of_plantel(c, d);
    match &sel {
        None => {
            for h in [
                "PanelPlayer",
                "GroupBox1",
                "LabelInjured",
                "LabelSuspended",
                "textDesiredSalary",
            ] {
                d.texts.insert(format!("{h}.visible"), "0".into());
            }
            d.texts.insert("buyBtn.enabled".into(), "0".into());
            d.texts
                .insert("SpeedButtonObserved.enabled".into(), "0".into());
            d.texts
                .insert("SpeedButtonObserved.checked".into(), "0".into());
        }
        Some((i, name)) => {
            let s = &club.slots[*i];
            for h in ["PanelPlayer", "GroupBox1"] {
                d.texts.insert(format!("{h}.visible"), "1".into());
            }
            d.text("PanelPlayer", "");
            d.text("textPlayerBehaviour", text::behaviour(s.aggression));
            d.text("textSeasonGoals", s.goals.to_string());
            d.text("textHistoryGamesPlayed", s.history[0].to_string());
            d.text("textHistoryGoals", s.history[1].to_string());
            d.text("textHistoryRedCards", s.history[2].to_string());
            d.text("textHistoryInjuries", s.history[3].to_string());
            d.text("LabelSuspended", suspended_text(s.ban));
            d.text("LabelInjured", injured_text(s.injured));
            d.text("LabelSuspended.visible", ((s.ban > 0) as u8).to_string());
            d.text("LabelInjured.visible", ((s.injured > 0) as u8).to_string());
            let buy = !own && c.world.user_can_buy(&club.name, *i);
            d.texts
                .insert("buyBtn.enabled".into(), (buy as u8).to_string());
            d.texts
                .insert("textDesiredSalary.visible".into(), (buy as u8).to_string());
            if let (true, Some(me)) = (buy, mine.as_deref().and_then(|m| c.world.find_club(m))) {
                let f = c.world.club_of(me).fin.f;
                let v = rustfoot_core::economy::player_value(
                    s.strength,
                    f,
                    c.world.economy,
                    c.world.scale,
                );
                d.text(
                    "textDesiredSalary",
                    format!("Salário pretendido:\n{v} {}", c.world.currency_info().unit),
                );
            }
            d.texts.insert(
                "SpeedButtonObserved.enabled".into(),
                (!own as u8).to_string(),
            );
            d.texts.insert(
                "SpeedButtonObserved.checked".into(),
                (c.world.is_observed(&club.name, name) as u8).to_string(),
            );
        }
    }
    // the buttons of the user's own club
    for b in ["balanceBtn", "loanBtn"] {
        d.texts
            .insert(format!("{b}.visible"), (own as u8).to_string());
    }
    d.texts.insert(
        "observedPlayersBtn.enabled".into(),
        (!c.world.observed.is_empty() as u8).to_string(),
    );
}

fn plantel_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    consume_pick(c, d);
    if let Some((combo, row)) = name.split_once('#') {
        match combo {
            "ComboCurrentTeam" => {
                if let Ok(i) = row.parse::<usize>() {
                    if let Some(n) = all_club_names(c).get(i) {
                        d.st.insert("club".into(), n.clone());
                        select_player(c, d, None);
                    }
                }
                d.texts.remove("ComboCurrentTeam.open");
            }
            "ListBoxOfPlayers" => {
                if let Ok(i) = row.parse::<usize>() {
                    d.texts.insert("ListBoxOfPlayers.sel".into(), i.to_string());
                }
            }
            _ => {}
        }
        return Outcome::Keep;
    }
    if name != "ComboCurrentTeam" {
        d.texts.remove("ComboCurrentTeam.open");
    }
    match name {
        "OKBtn" => return Outcome::Close,
        "ComboCurrentTeam" => {
            let open = d
                .texts
                .get("ComboCurrentTeam.open")
                .is_some_and(|v| v == "1");
            if open {
                d.texts.remove("ComboCurrentTeam.open");
            } else {
                d.texts.insert("ComboCurrentTeam.open".into(), "1".into());
            }
        }
        "buyBtn" => {
            if let (Some((i, player)), Some(club)) =
                (selected_of_plantel(c, d), d.st.get("club").cloned())
            {
                if let Err(e) = c.world.human_buy(&club, i, &player) {
                    c.message("Elifoot 98", &e);
                }
            }
        }
        "SpeedButtonObserved" => {
            if let (Some((_, player)), Some(club)) =
                (selected_of_plantel(c, d), d.st.get("club").cloned())
            {
                if c.world.is_observed(&club, &player) {
                    c.world.unobserve(&club, &player);
                } else {
                    c.world.observe(&club, &player);
                    while c.world.observed.len() > MAX_OBSERVED {
                        c.world.observed.remove(0);
                    }
                }
            }
        }
        "findBtn" => return Outcome::Open(Box::new(open_search(c, true))),
        "observedPlayersBtn" => {
            if !c.world.observed.is_empty() {
                return Outcome::Open(Box::new(open_observed(c, true)));
            }
        }
        "calendarBtn" => {
            if let Some(cal) = super::open_for_button(c, "opponentCalendarBtn") {
                return Outcome::Open(Box::new(cal));
            }
        }
        "loanBtn" => {
            if let Some(l) = super::open_for_menu(c, "Pediremprestimo1") {
                return Outcome::Open(Box::new(l));
            }
        }
        "balanceBtn" => return Outcome::Open(Box::new(super::misc::balance())),
        _ => {}
    }
    Outcome::Keep
}

fn plantel_key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    consume_pick(c, d);
    if matches!(key, Key::Up | Key::Down) {
        let n = plantel_club(c, d).map_or(0, |cl| cl.slots.len());
        if n > 0 {
            let cur = d
                .texts
                .get("ListBoxOfPlayers.sel")
                .and_then(|v| v.parse::<usize>().ok());
            d.texts.insert(
                "ListBoxOfPlayers.sel".into(),
                step_row(cur, n, key).to_string(),
            );
            return Some(Outcome::Keep);
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------------------------------------
// The watched players (TObservedPlayersDlg, 1030:1dec)
// ---------------------------------------------------------------------------------------------------------------------------

/// Jogador > Jogadores sob observação...
pub fn open_observed(c: &Classic, from_plantel: bool) -> Dialog {
    let _ = c;
    let mut d = Dialog::new(Kind::Observed, "TOBSERVEDPLAYERSDLG");
    d.focus = "ListBoxObservedPlayers".into();
    if from_plantel {
        d.st.insert("from_plantel".into(), "1".into());
    }
    d
}

fn observed_bind(c: &Classic, d: &mut Dialog) {
    let list = c.world.observed_players();
    let rows: Vec<String> = list.iter().map(|o| offer_row(c, o)).collect();
    // keep the selection inside the list
    if let Some(sel) = d
        .texts
        .get("ListBoxObservedPlayers.sel")
        .and_then(|v| v.parse::<usize>().ok())
    {
        if sel >= rows.len() {
            d.texts.remove("ListBoxObservedPlayers.sel");
        }
    }
    let has_sel = d.texts.contains_key("ListBoxObservedPlayers.sel");
    d.lists.insert("ListBoxObservedPlayers".into(), rows);
    d.texts.insert(
        "ListBoxObservedPlayers.cols".into(),
        "4,24,164,300,356,390,430".into(),
    );
    d.texts
        .insert("ListBoxObservedPlayers.align".into(), "llllrll".into());
    for b in ["removeBtn", "teamBtn"] {
        d.texts
            .insert(format!("{b}.enabled"), (has_sel as u8).to_string());
    }
    d.texts.insert(
        "clearBtn.enabled".into(),
        (!list.is_empty() as u8).to_string(),
    );
    d.texts.insert("Memo1.visible".into(), "0".into());
}

fn observed_selected(c: &Classic, d: &Dialog) -> Option<Offer> {
    let row = d
        .texts
        .get("ListBoxObservedPlayers.sel")?
        .parse::<usize>()
        .ok()?;
    c.world.observed_players().get(row).cloned()
}

fn observed_click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match name {
        "closeBtn" => Outcome::Close,
        "removeBtn" => {
            if let Some(o) = observed_selected(c, d) {
                c.world.unobserve(&o.club, &o.player);
                // the player may be recorded under his old club
                c.world.observed.retain(|(_, p)| *p != o.player);
            }
            d.texts.remove("ListBoxObservedPlayers.sel");
            Outcome::Keep
        }
        "clearBtn" => {
            c.world.observed.clear();
            d.texts.remove("ListBoxObservedPlayers.sel");
            Outcome::Keep
        }
        "teamBtn" => match observed_selected(c, d) {
            Some(o) => pick_player(c, d, o.club, o.player),
            None => Outcome::Keep,
        },
        _ => {
            if let Some(row) = name
                .strip_prefix("ListBoxObservedPlayers#")
                .and_then(|r| r.parse::<usize>().ok())
            {
                d.texts
                    .insert("ListBoxObservedPlayers.sel".into(), row.to_string());
            }
            Outcome::Keep
        }
    }
}

fn observed_key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    match key {
        Key::Escape => Some(Outcome::Close),
        Key::Up | Key::Down => {
            let n = c.world.observed_players().len();
            if n > 0 {
                let cur = d
                    .texts
                    .get("ListBoxObservedPlayers.sel")
                    .and_then(|v| v.parse::<usize>().ok());
                d.texts.insert(
                    "ListBoxObservedPlayers.sel".into(),
                    step_row(cur, n, key).to_string(),
                );
            }
            Some(Outcome::Keep)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------------------
// The dispatchers' entry points
// ---------------------------------------------------------------------------------------------------------------------------

pub fn bind(c: &Classic, d: &mut Dialog) {
    match &d.kind {
        Kind::Auction => {
            let n = d
                .texts
                .get("textHumanOffer")
                .map_or(0, |t| t.chars().count());
            d.texts.insert("textHumanOffer.caret".into(), n.to_string());
        }
        Kind::Search => search_bind(c, d),
        Kind::Plantel => plantel_bind(c, d),
        Kind::Observed => observed_bind(c, d),
        _ => {}
    }
}

pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match &d.kind {
        Kind::Auction => auction_click(c, d, name),
        Kind::Search => search_click(c, d, name),
        Kind::Plantel => plantel_click(c, d, name),
        Kind::Observed => observed_click(c, d, name),
        Kind::Other(n) if n == ASK_MORE_SALARY => ask_click(c, d, name),
        _ => Outcome::Close,
    }
}

pub fn key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    match &d.kind {
        Kind::Auction => {
            // an auction that is over closes with any of these; one in progress is declined with Escape
            let over = d.st.get("mode").map(String::as_str) == Some("result");
            match key {
                Key::Escape => Some(Outcome::Close),
                Key::Enter | Key::Char(' ') if over => Some(Outcome::Close),
                _ => None,
            }
        }
        Kind::Search => search_key(c, d, key),
        Kind::Plantel => plantel_key(c, d, key),
        Kind::Observed => observed_key(c, d, key),
        _ => None,
    }
}

pub fn tick(c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    match &d.kind {
        Kind::Auction if d.st.get("mode").map(String::as_str) == Some("result") => {
            let t = d.num("t") + ms as i64;
            d.set_num("t", t);
            if t >= RESULT_MS {
                return Outcome::Close;
            }
        }
        Kind::Plantel => consume_pick(c, d),
        _ => {}
    }
    Outcome::Keep
}

/// The dialog a menu entry or a button of the main window opens when it belongs to this module (see `open_for_menu` /
/// `open_for_button`).
pub fn open_for(c: &Classic, name: &str) -> Option<Dialog> {
    match name {
        "JogadorProcurar1" => Some(open_search(c, false)),
        "Jogadoressobobservao1" => Some(open_observed(c, false)),
        "opponentPlantelBtn" => open_plantel_of_opponent(c),
        _ => None,
    }
}

/// Opens a dialog or page by name, for `classic_shot` (`open=search|observed|plantel|auction|sell|contract|message`).
pub fn open_by_name(c: &mut Classic, name: &str) -> Result<(), String> {
    match name {
        "search" => c.open(open_search(c, false)),
        "observed" => c.open(open_observed(c, false)),
        "plantel" => {
            let d = open_plantel_of_opponent(c).ok_or("no opponent")?;
            c.open(d)
        }
        "auction" => {
            queue_auctions(c);
            if c.queue.is_empty() {
                // nothing on the board: show the first player of the first other club
                let me = c.world.human_location().ok_or("no club")?;
                let loc = c
                    .world
                    .all_clubs()
                    .into_iter()
                    .find(|l| *l != me)
                    .ok_or("no clubs")?;
                let cl = c.world.club_of(loc);
                let (club, player) = (cl.name.clone(), cl.names[0].clone());
                let d = auction_dialog(c, &club, &player, 609_000).ok_or("no player")?;
                c.queue.push_back(d);
            }
            if let Some(d) = c.queue.pop_front() {
                c.open(d);
            }
        }
        "auction_demo" => {
            // the values of the capture `analysis/shots/match38.png`, for comparing the layout
            let me = c.world.human_location().ok_or("no club")?;
            let loc = c
                .world
                .all_clubs()
                .into_iter()
                .find(|l| *l != me)
                .ok_or("no clubs")?;
            let cl = c.world.club_of(loc);
            let (club, player) = (cl.name.clone(), cl.names[0].clone());
            let mut d = auction_dialog(c, &club, &player, 609_000).ok_or("no player")?;
            for (k, v) in [
                ("textTeamName", "FLUMINENSE"),
                ("textTeamName.color", "0,255,0"),
                ("textTeamName.bgcolor", "255,0,0"),
                ("textPlayerName", "Paulinho"),
                ("textPlayerSkill", "21"),
                ("textDesiredSalary", "11000 reais"),
                ("textPlayerBehaviour", "Caneleiro"),
                ("textSeasonGoals", "0"),
                ("textHistoryGamesPlayed", "3"),
                ("textHistoryGoals", "0"),
                ("textHistoryRedCards", "0"),
                ("textHistoryInjuries", "0"),
                ("textCash", "Dinheiro em caixa: 1 milhão e 175 mil reais"),
                ("LabelSuspended.visible", "0"),
                ("labelInjured.visible", "0"),
            ] {
                d.text(k, v);
            }
            d.text("textPlayerPosition", "Avançado");
            c.open(d)
        }
        "sell" => {
            c.main.selected.get_or_insert(0);
            vender(c)
        }
        "contract" => {
            c.main.selected.get_or_insert(0);
            change_salary(c)
        }
        "message" => show_message_page(c, "Mensagem de teste"),
        _ => return Err(format!("unknown dialog {name}")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_and_thousands() {
        assert_eq!(parse_digits("123"), Some(123));
        assert_eq!(parse_digits(" 7 "), Some(7));
        assert_eq!(parse_digits(""), None);
        assert_eq!(parse_digits("12a"), None);
        assert_eq!(parse_digits("-3"), None);
        // the sell price is in thousands, empty means any price
        assert_eq!(sell_price_of(""), Some(0));
        assert_eq!(sell_price_of("609"), Some(609_000));
        assert_eq!(sell_price_of("6x"), None);
    }

    #[test]
    fn price_combo_follows_the_originals_table() {
        let items = price_items(5);
        assert_eq!(items.len(), 8);
        assert_eq!(items[0], "Qualquer");
        assert_eq!(items[1], " Até 250 000");
        assert_eq!(items[2], " De 250 000 até 1 000 000");
        assert_eq!(items[7], " Mais de 25 000 000");
        assert_eq!(price_filter(5, None), PriceFilter::Any);
        assert_eq!(price_filter(5, Some(0)), PriceFilter::ForSale);
        assert_eq!(price_filter(5, Some(1)), PriceFilter::Between(1, 250_000));
        assert_eq!(
            price_filter(5, Some(7)),
            PriceFilter::Between(25_000_000, i64::MAX)
        );
        assert_eq!(skill_items()[1], "5 a 15");
    }

    #[test]
    fn suspension_and_injury_sentences() {
        assert_eq!(suspended_text(0), "");
        assert_eq!(suspended_text(1), "Suspenso por 1 jogo");
        assert_eq!(suspended_text(3), "Suspenso por 3 jogos");
        assert_eq!(injured_text(1), "Lesionado por 1 jogo");
        assert_eq!(injured_text(4), "Lesionado por 4 jogos");
    }

    #[test]
    fn rows_step_inside_the_list() {
        assert_eq!(step_row(None, 5, Key::Down), 0);
        assert_eq!(step_row(Some(4), 5, Key::Down), 4);
        assert_eq!(step_row(Some(0), 5, Key::Up), 0);
        assert_eq!(step_row(Some(2), 5, Key::Up), 1);
    }
}

/// Flows on a saved game; they need the skin pack and the user's own save (`ELIFOOT_ROOT`), and pass without doing anything when
/// those are not there.
#[cfg(test)]
mod flows {
    use super::*;
    use crate::classic::{dialogs, Data, Ev};
    use crate::win31::{Forms, Skin};
    use std::path::PathBuf;

    fn game() -> Option<Classic> {
        let root = PathBuf::from(std::env::var("ELIFOOT_ROOT").ok()?);
        let skin = Skin::load(&root.join("analysis/out/skin")).ok()?;
        let forms = Forms::from_json(
            &std::fs::read_to_string(root.join("analysis/out/skin/forms.json")).ok()?,
        )
        .ok()?;
        let data = Data::from_txe(
            &std::fs::read(root.join("original/elifoot98/COUNTRY.TXE")).ok()?,
            &std::fs::read(root.join("original/elifoot98/REFEREE.TXE")).ok()?,
        );
        let save =
            rustfoot_core::e98::parse(&std::fs::read(root.join("original/saves/snap2.e98")).ok()?)
                .ok()?;
        let world = rustfoot_core::world::World::from_e98(1, &save).ok()?;
        let mut c = Classic::new(world, skin, forms, data);
        c.coach = "joga1".into();
        c.draw();
        Some(c)
    }

    fn closes(o: Outcome) -> bool {
        matches!(o, Outcome::Close)
    }

    fn squad(c: &Classic) -> usize {
        let (d, i) = c.world.human_location().unwrap();
        c.world.divisions[d][i].slots.len()
    }

    #[test]
    fn the_finances_list_and_the_squad_window_open_the_balance_sheet() {
        let Some(mut c) = game() else { return };
        c.main.page = Some(Page::Finances);
        c.draw();
        let h = c
            .hits
            .iter()
            .find(|h| h.name == "ListBoxBalance#7")
            .cloned()
            .expect("the rows of the Finanças list are hit targets");
        let (x, y) = (h.x + 2, h.y + 2);
        c.event(Ev::Down(x, y));
        c.event(Ev::Up(x, y));
        assert_eq!(c.main.balance_sel, Some(7));
        let d = c.dialogs.last().expect("the balance sheet is open");
        assert_eq!(d.kind, Kind::Other(super::super::misc::BALANCE.into()));
        c.draw();
        let d = c.dialogs.last().unwrap();
        let year = c.world.year;
        assert!(d.lists["ListBoxBalance"][0].ends_with(&format!(
            "Z{year}\t\x06Z{}\t\x06Z{}\t",
            year - 1,
            year - 2
        )));
        c.event(Ev::Key(Key::Enter));
        assert!(c.dialogs.is_empty());
        // the squad window's button opens it over the squad window
        let mut d = Dialog::new(Kind::Plantel, "TPLANTELDLG");
        let o = dialogs::click(&mut c, &mut d, "balanceBtn");
        assert!(matches!(o, Outcome::Open(b) if b.form == "TBALANCEDLG"));
    }

    #[test]
    fn selling_from_the_vender_page() {
        let Some(mut c) = game() else { return };
        let before = squad(&c);
        c.main.selected = Some(3);
        vender(&mut c);
        assert_eq!(c.main.page, Some(Page::Sell));
        // letters are ignored, digits fill the price; an empty price accepts any bid
        for k in ['x', '5', '0', '0'] {
            main_key(&mut c, Key::Char(k));
        }
        assert_eq!(c.main.market.sell_price, "500");
        for _ in 0..3 {
            main_key(&mut c, Key::Backspace);
        }
        assert!(main_key(&mut c, Key::Enter));
        assert_eq!(c.main.page, Some(Page::Game));
        let d = c.dialogs.last().expect("the auction is shown");
        assert_eq!(d.kind, Kind::Auction);
        assert!(d.texts["textTransferred"].starts_with("Vendido ao "));
        assert_eq!(d.texts["PanelHumanOffer.visible"], "0");
        assert_eq!(squad(&c), before - 1);
        // the result goes away with Enter
        c.event(Ev::Key(Key::Enter));
        assert!(c.dialogs.is_empty());
    }

    #[test]
    fn a_player_who_cannot_be_sold_gets_the_reason_on_the_message_page() {
        let Some(mut c) = game() else { return };
        let (d, i) = c.world.human_location().unwrap();
        let name = {
            let club = &c.world.divisions[d][i];
            club.names[main_win::roster_order(club)[3]].clone()
        };
        c.world.sell_tried.push(name);
        c.main.selected = Some(3);
        vender(&mut c);
        assert_eq!(c.main.page, Some(Page::Message));
        assert!(c.main.market.message.contains("próximo jogo"));
        // any key brings the player page back
        assert!(main_key(&mut c, Key::Enter));
        assert_eq!(c.main.page, Some(Page::Player));
    }

    #[test]
    fn the_contract_page_renews_or_puts_the_player_up_for_auction() {
        let Some(mut c) = game() else { return };
        let before = squad(&c);
        c.main.selected = Some(3);
        change_salary(&mut c);
        assert_eq!(c.main.page, Some(Page::Contract));
        c.main.market.spin = "1".into();
        assert!(main_key(&mut c, Key::Enter));
        let mut d = c.dialogs.pop().expect("the player demands more");
        assert_eq!(d.kind, Kind::Other(ASK_MORE_SALARY.into()));
        assert!(d.texts["textMessage"].starts_with("Nem pensar! Exijo "));
        // refuse: he is sold
        let out = dialogs::click(&mut c, &mut d, "NoBtn");
        let Outcome::Replace(r) = out else {
            panic!("the auction replaces the question");
        };
        assert_eq!(r.kind, Kind::Auction);
        assert_eq!(c.main.page, Some(Page::Game));
        assert_eq!(squad(&c), before - 1);
        // another player accepts the demand
        c.main.selected = Some(0);
        change_salary(&mut c);
        c.main.market.spin = "1".into();
        main_key(&mut c, Key::Enter);
        let mut d = c.dialogs.pop().unwrap();
        let amount = d.num("amount");
        assert!(closes(dialogs::click(&mut c, &mut d, "YesBtn")));
        let (dv, i) = c.world.human_location().unwrap();
        let club = &c.world.divisions[dv][i];
        let slot = &club.slots[main_win::roster_order(club)[0]];
        assert!(slot.fresh && slot.wage == amount);
        assert_eq!(c.main.page, Some(Page::Player));
        // a renewed player cannot renew again
        change_salary(&mut c);
        assert_eq!(c.main.page, Some(Page::Player));
    }

    #[test]
    fn bidding_in_an_auction_of_the_board() {
        let Some(mut c) = game() else { return };
        let me = c.world.human_location().unwrap();
        // the first player of another club the user may bid for (not a foreigner over the limit)
        let candidates: Vec<(String, String)> = c
            .world
            .all_clubs()
            .into_iter()
            .filter(|l| *l != me)
            .flat_map(|l| {
                let cl = c.world.club_of(l);
                cl.names
                    .iter()
                    .map(|n| (cl.name.clone(), n.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut player = String::new();
        c.world.board.clear(); // the save's own listings
        for (club, name) in candidates {
            c.world.board.push(rustfoot_core::world::Listing {
                seller: club,
                player: name.clone(),
                ask: 1000,
                best: None,
                round: 0,
            });
            if c.world.listing_open_to_user(0) {
                player = name;
                break;
            }
            c.world.board.clear();
        }
        assert!(c.world.listing_open_to_user(0));
        queue_auctions(&mut c);
        // every human who may bid gets the auction; the active one's
        let seat = c.world.active_seat().to_string();
        c.queue.retain(|d| d.st.get("seat") == Some(&seat));
        assert_eq!(c.queue.len(), 1);
        let mut d = c.queue.pop_front().unwrap();
        assert_eq!(d.texts["textHumanOffer"], "");
        let before = squad(&c);
        // nonsense is ignored, a real offer wins the player
        d.texts.insert("textHumanOffer".into(), "x".into());
        assert!(!closes(dialogs::click(&mut c, &mut d, "OKBtn")));
        d.texts.insert("textHumanOffer".into(), "2".into());
        let _ = dialogs::click(&mut c, &mut d, "OKBtn");
        assert_eq!(d.st["mode"], "result");
        assert!(d.texts["textTransferred"].contains("2 mil reais"));
        assert_eq!(squad(&c), before + 1);
        assert!(c.world.board.is_empty());
        let (dv, i) = c.world.human_location().unwrap();
        assert!(c.world.divisions[dv][i].names.contains(&player));
    }

    #[test]
    fn searching_buying_and_watching_players() {
        let Some(mut c) = game() else { return };
        c.world.pick_human();
        let mut d = open_search(&c, false);
        d.draw(&mut c);
        // everybody who is for sale: choose "Qualquer" in the price combo
        assert!(!closes(dialogs::click(&mut c, &mut d, "comboPrice")));
        assert_eq!(d.texts["comboPrice.open"], "1");
        dialogs::click(&mut c, &mut d, "comboPrice#0");
        assert!(!d.texts.contains_key("comboPrice.open"));
        dialogs::click(&mut c, &mut d, "findBtn");
        d.draw(&mut c);
        let rows = d.lists["ListBoxOfPlayers"].clone();
        assert!(!rows.is_empty());
        assert_eq!(d.texts["findBtn.enabled"], "0");
        assert_eq!(d.texts["teamBtn.enabled"], "0");
        let names: Vec<&str> = rows.iter().map(|r| r.split('|').nth(1).unwrap()).collect();
        let mut sorted = names.clone();
        sorted.sort_by_key(|n| n.to_uppercase());
        assert_eq!(names, sorted, "the list is sorted by name");
        // the squad viewer opens on the club of the chosen player
        dialogs::click(&mut c, &mut d, "ListBoxOfPlayers#0");
        d.draw(&mut c);
        assert_eq!(d.texts["teamBtn.enabled"], "1");
        let club_of_row = rows[0].split('|').nth(2).unwrap().to_string();
        let Outcome::Replace(mut p) = dialogs::click(&mut c, &mut d, "teamBtn") else {
            panic!("the squad viewer takes over");
        };
        assert_eq!(p.kind, Kind::Plantel);
        p.draw(&mut c);
        assert!(p.texts["@caption"].starts_with(&club_of_row));
        assert_eq!(p.texts["buyBtn.enabled"], "1");
        assert_eq!(p.texts["SpeedButtonObserved.checked"], "0");
        // watching and buying
        dialogs::click(&mut c, &mut p, "SpeedButtonObserved");
        p.draw(&mut c);
        assert_eq!(p.texts["SpeedButtonObserved.checked"], "1");
        assert_eq!(c.world.observed.len(), 1);
        let cash = {
            let (dv, i) = c.world.human_location().unwrap();
            c.world.divisions[dv][i].fin.cash
        };
        dialogs::click(&mut c, &mut p, "buyBtn");
        p.draw(&mut c);
        assert_eq!(p.texts["buyBtn.enabled"], "0", "bought");
        let (dv, i) = c.world.human_location().unwrap();
        assert!(c.world.divisions[dv][i].fin.cash < cash);
        assert_eq!(c.world.pending.len(), 1);
        // the list of watched players
        let mut o = open_observed(&c, false);
        o.draw(&mut c);
        assert_eq!(o.lists["ListBoxObservedPlayers"].len(), 1);
        dialogs::click(&mut c, &mut o, "ListBoxObservedPlayers#0");
        o.draw(&mut c);
        assert_eq!(o.texts["removeBtn.enabled"], "1");
        dialogs::click(&mut c, &mut o, "removeBtn");
        assert!(c.world.observed.is_empty());
    }
}
