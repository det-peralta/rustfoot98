//! The dialogs of the original, one module per group. Each module offers `open_*` functions that build a [`Dialog`], and is called from
//! the four dispatchers below by the dialog's [`Kind`]: `bind` (fill texts and lists from the game before every frame), `click`
//! (a control was pressed), `key` (a key the generic handling of `dialog::generic_key` did not use) and `tick` (time passed).
//!
//! Adding a dialog: create the module, add its arms to the dispatchers. See `docs/classic-dialogs.md`.
use super::dialog::{Dialog, Kind, Outcome};
use super::{Classic, Key};

pub mod about;
pub mod coach;
pub mod market;
pub mod matchday;
pub mod misc;
pub mod newgame;
pub mod tables;

/// A message box with an OK button (the VCL's `ShowMessage`).
pub fn message(caption: &str, text: &str) -> Dialog {
    misc::message(caption, text)
}

/// Fills the texts and lists of the dialog from the game state; runs before each frame.
pub fn bind(c: &Classic, d: &mut Dialog) {
    match &d.kind {
        Kind::About => about::bind(c, d),
        Kind::SelectGame | Kind::CoachName | Kind::Progress => newgame::bind(c, d),
        Kind::Halftime
        | Kind::ForcedSubstitution
        | Kind::Penalty
        | Kind::Shootout
        | Kind::CupDraw
        | Kind::EndOfSeason
        | Kind::RefereeAid => matchday::bind(c, d),
        Kind::Standings
        | Kind::Scorers
        | Kind::Calendar
        | Kind::Winners
        | Kind::AllTimeScorers
        | Kind::Transactions
        | Kind::Acquisitions => tables::bind(c, d),
        Kind::Sell
        | Kind::Contract
        | Kind::Search
        | Kind::Observed
        | Kind::Auction
        | Kind::Plantel => market::bind(c, d),
        Kind::CoachHistory
        | Kind::CoachManager
        | Kind::CoachRanking
        | Kind::CoachProfile
        | Kind::TeamHistory
        | Kind::Invitation => coach::bind(c, d),
        Kind::Message | Kind::Options | Kind::Stadium | Kind::Loan | Kind::Other(_) => {
            misc::bind(c, d)
        }
    }
}

/// A control was pressed (`name` is the control's name; list rows are `List#row`).
pub fn click(c: &mut Classic, d: &mut Dialog, name: &str) -> Outcome {
    match &d.kind {
        Kind::About => about::click(c, d, name),
        Kind::SelectGame | Kind::CoachName | Kind::Progress => newgame::click(c, d, name),
        Kind::Halftime
        | Kind::ForcedSubstitution
        | Kind::Penalty
        | Kind::Shootout
        | Kind::CupDraw
        | Kind::EndOfSeason
        | Kind::RefereeAid => matchday::click(c, d, name),
        Kind::Standings
        | Kind::Scorers
        | Kind::Calendar
        | Kind::Winners
        | Kind::AllTimeScorers
        | Kind::Transactions
        | Kind::Acquisitions => tables::click(c, d, name),
        Kind::Sell
        | Kind::Contract
        | Kind::Search
        | Kind::Observed
        | Kind::Auction
        | Kind::Plantel => market::click(c, d, name),
        Kind::CoachHistory
        | Kind::CoachManager
        | Kind::CoachRanking
        | Kind::CoachProfile
        | Kind::TeamHistory
        | Kind::Invitation => coach::click(c, d, name),
        Kind::Message | Kind::Options | Kind::Stadium | Kind::Loan | Kind::Other(_) => {
            misc::click(c, d, name)
        }
    }
}

