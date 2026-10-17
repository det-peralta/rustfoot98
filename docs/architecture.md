# Architecture

## Current site (reference)
Browser → js-dos (DOSBox→WASM) → Windows 3.1 (disk image) → `ELIFOOT.EXE`.
Angular/Ionic shell, saves in IndexedDB, hosted on GitHub Pages. ~6.7 MB bundle + 1.4 MB WASM.

## Target
```
core/   pure TS, no DOM: formats (EFT, e98, TXE), game rules, match engine, AI, RNG
ui/     DOM/Canvas screens rebuilt from the Delphi form resources
data/   converted assets (flags BMP→PNG, teams → JSON)
tests/  differential tests vs. the original running in js-dos (same seed → same result)
```
Language: TypeScript first. Core stays UI-free so hot paths could move to Rust→WASM later.
