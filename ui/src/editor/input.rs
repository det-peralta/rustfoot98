//! The editor's answers to the mouse and the keyboard: the menus, the main window's labels and player list (`TEditor_Main`'s
//! `OnClick` handlers) and the dialogs on top.
use super::dialogs::{self, After, Step};
use super::{menu, Editor, Open, Request, MAX_PLAYERS};
use crate::classic::dialog::{self, Dialog, KeyAct, Outcome};
use crate::classic::menu::{bar_items_at, entry_at_in};
use crate::classic::{widget_input, Key};
use rustfoot_core::eft::{EftTeam, Position};

/// Input events from the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Move(i32, i32),
    /// A press of the left (`false`) or right (`true`) button.
    Down(i32, i32, bool),
    /// A double click (after its first `Down`).
    DblClick(i32, i32),
    Key(Key),
}

impl Editor {
    pub fn event(&mut self, ev: Ev) {
        self.dirty = true;
        if let Ev::Move(x, y) = ev {
            self.mouse = (x, y);
        }
        match (ev, self.dialogs.last()) {
            (Ev::Move(x, y), Some(d)) => {
                self.hover
                    .moved(&d.form, self.forms.get(&d.form), &d.hits, x, y);
            }
            (Ev::Move(x, y), None) => {
                let texts = self.view().0;
                self.hover.moved_with(
                    "TEDITOR_MAIN",
                    self.forms.get("TEDITOR_MAIN"),
                    &self.hits,
                    Some(&texts),
                    x,
                    y,
                );
            }
            (Ev::Down(..) | Ev::DblClick(..) | Ev::Key(_), _) => {
                self.hover.cancel();
            }
        }
        if !self.dialogs.is_empty() {
            self.dialog_event(ev);
        } else {
            self.main_event(ev);
        }
    }

    /// A menu command or its shortcut (the names of `TEditor_Main`'s menu items).
    pub fn command(&mut self, name: &str) {
        let has_team = self.open.is_some();
        let d = match name {
            "Novaequipa" => self.ask_save(After::New),
            "Abrir1" => self.ask_save(After::Open),
            "FicheiroSair" => self.ask_save(After::Quit),
            "JogadorAdicionar"
                if self
                    .open
                    .as_ref()
                    .is_some_and(|o| o.team.players.len() < MAX_PLAYERS) =>
            {
                self.new_steps = Some(Vec::new());
                Some(dialogs::open_player(self, None, self.last_position))
            }
            "JogadorEditar" => self
                .selected
                .map(|i| dialogs::open_player(self, Some(i), Position::Goalkeeper)),
            "JogadorRemover" => {
                self.remove_selected();
                None
            }
            "JogadorTransferir" => self.selected.map(|i| dialogs::open_transfer(self, i)),
            "Nome1" if has_team => self.team_dialog(dialogs::open_name),
            "TeamColor" if has_team => self.team_dialog(dialogs::open_colors),
            "EquipaPais" if has_team => {
                let code = self
                    .open
                    .as_ref()
                    .map_or(String::new(), |o| o.team.country.clone());
                Some(dialogs::open_country(self, &code, None))
            }
            "EquipaNivelInicial" if has_team => self.team_dialog(dialogs::open_level),
            "EquipaTreinador" if has_team => self.team_dialog(dialogs::open_coach),
            "Acerca1" => Some(dialogs::open_about()),
            _ => None,
        };
        if let Some(d) = d {
            self.dialogs.push(d);
        }
    }

    /// A dialog of the team (`Equipa`): the list's selection is dropped when it closes.
    fn team_dialog(&mut self, open: fn(&EftTeam) -> Dialog) -> Option<Dialog> {
        self.open.as_ref().map(|o| open(&o.team))
    }

    /// *Remover* (`1000:2191`): the player goes at once, the selection stays on the same row.
    fn remove_selected(&mut self) {
        let (Some(i), Some(o)) = (self.selected, self.open.as_mut()) else {
            return;
        };
        if i < o.team.players.len() {
            o.team.players.remove(i);
            self.modified = true;
            let n = o.team.players.len();
            self.selected = (n > 0).then(|| i.min(n - 1));
        }
    }

