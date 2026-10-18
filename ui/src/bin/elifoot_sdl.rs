//! Desktop front end: `cargo run -p elifoot-ui --features sdl --release`.
//! Headless screenshot (CI, docs): `SDL_VIDEODRIVER=dummy ELIFOOT_SHOT=out.bmp [ELIFOOT_KEYS=space,space,tab] elifoot-sdl`.
use elifoot_ui::sdl::{find_font, map_event, Backend};
use elifoot_ui::{App, Event, Key, Request, HEIGHT, WIDTH};
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use std::path::PathBuf;

fn key_by_name(n: &str) -> Option<Key> {
    Some(match n {
        "space" => Key::Space,
        "s" => Key::S,
        "tab" => Key::Tab,
        "1" => Key::Num(1),
        "2" => Key::Num(2),
        "3" => Key::Num(3),
        "4" => Key::Num(4),
        _ => return None,
    })
}

/// The save slot: `$ELIFOOT_SAVES` or `~/.local/share/elifoot98-rea` (created on demand), file `elifoot.sav`.
fn save_path() -> PathBuf {
    let dir = std::env::var_os("ELIFOOT_SAVES")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/elifoot98-rea"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join("elifoot.sav")
}

/// Opens a file the user dropped on the window (or passed in `ELIFOOT_LOAD`): an original `.e98` save or one of ours.
fn open_file(app: &mut App, path: &std::path::Path, seed: u32) {
    let Ok(bytes) = std::fs::read(path) else {
        app.message = format!("Não foi possível ler {}.", path.display());
        return;
    };
    let is_e98 = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("e98"));
    if is_e98 {
        let _ = app.import_e98(&bytes, seed);
    } else {
        let _ = app.load_text(&String::from_utf8_lossy(&bytes));
    }
}

fn serve(app: &mut App, seed: u32) {
    match app.take_request() {
        Some(Request::Save) => {
            let path = save_path();
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            app.message = match std::fs::write(&path, app.save_text()) {
                Ok(()) => format!("Jogo gravado em {}.", path.display()),
                Err(e) => format!("Não foi possível gravar: {e}"),
            };
        }
        Some(Request::Load) => open_file(app, &save_path(), seed),
        Some(Request::Import) => {
            app.message = "Arrasta um ficheiro .e98 para a janela para o importar.".into();
        }
        None => {}
    }
}

fn main() -> Result<(), String> {
    let font = find_font().ok_or("no TrueType font found; set ELIFOOT_FONT")?;
    let ttf = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let seed = std::env::var("ELIFOOT_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1998);
    let mut app = App::new(seed);
    if let Some(p) = std::env::var_os("ELIFOOT_LOAD") {
        open_file(&mut app, std::path::Path::new(&p), seed);
    }

    if let Ok(path) = std::env::var("ELIFOOT_SHOT") {
        for name in std::env::var("ELIFOOT_KEYS").unwrap_or_default().split(',') {
            if let Some(k) = key_by_name(name.trim()) {
                app.event(Event::Key(k));
            }
        }
        let surface = Surface::new(WIDTH as u32, HEIGHT as u32, PixelFormatEnum::ARGB8888)?;
        let mut canvas = surface.into_canvas()?;
        let creator = canvas.texture_creator();
        let mut backend = Backend::new(&mut canvas, &creator, &ttf, font);
        app.draw(&mut backend);
        let surface = canvas.into_surface();
        surface.save_bmp(&path)?;
        println!("wrote {path}");
        return Ok(());
    }

    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let window = video
        .window("Elifoot 98 (port)", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .resizable()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;
    canvas
        .set_logical_size(WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;
    video.text_input().start();
    let creator = canvas.texture_creator();
    let mut events = sdl.event_pump()?;
    'main: loop {
        for e in events.poll_iter() {
            if matches!(e, sdl2::event::Event::Quit { .. }) {
                break 'main;
            }
            if let sdl2::event::Event::DropFile { filename, .. } = &e {
                open_file(&mut app, std::path::Path::new(filename), seed);
            }
            if let Some(ev) = map_event(&e) {
                if ev == Event::Key(Key::Escape) && app.focus.is_none() {
                    break 'main;
                }
                app.event(ev);
            }
        }
        let mut backend = Backend::new(&mut canvas, &creator, &ttf, font.clone());
        app.draw(&mut backend);
        serve(&mut app, seed);
        canvas.present();
    }
    Ok(())
}
