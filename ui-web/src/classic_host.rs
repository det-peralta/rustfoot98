//! C ABI of the pixel-exact classic interface for the browser: the page loads the skin pack and the game's data files with
//! `cl_put(kind, len)`, starts with `cl_start(seed)`, forwards input with `cl_event` and `cl_tick`, and blits the RGBA frame at
//! `cl_fb_ptr()` (800x600) whenever `cl_frame()` returns 1.
use elifoot_core::eft;
use elifoot_ui::classic::{Classic, Data, Ev, Key, Request};
use elifoot_ui::win31::{Forms, Skin};
use std::cell::RefCell;

#[derive(Default)]
struct Host {
    app: Option<Classic>,
    /// Input buffer the page writes files into.
    io: Vec<u8>,
    fonts: Vec<u8>,
    glyphs: Vec<u8>,
    forms: Vec<u8>,
    country: Vec<u8>,
    referee: Vec<u8>,
    teams: Vec<eft::EftTeam>,
    /// The host's list of saved games (see `Data::set_saves`).
    saves: String,
    rgba: Vec<u8>,
    out: Vec<u8>,
    error: String,
}

thread_local! {
    static HOST: RefCell<Host> = RefCell::new(Host::default());
}

/// Makes the input buffer `len` bytes long and returns its address, for the page to fill with a file.
#[no_mangle]
pub extern "C" fn cl_alloc(len: usize) -> *mut u8 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        h.io.clear();
        h.io.resize(len, 0);
        h.io.as_mut_ptr()
    })
}

/// Stores the input buffer: kind 0 fonts.bin, 1 glyphs.bin, 2 forms.json, 3 COUNTRY.TXE, 4 REFEREE.TXE, 5 a `.EFT` team (1 when valid),
/// 6 forget the teams, 7 the names of the saved games (one per line).
#[no_mangle]
pub extern "C" fn cl_put(kind: u32) -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let data = std::mem::take(&mut h.io);
        match kind {
            0 => h.fonts = data,
            1 => h.glyphs = data,
            2 => h.forms = data,
            3 => h.country = data,
            4 => h.referee = data,
            5 => match eft::parse(&data) {
                Ok(t) => {
                    h.teams.push(t);
                    return 1;
                }
                Err(_) => return 0,
            },
            6 => h.teams.clear(),
            7 => {
                h.saves = String::from_utf8_lossy(&data).to_string();
                let saves = h.saves.clone();
                if let Some(a) = h.app.as_mut() {
                    a.data.set_saves(&saves);
                }
            }
            _ => return 0,
        }
        1
    })
}

fn build(seed: u32, h: &mut Host) -> Result<Classic, String> {
    let skin = Skin::from_bytes(&h.fonts, &h.glyphs)?;
    let forms = Forms::from_json(&String::from_utf8_lossy(&h.forms))?;
    let mut data = Data::from_txe(&h.country, &h.referee);
    data.set_saves(&h.saves);
    Ok(Classic::start(skin, forms, data, h.teams.clone(), seed))
}

/// Starts the interface: the About box, then the choice of game, the names and the draw of the club (see `dialogs/newgame.rs`). The new
/// game is made of the teams added (or invented clubs when none were) with `seed`; 1 on success, else read the message with `cl_out`.
#[no_mangle]
pub extern "C" fn cl_start(seed: u32) -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        match build(seed.max(1), &mut h) {
            Ok(app) => {
                h.app = Some(app);
                1
            }
            Err(e) => {
                h.error = e;
                h.out = h.error.clone().into_bytes();
                0
            }
        }
    })
}

/// Hands the interface the text of the game it asked for with a load request (the text is in the input buffer); 1 on success. On a bad
/// text the interface shows its own message.
#[no_mangle]
pub extern "C" fn cl_load() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let text = String::from_utf8_lossy(&std::mem::take(&mut h.io)).to_string();
        match h.app.as_mut().map(|a| a.load_text(&text)) {
            Some(Ok(())) => 1,
            Some(Err(e)) => {
                h.out = e.into_bytes();
                0
            }
            None => 0,
        }
    })
}

/// The wall clock, for the description of saved games (`dd/mm/yy hh:mm`).
#[no_mangle]
pub extern "C" fn cl_clock(year: u32, month: u32, day: u32, hour: u32, minute: u32) {
    HOST.with(|h| {
        if let Some(a) = h.borrow_mut().app.as_mut() {
            a.clock = format!(
                "{day:02}/{month:02}/{:02} {hour:02}:{minute:02}",
                year % 100
            );
        }
    })
}

/// Serialises the game into the output buffer (`cl_out_ptr()` / `cl_out_len()`).
#[no_mangle]
pub extern "C" fn cl_save() {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let text = h.app.as_ref().map(|a| a.save_text()).unwrap_or_default();
        h.out = text.into_bytes();
    })
}

/// Input: kind 0 move, 1 down, 2 up (a, b = position), 3 key (a = code below), 4 typed character (a = code point).
/// Key codes: 0 Enter, 1 Escape, 2 Tab, 3 Backspace, 4 Up, 5 Down, 6 Left, 7 Right, 8 BackTab, 100 + n function key Fn.
#[no_mangle]
pub extern "C" fn cl_event(kind: u32, a: i32, b: i32) {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some(app) = h.app.as_mut() else { return };
        let ev = match kind {
            0 => Ev::Move(a, b),
            1 => Ev::Down(a, b),
            2 => Ev::Up(a, b),
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
                n if (101..=112).contains(&n) => Key::F((n - 100) as u8),
                _ => return,
            }),
            4 => match char::from_u32(a as u32) {
                Some(c) => Ev::Key(Key::Char(c)),
                None => return,
            },
            _ => return,
        };
        app.event(ev);
    })
}

/// Time passes (milliseconds since the last call).
#[no_mangle]
pub extern "C" fn cl_tick(ms: u32) {
    HOST.with(|h| {
        if let Some(app) = h.borrow_mut().app.as_mut() {
            app.tick(ms.min(250));
        }
    })
}

/// Draws a frame if something changed; 1 when the RGBA picture at `cl_fb_ptr()` is new.
#[no_mangle]
pub extern "C" fn cl_frame() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Host { app, rgba, .. } = &mut *h;
        let Some(app) = app.as_mut() else { return 0 };
        if !app.frame() {
            return 0;
        }
        rgba.clear();
        rgba.reserve(app.fb.px.len() * 4);
        for p in &app.fb.px {
            rgba.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255]);
        }
        1
    })
}

#[no_mangle]
pub extern "C" fn cl_fb_ptr() -> *const u8 {
    HOST.with(|h| h.borrow().rgba.as_ptr())
}

/// The host's pending request: 0 none, 1 quit, 2 save (name in the output buffer), 3 load (name in the output buffer).
#[no_mangle]
pub extern "C" fn cl_request() -> i32 {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some(r) = h.app.as_mut().and_then(|a| {
            if a.requests.is_empty() {
                None
            } else {
                Some(a.requests.remove(0))
            }
        }) else {
            return 0;
        };
        match r {
            Request::Quit => 1,
            Request::Save(n) => {
                h.out = n.into_bytes();
                2
            }
            Request::Load(n) => {
                h.out = n.into_bytes();
                3
            }
        }
    })
}

#[no_mangle]
pub extern "C" fn cl_out_ptr() -> *const u8 {
    HOST.with(|h| h.borrow().out.as_ptr())
}

#[no_mangle]
pub extern "C" fn cl_out_len() -> usize {
    HOST.with(|h| h.borrow().out.len())
}
