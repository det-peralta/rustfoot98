use super::dialogs;
use super::{Editor, Request};
use crate::classic::dialog::Kind;
use crate::win31::{Forms, Skin};
use rustfoot_core::eft::{self, EftPlayer, EftTeam, Position};

fn player(name: &str, nat: &str, position: Position) -> EftPlayer {
    EftPlayer {
        nationality: nat.into(),
        name: name.into(),
        position,
    }
}

/// A team that can be saved: 2 keepers and 14 field players, one of them foreign.
fn team(name: &str) -> EftTeam {
    let mut players = vec![
        player("Keeper A", "POR", Position::Goalkeeper),
        player("Keeper B", "POR", Position::Goalkeeper),
    ];
    for i in 0..14 {
        players.push(player(
            &format!("Field {i}"),
            if i == 0 { "BRA" } else { "POR" },
            Position::Midfielder,
        ));
    }
    EftTeam {
        full_name: format!("{name} FC"),
        short_name: name.into(),
        background: [255, 255, 255],
        foreground: [0, 0, 128],
        country: "POR".into(),
        level: 10,
        players,
        coach: "Coach".into(),
    }
}

fn editor() -> Editor {
    let countries = vec![
        ("POR".into(), "Portugal".into()),
        ("BRA".into(), "Brasil".into()),
    ];
    let files = vec![
        ("AAA.EFT".into(), team("AAA")),
        ("BBB.EFT".into(), team("BBB")),
    ];
    Editor::new(Skin::default(), Forms::default(), countries, files)
}

fn top(e: &Editor) -> &str {
    match &e.dialogs.last().expect("a dialog").kind {
        Kind::Other(k) => k,
        _ => "",
    }
}

#[test]
fn reasons_follow_the_bits_of_the_check() {
    let e = editor();
    let mut t = team("AAA");
    assert!(dialogs::reasons(&e, &t).is_empty());
    t.players.retain(|p| p.position != Position::Goalkeeper);
    t.players.truncate(9);
    t.coach.clear();
    t.level = 0;
    assert_eq!(
        dialogs::reasons(&e, &t),
        [
            "Não está definido o nível da equipa",
            "Não tem jogadores suficientes",
            "Não tem guarda-redes",
            "Não tem jogadores de campo suficientes",
            "Não está definido o treinador",
        ]
    );
    // Brazilians are of Portugal's group (PLOP): only the others count as foreigners
    let mut t = team("AAA");
    for p in t.players.iter_mut().skip(2).take(8) {
        p.nationality = "BRA".into();
    }
    assert!(dialogs::reasons(&e, &t).is_empty());
    for p in t.players.iter_mut().skip(2).take(6) {
        p.nationality = "ARG".into();
    }
    assert_eq!(
        dialogs::reasons(&e, &t),
        ["Tem demasiados jogadores estrangeiros"]
    );
}

#[test]
fn a_new_team_asks_its_steps_then_players_until_cancel() {
    let mut e = editor();
    e.command("Novaequipa");
    assert_eq!(top(&e), "ed_name");
    let d = e.dialogs.last_mut().unwrap();
    d.text("TeamLongName", "Clube Novo");
    d.text("TeamShortName", "novo");
    e.press("OKBtn");
    assert_eq!(top(&e), "ed_color");
    e.press("ColorMatrix#9");
    e.press("ColorMatrix#15!r");
    e.press("OKBtn");
    assert_eq!(top(&e), "ed_country");
    e.press("OKBtn");
    assert_eq!(top(&e), "ed_level");
    e.press("OKBtn");
    assert_eq!(top(&e), "ed_coach");
    e.dialogs.last_mut().unwrap().text("coachName", "Treinador");
    e.press("OKBtn");
    for name in ["Um", "Dois"] {
        assert_eq!(top(&e), "ed_player");
        e.dialogs.last_mut().unwrap().text("textName", name);
        e.press("RadioDefender");
        e.press("OKBtn");
    }
    e.press("CancelBtn");
    assert!(e.dialogs.is_empty());
    let t = &e.open.as_ref().unwrap().team;
    assert_eq!(
        (t.full_name.as_str(), t.short_name.as_str()),
        ("Clube Novo", "NOVO")
    );
    assert_eq!((t.background, t.foreground), ([255, 0, 0], [255, 255, 255]));
    // the sorted country list starts with Brasil
    assert_eq!(t.country, "BRA");
    assert_eq!(t.coach, "Treinador");
    assert_eq!(t.players.len(), 2);
    assert!(t
        .players
        .iter()
        .all(|p| p.position == Position::Defender && p.nationality == "BRA"));
    assert!(e.modified);
}

