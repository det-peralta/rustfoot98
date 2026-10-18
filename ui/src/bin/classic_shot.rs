//! Renders the classic interface from a saved game into a BMP: `classic_shot game.e98 out.bmp [key=value ...]`
//! (`human=NAME`, `page=0..4`, `select=N`, `plan=0..11`). Needs the skin pack in `analysis/out/skin`.
use elifoot_ui::classic::{Classic, Data, Page};
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
            "noplan" => app.world.set_plan(Default::default()),
            _ => return Err(format!("unknown option {k}")),
        }
    }
    app.draw();
    std::fs::write(&out, app.fb.bmp()).map_err(|e| e.to_string())?;
    println!("wrote {out}");
    Ok(())
}
