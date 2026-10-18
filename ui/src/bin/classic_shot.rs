//! Renders the classic interface from a saved game into a BMP: `classic_shot game.e98 out.bmp [key=value ...]`
//! (`human=NAME`, `page=0..4`, `select=N`, `plan=0..11`); `-` instead of the game starts with no game (About box first). Needs the skin pack in `analysis/out/skin`.
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
            .filter_map(|p| elifoot_core::eft::parse(&std::fs::read(p).ok()?).ok())
            .collect();
        Classic::start(skin, forms, data, teams, 1)
    } else {
        let save = elifoot_core::e98::parse(&std::fs::read(&game).map_err(|e| e.to_string())?)?;
        let world = elifoot_core::world::World::from_e98(1, &save)?;
        let mut app = Classic::new(world, skin, forms, data);
        app.coach = "joga1".into();
        app
    };
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
            "noplan" => app.world.set_plan(Default::default()),
            _ => return Err(format!("unknown option {k}")),
        }
    }
    app.draw();
    std::fs::write(&out, app.fb.bmp()).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}
