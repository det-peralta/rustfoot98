//! The coach dialogs: the history of the user's club (`Equipa > Historial`) and coach (`Treinador > História`), the ranking, the
//! manager of the user's coaches (`TCoachManagerDlg`) and the coach profile (`TCoachOptionsDlg`). The lists are `TPrintListDlg`s
//! (`tables.rs`) with the rows of `TTeamEvent` (`1010:bb49`), `TCoachEvent` (`1010:b6df`) and `TCoach` (`1010:879d`).
//!
//! The invitation dialog of a sacked club (`TInviteCoachDlg`) is not here: the match-day module owns it.
use super::tables::{
    self, bind_list, field, human_coach, numbered, Spec, NUMBERED, SELECT_LAST, WIDE,
};
use crate::classic::dialog::{Dialog, Kind, Outcome};
use crate::classic::{Classic, Key};
use elifoot_core::history::CoachOptions;
use elifoot_core::world::World;

fn coach_kind(k: &Kind) -> bool {
    matches!(
        k,
        Kind::CoachHistory | Kind::CoachRanking | Kind::TeamHistory
    )
}

/// `TTeamEvent`: year, the title or place and the coach (the user's coach in bold).
pub fn team_history(w: &World) -> Spec {
    let Some(h) = w.human.as_ref() else {
        return Spec {
            caption: String::new(),
            rows: Vec::new(),
            flags: SELECT_LAST,
        };
    };
    let club = w
        .divisions
        .iter()
        .flatten()
        .chain(w.pool.iter())
        .find(|c| c.name == h.club);
    let me = human_coach(w);
    let rows = w
        .history
        .club_events
        .get(&h.club)
        .map(|v| {
            v.iter()
                .map(|e| {
                    let mut r = field("", 0x1e, &e.year.to_string());
                    let bold = !e.coach.is_empty() && e.coach == me;
                    if bold {
                        r.push('\u{2}');
                    }
                    let title = match e.kind {
                        1 => "Campeonato",
                        2 => "Taça",
                        3 => "DOBRADINHA",
                        4 => "2º lugar",
                        5 => "3º lugar",
                        _ => "",
                    };
                    if !title.is_empty() {
                        r += &field("", 0x50, title);
                    }
                    if !e.coach.is_empty() {
                        r += &field("", 0, &e.coach);
                    }
                    r
                })
                .collect()
        })
        .unwrap_or_default();
    Spec {
        caption: club.map(|c| c.full_name.clone()).unwrap_or_default(),
        rows,
        flags: SELECT_LAST,
    }
}

/// `TCoachEvent`: the career of the user's coach.
pub fn coach_history(w: &World) -> Spec {
    let me = human_coach(w);
    let rows = w
        .history
        .coach_events
        .get(&me)
        .map(|v| {
            v.iter()
                .map(|e| {
                    let mut r = field("", 0x1e, &e.year.to_string());
                    r += &match e.kind {
                        1 => field("", 0, &format!("Contratado pelo {}", e.club)),
                        2 => field("", 0, "Despedido"),
                        3 => field("\u{2}", 0, "VENCEDOR DO CAMPEONATO"),
                        4 => field("\u{2}", 0, "VENCEDOR DA TAÇA"),
                        5 => field("", 0, &format!("Ingresso no {}", e.club)),
                        6 => field("", 0, &format!("Despromovido para o {}", e.club)),
                        7 => field("", 0, &format!("Promovido para o {}", e.club)),
                        8 => field("\u{2}", 0, "DOBRADINHA"),
                        9 => field("", 0x50, "2º lugar"),
                        10 => field("", 0x50, "3º lugar"),
                        _ => String::new(),
                    };
                    r
                })
                .collect()
        })
        .unwrap_or_default();
    Spec {
        caption: me,
        rows,
        flags: SELECT_LAST,
    }
}

/// `TCoach` rows of the ranking: coach (the user's in bold, for the whole row), club, `league titles + cup titles = total` and the
/// score of the coach (the add-in "Ver pontuação dos treinadores", offered here).
pub fn ranking(w: &World) -> Spec {
    let rows = w
        .coach_ranking()
        .into_iter()
        .enumerate()
        .map(|(k, r)| {
            let ((l, c), human, score) = (r.titles, r.human, r.score);
            let (coach, club) = (r.coach, r.club);
            let mut r = String::new();
            if human {
                r.push('\u{2}');
            }
            r += &field("", 0x82, &coach);
            r += &field("", 0xb4, &club);
            r += &field("\u{6}", 0x14, &format!("{l} + "));
            r += &field("\u{6}", 0x14, &format!("{c} = "));
            r += &field("\u{6}", 0x14, &(l + c).to_string());
            r += &field("\u{6}", 0x28, &score.to_string());
            numbered(k, r)
        })
        .collect();
    Spec {
        caption: "Ranking de Treinadores".into(),
        rows,
        flags: NUMBERED | WIDE,
    }
}

