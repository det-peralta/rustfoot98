//! The team editor (`EDITEQ.EXE`, *Editor de Equipas*): opens a team file (`.EFT`), shows its names, coach, country, level and squad
//! in the club's colours, and edits them with the original's dialogs; the files are kept by the host.
//!
//! The order of things is the original's (`1000:10a7` .. `1000:2191` of `EDITEQ.EXE`): a team changed and not saved is offered for
//! saving before another one is opened or the program left; *Nova equipa* asks the name, colours, country, level and coach and then
//! players until one is cancelled; *Transferir* moves a player to another team when that team is next opened.
use crate::classic::dialog::Dialog;
use crate::win31::draw::{controls, dialog_frame_menu_states, Ctx, Hit};
use crate::win31::fb::{Fb, Rgb};
use crate::win31::{Forms, Skin};
use rustfoot_core::eft::{EftPlayer, EftTeam, Position};
use std::cell::RefCell;
use std::collections::BTreeMap;

pub mod dialogs;
pub mod input;
pub mod menu;
#[cfg(test)]
mod tests;

pub use input::Ev;

/// The Windows 3.1 desktop the editor sits on (the original runs over Program Manager).
pub const DESKTOP: Rgb = Rgb(0, 64, 128);

/// The size of the main window's client area (`TEditor_Main`).
pub(crate) const CLIENT: (i32, i32) = (460, 381);

/// What the host has to do for the editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Store this team file (name in `EQUIPAS`, bytes of the `.EFT`).
    Save(String, Vec<u8>),
    /// Leave the editor.
    Quit,
}

/// The team being edited.
#[derive(Clone, Debug)]
pub struct Open {
    /// File name in `EQUIPAS` (empty for a new team not yet saved).
    pub file: String,
    pub team: EftTeam,
}

pub struct Editor {
    pub skin: Skin,
    pub forms: Forms,
    pub fb: Fb,
    /// `code -> name` of `COUNTRY.TXE`, in the file's order.
    pub countries: Vec<(String, String)>,
    /// The team files of `EQUIPAS` (file name, team), as the host gave them.
    pub files: Vec<(String, EftTeam)>,
    pub open: Option<Open>,
    /// The team was changed since it was opened or saved (`10c8:238e`).
    pub modified: bool,
    /// The player selected in the list (`PlayerWasSelected` enables *Editar*, *Remover*, *Transferir*).
    pub selected: Option<usize>,
    /// The first row the list shows.
    pub top: usize,
    pub dialogs: Vec<Dialog>,
    /// The open menu: top-level index and the highlighted entry.
    pub menu: Option<(usize, Option<usize>)>,
    /// Players on their way to another team: (team file, player), placed when that team is opened.
    pub transfers: Vec<(String, EftPlayer)>,
    pub requests: Vec<Request>,
    /// The steps still to come of a new team (`Some(empty)` while *Adicionar* asks for players).
    pub new_steps: Option<Vec<dialogs::Step>>,
    /// The position of the last player added (the next new player starts there).
    pub last_position: Position,
    pub mouse: (i32, i32),
    /// The hint of the control under the pointer in the topmost dialog.
    pub hover: crate::win31::hint::Hover,
    pub hits: Vec<Hit>,
    pub dirty: bool,
}

impl Editor {
    pub fn new(
        skin: Skin,
        forms: Forms,
        countries: Vec<(String, String)>,
        files: Vec<(String, EftTeam)>,
    ) -> Editor {
        Editor {
            skin,
            forms,
            fb: Fb::new(),
            countries,
            files,
            open: None,
            modified: false,
            selected: None,
            top: 0,
            dialogs: Vec::new(),
            menu: None,
            transfers: Vec::new(),
            requests: Vec::new(),
            new_steps: None,
            last_position: Position::Goalkeeper,
            mouse: (0, 0),
            hover: Default::default(),
            hits: Vec::new(),
            dirty: true,
        }
    }

