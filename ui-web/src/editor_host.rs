//! C ABI of the team editor (`ui/src/editor`) for the browser: the page loads the skin pack, `COUNTRY.TXE` and the team files with
//! `ed_put`, starts with `ed_start()`, forwards input with `ed_event`, blits the RGBA frame at `ed_fb_ptr()` (800x600) whenever
//! `ed_frame()` returns 1 and stores the files the editor saves (`ed_request`).
use rustfoot_core::eft;
use rustfoot_ui::classic::Key;
use rustfoot_ui::editor::{Editor, Ev, Request};
use rustfoot_ui::win31::{Forms, Skin};
use std::cell::RefCell;

#[derive(Default)]
struct Host {
    app: Option<Editor>,
    io: Vec<u8>,
    fonts: Vec<u8>,
    glyphs: Vec<u8>,
    forms: Vec<u8>,
    country: Vec<u8>,
    /// The name of the next team file (`ed_put(4)` before `ed_put(5)`).
    name: String,
    teams: Vec<(String, eft::EftTeam)>,
    rgba: Vec<u8>,
    /// Output: a text (file name, error) and a file's bytes.
    out: Vec<u8>,
    file: Vec<u8>,
}

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host::default());
}

/// Makes the input buffer `len` bytes long and returns its address, for the page to fill.
#[no_mangle]
pub extern "C" fn ed_alloc(len: usize) -> *mut u8 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        h.io.clear();
        h.io.resize(len, 0);
        h.io.as_mut_ptr()
    })
}

/// Stores the input buffer: kind 0 fonts.bin, 1 glyphs.bin, 2 forms.json, 3 COUNTRY.TXE, 4 the name of the next team file, 5 a `.EFT`
/// team under that name (replacing a team of the same name; 1 when valid), 6 forget the teams.
#[no_mangle]
pub extern "C" fn ed_put(kind: u32) -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let data = std::mem::take(&mut h.io);
        match kind {
            0 => h.fonts = data,
            1 => h.glyphs = data,
            2 => h.forms = data,
            3 => h.country = data,
            4 => h.name = String::from_utf8_lossy(&data).to_uppercase(),
            5 => {
                let Ok(t) = eft::parse(&data) else {
                    return 0;
                };
                let name = h.name.clone();
                h.teams.retain(|(f, _)| *f != name);
                h.teams.push((name, t));
            }
            6 => h.teams.clear(),
            _ => return 0,
        }
        1
    })
}

/// Starts the editor with an empty window; 1 on success, else read the message with `ed_out_ptr` / `ed_out_len`.
#[no_mangle]
pub extern "C" fn ed_start() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let built = Skin::from_bytes(&h.fonts, &h.glyphs).and_then(|skin| {
            let forms = Forms::from_json(&String::from_utf8_lossy(&h.forms))?;
            let countries = rustfoot_core::txe::parse(&h.country)
                .iter()
                .skip(1)
                .filter_map(|l| {
                    l.split_once(' ')
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                })
                .collect();
            let mut teams = h.teams.clone();
            teams.sort_by(|a, b| a.0.cmp(&b.0));
            Ok(Editor::new(skin, forms, countries, teams))
        });
        match built {
            Ok(app) => {
                h.app = Some(app);
                1
            }
            Err(e) => {
                h.out = e.into_bytes();
                0
            }
        }
    })
}

/// Input: kind 0 move, 1 left button down, 2 right button down, 3 key (a = code below), 4 typed character (a = code point), 5 double
/// click (a, b = position for the others). Key codes: 0 Enter, 1 Escape, 2 Tab, 3 Backspace, 4 Up, 5 Down, 6 Left, 7 Right, 8 BackTab,
/// 9 Insert, 10 Delete, 101..112 F1..F12; Ctrl+N comes as character 14.
#[no_mangle]
pub extern "C" fn ed_event(kind: u32, a: i32, b: i32) {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some(app) = h.app.as_mut() else { return };
        let ev = match kind {
            0 => Ev::Move(a, b),
            1 => Ev::Down(a, b, false),
            2 => Ev::Down(a, b, true),
            3 => Ev::Key(match a {
                0 => Key::Enter,
                1 => Key::Escape,
                2 => Key::Tab,
                3 => Key::Backspace,
                4 => Key::Up,
                5 => Key::Down,
                6 => Key::Left,
                7 => Key::Right,
                8 => Key::BackTab,
                9 => Key::Insert,
                10 => Key::Delete,
                n if (101..=112).contains(&n) => Key::F((n - 100) as u8),
                _ => return,
            }),
            4 => match char::from_u32(a as u32) {
                Some(c) => Ev::Key(Key::Char(c)),
                None => return,
            },
            5 => Ev::DblClick(a, b),
            _ => return,
        };
        app.event(ev);
    })
}

