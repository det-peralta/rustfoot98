# elifoot98-rea

Reverse-engineering notes and a clean-room native browser port of **Elifoot 98**
(André Elias, 1998, shareware), using [REA](https://github.com/morluto/rea) + Ghidra.

**Goal:** run the game natively in the browser (TypeScript, optionally Rust→WASM later)
instead of emulating Windows 3.1 inside DOSBox/js-dos as
[elifoot98.com.br](https://www.elifoot98.com.br) does today.

> Private research repo. Original game binaries are **not** stored here (see `.gitignore`).

## Docs
| File | Content |
|---|---|
| [docs/installation.md](docs/installation.md) | Tool setup (Ghidra, REA, Node) and getting the original files |
| [docs/architecture.md](docs/architecture.md) | How the current site works, and the target port design |
| [docs/analysis-plan.md](docs/analysis-plan.md) | Step-by-step reverse-engineering plan and status |
| [docs/findings.md](docs/findings.md) | Facts established about the binary and file formats |
| [docs/classes.md](docs/classes.md) | Delphi class table |
| [docs/match-engine.md](docs/match-engine.md) | Verified match-engine rules |
| [docs/ui-strategy.md](docs/ui-strategy.md) | UI architecture: shared immediate-mode screens, SDL2 / canvas back ends, SDL1->SDL2 notes |
| [docs/decompilation-map.md](docs/decompilation-map.md) | Every function of the binary: unit, kind, named / documented, work queue |
| [docs/season.md](docs/season.md) | Season structure: fixtures, rounds, standings, knock-out cup |
| [docs/finance.md](docs/finance.md) | Economy index, player value, loans; transfer market map |
| [docs/references.md](docs/references.md) | Third-party research (leads to verify) |

## Try it (browser demo)
```bash
rustup target add wasm32-unknown-unknown     # once
web/build.sh serve                            # builds the 143 KB WebAssembly core and serves http://localhost:8080
```
Plays full seasons of a 4-division league using the decoded rules (no emulator), with **you managing one club**
(formation, starters, bench and up to three planned substitutions per round): fictional clubs by default, or the
real teams if you pick the `.EFT` files of your own Elifoot 98 install (read locally in the browser, never uploaded or bundled). **Offline / install:** the page registers a service worker (`web/sw.js`) that caches the app and the WebAssembly core, and has a
web manifest, so browsers offer to install it as an app and it runs without a connection (needs https or localhost). While developing
open it with `?nosw` to skip the cache. CI (`.github/workflows/ci.yml`) runs fmt, clippy, the tests and the WASM build.

**Saving:** Guardar/Carregar use the browser's storage (optional autosave after every round); Exportar/Importar move a
text file (`ELIFOOT98-RS 1`, our own format: `core/src/save.rs`). A loaded game continues identically (same RNG seed).
**Importar .e98 original** reads a `JOGOS\*.e98` save of the original game in your browser (nothing is uploaded): squads, standings,
morale, year and round carry over; injuries and the cup restart; cash, loans and the economic factor are read as well (see `docs/findings.md`).
The computer-managed clubs trade players with each other (`core/src/market.rs`, shown in the "Transferências" card); you can bid in open auctions ("Mercado" card) and sell squad players for the best computer bid.
Native checks:
`cargo test`, `cargo run --release -p elifoot-core --example sim|season|world`, and the browser smoke test
`node tools/capture/web_test.mjs`.

## Front ends
The DOM page (`web/index.html`) is the most complete; the shared UI (`ui/`) runs on SDL2 (`cargo run -p elifoot-ui --features sdl --release`) and in the browser (`web/canvas.html`). See [docs/ui-strategy.md](docs/ui-strategy.md).

## Layout
- `core/` pure Rust: formats, RNG, match engine, season, world.  `wasm/` C-ABI bindings.  `web/` the demo page.
- `tools/` reverse-engineering helpers (Ghidra scripts, decoders).  `docs/` findings (see below).

## Status
See the checklist in [docs/analysis-plan.md](docs/analysis-plan.md).