/// A key the generic handling did not use (arrows in lists, shortcut letters...). `None` when the dialog ignores it.
pub fn key(c: &mut Classic, d: &mut Dialog, key: Key) -> Option<Outcome> {
    match &d.kind {
        Kind::Halftime | Kind::Penalty | Kind::Shootout | Kind::CupDraw | Kind::EndOfSeason => {
            matchday::key(c, d, key)
        }
        Kind::Standings
        | Kind::Scorers
        | Kind::Calendar
        | Kind::Winners
        | Kind::AllTimeScorers
        | Kind::Transactions
        | Kind::Acquisitions => tables::key(c, d, key),
        Kind::Sell
        | Kind::Contract
        | Kind::Search
        | Kind::Observed
        | Kind::Auction
        | Kind::Plantel => market::key(c, d, key),
        Kind::SelectGame | Kind::CoachName => newgame::key(c, d, key),
        _ => None,
    }
}

/// Time passed (milliseconds): animations, timers of the forms.
pub fn tick(c: &mut Classic, d: &mut Dialog, ms: u32) -> Outcome {
    match &d.kind {
        Kind::About => about::tick(c, d, ms),
        Kind::Progress => newgame::tick(c, d, ms),
        Kind::Halftime | Kind::Penalty | Kind::Shootout | Kind::CupDraw | Kind::EndOfSeason => {
            matchday::tick(c, d, ms)
        }
        Kind::Standings
        | Kind::Scorers
        | Kind::Calendar
        | Kind::Winners
        | Kind::AllTimeScorers
        | Kind::Transactions
        | Kind::Acquisitions => tables::tick(c, d, ms),
        _ => Outcome::Keep,
    }
}

/// The dialog a menu entry opens (`name` is the entry's control name in `TPREPAREGAMEDLG`).
pub fn open_for_menu(c: &Classic, name: &str) -> Option<Dialog> {
    let (kind, form) = match name {
        "Acerca1" => (Kind::About, "TABOUTDLG"),
        "opcoesElifoot" => (Kind::Options, "TOPTIONSDLG"),
        "Pediremprestimo1" => (Kind::Loan, "TLOANDLG"),
        "Estdio1" => (Kind::Stadium, "TSTADIUMDLG"),
        "Historial1" => (Kind::TeamHistory, "TPRINTLISTDLG"),
        "Jogadoressobobservao1" => (Kind::Observed, "TOBSERVEDPLAYERSDLG"),
        "JogadorProcurar1" => (Kind::Search, "TCHECKPLAYERDLG"),
        "ltimastransferncias1" => (Kind::Transactions, "TPRINTLISTDLG"),
        "EquipaAquisicoes1" => (Kind::Acquisitions, "TPRINTLISTDLG"),
        "CampeonatoClassificao" => (Kind::Standings, "TCLASSIFICATIONDLG"),
        "campeonatoMelhoresmarcadores" => (Kind::Scorers, "TPRINTLISTDLG"),
        "menuCalendrio" => (Kind::Calendar, "TPRINTLISTDLG"),
        "ltimosvencedores1" => (Kind::Winners, "TPRINTLISTDLG"),
        "Melhoresmarcadoresdesempre1" => (Kind::AllTimeScorers, "TPRINTLISTDLG"),
        "treinadorHistoria" => (Kind::CoachHistory, "TPRINTLISTDLG"),
        "TreinadorGestao" => (Kind::CoachManager, "TCOACHMANAGERDLG"),
        "coachRanking" => (Kind::CoachRanking, "TPRINTLISTDLG"),
        "Opesindividuais1" => (Kind::CoachProfile, "TCOACHOPTIONSDLG"),
        _ => return None,
    };
    let _ = c;
    Some(Dialog::new(kind, form))
}

/// The dialog a button of the main window opens.
pub fn open_for_button(c: &Classic, name: &str) -> Option<Dialog> {
    let (kind, form) = match name {
        "opponentCalendarBtn" => (Kind::Calendar, "TPRINTLISTDLG"),
        "opponentPlantelBtn" => (Kind::Plantel, "TPLANTELDLG"),
        _ => return None,
    };
    let _ = c;
    let mut d = Dialog::new(kind, form);
    if name == "opponentCalendarBtn" {
        // the calendar of the next opponent, not of the user's club
        d.st.insert("opponent".into(), "1".into());
    }
    Some(d)
}
