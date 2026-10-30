# rustfoot98

[![ci](https://github.com/det-peralta/rustfoot98/actions/workflows/ci.yml/badge.svg)](https://github.com/det-peralta/rustfoot98/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

**Elifoot 98** (André Elias and Ricardo Correia, 1998), the Portuguese football manager for Windows 3.1, reverse-engineered from its
binary and rebuilt in **Rust** and **WebAssembly**. It runs in a browser with the original's look and feel, pixel for pixel: the same
800x600 screens, fonts, dialogs and rules, without emulating Windows.

- **Rules** recovered with Ghidra from the 16-bit Delphi executable and checked against the original's own saves and screens.
- **Interface** drawn by a renderer that reproduces Windows 3.1 and the Delphi VCL from the forms, fonts and bitmaps of your copy.
- **Verified screen by screen**: 77 screens of the game and the team editor captured from the original under emulation and compared
  with the port's render of the same state; 63 of them pixel for pixel, typically within a hundred or so of 480 000 pixels (single
  pixels of TrueType glyphs). See [docs/verification/screens.md](docs/verification/screens.md).
- **Everything** the original offers, apart from the registered-copy features: new game, up to six human coaches (hot seat),
  four divisions and the cup, match days with half-time decisions and penalties, the transfer market and auctions, finances,
  stadium and loans, coaches sacked and swapped, the end of season, saved games. The team editor (`EDITEQ.EXE`) is ported too.

**Docs:** <https://det-peralta.github.io/rustfoot98/> · **Play:** <https://det-peralta.github.io/rustfoot98/play/classic.html>
(the page asks for the folder of a skin pack built from your own copy of the game, see [Quick start](#quick-start)).

> This is an independent study and preservation project, not affiliated with the authors of Elifoot. No file of the original game
> or of Windows 3.1 is in this repository: you bring your own copy. See [NOTICE.md](NOTICE.md).

## Documentation

The docs are a site (mkdocs-material, `docs/`), published at <https://det-peralta.github.io/rustfoot98/> by `tools/site/publish.sh`
(build only: `tools/site/build.sh`).

| | |
|---|---|
| [How the game works](docs/game/index.md) | The algorithms of Elifoot 98 explained: match engine, season and cup, economy, transfer market, coaches and referees, files |
| [Verification](docs/verification/screens.md) | Every screen of the original against the port, with the remaining differing pixels per screen |
| [Reverse engineering](docs/installation.md) | Tools and method; [findings](docs/findings.md), [function map](docs/decompilation-map.md), [save format](docs/e98-format.md) |
| [The port](docs/architecture.md) | Code layout, the [classic interface](docs/classic-dialogs.md), the [match day](docs/matchday.md), the [roadmap](docs/roadmap.md) |

## Quick start

Requirements: Rust (stable) with the `wasm32-unknown-unknown` target, Python 3 with `freetype-py`, and your own copies of Elifoot 98
and of the Windows 3.1 disks (where the fonts come from), placed as described in [docs/installation.md](docs/installation.md).

```bash
# once: the skin pack (fonts, forms, bitmaps, teams) from YOUR copy, kept out of git
pip install freetype-py
python3 tools/ui/skin.py && python3 tools/ui/webpack.py
python3 tools/ui/glyphfix.py apply   # optional: the glyph corrections taken from captures of the original (see docs/verification/method.md)
rustup target add wasm32-unknown-unknown

# build the WebAssembly and serve the game
web/build.sh serve            # http://localhost:8080 (the team editor is at /editor.html)
```

The game opens with the About box, then the choice of game. It saves in the browser (every three rounds and on *Sair*) and works
offline after the first visit. Without `web/skin/`, the page asks for the folder of a skin pack and keeps it in the browser.

Native tools render any screen to a bitmap, which is how the port is compared with the original:

```bash
cargo run --release -p rustfoot-ui --bin classic_shot -- original/saves/snap2.e98 out.bmp human=FIGUEIRENSE page=2
cargo run --release -p rustfoot-ui --bin editor_shot -- out.bmp team=ARSENAL.EFT do=TeamColor
```

## Repository layout

| Path | Content |
|---|---|
| `core/` | The rules, in plain Rust: `.e98`/`.EFT` formats, the Delphi random generator, match engine, season and cup, finances, market, history. `core/examples/` hold the checks against the original's saves. |
| `ui/` | `win31/`: the Windows 3.1 / Delphi VCL renderer (frames, buttons, lists, fonts, hints). `classic/`: the game's windows and dialogs as a state machine. `editor/`: the team editor. Native bins `classic_shot`, `editor_shot`. |
| `ui-web/` | The WebAssembly host of `ui/`; `web/` holds the pages (`classic.html`, `editor.html`), the service worker and the build script. |
| `wasm/` | The first, simplified web front end (`web/legacy.html`), kept for reference. |
| `tools/` | `ghidra/` decompiler scripts, `capture/` drivers of the original under js-dos, `screens/` the screen catalogue, `ui/` the skin-pack builders and pixel checks, `site/` the docs site build. |
| `docs/` | The documentation site. |

## Development

```bash
cargo fmt --all
cargo clippy --release --workspace -- -D warnings
ELIFOOT_ROOT=$PWD cargo test --release --workspace   # some tests read your copy of the game when ELIFOOT_ROOT is set
```

CI (`.github/workflows/ci.yml`) runs the format check, clippy, the tests and the WebAssembly build. The screen catalogue
(`tools/screens/`) captures the original under js-dos and compares every screen with the port: see
[docs/verification/screens.md](docs/verification/screens.md).

## Status

The port is complete apart from the registered-copy features, which the unregistered original hides too (loans' extras,
*Procurar jogadores*, coach management). Known differences are listed per screen in the verification pages and per subsystem in
[docs/roadmap.md](docs/roadmap.md). In particular, the computer clubs' auction bids use a divisor calibrated on the original's
auctions (2.2 against the binary's 1.6, see [docs/finance.md](docs/finance.md)). The original seeds its random numbers at start-up,
so its games cannot be replayed move for move; the rules were checked on its own saves and screens instead.

## License

MIT for this repository's code, tools and documentation ([LICENSE](LICENSE)). Elifoot 98 belongs to its authors; see
[NOTICE.md](NOTICE.md).
