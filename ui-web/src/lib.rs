//! C ABI of the shared UI for the browser: `ui_new(seed)`, `ui_event(kind, a, b)`, `ui_frame()` and then the JSON command list at
//! `frame_ptr()` / `frame_len()` (valid until the next frame). The commands are the [`elifoot_ui::canvas::Cmd`] list of one frame.
use elifoot_ui::canvas::{Canvas, Cmd, DrawList};
use elifoot_ui::{App, Event, Key};
use std::cell::RefCell;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
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

fn rgb(c: elifoot_ui::canvas::Rgb) -> String {
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
