# UI strategy: keeping one UI alive on SDL2, the web and the desktop

## What the original UI is
* A Delphi 1 / Win16 VCL program: **49 forms** (`.dfm` resources) with absolute pixel layouts for 640x480, ~900 application functions of which
  most are event handlers (`docs/decompilation-map.md`), and ~2250 functions of **library code** (VCL, Delphi RTL, VBX glue) that is
  replaced, not ported.
* Every game rule lives in `core/` (pure Rust, no dependencies, deterministic). The UI only reads state and sends commands
  (`play_round`, `human_plan`, `human_bid`, ...). That boundary is what makes several front ends cheap.
* The forms are *data*: `tools/dfm.py` exports each one to JSON (`analysis/out/forms/*.json`: class, position, size, caption, events). The layouts
  and captions come from the game's resources and are copyrighted, so they stay in `analysis/out/` (git-ignored); the repository ships tools,
  not the data.

## Requirements
1. One set of screens, written once, tested without a window.
2. Desktop (Linux/macOS/Windows), browser (PWA, offline) and, later, phones.
3. No dependency that goes stale: the toolkit must be small enough that replacing a back end is a day's work.
4. Fonts and strings that we may redistribute (the original bitmap fonts and texts are not).

## Options studied
| option | verdict |
|---|---|
| Keep porting the DOM UI only (`web/index.html`, `app.js`) | works today and is the most complete (cup, market, finances), but it cannot run natively and the DOM details leak into game logic calls |
| **SDL 1.2 program** run through `sdl12-compat` | the compat layer (installed here: `sdl12-compat 1.2.68`) lets old SDL1 code run on SDL2; useful to keep a *legacy* binary alive, not a reason to write new SDL1 |
| **SDL2 native** (crate `sdl2 0.38`, `sdl2_ttf`) | small, stable, hardware accelerated, same API on every desktop; on this machine `sdl2-compat 2.32.74` (SDL2 API over SDL3) is what runs it, so SDL2-API code keeps working when distributions move to SDL3 |
| SDL2 compiled to the web with Emscripten (`-sUSE_SDL=2`, Rust target `wasm32-unknown-emscripten`) | one binary for desktop and browser, but the Emscripten toolchain is heavy and the Rust target is second-class; the web would show a canvas only (no DOM, no text selection, poor accessibility) |
| egui / iced / Slint | productive but large dependencies and their own event models; harder to keep identical across web and native |
| Reuse the original executable through DOSBox/js-dos (what elifoot98.com.br does) | not a port; needs the original files and a Windows 3.1 image |

## Decision: a tiny immediate-mode UI with three back ends
```
core (rules)  <-  ui (screens as functions state -> draw commands, input events)  ->  Canvas trait
                                                                                       |-- SDL2 renderer (ui/src/sdl.rs)           desktop
                                                                                       |-- command list -> <canvas> (ui-web/)      browser
                                                                                       |-- DrawList (tests, golden checks)          CI
```
* `ui/` has **no windowing dependency**. `App` owns the `World`, takes `Event`s (mouse, wheel, keys) and draws a `Screen` through the `Canvas`
  trait (`fill`, `outline`, `text`, `text_width`). Buttons are immediate (`App::button` returns true on the frame of the click), so a screen is
  one function and there is no widget tree or callback to keep in sync.
* **SDL2 back end** (`ui/src/sdl.rs`, feature `sdl`): `Backend<T: RenderTarget>` works on a window *or* an offscreen surface; text comes from
  SDL2_ttf with a system font (`find_font`, override with `ELIFOOT_FONT`). `cargo run -p elifoot-ui --features sdl --release`.
* **Web back end** (`ui-web/` + `web/canvas.js`): the same screens compiled to `wasm32-unknown-unknown` (205 KB); each frame is a JSON command
  list that the page replays with Canvas2D. `web/canvas.html`.
* **Headless**: `SDL_VIDEODRIVER=dummy ELIFOOT_SHOT=out.bmp ELIFOOT_KEYS=space,space,tab cargo run -p elifoot-ui --features sdl` renders one frame
  to a BMP for documentation and golden comparisons; the unit tests assert on `DrawList` (no window, no GPU).
The existing DOM page stays as the accessible, most complete front end until the shared screens catch up; nothing in `core/` or `wasm/` changes.

