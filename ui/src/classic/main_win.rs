//! The main window (`TPrepareGameDlg`): the squad list on the left and a notebook with the pages Jogo, Jogador, Finanças, Selecção and
//! Adversário on the right, painted in the colours of the user's club.
use super::{text, Classic, Page};
use crate::win31::draw::{controls, dialog_frame_menu, text as draw_text, Ctx, Hit, DESKTOP};
use crate::win31::fb::{Rgb, BLACK, SILVER};
use crate::win31::Node;
use elifoot_core::season::{fixtures, Club};
use elifoot_core::sector::Status;
use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct MainState {
    pub page: Option<Page>,
    /// Index into the roster order of the selected player.
    pub selected: Option<usize>,
}

/// The squad in the order of the list: goalkeepers, defenders, midfielders, attackers, each group by name.
pub fn roster_order(club: &Club) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..club.slots.len()).collect();
    idx.sort_by_key(|&i| (club.slots[i].position, club.names[i].to_lowercase()));
    idx
}

fn rgb(c: [u8; 3]) -> Rgb {
    Rgb(c[0], c[1], c[2])
}

fn col(c: Rgb) -> String {
    format!("{},{},{}", c.0, c.1, c.2)
}

/// Collects the names of the labels of `n` (all depths).
fn labels(n: &Node, out: &mut Vec<String>) {
    if n.class == "TLabel" || n.class == "TGroupBox" {
        out.push(n.name.clone());
    }
    for c in &n.children {
        labels(c, out);
    }
}

pub struct View {
    pub texts: BTreeMap<String, String>,
    pub lists: BTreeMap<String, Vec<String>>,
}

/// Opponent of the user's club in the next round: `(division, index, is_home)`.
pub fn next_opponent(c: &Classic) -> Option<(usize, usize, bool)> {
    let (d, i) = c.world.human_location()?;
    let round = c.world.round + 1;
    if round > 14 {
        return None;
    }
    fixtures(round).into_iter().find_map(|(h, a)| {
        if h == i {
            Some((d, a, true))
        } else if a == i {
            Some((d, h, false))
        } else {
            None
        }
    })
}

/// Display-only choice of the referee of a match (it does not use the game's random numbers): a referee of another country than both
/// clubs, picked from a hash of the round and the clubs.
pub fn referee(c: &Classic, home: &Club, away: &Club) -> String {
    if c.data.referees.is_empty() {
        return String::new();
    }
    let seed = (c.world.year as u64) * 1_000_003
        + (c.world.round as u64) * 7919
        + home.master as u64 * 31
        + away.master as u64 * 17;
    let n = c.data.referees.len();
    let mut k = (seed
        .wrapping_mul(2862933555777941757)
        .wrapping_add(3037000493)
        >> 33) as usize
        % n;
    let code = |club: &Club| String::from_utf8_lossy(&club.country).to_string();
    for _ in 0..n {
        let (cc, _) = &c.data.referees[k];
        if *cc != code(home) && *cc != code(away) {
            break;
        }
        k = (k + 1) % n;
    }
    let (cc, name) = &c.data.referees[k];
    let country = c
        .data
        .countries
        .get(cc)
        .cloned()
        .unwrap_or_else(|| cc.clone());
    format!("{name} ({country})")
}

