//! Renders the team editor into a BMP: `editor_shot out.bmp [key=value ...]` with `dir=EQUIPAS directory` (default the original's),
//! `team=FILE.EFT` (open it), `sel=N` (select a player), `menu=N` and `hl=N` (an open menu), then `do=` a `;`-separated script of
//! menu commands (`TeamColor`), clicks (`@x,y` or `@x,y,r` for the right button), presses of a dialog's control (`!OKBtn`) and keys
//! (`#Enter`, `#Tab`, `#a`); `save=path` writes the last team file the script saved. Needs the skin
//! pack in `analysis/out/skin`.
use rustfoot_ui::classic::Key;
use rustfoot_ui::editor::{Editor, Ev};
use rustfoot_ui::win31::{Forms, Skin};
use std::path::PathBuf;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let out = args
        .next()
        .ok_or("usage: editor_shot out.bmp [key=value ...]")?;
    let kv: Vec<(String, String)> = args
        .filter_map(|a| {
            a.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect();
    let get = |k: &str| kv.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
    let root = PathBuf::from(std::env::var("ELIFOOT_ROOT").unwrap_or_else(|_| ".".into()));
    let skin = Skin::load(&root.join("analysis/out/skin"))?;
    let forms = Forms::from_json(
        &std::fs::read_to_string(root.join("analysis/out/skin/forms.json"))
            .map_err(|e| e.to_string())?,
    )?;
    let countries = rustfoot_core::txe::parse(
        &std::fs::read(root.join("original/elifoot98/COUNTRY.TXE")).map_err(|e| e.to_string())?,
    )
    .iter()
    .skip(1)
    .filter_map(|l| {
        l.split_once(' ')
            .map(|(a, b)| (a.to_string(), b.to_string()))
    })
    .collect();
    let dir = get("dir").map_or_else(|| root.join("original/elifoot98/EQUIPAS"), PathBuf::from);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("eft")))
        .collect();
    files.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    let teams = files
        .iter()
        .filter_map(|p| {
            let t = rustfoot_core::eft::parse(&std::fs::read(p).ok()?).ok()?;
            Some((p.file_name()?.to_string_lossy().to_uppercase(), t))
        })
        .collect();
    let mut e = Editor::new(skin, forms, countries, teams);
    if let Some(t) = get("team") {
        e.open_file(&t);
    }
    if let Some(s) = get("sel").and_then(|v| v.parse().ok()) {
        e.selected = Some(s);
    }
    if let Some(m) = get("menu").and_then(|v| v.parse().ok()) {
        e.menu = Some((m, get("hl").and_then(|v| v.parse().ok())));
    }
    for step in get("do")
        .unwrap_or_default()
        .split(';')
        .filter(|s| !s.is_empty())
    {
        if let Some(at) = step.strip_prefix('@') {
            let v: Vec<&str> = at.split(',').collect();
            let (x, y) = (v[0].parse().unwrap_or(0), v[1].parse().unwrap_or(0));
            e.draw();
            e.event(Ev::Move(x, y));
            e.event(Ev::Down(x, y, v.get(2) == Some(&"r")));
        } else if let Some(name) = step.strip_prefix('!') {
            e.draw();
            e.press(name);
        } else if let Some(k) = step.strip_prefix('#') {
            let key = match k {
                "Enter" => Key::Enter,
                "Escape" => Key::Escape,
                "Tab" => Key::Tab,
                "Up" => Key::Up,
                "Down" => Key::Down,
                "Ins" => Key::Insert,
                "Del" => Key::Delete,
                _ => Key::Char(k.chars().next().unwrap_or(' ')),
            };
            e.draw();
            e.event(Ev::Key(key));
        } else {
            e.command(step);
        }
    }
    e.draw();
    // `save=path`: the last team file the script saved
    if let Some(path) = get("save") {
        let last = e.requests.iter().rev().find_map(|r| match r {
            rustfoot_ui::editor::Request::Save(_, bytes) => Some(bytes.clone()),
            _ => None,
        });
        if let Some(bytes) = last {
            std::fs::write(&path, bytes).map_err(|x| x.to_string())?;
        }
    }
    std::fs::write(&out, e.fb.bmp()).map_err(|x| x.to_string())?;
    println!("wrote {out}");
    Ok(())
}