pub fn bind(c: &Classic, d: &mut Dialog) {
    if coach_kind(&d.kind) {
        if !d.st.contains_key("built") {
            let spec = match d.kind {
                Kind::TeamHistory => team_history(&c.world),
                Kind::CoachHistory => coach_history(&c.world),
                _ => ranking(&c.world),
            };
            bind_list(d, spec);
        }
        return;
    }
    match d.kind {
        Kind::CoachManager => bind_manager(c, d),
        Kind::CoachProfile => bind_profile(c, d),
        _ => {}
    }
}

pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if coach_kind(&d.kind) {
        return tables::click_list(d, name);
    }
    match d.kind {
        Kind::CoachManager => click_manager(c, d, name),
        Kind::CoachProfile => click_profile(c, d, name),
        _ => Outcome::Keep,
    }
}

pub fn key(_c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    if coach_kind(&d.kind) {
        return tables::key_list(d, key);
    }
    None
}

pub fn tick(_c: &mut Classic, _d: &mut Dialog, _ms: u32) -> Outcome {
    Outcome::Keep
}

// ---- TCoachManagerDlg ---------------------------------------------------------------------------------------------------

/// The user's coaches as the manager lists them (`TCoach` row with flags `0x13` and `0x8000`: name, club, "férias"). The port has
/// one user.
fn manager_rows(w: &World) -> Vec<String> {
    let Some(h) = w.human.as_ref() else {
        return Vec::new();
    };
    let mut r = field("", 0x82, &human_coach(w));
    r += &field("", 0xb4, &h.club);
    if w.history.holiday {
        r += &field("", 0x50, "férias");
    }
    vec![r]
}

fn bind_manager(c: &Classic, d: &mut Dialog) {
    d.lists
        .insert("ListBoxOfCoaches".into(), manager_rows(&c.world));
    let sel = d.texts.contains_key("ListBoxOfCoaches.sel");
    // the original allows two users and offers adding or removing one; the port plays one coach, so both buttons stay off
    d.text("addBtn.enabled", "0");
    d.text("removeBtn.enabled", "0");
    d.text("CheckBoxOnHoliday.enabled", if sel { "1" } else { "0" });
    d.text(
        "CheckBoxOnHoliday.checked",
        if sel && c.world.history.holiday {
            "1"
        } else {
            "0"
        },
    );
}

fn click_manager(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if let Some(row) = name.strip_prefix("ListBoxOfCoaches#") {
        d.text("ListBoxOfCoaches.sel", row);
        return Outcome::Keep;
    }
    match name {
        "closeBtn" => Outcome::Close,
        "CheckBoxOnHoliday" if d.texts.contains_key("ListBoxOfCoaches.sel") => {
            c.world.history.holiday = !c.world.history.holiday;
            Outcome::Keep
        }
        _ => Outcome::Keep,
    }
}

// ---- TCoachOptionsDlg ---------------------------------------------------------------------------------------------------

const BOXES: [&str; 3] = [
    "CheckBoxNationalPlayersSalary",
    "CheckBoxBosmanPlayersSalary",
    "CheckBoxForeignerPlayersSalary",
];

fn bind_profile(c: &Classic, d: &mut Dialog) {
    if d.st.contains_key("built") {
        return;
    }
    // the working copy of the options: written to the world by OK
    let o = c.world.history.options;
    for (k, b) in BOXES.iter().enumerate() {
        d.text(&format!("{b}.checked"), if o.salary[k] { "1" } else { "0" });
    }
    d.text(
        "CheckBoxEncryptOffers.checked",
        if o.encrypt { "1" } else { "0" },
    );
    d.text("RadioGroupBuyInAuction.index", o.auction.to_string());
    d.text("TabbedNotebook1.page", "0");
    d.text("@caption", format!("Perfil de {}", human_coach(&c.world)));
    d.focus = "OKBtn".into();
    d.st.insert("built".into(), "1".into());
}