#[test]
fn cancelling_the_name_drops_the_new_team() {
    let mut e = editor();
    e.command("Novaequipa");
    e.press("CancelBtn");
    assert!(e.dialogs.is_empty());
    assert!(e.open.is_none());
    assert!(!e.modified);
}

#[test]
fn a_changed_team_is_saved_before_another_is_opened() {
    let mut e = editor();
    e.open_file("AAA.EFT");
    e.command("EquipaTreinador");
    e.dialogs.last_mut().unwrap().text("coachName", "Outro");
    e.press("OKBtn");
    assert!(e.modified);
    e.command("Abrir1");
    assert_eq!(top(&e), "ed_save");
    e.press("OKBtn");
    let Some(Request::Save(file, bytes)) = e.requests.first() else {
        panic!("no save: {:?}", e.requests);
    };
    assert_eq!(file, "AAA.EFT");
    assert_eq!(eft::parse(bytes).unwrap().coach, "Outro");
    assert_eq!(top(&e), "ed_open");
    assert!(!e.modified);
}

#[test]
fn a_team_that_breaks_the_rules_is_not_saved() {
    let mut e = editor();
    e.open_file("AAA.EFT");
    e.selected = Some(0);
    e.command("JogadorRemover");
    e.selected = Some(0);
    e.command("JogadorRemover");
    assert_eq!(e.selected, Some(0));
    e.command("FicheiroSair");
    e.press("OKBtn");
    assert_eq!(top(&e), "ed_cantsave");
    assert_eq!(
        e.dialogs.last().unwrap().lists["listOfReasons"],
        ["Não tem guarda-redes"]
    );
    assert!(e.requests.is_empty());
    // without "Gravar a equipa" the changes are dropped and the editor ends
    e.press("OKBtn");
    e.command("FicheiroSair");
    let d = e.dialogs.last_mut().unwrap();
    d.text("saveTheTeam.checked", "0");
    e.press("OKBtn");
    assert_eq!(e.requests, [Request::Quit]);
}

#[test]
fn a_transferred_player_arrives_when_his_new_team_is_opened() {
    let mut e = editor();
    e.open_file("AAA.EFT");
    e.selected = Some(2);
    e.command("JogadorTransferir");
    assert_eq!(e.dialogs.last().unwrap().lists["listBoxOfTeams"], ["BBB"]);
    e.press("listBoxOfTeams#0");
    e.press("OKBtn");
    assert_eq!(e.open.as_ref().unwrap().team.players.len(), 15);
    assert_eq!(e.transfers.len(), 1);
    e.modified = false;
    e.open_file("BBB.EFT");
    let t = &e.open.as_ref().unwrap().team;
    assert_eq!(t.players.len(), 17);
    assert_eq!(t.players[16].name, "Field 0");
    assert!(e.transfers.is_empty());
    assert!(e.modified);
}

#[test]
fn the_shipped_teams_pass_the_check_if_available() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../original/elifoot98");
    let (Ok(rd), Ok(txe)) = (
        std::fs::read_dir(root.join("EQUIPAS")),
        std::fs::read(root.join("COUNTRY.TXE")),
    ) else {
        return;
    };
    let countries = rustfoot_core::txe::parse(&txe)
        .iter()
        .skip(1)
        .filter_map(|l| {
            l.split_once(' ')
                .map(|(a, b)| (a.to_string(), b.to_string()))
        })
        .collect();
    let e = Editor::new(Skin::default(), Forms::default(), countries, Vec::new());
    let mut bad = Vec::new();
    for f in rd.flatten() {
        let t = eft::parse(&std::fs::read(f.path()).unwrap()).unwrap();
        // (two shipped files, CRUZEIRO and INTER DE LIMEIRA, have no coach: the original's editor would not save them as they are)
        let why: Vec<_> = dialogs::reasons(&e, &t)
            .into_iter()
            .filter(|w| *w != "Não está definido o treinador")
            .collect();
        if !why.is_empty() {
            bad.push(format!("{}: {why:?}", t.short_name));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}
