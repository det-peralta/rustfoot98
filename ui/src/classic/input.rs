//! Input: mouse and keyboard events and the passing of time, routed to the dialogs, the menus and the main window.
use super::dialog::{self, Dialog, KeyAct, Outcome};
use super::{dialogs, main_win, menu, round_view, Classic, Ev, Key, Page, Request, Screen};
use elifoot_core::season::FORMATIONS;

impl Classic {
    pub fn event(&mut self, ev: Ev) {
        self.dirty = true;
        if let Ev::Move(x, y) | Ev::Down(x, y) | Ev::Up(x, y) = ev {
            self.mouse = (x, y);
        }
        if !self.dialogs.is_empty() {
            self.dialog_event(ev);
        } else if matches!(self.screen, Screen::Main) {
            self.main_event(ev);
        }
        self.next_queued();
    }

    /// Time passes: the round clock runs and dialogs animate.
    pub fn tick(&mut self, ms: u32) {
        if let Some(mut d) = self.dialogs.pop() {
            let out = dialogs::tick(self, &mut d, ms);
            self.apply(d, out);
            self.dirty = true;
            self.next_queued();
            return;
        }
        let mut finished = false;
        let mut halftime = false;
        if let Screen::Round(rv) = &mut self.screen {
            let before = rv.minute;
            // the "Tempo de jogo" option: a match minute lasts `minute_ms` instead of the replay's own pace
            let ms =
                (ms as u64 * round_view::MINUTE_MS as u64 / self.opts.minute_ms() as u64) as u32;
            if !rv.tick(ms) {
                finished = true;
            }
            if before < 45
                && rv.minute >= 45
                && rv.human_plays()
                && !rv.halftime_seen
                && self.opts.half_time_subs
            {
                rv.halftime_seen = true;
                rv.minute = 45;
                halftime = true;
            }
            self.dirty = true;
        }
        if halftime {
            self.open(dialogs::matchday::halftime(self));
        }
        if finished {
            self.finish_round();
        }
    }

    /// Shows the round just played: the clock runs from 0 to 90.
    pub fn start_round_view(&mut self) {
        self.screen = Screen::Round(Box::new(round_view::RoundView::of_round(&self.world)));
        self.dirty = true;
    }

    /// The final whistle: back to the main window, then the messages of the round.
    pub fn finish_round(&mut self) {
        self.screen = Screen::Main;
        self.dirty = true;
        dialogs::matchday::end_of_round(self);
    }

    /// Plays the next round of the league (and cup) and shows it.
    pub fn play_round(&mut self) {
        self.main.menu = None;
        self.world.play_round();
        if self.opts.saves_after(self.world.round) {
            self.save_now();
        }
        self.start_round_view();
    }

    fn apply(&mut self, d: Dialog, out: Outcome) {
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
                .map(|h| (h.name.clone(), h.class.clone(), h.enabled))
            {
                Some((name, class, true)) => {
                    if class != "ListRow" && class != "TabSetTab" {
                        d.focus = name.clone();
                    }
                    if class == "TEdit" {
                        Outcome::Keep
                    } else {
                        dialogs::click(self, &mut d, &name)
                    }
                }
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
            return;
        }
        if dialogs::market::main_click(self, name) {
            return;
        }
        match name {
            "OKBtn" => self.play_round(),
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
            Key::Enter if self.play_enabled() => self.play_round(),
            _ => {}
        }
    }

    /// Whether the line-up is complete (the Jogar button is enabled).
    pub fn play_enabled(&self) -> bool {
        main_win::human_club(self).is_some_and(|(_, club)| {
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
                return;
            }
        }
        match name {
            "Melhores1" | "Automtico1" => {
                if let Some((d, i)) = self.world.human_location() {
                    if let Some(k) =
                        elifoot_core::season::best_formation(&self.world.divisions[d][i].slots)
                    {
                        self.world.set_formation(k);
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