/// The options the dialog's working copy holds.
pub fn profile_options(d: &Dialog) -> CoachOptions {
    let on = |k: &str| d.texts.get(k).is_some_and(|v| v == "1");
    CoachOptions {
        salary: [
            on("CheckBoxNationalPlayersSalary.checked"),
            on("CheckBoxBosmanPlayersSalary.checked"),
            on("CheckBoxForeignerPlayersSalary.checked"),
        ],
        auction: d
            .texts
            .get("RadioGroupBuyInAuction.index")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1u8)
            .min(2),
        encrypt: on("CheckBoxEncryptOffers.checked"),
    }
}

fn click_profile(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    if let Some(i) = name.strip_prefix("TabbedNotebook1#") {
        d.text("TabbedNotebook1.page", i);
        return Outcome::Keep;
    }
    if let Some(i) = name.strip_prefix("RadioGroupBuyInAuction#") {
        d.text("RadioGroupBuyInAuction.index", i);
        return Outcome::Keep;
    }
    match name {
        "OKBtn" => {
            c.world.history.options = profile_options(d);
            Outcome::Close
        }
        "CancelBtn" => Outcome::Close,
        n if BOXES.contains(&n) || n == "CheckBoxEncryptOffers" => {
            let k = format!("{n}.checked");
            let now = d.texts.get(&k).is_some_and(|v| v == "1");
            d.text(&k, if now { "0" } else { "1" });
            Outcome::Keep
        }
        _ => Outcome::Keep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coach_rows_follow_the_originals_strings() {
        let mut w = World::synthetic(8);
        w.play_season();
        let r = ranking(&w);
        assert_eq!(r.rows.len(), 38);
        assert_eq!(r.flags, NUMBERED | WIDE);
        // the first row: the number, then the coach
        assert!(r.rows[0].starts_with("\u{6}\u{14}1\t"));
        assert_eq!(team_history(&w).flags, SELECT_LAST);
        assert_eq!(coach_history(&w).caption, human_coach(&w));
    }

    #[test]
    fn team_history_lists_titles_and_places() {
        let mut w = World::synthetic(8);
        let (d, i) = w.human_location().unwrap();
        w.divisions[d][i].coach_name = "Silva".into();
        let club = w.human.as_ref().unwrap().club.clone();
        let coach = human_coach(&w);
        w.history.add_club_event(&club, 1, 1999, &coach);
        w.history.add_club_event(&club, 5, 2000, "Outro");
        let r = team_history(&w);
        assert_eq!(r.rows.len(), 2);
        assert_eq!(
            r.rows[0],
            format!("\u{1e}1999\t\u{2}\u{50}Campeonato\t\u{0}{coach}\t")
        );
        assert_eq!(r.rows[1], "\u{1e}2000\t\u{50}3º lugar\t\u{0}Outro\t");
    }

    #[test]
    fn the_profile_dialog_holds_a_working_copy_of_the_options() {
        let mut d = Dialog::new(Kind::CoachProfile, "TCOACHOPTIONSDLG");
        d.text("CheckBoxNationalPlayersSalary.checked", "1");
        d.text("CheckBoxBosmanPlayersSalary.checked", "0");
        d.text("CheckBoxForeignerPlayersSalary.checked", "1");
        d.text("CheckBoxEncryptOffers.checked", "1");
        d.text("RadioGroupBuyInAuction.index", "2");
        assert_eq!(
            profile_options(&d),
            CoachOptions {
                salary: [true, false, true],
                auction: 2,
                encrypt: true
            }
        );
    }

    #[test]
    fn coach_history_rows_per_event_kind() {
        let mut w = World::synthetic(8);
        let (d, i) = w.human_location().unwrap();
        w.divisions[d][i].coach_name = "Silva".into();
        let me = human_coach(&w);
        w.history.add_coach_event(&me, 5, 2026, "Santos");
        w.history.add_coach_event(&me, 3, 2027, "Santos");
        w.history.add_coach_event(&me, 2, 2028, "");
        let r = coach_history(&w);
        assert_eq!(r.rows[0], "\u{1e}2026\t\u{0}Ingresso no Santos\t");
        assert_eq!(r.rows[1], "\u{1e}2027\t\u{2}\u{0}VENCEDOR DO CAMPEONATO\t");
        assert_eq!(r.rows[2], "\u{1e}2028\t\u{0}Despedido\t");
    }
}