pub fn view(c: &Classic, club: &Club, page: Page) -> View {
    let mut texts = BTreeMap::new();
    let mut lists = BTreeMap::new();
    let (bg, fg) = (rgb(club.colors.0), rgb(club.colors.1));
    let mut t = |k: &str, v: String| {
        texts.insert(k.to_string(), v);
    };
    // all labels take the club's background colour as text colour
    if let Some(form) = c.forms.get("TPREPAREGAMEDLG") {
        let mut names = Vec::new();
        labels(form, &mut names);
        for n in names {
            t(&format!("{n}.color"), col(bg));
        }
    }
    t("@caption", club.full_name.clone());
    t("@color", col(fg));
    t("@client", "632x434".into());
    t("textSeason", c.world.year.to_string());
    t("textCoachName", c.coach.clone());
    let country = String::from_utf8_lossy(&club.country).to_string();
    t(
        "textTeamCountry",
        c.data
            .countries
            .get(&country)
            .cloned()
            .unwrap_or(country.clone()),
    );
    t("bitmapTeamCountry.image", format!("FLAGS/{country}"));
    let (d, _) = c.world.human_location().unwrap_or((3, 0));
    t("textMyDivision", format!("{}ª Divisão", d + 1));
    // the squad
    let order = roster_order(club);
    let rows: Vec<String> = order
        .iter()
        .map(|&i| {
            let s = &club.slots[i];
            let mark = match s.status {
                Status::Starter => "+",
                Status::Sub => "-",
                Status::Out => "",
            };
            let name = if s.foreign > 0 {
                format!("{}*", club.names[i])
            } else {
                club.names[i].clone()
            };
            format!(
                "{}|{}|{}|{}|{}|{}",
                ["G", "D", "M", "A"][s.position as usize],
                name,
                s.strength,
                s.wage,
                if s.fresh { "+" } else { "" },
                mark
            )
        })
        .collect();
    lists.insert("ListBoxOfPlayers".to_string(), rows);
    t("ListBoxOfPlayers.color", col(fg));
    t("ListBoxOfPlayers.bg", col(bg));
    t("ListBoxOfPlayers.focus", c.main.selected.unwrap_or(0).to_string());
    if let Some(s) = c.main.selected {
        t("ListBoxOfPlayers.sel", s.to_string());
    }
    t("Notebook.page", page.notebook_index().to_string());
    let tabs = "Jogo|Jogador|Finanças|Selecção|Adversário";
    t("TabSet.tabs", tabs.into());
    t(
        "TabSet.index",
        Page::ALL
            .iter()
            .position(|p| *p == page)
            .unwrap()
            .to_string(),
    );
    t("TabSet.color", col(bg));
    // the next opponent
    let opp = next_opponent(c).map(|(d, j, home)| (&c.world.divisions[d][j], home));
    if let Some((o, home)) = opp {
        let (obg, ofg) = (rgb(o.colors.0), rgb(o.colors.1));
        t(
            "textOpponent",
            format!(
                " {}\n{} {}ª Jornada",
                o.name,
                if home { "CASA" } else { "FORA" },
                c.world.round + 1
            ),
        );
        t("textOpponent.color", col(obg));
        t("textOpponent.bgcolor", col(ofg));
        t("textOpponent.pad", "4".into());
        let (h, a) = if home { (club, o) } else { (o, club) };
        t("textReferee", referee(c, h, a));
    }
    let tc = |c: &Club| -> [String; 6] {
        [
            c.wins.to_string(),
            c.draws.to_string(),
            c.losses.to_string(),
            c.goals_for.to_string(),
            c.goals_against.to_string(),
            c.points.to_string(),
        ]
    };
    let mut class_rows = Vec::new();
    for (cl, text_c, back_c) in std::iter::once((club, bg, fg))
        .chain(opp.map(|(o, _)| (o, rgb(o.colors.0), rgb(o.colors.1))))
    {
        let v = tc(cl);
        class_rows.push(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}",
            cl.name,
            v[0],
            v[1],
            v[2],
            v[3],
            v[4],
            v[5],
            col(text_c),
            col(back_c)
        ));
    }
    lists.insert("ListBoxClassification".to_string(), class_rows);
    t("LabelLastResult.visible", "0".into());
    t("textLastResult.visible", "0".into());
    t(
        "textMoney2",
        format!(
            "Dinheiro em caixa\n{}",
            text::money_words(club.fin.cash, "reais")
        ),
    );
    t(
        "gaugeMoral.value",
        ((club.k * 10.0).round() as i32).clamp(0, 20).to_string(),
    );
    t("gaugeMoral.barcolor", "248,252,0".into());
    {
        let sel = c.main.selected.and_then(|r| roster_order(club).get(r).copied());
        let hide = [
            "textSelectedPlayerName",
            "bitmapPlayerCountry",
            "textSelectedPlayerCountry",
            "textBosmanDescription",
            "LabelSuspended",
            "LabelInjured",
            "PanelPlayerSalary",
            "PanelPlayer",
        ];
        match sel {
            None => {
                for h in hide {
                    t(&format!("{h}.visible"), "0".into());
                }
            }
            Some(i) => {
                let s = &club.slots[i];
                t("PanelPlayer", String::new());
                t("PanelPlayerSalary", String::new());
                t("textSelectedPlayerName", club.names[i].clone());
                let nat = String::from_utf8_lossy(&s.nat).to_string();
                t(
                    "textSelectedPlayerCountry",
                    c.data.countries.get(&nat).cloned().unwrap_or(nat.clone()),
                );
                t("bitmapPlayerCountry.image", format!("FLAGS/{nat}"));
                if s.foreign != 1 {
                    t("textBosmanDescription.visible", "0".into());
                }
                t("LabelSuspended.visible", ((s.ban > 0) as u8).to_string());
                t("LabelInjured.visible", ((s.injured > 0) as u8).to_string());
                let word = match s.aggression {
                    0 | 1 => "Cavalheiro",
                    2 | 3 => "Caneleiro",
                    _ => "Sarrafeiro",
                };
                t("textPlayerBehaviour", word.into());
                t("textSeasonGoals", s.goals.to_string());
                t("textHistoryGamesPlayed", s.history[0].to_string());
                t("textHistoryGoals", s.history[1].to_string());
                t("textHistoryRedCards", s.history[2].to_string());
                t("textHistoryInjuries", s.history[3].to_string());
            }
        }
    }
    {
        let starters = club.slots.iter().filter(|s| s.starter()).count();
        let keepers = club
            .slots
            .iter()
            .filter(|s| s.starter() && s.position == 0)
            .count();
        t("OKBtn.enabled", ((starters == 11 && keepers == 1) as u8).to_string());
    }
    if let Some((o, _)) = opp {
        t("textOpponentName", o.name.clone());
        t("textOpponentName.color", col(rgb(o.colors.0)));
        t("textOpponentCoach", o.coach_name.clone());
        t("textOpponentCoach.color", col(bg));
        t(
            "gaugeOpponentMoral.value",
            ((o.k * 10.0).round() as i32).clamp(0, 20).to_string(),
        );
        t("gaugeOpponentMoral.barcolor", "248,252,0".into());
        t("opponentPlantelBtn.visible", "0".into());
    }
    if page == Page::Finances {
        let f = &club.fin;
        let revenue = f.gate + f.sales + f.prizes;
        let expenses = f.wages + f.purchases + f.works + f.interest;
        let rows = [
            ("Época", c.world.year.to_string(), true),
            ("Receitas", String::new(), true),
            ("Bilhetes", text::int(f.gate), false),
            ("Jogadores vendidos", text::int(f.sales), false),
            ("Prémios", text::int(f.prizes), false),
            ("Despesas", String::new(), true),
            ("Salários", text::int(f.wages), false),
            ("Jogadores comprados", text::int(f.purchases), false),
            ("Bancadas", text::int(f.works), false),
            ("Juros", text::int(f.interest), false),
            ("", String::new(), false),
            ("Total de receitas", text::int(revenue), false),
            ("Total de despesas", text::int(expenses), false),
            ("Saldo", text::int(revenue + expenses), false),
        ];
        lists.insert(
            "ListBoxBalance".to_string(),
            rows.iter()
                .map(|(a, b, bold)| format!("{a}|{b}|{}", if *bold { "b" } else { "" }))
                .collect(),
        );
        t("ListBoxBalance.color", col(bg));
        let wages: i64 = club.slots.iter().map(|s| s.wage).sum();
        t("textTotalSalaries", text::int(wages));
        t(
            "textLoanInterest",
            text::int(elifoot_core::finance::loan_interest(f.loan)),
        );
        t("textMoney", text::int(f.cash));
        let price = c.world.money_ctx(d).price;
        t("textTicketPrice", format!("{price:.0} reais"));
    }
    View { texts, lists }
}