    /// The save prompt of a changed team (`1000:10a7`), or straight on with what was asked.
    fn ask_save(&mut self, after: After) -> Option<Dialog> {
        if self.modified && self.open.is_some() {
            return Some(dialogs::open_save(self, after));
        }
        match self.go_on(match after {
            After::Open => "open",
            After::New => "new",
            After::Quit => "quit",
        }) {
            Outcome::Replace(d) | Outcome::Open(d) => Some(*d),
            _ => None,
        }
    }

    /// What follows the save prompt: the list of teams, a new team's first step, or the end.
    pub fn go_on(&mut self, after: &str) -> Outcome {
        match after {
            "open" => Outcome::Replace(Box::new(dialogs::open_team(self))),
            "new" => {
                let team = EftTeam {
                    full_name: String::new(),
                    short_name: String::new(),
                    background: [0, 0, 0],
                    foreground: [255, 255, 255],
                    country: String::new(),
                    level: 10,
                    players: Vec::new(),
                    coach: String::new(),
                };
                let d = dialogs::open_name(&team);
                self.open = Some(Open {
                    file: String::new(),
                    team,
                });
                self.selected = None;
                self.modified = true;
                self.new_steps = Some(vec![
                    Step::Colors,
                    Step::Country,
                    Step::Level,
                    Step::Coach,
                    Step::Players,
                ]);
                Outcome::Replace(Box::new(d))
            }
            "quit" => {
                self.requests.push(Request::Quit);
                Outcome::Close
            }
            _ => Outcome::Close,
        }
    }

    /// Presses the named control of the dialog on top (as a click on it would).
    pub fn press(&mut self, name: &str) {
        if let Some(mut d) = self.dialogs.pop() {
            // (bound first: the enabled buttons follow what was typed)
            dialogs::bind(self, &mut d);
            let out = dialogs::click(self, &mut d, name);
            self.apply(d, out);
            self.dirty = true;
        }
    }

    fn apply(&mut self, d: Dialog, out: Outcome) {
        let team = ["ed_name", "ed_color", "ed_country", "ed_level", "ed_coach"]
            .iter()
            .any(|k| d.kind == crate::classic::dialog::Kind::Other(k.to_string()));
        if team && !matches!(out, Outcome::Keep | Outcome::Open(_)) && !d.st.contains_key("player")
        {
            self.selected = None;
        }
        match out {
            Outcome::Keep => self.dialogs.push(d),
            Outcome::Close => {}
            Outcome::Open(n) => {
                self.dialogs.push(d);
                self.dialogs.push(*n);
            }
            Outcome::Replace(n) => self.dialogs.push(*n),
        }
    }

    fn dialog_event(&mut self, ev: Ev) {
        let Some(mut d) = self.dialogs.pop() else {
            return;
        };
        dialogs::bind(self, &mut d);
        let out = match ev {
            Ev::Move(..) => Outcome::Keep,
            Ev::Down(x, y, right) => self.dialog_press(&mut d, x, y, right, false),
            Ev::DblClick(x, y) => self.dialog_press(&mut d, x, y, false, true),
            // a spin edit keeps Enter for itself (checked on the level dialog: OK needs the mouse or Tab)
            Ev::Key(Key::Enter)
                if d.hits
                    .iter()
                    .any(|h| h.name == d.focus && h.class == "TSpinEdit") =>
            {
                Outcome::Keep
            }
            // type-ahead in a list box: the next row that starts with the letter
            Ev::Key(Key::Char(c))
                if c.is_alphanumeric()
                    && d.hits
                        .iter()
                        .any(|h| h.name == d.focus && h.class == "TListBox") =>
            {
                let list = d.focus.clone();
                type_ahead(&mut d, &list, c);
                Outcome::Keep
            }
            Ev::Key(k) => match dialog::generic_key_in(&self.forms, &mut d, k) {
                KeyAct::Press(name) => dialogs::click(self, &mut d, &name),
                KeyAct::Done | KeyAct::None => Outcome::Keep,
            },
        };
        self.apply(d, out);
    }

