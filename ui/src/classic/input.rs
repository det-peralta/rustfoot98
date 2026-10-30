//! Input: mouse and keyboard events and the passing of time, routed to the dialogs, the menus and the main window.
use super::dialog::{self, Dialog, KeyAct, Outcome};
use super::{dialogs, main_win, menu, Classic, Ev, Key, Page, Request, Screen};
use rustfoot_core::season::FORMATIONS;

impl Classic {
    pub fn event(&mut self, ev: Ev) {
        self.dirty = true;
        if let Ev::Move(x, y) | Ev::Down(x, y) | Ev::Up(x, y) = ev {
            self.mouse = (x, y);
        }
        match (ev, self.dialogs.last()) {
            (Ev::Move(x, y), Some(d)) => {
                self.hover
                    .moved(&d.form, self.forms.get(&d.form), &d.hits, x, y);
            }
            (Ev::Move(..), None) => {
                self.hover.clear();
            }
            (Ev::Down(..) | Ev::Key(_), _) => {
                self.hover.cancel();
            }
            _ => {}
        }
        if !self.dialogs.is_empty() {
            self.dialog_event(ev);
        } else if matches!(self.screen, Screen::Main) {
            self.main_event(ev);
        } else if let (Screen::Round(_), Ev::Down(x, y)) = (&self.screen, ev) {
            self.round_click(x, y);
        }
        self.next_queued();
    }

    /// Time passes: the round clock runs and dialogs animate.
    pub fn tick(&mut self, ms: u32) {
        if self.hover.tick(ms) {
            self.dirty = true;
        }
        if let Some(mut d) = self.dialogs.pop() {
            let out = dialogs::tick(self, &mut d, ms);
            self.apply(d, out);
            self.dirty = true;
            self.next_queued();
            return;
        }
        if matches!(self.screen, Screen::Round(_)) {
            self.round_tick(ms);
            // dialogs queued by the end of a round (auctions...) show without waiting for an input event
            self.next_queued();
        }
    }