/// Draws the desktop, the application's caption bar and the main window.
pub fn draw(c: &mut Classic) {
    let Some((d, i)) = c.world.human_location() else {
        return;
    };
    let club = c.world.divisions[d][i].clone();
    let page = c.main.page.unwrap_or(Page::Game);
    let v = view(c, &club, page);
    c.fb.fill(0, 0, 800, 600, DESKTOP);
    // the hidden application window: a silver caption bar across the screen
    c.fb.fill(0, 0, 800, 18, SILVER);
    c.fb.fill(0, 18, 800, 1, BLACK);
    if let Some(f) = c.skin.font("system", 13, true, false) {
        let w = f.width("Elifoot 98");
        draw_text(&mut c.fb, f, (800 - w) / 2, 2, "Elifoot 98", BLACK, false);
    }
    let Some(form) = c.forms.get("TPREPAREGAMEDLG") else {
        return;
    };
    let menu: Vec<String> = form
        .children
        .iter()
        .filter(|n| n.class == "TMainMenu")
        .flat_map(|m| m.children.iter())
        .filter(|i| i.flag("Visible", true))
        .map(|i| i.str("Caption").unwrap_or("").to_string())
        .collect();
    let bgc = rgb(club.colors.1);
    let (ox, oy) = dialog_frame_menu(&mut c.fb, &c.skin, &club.full_name, 632, 434, bgc, &menu);
    let hits = RefCell::new(Vec::<Hit>::new());
    let cx = Ctx {
        skin: &c.skin,
        form: "TPREPAREGAMEDLG",
        texts: &v.texts,
        ox,
        oy,
        bg: bgc,
        focus: "",
        lists: &v.lists,
        hits: &hits,
    };
    controls(&mut c.fb, &cx, form);
    c.hits = hits.into_inner();
}