    /// The name of a country by its code (the code itself when `COUNTRY.TXE` lacks it).
    pub fn country_name(&self, code: &str) -> String {
        self.countries
            .iter()
            .find(|(c, _)| c == code)
            .map_or_else(|| code.to_string(), |(_, n)| n.clone())
    }

    /// Opens the team of `file` (*Abrir*): the players transferred to it on the way are added.
    pub fn open_file(&mut self, file: &str) {
        let Some((name, team)) = self
            .files
            .iter()
            .find(|(f, _)| f.eq_ignore_ascii_case(file))
            .cloned()
        else {
            return;
        };
        let mut team = team;
        let mut arrived = false;
        self.transfers.retain(|(to, p)| {
            if to.eq_ignore_ascii_case(&name) && team.players.len() < MAX_PLAYERS {
                team.players.push(p.clone());
                arrived = true;
                false
            } else {
                true
            }
        });
        self.open = Some(Open { file: name, team });
        self.modified = arrived;
        self.selected = None;
        self.top = 0;
        self.dirty = true;
    }

    /// The team the window shows: none while a new team is still being named, coloured, placed and given a coach (the window is
    /// drawn again only after those steps, `1000:180e`).
    pub fn shown(&self) -> Option<&Open> {
        let naming = self.new_steps.as_ref().is_some_and(|s| !s.is_empty());
        self.open.as_ref().filter(|_| !naming)
    }

    /// Time passes: the *Acerca* box scrolls its banner, a hint opens.
    pub fn tick(&mut self, ms: u32) {
        if self.hover.tick(ms) {
            self.dirty = true;
        }
        let Some(mut d) = self.dialogs.pop() else {
            return;
        };
        if d.kind == crate::classic::dialog::Kind::Other("ed_about".into()) {
            crate::classic::dialogs::about::tick_box(&self.skin, &self.forms, &mut d, ms);
            self.dirty = true;
        }
        self.dialogs.push(d);
    }

    /// Draws the whole screen into `self.fb`.
    pub fn draw(&mut self) {
        self.fb
            .fill(0, 0, crate::win31::fb::W, crate::win31::fb::H, DESKTOP);
        self.hits.clear();
        // the windows below the topmost dialog have the inactive caption
        let inactive = |on: bool| crate::win31::draw::INACTIVE.with(|i| i.set(on));
        inactive(!self.dialogs.is_empty());
        self.draw_main();
        let mut dialogs = std::mem::take(&mut self.dialogs);
        let n = dialogs.len();
        for (i, d) in dialogs.iter_mut().enumerate() {
            inactive(i + 1 < n);
            dialogs::bind(self, d);
            d.render(&mut self.fb, &self.skin, &self.forms);
        }
        inactive(false);
        if let Some(d) = dialogs.last() {
            self.hits = d.hits.clone();
            self.hover.draw(&mut self.fb, &self.skin, &d.form);
        } else {
            self.hover.draw(&mut self.fb, &self.skin, "TEDITOR_MAIN");
        }
        self.dialogs = dialogs;
        if let (Some((open, hl)), true) = (self.menu, self.dialogs.is_empty()) {
            let tops = menu::model(self);
            menu::draw_open(self, &tops, open, hl);
        }
        self.dirty = false;
    }

    fn draw_main(&mut self) {
        let Some(form) = self.forms.get("TEDITOR_MAIN") else {
            return;
        };
        let tops = menu::model(self);
        let items: Vec<(String, bool)> =
            tops.iter().map(|t| (t.label.clone(), t.enabled)).collect();
        let (texts, lists, bg) = self.view();
        let (ox, oy) = dialog_frame_menu_states(
            &mut self.fb,
            &self.skin,
            "Editor de Equipas",
            CLIENT.0,
            CLIENT.1,
            bg,
            &items,
        );
        let hits = RefCell::new(Vec::<Hit>::new());
        let cx = Ctx {
            skin: &self.skin,
            form: "TEDITOR_MAIN",
            texts: &texts,
            ox,
            oy,
            bg,
            focus: "PlayerList",
            lists: &lists,
            hits: &hits,
        };
        controls(&mut self.fb, &cx, form);
        self.hits = hits.into_inner();
    }

