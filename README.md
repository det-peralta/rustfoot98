# rustfoot98

A native Rust/WebAssembly port of **Elifoot 98** (André Elias and Ricardo Correia, 1998, shareware, a Windows 3.1 / Delphi 1 program) that
runs in a browser with the **look and feel of the original**: the same 800x600 screens, fonts, dialogs and rules, without emulating Windows.
The rules were recovered from the program with [REA](https://github.com/morluto/rea) + Ghidra; the interface is drawn by a renderer that
reproduces Windows 3.1 and the Delphi VCL pixel for pixel from the data the original itself carries (forms, fonts, bitmaps).

> Private research repo. The original game files, fonts and bitmaps are **not** stored here (see `.gitignore`): you bring your own copy.

## Play it (classic interface)
```bash
# 1. once: what the renderer needs, built from YOUR copy of the game (original/elifoot98/ and original/run/, see docs/installation.md)
pip install freetype-py && python3 tools/ui/skin.py && python3 tools/ui/webpack.py
rustup target add wasm32-unknown-unknown
# 2. build the WebAssembly and serve it
web/build.sh serve            # http://localhost:8080  (opens the classic interface; works offline after a visit, ?nosw turns that off)
```
Open `http://localhost:8080/` : About box, new game (name the file, name the coaches, "Sortear equipas"), then the main window with its
menus, the squad list, the pages Jogo / Jogador / Finanças / Selecção / Adversário, the round view with the pie clock, half-time
substitutions, penalties, cup draw, auctions, end of season. If `web/skin/` is missing the page asks you to pick the folder (it stays in the
browser's IndexedDB; nothing is uploaded). Games are saved in the browser (autosave every 3 rounds, "Sair" saves); the original's `.e98`
files can be read by the native tools.

The team editor (`EDITEQ.EXE`) is at `http://localhost:8080/editor.html`: open, create and change `.EFT` teams with the original's
windows; the teams it saves stay in the browser and new games use them (*Importar* / *Exportar* move `.EFT` files in and out).

Up to six human coaches share the machine (hot seat), with the original's turn order. Not modelled: the registered-copy features (the
original hides loans, "Procurar jogadores" and the *Controle* page in an unregistered copy and so does the port; no key check is
implemented), hover hints (`ShowHint`). The computer clubs' bids use a divisor calibrated on the
original's auctions (2.2 instead of the binary's 1.6, see `docs/finance.md`). The original seeds its random numbers at start-up, so its games cannot be replayed move for
move; the money rules were checked on the original's own saves instead (`docs/finance.md`).

## How it is built
| Crate / dir | Content |
|---|---|
| `core/` | pure Rust rules: formats (`.e98`, `.eft`), RNG, match engine, season, finance, market, history |
| `ui/` | `win31/` renderer (frames, widgets, fonts, list dialogs) and `classic/` (the original's windows and dialogs as a state machine) |
| `ui-web/` | WebAssembly host of `ui/`; `web/classic.html` + `web/classic.js` drive the game, `web/editor.html` + `web/editor.js` the team editor (`EDITEQ.EXE`), whose saved teams the game's new games use |
| `wasm/`, `web/legacy.html`, `web/canvas.html` | the older, simplified front ends (DOM / shared canvas UI) |
| `tools/` | skin-pack builders, capture drivers (headless Chromium + the original under js-dos), Ghidra helpers |

Native helpers: `cargo run --release -p rustfoot-ui --bin classic_shot -- game.e98|- out.bmp key=value ...` renders any window or plays scripted
sessions (`docs/classic-dialogs.md`); `tools/ui/regions.py` lists where a render differs from a capture of the original;
`tools/capture/classic_season.mjs` plays whole seasons in the browser build. `editor_shot out.bmp team=FILE.EFT do='TeamColor;@x,y;!OKBtn'` renders
the team editor after a script; `tools/capture/editor_web.mjs` drives its browser build.

## Docs
| File | Content |
|---|---|
| [docs/installation.md](docs/installation.md) | Tool setup (Ghidra, REA, Node) and getting the original files |
| [docs/architecture.md](docs/architecture.md) | How the current site works, and the target port design |
| [docs/analysis-plan.md](docs/analysis-plan.md) | Step-by-step reverse-engineering plan and status |
| [docs/findings.md](docs/findings.md) | Facts established about the binary and file formats |
| [docs/classes.md](docs/classes.md) | Delphi class table |
| [docs/match-engine.md](docs/match-engine.md) | Verified match-engine rules |
| [docs/e98-format.md](docs/e98-format.md) | Layout of the original `.e98` save files |
| [docs/ui-strategy.md](docs/ui-strategy.md) | UI architecture: shared immediate-mode screens, SDL2 / canvas back ends, SDL1->SDL2 notes |
| [docs/decompilation-map.md](docs/decompilation-map.md) | Every function of the binary: unit, kind, named / documented, work queue |
| [docs/season.md](docs/season.md) | Season structure: fixtures, rounds, standings, knock-out cup |
| [docs/finance.md](docs/finance.md) | Economy index, player value, loans; transfer market map |
| [docs/references.md](docs/references.md) | Third-party research (leads to verify) |
| [docs/pixel-exact-ui.md](docs/pixel-exact-ui.md) | How the original's look is rebuilt; what was verified against captures |
| [docs/classic-dialogs.md](docs/classic-dialogs.md) | The classic interface: dialogs, the `Dialog` model, `classic_shot` |
| [docs/matchday.md](docs/matchday.md) | The match day: round view, half-time, penalties, cup draw, end of season |


## The older DOM demo
`web/legacy.html` (and `web/canvas.html`) are the first, simplified front ends that came before the classic interface: a card-style page
with a savegame exporter, the real `.EFT` team loader and the `.e98` importer. They are kept for reference.
Checks: `cargo test --workspace`, `cargo run --release -p rustfoot-core --example sim|season|world`, `node tools/capture/web_test.mjs`.
CI (`.github/workflows/ci.yml`) runs fmt, clippy, the tests and the WASM build.

## Status
See the checklist in [docs/analysis-plan.md](docs/analysis-plan.md); the pixel-exact work and what was measured against the original is in
[docs/pixel-exact-ui.md](docs/pixel-exact-ui.md).
