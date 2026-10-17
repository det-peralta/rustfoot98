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
| [docs/season.md](docs/season.md) | Season structure: fixtures, rounds, standings |
| [docs/references.md](docs/references.md) | Third-party research (leads to verify) |

## Status
See the checklist in [docs/analysis-plan.md](docs/analysis-plan.md).
