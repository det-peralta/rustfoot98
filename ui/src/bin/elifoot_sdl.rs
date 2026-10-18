//! Desktop front end: `cargo run -p elifoot-ui --features sdl --release`.
//! Headless screenshot (CI, docs): `SDL_VIDEODRIVER=dummy ELIFOOT_SHOT=out.bmp [ELIFOOT_KEYS=space,space,tab] elifoot-sdl`.
use elifoot_ui::sdl::{find_font, map_event, Backend};
use elifoot_ui::{App, Event, Key, HEIGHT, WIDTH};
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;

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

fn main() -> Result<(), String> {
    let font = find_font().ok_or("no TrueType font found; set ELIFOOT_FONT")?;
    let ttf = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let seed = std::env::var("ELIFOOT_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1998);
    let mut app = App::new(seed);

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
    let creator = canvas.texture_creator();
    let mut events = sdl.event_pump()?;
    'main: loop {
        for e in events.poll_iter() {
            if matches!(e, sdl2::event::Event::Quit { .. }) {
                break 'main;
            }
            if let Some(ev) = map_event(&e) {
                if ev == Event::Key(Key::Escape) {
                    break 'main;
                }
                app.event(ev);
            }
        }
        let mut backend = Backend::new(&mut canvas, &creator, &ttf, font.clone());
        app.draw(&mut backend);
        canvas.present();
    }
    Ok(())
}