## SDL1.2 -> SDL2 conversion notes (for anyone bringing SDL1 code over)
1. Start with `sdl12-compat`: unmodified SDL 1.2 code runs on top of SDL2 (`SDL_SetVideoMode` becomes a window plus a texture).
2. Then port for real, in this order: `SDL_SetVideoMode` -> `SDL_CreateWindow` + `SDL_CreateRenderer`; `SDL_Surface` blits -> `SDL_Texture` copies
   (`SDL_CreateTextureFromSurface`); `SDL_Flip` -> `SDL_RenderPresent`; `SDL_PollEvent`: `event.key.keysym.sym` is now `SDL_Keycode`
   (`SDLK_*` names changed, `SDLK_KP1` -> `SDLK_KP_1`), text input moves to `SDL_TEXTINPUT`, mouse wheel is a real event
   (`SDL_MOUSEWHEEL`) instead of buttons 4/5; `SDL_SetColorKey` takes `SDL_TRUE`; audio callback signature and `SDL_OpenAudioDevice`;
   `SDL_ListModes` -> display modes API; no more `SDL_SWSURFACE/HWSURFACE` flags.
3. Keep the drawing behind a trait like `Canvas` (as here) so the next migration (SDL2 -> SDL3, `sdl2-compat` already sits on SDL3) touches one file.
4. Emscripten gives SDL2 on the web (`-sUSE_SDL=2` / `--use-port=sdl2`) without code changes; it is the escape hatch if a pixel-identical web build is
   ever required.

## Data-driven forms (planned)
* `tools/ui/` will turn our *own* layout descriptions (RON/JSON written by us, modelled on the 49 dialogs) into screen code or tables the
  immediate-mode screens read, so a dialog is data plus a function from state to commands. The DFM export stays a *reference* for behaviour and
  proportions, not a shipped asset.
* Strings: write our own Portuguese/English catalogue (the original texts are copyrighted); keep keys stable so translations are plain files.
* Layout is 800x600 logical pixels scaled by the back end (SDL `set_logical_size`, CSS `width:100%`).

## Roadmap
1. **Done**: `ui` (league, squad, finance with loan/stadium buttons, cup), SDL2 renderer, headless shots, canvas back end, tests.
2. **Done**: `ui/src/dialogs.rs`: `Táctica` (12 formations, click-to-swap line-up editor, up to 3 planned substitutions), `Mercado` (open auctions with one-click bids, selling with an optional asking price, player search with name/position/strength/price filters and direct purchase) and `Opções` (accept coach invitations, indebted clubs, coach changes). Not ported yet: balance-sheet detail, referee-aid gauges, salary-demand dialog (the computer renegotiates), coach selection.
3. **Done**: text-input widget (`App::text_field`; `Event::Text`, Backspace/Enter/Esc; the SDL back end maps `SDL_TEXTINPUT`, the web page sends typed characters while `ui_wants_text()` is 1) and wheel scrolling of two lists on the market screen.
4. Sound/animation hooks (match ticker) behind a `Host` trait so back ends can opt in.
5. **Decided: the DOM page is kept** as the accessible front end (a screen reader cannot read a canvas) and as the most complete one for now. It got the same 8 save slots, stored under the same localStorage keys and format as the canvas UI, so a game saved in one opens in the other. New features go to `core/` + the shared UI first; the DOM page follows when a feature matters for accessibility.

## Saving, loading and importing
The screens never touch files: `Opções` lists **8 save slots** (label, Gravar / Carregar / Apagar) and a "Importar .e98 original" button. A click sets
`App::request` (`Save(n)`, `Load(n)`, `Delete(n)`, `Import`); the host serves it (`App::take_request`), then calls `App::save_text` / `load_text` /
`import_e98` and reports what a slot holds with `App::set_slot(n, label)` (`App::slot_label` gives "Época 2026 · J2/14 · club").
* SDL2: `$ELIFOOT_SAVES` (default `~/.local/share/elifoot98-rea/`) holds `slotN.sav` + `slotN.label`; slots are scanned at start-up; drop a `.e98`
  or a `.sav` on the window to open it; `ELIFOOT_LOAD=file` opens one at start-up (also in headless shots).
* Web canvas: localStorage `elifoot98-rea.slot.N` = `{label, text}`; "Importar .e98" opens a file picker (`.e98` is parsed by `core/src/e98.rs`,
  anything else is read as our save text). ABI: `ui_take_request` (`1+n` save, `11+n` load, `21+n` delete, 30 import), `ui_alloc`/`io_ptr`/`io_len`,
  `ui_save`, `ui_slot_label`, `ui_set_slot`, `ui_load`, `ui_import_e98`, `ui_notice`.

## Commands
```bash
cargo test -p elifoot-ui                                   # screens, clicks and keys without a window
cargo run  -p elifoot-ui --features sdl --release          # desktop window
SDL_VIDEODRIVER=dummy ELIFOOT_SHOT=shot.bmp ELIFOOT_KEYS=space,tab cargo run -p elifoot-ui --features sdl --release
web/build.sh serve    # then open http://localhost:8080/canvas.html (shared UI) or / (DOM UI)
```
