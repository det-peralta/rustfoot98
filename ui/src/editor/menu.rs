//! The menus of the team editor (`TEditor_Main`): *Ficheiro*, *Jogador*, *Equipa*, *Elifoot*, with the states the original gives them
//! (checked on the original): *Jogador* and *Equipa* only with a team open; *Adicionar* while the squad has room; *Editar*, *Remover* and
//! *Transferir* once a player is selected; *Próxima equipa* off after *Abrir*.
use super::{Editor, MAX_PLAYERS};
use crate::classic::menu::{draw_open_at, Entry, Top};

/// Frame position of the editor's window (client 460x381 with a menu bar, centred).
pub const FRAME: (i32, i32) = (169, 89);

pub fn model(e: &Editor) -> Vec<Top> {
    let Some(form) = e.forms.get("TEDITOR_MAIN") else {
        return Vec::new();
    };
    let open = e.shown();
    let mut tops = Vec::new();
    for m in form.children.iter().filter(|n| n.class == "TMainMenu") {
        for t in &m.children {
            let entries = t
                .children
                .iter()
                .map(|x| {
                    let label = x.str("Caption").unwrap_or("").replace('&', "");
                    let enabled = match x.name.as_str() {
                        "FicheiroProximaequipa" => false,
                        "JogadorAdicionar" => {
                            open.is_some_and(|o| o.team.players.len() < MAX_PLAYERS)
                        }
                        "JogadorEditar" | "JogadorRemover" | "JogadorTransferir" => {
                            e.selected.is_some()
                        }
                        _ => x.flag("Enabled", true),
                    };
                    Entry {
                        name: x.name.clone(),
                        shortcut: x.str("ShortCutText").unwrap_or("").to_string(),
                        enabled,
                        separator: label == "-",
                        label,
                    }
                })
                .collect();
            tops.push(Top {
                name: t.name.clone(),
                label: t.str("Caption").unwrap_or("").replace('&', ""),
                entries,
                enabled: !matches!(t.name.as_str(), "Jogador" | "Equipa") || open.is_some(),
            });
        }
    }
    tops
}

/// Draws the open menu over the window.
pub fn draw_open(e: &mut Editor, tops: &[Top], open: usize, hl: Option<usize>) {
    let Editor { fb, skin, .. } = e;
    draw_open_at(fb, skin, FRAME, tops, open, hl);
}
