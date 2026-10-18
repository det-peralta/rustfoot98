//! Renders the classic interface from a saved game into a BMP: `classic_shot game.e98 out.bmp [key=value ...]`
//! (`human=NAME`, `page=0..4`, `select=N`, `plan=0..11`). Needs the skin pack in `analysis/out/skin`.
use elifoot_ui::classic::{Classic, Data, Ev, Key, Page};
use elifoot_ui::win31::{Forms, Skin};
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
    let save = elifoot_core::e98::parse(&std::fs::read(&game).map_err(|e| e.to_string())?)?;
    let world = elifoot_core::world::World::from_e98(1, &save)?;
    let mut app = Classic::new(world, skin, forms, data);
    app.coach = "joga1".into();
    for (k, v) in &kv {
        match k.as_str() {
            "human" => {
                let loc = app
                    .world
                    .divisions
                    .iter()
                    .enumerate()
                    .find_map(|(d, v2)| v2.iter().position(|c| c.name == *v).map(|i| (d, i)))
                    .ok_or("no such club")?;
                app.world.set_human(loc.0, loc.1);
            }
            "coach" => app.coach = v.clone(),
            "page" => {
                app.main.page = Some(Page::ALL[v.parse::<usize>().map_err(|e| e.to_string())?])
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
                for step in v.split(';') {
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
                            for _ in 0..ms.parse::<u32>().unwrap_or(0) / 20 {
                                if !app.dialogs.is_empty() {
                                    app.frame(); // the hits of the dialog
                                }
                                let now = app
                                    .dialogs
                                    .last()
                                    .map(|d| format!("{:?}", d.kind))
                                    .unwrap_or_else(|| match &app.screen {
                                        elifoot_ui::classic::Screen::Main => "main".into(),
                                        elifoot_ui::classic::Screen::Round(r) if r.backdrop => {
                                            "backdrop".into()
                                        }
                                        elifoot_ui::classic::Screen::Round(r) if r.cup => {
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
                                            && matches!(&app.screen, elifoot_ui::classic::Screen::Round(r) if r.cup && r.minute > 3)))
                                {
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
                                    && matches!(app.screen, elifoot_ui::classic::Screen::Main)
                                {
                                    break;
                                }
                            }
                        }
                        "idle" => {
                            // tick until the main window is back with no dialog open (at most `ms`)
                            for _ in 0..arg.parse::<u32>().unwrap_or(0) / 20 {
                                if app.dialogs.is_empty()
                                    && app.queue.is_empty()
                                    && matches!(app.screen, elifoot_ui::classic::Screen::Main)
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
            "open" => {
                // a dialog of the match day with sample data: `open=invite`, `open=lashes`
                let d = match v.as_str() {
                    "invite" => {
                        elifoot_ui::classic::dialogs::matchday::invitation(&app, "PALMEIRAS")
                    }
                    "lashes" => elifoot_ui::classic::dialogs::matchday::lashes_dialog(vec![
                        "Fulano foi despedido do SANTOS".into(),
                        "Troca com Beltrano (FLAMENGO)".into(),
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