/// 1 while the control with the keyboard focus takes typed text (`Editor::wants_text`): the page opens a phone's keyboard then.
#[no_mangle]
pub extern "C" fn ed_wants_text() -> i32 {
    HOST.with(|h| i32::from(h.borrow().app.as_ref().is_some_and(Editor::wants_text)))
}

/// Time passes (milliseconds since the last call): the *Acerca* box's banner.
#[no_mangle]
pub extern "C" fn ed_tick(ms: u32) {
    HOST.with(|h| {
        if let Some(app) = h.borrow_mut().app.as_mut() {
            app.tick(ms.min(250));
        }
    })
}

/// Draws a frame if something changed; 1 when the RGBA picture at `ed_fb_ptr()` is new.
#[no_mangle]
pub extern "C" fn ed_frame() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Host { app, rgba, .. } = &mut *h;
        let Some(app) = app.as_mut() else { return 0 };
        if !app.dirty {
            return 0;
        }
        app.draw();
        rgba.clear();
        rgba.reserve(app.fb.px.len() * 4);
        for p in &app.fb.px {
            rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]);
        }
        1
    })
}

#[no_mangle]
pub extern "C" fn ed_fb_ptr() -> *const u8 {
    HOST.with(|h| h.borrow().rgba.as_ptr())
}

/// The host's pending request: 0 none, 1 quit, 2 store a team file (its name in the output text, its bytes in the output file).
#[no_mangle]
pub extern "C" fn ed_request() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some(r) = h
            .app
            .as_mut()
            .and_then(|a| (!a.requests.is_empty()).then(|| a.requests.remove(0)))
        else {
            return 0;
        };
        match r {
            Request::Quit => 1,
            Request::Save(name, bytes) => {
                h.out = name.into_bytes();
                h.file = bytes;
                2
            }
        }
    })
}

/// The team being edited, as it stands, into the output (name and bytes of its `.EFT`); 0 when no team is open.
#[no_mangle]
pub extern "C" fn ed_export() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some(o) = h.app.as_ref().and_then(|a| a.open.clone()) else {
            return 0;
        };
        let name = if o.file.is_empty() {
            format!(
                "{}.EFT",
                o.team
                    .short_name
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric())
                    .take(8)
                    .collect::<String>()
            )
        } else {
            o.file
        };
        h.out = name.into_bytes();
        h.file = eft::write(&o.team);
        1
    })
}

/// Adds or replaces a team file while the editor runs (an import): the name was given with `ed_put(4)`, the bytes are in the input
/// buffer; 1 when valid.
#[no_mangle]
pub extern "C" fn ed_import() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let data = std::mem::take(&mut h.io);
        let name = h.name.clone();
        let (Ok(t), Some(app)) = (eft::parse(&data), h.app.as_mut()) else {
            return 0;
        };
        app.files.retain(|(f, _)| *f != name);
        app.files.push((name, t));
        app.files.sort_by(|a, b| a.0.cmp(&b.0));
        1
    })
}

#[no_mangle]
pub extern "C" fn ed_out_ptr() -> *const u8 {
    HOST.with(|h| h.borrow().out.as_ptr())
}

#[no_mangle]
pub extern "C" fn ed_out_len() -> usize {
    HOST.with(|h| h.borrow().out.len())
}

#[no_mangle]
pub extern "C" fn ed_file_ptr() -> *const u8 {
    HOST.with(|h| h.borrow().file.as_ptr())
}

#[no_mangle]
pub extern "C" fn ed_file_len() -> usize {
    HOST.with(|h| h.borrow().file.len())
}
