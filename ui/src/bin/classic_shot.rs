//! Renders the classic interface from a saved game into a BMP: `classic_shot game.e98 out.bmp [key=value ...]`
//! (`human=CLUB` (makes the human of that club the active one; any other club becomes the active human's), `seat=N` (the N-th human, from 0), `page=0..4`, `select=N`, `plan=0..11`, `dialog=MENUENTRY`, `play=ROUNDS`); `-` instead of the game starts with no game (About box first). Needs the skin pack in `analysis/out/skin`.
use rustfoot_ui::classic::{Classic, Data, Ev, Key, Page};
use rustfoot_ui::win31::{Forms, Skin};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let game = args
        .next()
        .ok_or("usage: classic_shot game.e98 out.bmp [key=value ...]")?;
    let out = args.next().ok_or("missing output file")?;
    let kv: Vec<(String, String)> = args
        .filter_map(|a| {
            a.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();
    let root = PathBuf::from(std::env::var("ELIFOOT_ROOT").unwrap_or_else(|_| ".".into()));
    let skin = Skin::load(&root.join("analysis/out/skin"))?;
    let forms = Forms::from_json(
        &std::fs::read_to_string(root.join("analysis/out/skin/forms.json"))
            .map_err(|e| e.to_string())?,
    )?;
    let data = Data::from_txe(
        &std::fs::read(root.join("original/elifoot98/COUNTRY.TXE")).map_err(|e| e.to_string())?,
        &std::fs::read(root.join("original/elifoot98/REFEREE.TXE")).map_err(|e| e.to_string())?,
    );
    let mut app = if game == "-" {
        // no game: the interface as it starts (About, choice of game...), with the teams of the original's EQUIPAS directory
        let mut files: Vec<_> = std::fs::read_dir(root.join("original/elifoot98/EQUIPAS"))
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("eft")))
            .collect();
        files.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
        let teams = files
            .iter()
            .filter_map(|p| rustfoot_core::eft::parse(&std::fs::read(p).ok()?).ok())
            .collect();
        Classic::start(skin, forms, data, teams, 1)
    } else {
        let save = rustfoot_core::e98::parse(&std::fs::read(&game).map_err(|e| e.to_string())?)?;
        let world = rustfoot_core::world::World::from_e98(1, &save)?;
        let mut app = Classic::new(world, skin, forms, data);
        app.coach = "joga1".into();
        app
    };
    for (k, v) in &kv {
        match k.as_str() {
            "seat" => app.switch_human(v.parse().map_err(|_| "bad seat")?),
            "human" => {
                if let Some(seat) = app.world.seat_of(v) {
                    app.switch_human(seat);
                    continue;
                }
                let loc = app
                    .world
                    .divisions
                    .iter()
                    .enumerate()
                    .find_map(|(d, v2)| v2.iter().position(|c| c.name == *v).map(|i| (d, i)))
                    .ok_or("no such club")?;
                app.world.set_human(loc.0, loc.1);
                app.sync_coach();
            }
            "coach" => app.coach = v.clone(),
            "page" => {
                app.main.page = Some(match v.as_str() {
                    "sell" => Page::Sell,
                    "contract" => Page::Contract,
                    "message" => Page::Message,
                    n => Page::ALL[n.parse::<usize>().map_err(|e| e.to_string())?],
                })
            }
            "select" => app.main.selected = v.parse().ok(),
            "plan" => {
                app.world.set_formation(v.parse().map_err(|_| "bad plan")?);
            }
            "menu" => app.main.menu = Some((v.parse().map_err(|_| "bad menu")?, Some(0))),
            "hl" => {
                if let Some((m, _)) = app.main.menu {
                    app.main.menu = Some((m, v.parse().ok()));
                }
            }
            "ev" => {
                // a script of events: `click:x,y`, `move:x,y`, `key:F5|Enter|a`, `tick:ms`, separated by `;`
                app.draw();
                // `season:N[:Kind]` stands for N rounds of (F5, behave like a user until the main window is back), the whole script
                // stopping when a dialog of that kind opens
                let steps: Vec<String> = v
                    .split(';')
                    .flat_map(|s| match s.strip_prefix("season:") {
                        Some(n) => {
                            let (count, stop) = n.split_once(':').unwrap_or((n, ""));
                            let auto = if stop.is_empty() {
                                "auto:3000000".to_string()
                            } else {
                                format!("auto:{stop}@3000000")
                            };
                            // one league round = a match day (every human picks a formation and presses Enter in turn; the
                            // last Enter starts the day) and, when a cup round was drawn, the cup day with the humans who play a tie
                            (0..count.parse::<usize>().unwrap_or(0))
                                .flat_map(|_| {
                                    ["turn:", auto.as_str(), "cupturn:", auto.as_str()]
                                        .map(|st| format!("\u{1}{st}")) // generated: skipped once the stop was reached
                                        .to_vec()
                                })
                                .collect()
                        }
                        None => vec![s.to_string()],
                    })
                    .collect();
                let mut halted = false;
                for step in steps.iter().map(|s| s.as_str()) {
                    let generated = step.starts_with('\u{1}');
                    if generated && halted {
                        continue;
                    }
                    let step = step.trim_start_matches('\u{1}');
                    let (kind, arg) = step.split_once(':').unwrap_or((step, ""));
                    let xy = || {
                        let (a, b) = arg.split_once(',').unwrap_or(("0", "0"));
                        (a.parse().unwrap_or(0), b.parse().unwrap_or(0))
                    };
                    match kind {
                        "click" => {
                            let (x, y) = xy();
                            app.event(Ev::Move(x, y));
                            app.event(Ev::Down(x, y));
                            app.event(Ev::Up(x, y));
                        }
                        "move" => {
                            let (x, y) = xy();
                            app.event(Ev::Move(x, y));
                        }
                        "tick" => {
                            for _ in 0..arg.parse::<u32>().unwrap_or(0) / 20 {
                                app.tick(20);
                            }
                        }
                        "wait" => {
                            // tick until a dialog opens (at most `ms`)
                            for _ in 0..arg.parse::<u32>().unwrap_or(0) / 20 {
                                if !app.dialogs.is_empty() {
                                    break;
                                }
                                app.tick(20);
                            }
                            if let Some(d) = app.dialogs.last() {
                                eprintln!("dialog {:?} ({})", d.kind, d.form);
                            }
                        }
                        "turn" | "cupturn" => {
                            // F5 (a formation) and Enter for every human who takes a turn on the main window now; `cupturn` only
                            // when the main window is waiting for a cup day
                            let main = matches!(app.screen, rustfoot_ui::classic::Screen::Main)
                                && app.dialogs.is_empty();
                            let cup = app.world.cup_day_due();
                            if main && (kind == "turn" || cup) {
                                let n = if cup {
                                    app.world.humans_in_cup_tie().len()
                                } else {
                                    app.world.human_count()
                                };
                                for _ in 0..n.max(1) {
                                    app.event(Ev::Key(Key::F(5)));
                                    app.event(Ev::Key(Key::Enter));
                                }
                            }
                        }
                        "seat" => app.switch_human(arg.parse().unwrap_or(0)),
                        "skip" => {
                            // play whole rounds without the interface
                            for _ in 0..arg.parse::<u32>().unwrap_or(0) {
                                app.world.play_round();
                            }
                            eprintln!(
                                "round {} cup {:?} left {}",
                                app.world.round,
                                app.world
                                    .last_cup
                                    .as_ref()
                                    .map(|c| (c.name.clone(), c.matches.len())),
                                app.world.cup.len()
                            );
                        }
                        "auto" => {
                            // behave like a user for `ms`: answer every dialog (pick the first player, press the default button);
                            // `auto:Kind@ms` stops when a dialog of that kind opens (and a view of the cup with `Cup@ms`)
                            let (stop, ms) = match arg.split_once('@') {
                                Some((k, m)) => (k, m),
                                None => ("", arg),
                            };
                            let mut seen = String::new();
                            let mut hit = false;
                            for _ in 0..ms.parse::<u32>().unwrap_or(0) / 20 {
                                if !app.dialogs.is_empty() {
                                    app.frame(); // the hits of the dialog
                                }
                                let now = app
                                    .dialogs
                                    .last()
                                    .map(|d| format!("{:?}", d.kind))
                                    .unwrap_or_else(|| match &app.screen {
                                        rustfoot_ui::classic::Screen::Main => "main".into(),
                                        rustfoot_ui::classic::Screen::Round(r) if r.backdrop => {
                                            "backdrop".into()
                                        }
                                        rustfoot_ui::classic::Screen::Round(r) if r.cup => {
                                            "cup view".into()
                                        }
                                        _ => "round view".into(),
                                    });
                                if now != seen {
                                    eprintln!("auto: now {now}");
                                    seen = now;
                                }
                                if !stop.is_empty()
                                    && (app
                                        .dialogs
                                        .last()
                                        .is_some_and(|d| format!("{:?}", d.kind) == stop)
                                        || (stop == "Attack"
                                            && app.dialogs.last().is_some_and(|d| {
                                                format!("{:?}", d.kind) == "Penalty"
                                                    && d.hits.iter().any(|h| h.name == "shootBtn")
                                            }))
                                        || (stop == "Cup"
                                            && matches!(&app.screen, rustfoot_ui::classic::Screen::Round(r) if r.cup && r.minute > 3)))
                                {
                                    hit = true;
                                    break;
                                }
                                let act = app.dialogs.last().map(|d| {
                                    let find = |n: &str| {
                                        d.hits
                                            .iter()
                                            .find(|h| h.name == n && h.enabled && h.w > 0)
                                            .map(|h| (h.x + h.w / 2, h.y + h.h / 2))
                                    };
                                    let enabled =
                                        |n: &str| d.hits.iter().any(|h| h.name == n && h.enabled);
                                    let pick = |list: &str, button: &str| {
                                        (
                                            if enabled(button) {
                                                find(button)
                                            } else {
                                                find(&format!("{list}#0"))
                                            },
                                            true,
                                        )
                                    };
                                    match format!("{:?}", d.kind).as_str() {
                                        "ForcedSubstitution" => {
                                            pick("ListBoxOfSubstitutePlayers", "OKBtn")
                                        }
                                        "Penalty"
                                            if d.hits.iter().any(|h| h.name == "shootBtn") =>
                                        {
                                            pick("ListBoxOfPlayers", "shootBtn")
                                        }
                                        "Penalty" => (None, true),
                                        // the classification closes by itself in the original (a timer or a click)
                                        "Standings" => (None, true),
                                        _ => (None, false),
                                    }
                                });
                                match act {
                                    Some((Some((x, y)), _)) => {
                                        if app.dialogs.last().is_some_and(|d| {
                                            d.hits.iter().any(|h| h.name == "shootBtn" && h.enabled)
                                        }) {
                                            eprintln!("auto: shoot");
                                        }
                                        app.event(Ev::Move(x, y));
                                        app.event(Ev::Down(x, y));
                                        app.event(Ev::Up(x, y));
                                    }
                                    Some((None, true)) => {
                                        if app
                                            .dialogs
                                            .last()
                                            .is_some_and(|d| format!("{:?}", d.kind) == "Standings")
                                        {
                                            app.dialogs.pop();
                                        }
                                    }
                                    Some((None, false)) => {
                                        eprintln!(
                                            "auto: {:?} {:?}",
                                            app.dialogs.last().map(|d| &d.kind),
                                            app.dialogs
                                                .last()
                                                .and_then(|d| d.texts.get("@caption"))
                                        );
                                        app.event(Ev::Key(Key::Enter));
                                    }
                                    None => {}
                                }
                                app.tick(20);
                                if app.dialogs.is_empty()
                                    && app.queue.is_empty()
                                    && matches!(app.screen, rustfoot_ui::classic::Screen::Main)
                                {
                                    break;
                                }
                            }
                            if hit {
                                app.frame();
                                halted = true;
                            }
                        }
                        "idle" => {
                            // tick until the main window is back with no dialog open (at most `ms`)
                            for _ in 0..arg.parse::<u32>().unwrap_or(0) / 20 {
                                if app.dialogs.is_empty()
                                    && app.queue.is_empty()
                                    && matches!(app.screen, rustfoot_ui::classic::Screen::Main)
                                {
                                    break;
                                }
                                app.tick(20);
                            }
                        }
                        "key" => {
                            let k = match arg {
                                "Enter" => Key::Enter,
                                "Escape" => Key::Escape,
                                "Tab" => Key::Tab,
                                "Up" => Key::Up,
                                "Down" => Key::Down,
                                "Left" => Key::Left,
                                "Right" => Key::Right,
                                "Backspace" => Key::Backspace,
                                f if f.starts_with('F') && f.len() > 1 => {
                                    Key::F(f[1..].parse().unwrap_or(1))
                                }
                                c => Key::Char(c.chars().next().unwrap_or(' ')),
                            };
                            app.event(Ev::Key(k));
                        }
                        _ => return Err(format!("bad step {step}")),
                    }
                    app.frame();
                }
            }
            "open" => rustfoot_ui::classic::dialogs::market::open_by_name(&mut app, v)?,
            "registered" => app.main.market.registered = v != "0",
            "dialog" => {
                // opens a dialog by the control name of its menu entry (`CampeonatoClassificao`, `menuCalendrio`...)
                if let Some(d) = rustfoot_ui::classic::dialogs::open_for_menu(&app, v)
                    .or_else(|| rustfoot_ui::classic::dialogs::open_for_button(&app, v))
                {
                    app.open(d);
                } else {
                    return Err(format!("no dialog {v}"));
                }
            }
            "play" => {
                // plays N rounds of the world without showing them (to have histories to look at)
                for _ in 0..v.parse::<usize>().map_err(|e| e.to_string())? {
                    app.world.play_round();
                }
            }
            "mdopen" => {
                // a dialog of the match day with sample data: `mdopen=invite`, `mdopen=lashes`
                let d = match v.as_str() {
                    "invite" => {
                        rustfoot_ui::classic::dialogs::matchday::invitation(&app, "", "PALMEIRAS")
                    }
                    "lashes" => rustfoot_ui::classic::dialogs::matchday::lashes_dialog(vec![
                        "Fulano foi despedido do SANTOS".into(),
                        "\u{2}Para o seu lugar foi contratado Beltrano".into(),
                        String::new(),
                    ]),
                    _ => return Err(format!("unknown dialog {v}")),
                };
                app.open(d);
            }
            "noplan" => app.world.set_plan(Default::default()),
            _ => return Err(format!("unknown option {k}")),
        }
    }
    app.draw();
    std::fs::write(&out, app.fb.bmp()).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}