    /// A press on a dialog: the colour grid takes both buttons, the other controls the left one; a double click on a list row is
    /// its OK.
    fn dialog_press(
        &mut self,
        d: &mut Dialog,
        x: i32,
        y: i32,
        right: bool,
        double: bool,
    ) -> Outcome {
        let Some((name, class, true)) = d
            .hit_at(x, y)
            .map(|h| (h.name.clone(), h.class.clone(), h.enabled))
        else {
            return Outcome::Keep;
        };
        if class == "ColorCell" {
            let name = if right { format!("{name}!r") } else { name };
            return dialogs::click(self, d, &name);
        }
        if right {
            return Outcome::Keep;
        }
        match widget_input::mouse(&self.forms, d, &name, &class) {
            widget_input::MouseAct::Click(_) if double && class == "ListRow" => {
                dialogs::click(self, d, "OKBtn")
            }
            widget_input::MouseAct::Click(n) => dialogs::click(self, d, &n),
            widget_input::MouseAct::Done => Outcome::Keep,
            widget_input::MouseAct::Default => {
                d.focus = name.clone();
                if class == "TEdit" {
                    Outcome::Keep
                } else {
                    dialogs::click(self, d, &name)
                }
            }
        }
    }

    fn main_event(&mut self, ev: Ev) {
        let tops = menu::model(self);
        let bar = bar_items_at(&self.skin, menu::FRAME, &tops);
        let bar_y = menu::FRAME.1 + 20..menu::FRAME.1 + 38;
        match ev {
            Ev::Move(x, y) => {
                if let Some((open, hl)) = self.menu {
                    if let Some(i) = bar.iter().position(|&(l, r)| x >= l && x < r) {
                        if bar_y.contains(&y) && i != open && tops[i].enabled {
                            self.menu = Some((i, Some(0)));
                            return;
                        }
                    }
                    let at = entry_at_in(&self.skin, menu::FRAME, &tops, open, x, y);
                    self.menu = Some((open, at.or(hl)));
                }
            }
            Ev::Down(x, y, false) | Ev::DblClick(x, y) => {
                if let Some((open, _)) = self.menu {
                    if let Some(i) = entry_at_in(&self.skin, menu::FRAME, &tops, open, x, y) {
                        let e = &tops[open].entries[i];
                        if e.enabled {
                            let name = e.name.clone();
                            self.menu = None;
                            self.command(&name);
                        }
                        return;
                    }
                    self.menu = None;
                    if !bar_y.contains(&y) {
                        return;
                    }
                }
                if bar_y.contains(&y) {
                    if let Some(i) = bar.iter().position(|&(l, r)| x >= l && x < r) {
                        if tops[i].enabled && self.menu.map(|m| m.0) != Some(i) {
                            self.menu = Some((i, Some(0)));
                        }
                    }
                    return;
                }
                let hit = self
                    .hits
                    .iter()
                    .rev()
                    .find(|h| h.contains(x, y) && h.enabled)
                    .map(|h| h.name.clone());
                self.main_click(hit.as_deref(), matches!(ev, Ev::DblClick(..)), (x, y));
            }
            Ev::Down(..) => {}
            Ev::Key(k) => self.main_key(k, &tops),
        }
    }

    /// A click on the main window: the labels open their dialogs (`OnClick`), the list selects (`PlayerWasSelected`) and edits on a
    /// double click, the bare window shows *Acerca* (`TEditor_Main.OnClick = About`).
    fn main_click(&mut self, hit: Option<&str>, double: bool, at: (i32, i32)) {
        let Some(name) = hit else {
            if self.inside_client(at) {
                self.command("Acerca1");
            }
            return;
        };
        if let Some(i) = name
            .strip_prefix("PlayerList#")
            .and_then(|i| i.parse::<usize>().ok())
        {
            if self.open.as_ref().is_some_and(|o| i < o.team.players.len()) {
                self.selected = Some(i);
                if double {
                    self.command("JogadorEditar");
                }
            }
            return;
        }
        if self.open.is_none() {
            return;
        }
        match name {
            "TeamShortName" | "TeamLongName" => self.command("Nome1"),
            "flagDraw" => self.command("EquipaPais"),
            "teamInitLevel" => self.command("EquipaNivelInicial"),
            "teamCoachName" => self.command("EquipaTreinador"),
            "PlayerList" => {}
            _ => {}
        }
    }