    /// The texts and lists of the main window and its background: the team in its colours, or the empty grey window.
    fn view(&self) -> (BTreeMap<String, String>, BTreeMap<String, Vec<String>>, Rgb) {
        let mut t = BTreeMap::new();
        let mut l = BTreeMap::new();
        let rgb = |c: [u8; 3]| format!("{},{},{}", c[0], c[1], c[2]);
        t.insert("MemoryAvailable.visible".into(), "0".into());
        let Some(o) = self.shown() else {
            for n in [
                "TeamShortName",
                "TeamLongName",
                "teamCoachName",
                "labelCoach",
                "teamInitLevel",
                "flagDraw",
            ] {
                t.insert(format!("{n}.visible"), "0".into());
            }
            // the empty list is grey (the form's colour), with the focus rectangle on its first row
            l.insert("PlayerList".into(), Vec::new());
            t.insert("PlayerList.bg".into(), "128,128,128".into());
            return (t, l, Rgb(128, 128, 128));
        };
        let team = &o.team;
        // (the file's "foreground" is the club's main colour, the window's; its "background" is the ink, as in the game)
        let fg = rgb(team.background);
        t.insert("TeamShortName".into(), team.short_name.clone());
        t.insert("TeamLongName".into(), team.full_name.clone());
        t.insert("teamCoachName".into(), team.coach.clone());
        t.insert("teamInitLevel".into(), team.level.to_string());
        // (AutoSize labels: the run-time caption sizes them, so the level's centring does not show and the names take clicks over
        // their text)
        for n in [
            "teamInitLevel",
            "TeamShortName",
            "TeamLongName",
            "teamCoachName",
        ] {
            t.insert(format!("{n}.autosize"), "1".into());
        }
        for n in [
            "TeamShortName",
            "TeamLongName",
            "teamCoachName",
            "labelCoach",
            "teamInitLevel",
        ] {
            t.insert(format!("{n}.color"), fg.clone());
        }
        t.insert("flagDraw.image".into(), format!("FLAGS/{}", team.country));
        // the flag's hint is the country's name (set by the program; the form shows hints)
        t.insert("flagDraw.hint".into(), self.country_name(&team.country));
        // the rows of `WritePlayer`: name, the country of a foreigner, the position, in the club's colours
        let rows: Vec<String> = team
            .players
            .iter()
            .map(|p| {
                let nat = if p.nationality == team.country {
                    String::new()
                } else {
                    format!("* {}", self.country_name(&p.nationality))
                };
                format!("{}|{}|{}", p.name, nat, position_word(p.position))
            })
            .collect();
        l.insert("PlayerList".into(), rows);
        t.insert("PlayerList.ink".into(), rgb(team.foreground));
        // (the list's ground is the text colour, T)
        t.insert("PlayerList.bg".into(), rgb(team.background));
        t.insert("PlayerList.selink".into(), fg);
        t.insert("PlayerList.top".into(), self.top.to_string());
        if let Some(s) = self.selected {
            t.insert("PlayerList.sel".into(), s.to_string());
        }
        let b = team.foreground;
        (t, l, Rgb(b[0], b[1], b[2]))
    }
}

/// A squad holds at most this many players (*Adicionar* is disabled with a full squad).
pub const MAX_PLAYERS: usize = 20;

/// The position as the list writes it.
pub fn position_word(p: Position) -> &'static str {
    match p {
        Position::Goalkeeper => "Guarda-redes",
        Position::Defender => "Defesa",
        Position::Midfielder => "Médio",
        Position::Attacker => "Avançado",
    }
}
