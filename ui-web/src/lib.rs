//! C ABI of the shared UI for the browser: `ui_new(seed)`, `ui_event(kind, a, b)`, `ui_frame()` and then the JSON command list at
//! `frame_ptr()` / `frame_len()` (valid until the next frame). The commands are the [`rustfoot_ui::canvas::Cmd`] list of one frame.
mod classic_host;
mod editor_host;
use rustfoot_ui::canvas::{Canvas, Cmd, DrawList};
use rustfoot_ui::{App, Event, Key};
use std::cell::RefCell;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static IO: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn rgb(c: rustfoot_ui::canvas::Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

#[no_mangle]
pub extern "C" fn ui_new(seed: u32) {
    APP.with(|a| *a.borrow_mut() = Some(App::new(seed.max(1))));
}

/// kind: 0 move, 1 down, 2 up, 3 wheel (a = delta), 4 key (a = 0 space, 1 s, 2 tab, 3..6 digits 1..4, 7 up, 8 down, 9 backspace,
/// 10 enter, 11 escape), 5 typed character (a = code point)
#[no_mangle]
pub extern "C" fn ui_event(kind: u32, a: i32, b: i32) {
    APP.with(|app| {
        let mut app = app.borrow_mut();
        let Some(app) = app.as_mut() else { return };
        let e = match kind {
            0 => Event::MouseMove(a, b),
            1 => Event::MouseDown(a, b),
            2 => Event::MouseUp(a, b),
            3 => Event::Wheel(a),
            5 => match char::from_u32(a as u32) {
                Some(ch) => Event::Text(ch),
                None => return,
            },
            _ => Event::Key(match a {
                0 => Key::Space,
                1 => Key::S,
                2 => Key::Tab,
                3..=6 => Key::Num((a - 2) as u8),
                7 => Key::Up,
                9 => Key::Backspace,
                10 => Key::Enter,
                11 => Key::Escape,
                _ => Key::Down,
            }),
        };
        app.event(e);
    });
}

/// 1 while a text field has the focus: the page then sends typed characters (kind 5) instead of shortcut keys.
#[no_mangle]
pub extern "C" fn ui_wants_text() -> i32 {
    APP.with(|app| app.borrow().as_ref().is_some_and(|a| a.focus.is_some()) as i32)
}

#[no_mangle]
pub extern "C" fn ui_frame() {
    let json = APP.with(|app| {
        let mut app = app.borrow_mut();
        let Some(app) = app.as_mut() else {
            return "[]".to_string();
        };
        let mut dl = DrawList::default();
        app.draw(&mut dl);
        let items: Vec<String> = dl
            .cmds
            .iter()
            .map(|c| match c {
                Cmd::Fill(r, col) => {
                    format!("[0,{},{},{},{},\"{}\"]", r.x, r.y, r.w, r.h, rgb(*col))
                }
                Cmd::Outline(r, col) => {
                    format!("[1,{},{},{},{},\"{}\"]", r.x, r.y, r.w, r.h, rgb(*col))
                }
                Cmd::Text(x, y, s, col, size) => {
                    format!("[2,{},{},{},\"{}\",{}]", x, y, esc(s), rgb(*col), size)
                }
            })
            .collect();
        format!("[{}]", items.join(","))
    });
    OUT.with(|o| *o.borrow_mut() = json);
}

#[no_mangle]
pub extern "C" fn frame_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn frame_len() -> usize {
    OUT.with(|o| o.borrow().len())
}

/// Text metrics used by the screens when no real font is available (kept in step with [`DrawList`]).
#[no_mangle]
pub extern "C" fn text_width(len: usize, size: i32) -> i32 {
    DrawList::default().text_width(&"x".repeat(len), size)
}

/// Host request taken after an event: 0 none, `1 + n` save slot `n`, `11 + n` load it, `21 + n` delete it, 30 import an `.e98` file.
#[no_mangle]
pub extern "C" fn ui_take_request() -> i32 {
    APP.with(
        |app| match app.borrow_mut().as_mut().and_then(App::take_request) {
            None => 0,
            Some(rustfoot_ui::Request::Save(n)) => 1 + n as i32,
            Some(rustfoot_ui::Request::Load(n)) => 11 + n as i32,
            Some(rustfoot_ui::Request::Delete(n)) => 21 + n as i32,
            Some(rustfoot_ui::Request::Import) => 30,
        },
    )
}

/// Serialises the game into the I/O buffer (`io_ptr()` / `io_len()`).
#[no_mangle]
pub extern "C" fn ui_save() {
    let text = APP.with(|app| {
        app.borrow()
            .as_ref()
            .map(App::save_text)
            .unwrap_or_default()
    });
    IO.with(|b| *b.borrow_mut() = text.into_bytes());
}

/// Makes the I/O buffer `len` bytes long and returns its address, for the page to fill with a file.
#[no_mangle]
pub extern "C" fn ui_alloc(len: usize) -> *mut u8 {
    IO.with(|b| {
        let mut b = b.borrow_mut();
        b.clear();
        b.resize(len, 0);
        b.as_mut_ptr()
    })
}

#[no_mangle]
pub extern "C" fn io_ptr() -> *const u8 {
    IO.with(|b| b.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn io_len() -> usize {
    IO.with(|b| b.borrow().len())
}

/// Loads a game saved by `ui_save` from the I/O buffer; 1 on success.
#[no_mangle]
pub extern "C" fn ui_load() -> i32 {
    let text = IO.with(|b| String::from_utf8_lossy(&b.borrow()).to_string());
    APP.with(|app| {
        app.borrow_mut()
            .as_mut()
            .is_some_and(|a| a.load_text(&text).is_ok()) as i32
    })
}

/// Imports an original `.e98` file from the I/O buffer; 1 on success.
#[no_mangle]
pub extern "C" fn ui_import_e98(seed: u32) -> i32 {
    let bytes = IO.with(|b| b.borrow().clone());
    APP.with(|app| {
        app.borrow_mut()
            .as_mut()
            .is_some_and(|a| a.import_e98(&bytes, seed.max(1)).is_ok()) as i32
    })
}

/// Lets the page report the outcome of a request: 1 saved, 2 nothing saved yet, 3 the browser refused to store.
#[no_mangle]
pub extern "C" fn ui_notice(code: i32) {
    APP.with(|app| {
        if let Some(a) = app.borrow_mut().as_mut() {
            a.message = match code {
                1 => "Jogo gravado no navegador.",
                2 => "Ainda não há nenhum jogo gravado.",
                _ => "O navegador não permitiu gravar.",
            }
            .into();
        }
    });
}

/// The one-line label of the current game (read it from `io_ptr()` / `io_len()` after calling this).
#[no_mangle]
pub extern "C" fn ui_slot_label() {
    let l = APP.with(|app| {
        app.borrow()
            .as_ref()
            .map(App::slot_label)
            .unwrap_or_default()
    });
    IO.with(|b| *b.borrow_mut() = l.into_bytes());
}

/// Tells the screens what slot `n` holds: the label is read from the I/O buffer (`len` bytes); `len == 0` marks it empty.
#[no_mangle]
pub extern "C" fn ui_set_slot(n: u32, len: usize) {
    let label =
        IO.with(|b| String::from_utf8_lossy(&b.borrow()[..len.min(b.borrow().len())]).to_string());
    APP.with(|app| {
        if let Some(a) = app.borrow_mut().as_mut() {
            a.set_slot(n as usize, (len > 0).then_some(label));
        }
    });
}

/// Chooses the look (1 classic, 0 modern); `ui_classic()` reads it back so the page can remember it.
#[no_mangle]
pub extern "C" fn ui_set_classic(on: i32) {
    rustfoot_ui::set_classic(on != 0);
}

#[no_mangle]
pub extern "C" fn ui_classic() -> i32 {
    APP.with(|app| app.borrow().as_ref().is_some_and(App::classic) as i32)
}