    fn inside_client(&self, (x, y): (i32, i32)) -> bool {
        let (fx, fy) = menu::FRAME;
        // (the client area starts under the caption and the menu bar)
        x > fx + 3 && x < fx + 4 + super::CLIENT.0 && y > fy + 38 && y < fy + 39 + super::CLIENT.1
    }

    fn main_key(&mut self, k: Key, tops: &[crate::classic::menu::Top]) {
        if let Some((open, hl)) = self.menu {
            let n = tops[open].entries.len();
            let step = |hl: Option<usize>, up: bool| {
                let mut i = hl.unwrap_or(if up { 0 } else { n.saturating_sub(1) });
                for _ in 0..n {
                    i = if up { (i + n - 1) % n } else { (i + 1) % n };
                    if !tops[open].entries[i].separator {
                        break;
                    }
                }
                Some(i)
            };
            let side = |d: usize| {
                let mut i = open;
                for _ in 0..tops.len() {
                    i = (i + d) % tops.len();
                    if tops[i].enabled {
                        break;
                    }
                }
                Some((i, Some(0)))
            };
            match k {
                Key::Escape => self.menu = None,
                Key::Down => self.menu = Some((open, step(hl, false))),
                Key::Up => self.menu = Some((open, step(hl, true))),
                Key::Left => self.menu = side(tops.len() - 1),
                Key::Right => self.menu = side(1),
                Key::Enter => {
                    if let Some(i) = hl {
                        let e = &tops[open].entries[i];
                        if e.enabled {
                            let name = e.name.clone();
                            self.menu = None;
                            self.command(&name);
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        // the shortcuts of the menus (`ShortCutText`), when their item is enabled
        let shortcut = match k {
            Key::Char('\u{e}') => "Ctrl+N",
            Key::F(3) => "F3",
            Key::F(1) => "F1",
            Key::Insert => "Ins",
            Key::Delete => "Del",
            Key::Enter => "Enter",
            Key::Up | Key::Down => {
                self.move_selection(k == Key::Up);
                return;
            }
            _ => return,
        };
        let entry = tops
            .iter()
            .filter(|t| t.enabled)
            .flat_map(|t| t.entries.iter())
            .find(|e| e.shortcut == shortcut && e.enabled)
            .map(|e| e.name.clone());
        if let Some(name) = entry {
            self.command(&name);
        }
    }

    /// The arrow keys move the list's selection.
    fn move_selection(&mut self, up: bool) {
        let n = self.open.as_ref().map_or(0, |o| o.team.players.len());
        if n == 0 {
            return;
        }
        self.selected = Some(match (self.selected, up) {
            (None, _) => 0,
            (Some(i), true) => i.saturating_sub(1),
            (Some(i), false) => (i + 1).min(n - 1),
        });
    }
}

/// Selects the next row of `list` (after the selected one, wrapping) whose text starts with `c`, ignoring case, and scrolls to it.
fn type_ahead(d: &mut Dialog, list: &str, c: char) {
    let rows = d.lists.get(list).cloned().unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    let up = |s: &str| {
        s.chars()
            .next()
            .map(|f| f.to_uppercase().collect::<String>())
    };
    let want = Some(c.to_uppercase().collect::<String>());
    let cur = d
        .texts
        .get(&format!("{list}.sel"))
        .and_then(|v| v.parse::<usize>().ok());
    let start = cur.map_or(0, |i| i + 1);
    let n = rows.len();
    if let Some(i) = (0..n)
        .map(|k| (start + k) % n)
        .find(|&i| up(&rows[i]) == want)
    {
        d.text(format!("{list}.sel"), i.to_string());
        // (the list scrolls so that the row is the last one shown, as the original's did; rows of 13 pixels)
        let fit = d
            .hits
            .iter()
            .find(|h| h.name == list && h.class == "TListBox")
            .map_or(1, |h| ((h.h - 2) / 13).max(1) as usize);
        let top = d
            .texts
            .get(&format!("{list}.top"))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        if i < top || i >= top + fit {
            d.text(
                format!("{list}.top"),
                (i + 1).saturating_sub(fit).to_string(),
            );
        }
    }
}