    pub(crate) fn apply(&mut self, d: Dialog, out: Outcome) {
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

    fn next_queued(&mut self) {
        if self.dialogs.is_empty() {
            if let Some(d) = self.queue.pop_front() {
                // a dialog of one human (his auction) shows on his main window
                if let Some(seat) = d.st.get("seat").and_then(|s| s.parse().ok()) {
                    self.switch_human(seat);
                }
                self.dialogs.push(d);
                self.dirty = true;
            }
        }
    }

    fn dialog_event(&mut self, ev: Ev) {
        let Some(mut d) = self.dialogs.pop() else {
            return;
        };
        let out = match ev {
            Ev::Move(..) | Ev::Up(..) => Outcome::Keep,
            Ev::Down(x, y) => match d
                .hit_at(x, y)
                // a label whose form gives it an `OnClick` handler reacts too (the referee of the squad dialog)
                .or_else(|| {
                    d.hits.iter().rev().find(|h| {
                        h.class == "TLabel"
                            && h.contains(x, y)
                            && self
                                .forms
                                .get(&d.form)
                                .and_then(|f| f.find(&h.name))
                                .is_some_and(|n| n.props.contains_key("OnClick"))
                    })
                })
                .map(|h| (h.name.clone(), h.class.clone(), h.enabled))
            {
                Some((name, class, true)) => {
                    match super::widget_input::mouse(&self.forms, &mut d, &name, &class) {
                        super::widget_input::MouseAct::Click(n) => dialogs::click(self, &mut d, &n),
                        super::widget_input::MouseAct::Done => Outcome::Keep,
                        super::widget_input::MouseAct::Default => {
                            if class != "ListRow" && class != "TabSetTab" {
                                d.focus = name.clone();
                            }
                            if class == "TEdit" && d.kind != dialog::Kind::SelectGame {
                                Outcome::Keep
                            } else {
                                dialogs::click(self, &mut d, &name)
                            }
                        }
                    }
                }
                // the standings close with a click anywhere on the form (`GoOn`)
                _ if d.kind == dialog::Kind::Standings => dialogs::click(self, &mut d, "@form"),
                _ => Outcome::Keep,
            },
            Ev::Key(k) => match dialog::generic_key(self, &mut d, k) {
                KeyAct::Press(name) => dialogs::click(self, &mut d, &name),
                KeyAct::Done => Outcome::Keep,
                KeyAct::None => dialogs::key(self, &mut d, k).unwrap_or(Outcome::Keep),
            },
        };
        self.apply(d, out);
    }

    fn menu_tops(&self) -> Vec<menu::Top> {
        match main_win::human_club(self) {
            Some((_, club)) => menu::model(self, &club),
            None => Vec::new(),
        }
    }

    fn main_event(&mut self, ev: Ev) {
        let tops = self.menu_tops();
        let bar = menu::bar_items(self, &tops);
        let bar_y = menu::FRAME.1 + 20..menu::FRAME.1 + 38;
        match ev {
            Ev::Move(x, y) => {
                if let Some((open, _)) = self.main.menu {
                    if let Some(i) = bar.iter().position(|&(l, r)| x >= l && x < r) {
                        if bar_y.contains(&y) && i != open {
                            self.main.menu = Some((i, Some(0)));
                            return;
                        }
                    }
                    let hl = menu::entry_at(self, &tops, open, x, y);
                    if hl.is_some() || self.main.menu.is_some_and(|(_, h)| h.is_some()) {
                        self.main.menu = Some((open, hl.or(self.main.menu.and_then(|(_, h)| h))));
                    }
                }
            }
            Ev::Down(x, y) => {
                if let Some((open, _)) = self.main.menu {
                    if let Some(i) = menu::entry_at(self, &tops, open, x, y) {
                        let e = &tops[open].entries[i];
                        if e.enabled {
                            let name = e.name.clone();
                            self.main.menu = None;
                            self.command(&name);
                        }
                        return;
                    }
                    self.main.menu = None;
                    if !bar_y.contains(&y) {
                        return;
                    }
                }
                if bar_y.contains(&y) {
                    if let Some(i) = bar.iter().position(|&(l, r)| x >= l && x < r) {
                        if self.main.menu.map(|m| m.0) != Some(i) {
                            self.main.menu = Some((i, Some(0)));
                        }
                    }
                    return;
                }
                let hit = self
                    .hits
                    .iter()
                    .rev()
                    .find(|h| h.contains(x, y) && h.enabled)
                    .map(|h| (h.name.clone(), h.class.clone()));
                if let Some((name, class)) = hit {
                    self.main_click(&name, &class);
                }
            }
            Ev::Up(..) => {}
            Ev::Key(k) => self.main_key(k, &tops),
        }
    }

    fn main_click(&mut self, name: &str, class: &str) {
        if let Some(i) = name.strip_prefix("TabSet#") {
            if let Some(&p) = i.parse::<usize>().ok().and_then(|i| Page::ALL.get(i)) {
                self.main.page = Some(p);
            }
            return;
        }
        if class == "ListRow" {
            if let Some(i) = name
                .strip_prefix("ListBoxOfPlayers#")
                .and_then(|i| i.parse().ok())
            {
                self.main.selected = Some(i);
            }
            // `ShowBalance` (`1018:3c45`): the list keeps the clicked row selected and opens the balance sheet
            if let Some(i) = name
                .strip_prefix("ListBoxBalance#")
                .and_then(|i| i.parse().ok())
            {
                self.main.balance_sel = Some(i);
                self.open(dialogs::misc::balance());
            }
            return;
        }
        if dialogs::market::main_click(self, name) {
            return;
        }
        match name {
            "OKBtn" => self.round_button(),
            _ => {
                if let Some(d) = dialogs::open_for_button(self, name) {
                    self.open(d);
                }
            }
        }
    }

    fn main_key(&mut self, k: Key, tops: &[menu::Top]) {
        if self.main.menu.is_none() && dialogs::market::main_key(self, k) {
            return;
        }
        if let Some((open, hl)) = self.main.menu {
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
            match k {
                Key::Escape => self.main.menu = None,
                Key::Down => self.main.menu = Some((open, step(hl, false))),
                Key::Up => self.main.menu = Some((open, step(hl, true))),
                Key::Left => self.main.menu = Some(((open + tops.len() - 1) % tops.len(), Some(0))),
                Key::Right => self.main.menu = Some(((open + 1) % tops.len(), Some(0))),
                Key::Enter => {
                    if let Some(i) = hl {
                        let e = &tops[open].entries[i];
                        if e.enabled {
                            let name = e.name.clone();
                            self.main.menu = None;
                            self.command(&name);
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        match k {
            Key::F(n) if (1..=12).contains(&n) => {
                // the formation menu in the order of its entries
                let labels: Vec<&menu::Entry> = tops
                    .iter()
                    .flat_map(|t| t.entries.iter())
                    .filter(|e| e.shortcut.starts_with('F'))
                    .collect();
                if let Some(e) = labels.get(n as usize - 1) {
                    if e.enabled {
                        let name = e.name.clone();
                        self.command(&name);
                    }
                }
            }
            Key::Char('a') | Key::Char('A') => self.command("Automtico1"),
            Key::Char('m') | Key::Char('M') => self.command("Melhores1"),
            Key::Up | Key::Down => {
                let n = self
                    .world
                    .human_location()
                    .map_or(0, |(d, i)| self.world.divisions[d][i].slots.len());
                if n > 0 {
                    let cur = self.main.selected;
                    self.main.selected = Some(match (cur, k) {
                        (None, _) => 0,
                        (Some(i), Key::Down) => (i + 1).min(n - 1),
                        (Some(i), _) => i.saturating_sub(1),
                    });
                }
            }
            Key::Enter if self.play_enabled() => self.round_button(),
            _ => {}
        }
    }

    /// Whether the line-up is complete (the Jogar button is enabled).
    pub fn play_enabled(&self) -> bool {
        self.main.lineup_shown
            && main_win::human_club(self).is_some_and(|(_, club)| {
                let s = &club.slots;
                s.iter().filter(|p| p.starter()).count() == 11
                    && s.iter().filter(|p| p.starter() && p.position == 0).count() == 1
            })
    }

    /// A menu command (the control name of the menu entry in the form).
    pub fn command(&mut self, name: &str) {
        self.dirty = true;
        if let Some(label) = self
            .forms
            .get("TPREPAREGAMEDLG")
            .and_then(|f| f.find(name))
            .and_then(|n| n.str("Caption"))
            .map(str::to_string)
        {
            if let Some(k) = menu::formation_of(&label) {
                self.world.set_formation(k);
                self.main.lineup_shown = true;
                return;
            }
        }
        match name {
            "Melhores1" | "Automtico1" => {
                if let Some((d, i)) = self.world.human_location() {
                    if let Some(k) =
                        rustfoot_core::season::best_formation(&self.world.divisions[d][i].slots)
                    {
                        self.world.set_formation(k);
                        self.main.lineup_shown = true;
                    }
                }
            }
            // "Sair" saves the game first, "Sair sem gravar" does not (`1018:3146`, `1018:3f07`)
            "jogoSair" => {
                self.save_now();
                self.requests.push(Request::Quit);
            }
            "Sairsemgravar1" => self.requests.push(Request::Quit),
            "Vender1" => dialogs::market::vender(self),
            _ => {
                if let Some(d) = dialogs::open_for_menu(self, name) {
                    self.open(d);
                }
            }
        }
        let _ = FORMATIONS;
    }
}
