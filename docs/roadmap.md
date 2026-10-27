# Roadmap to a finished port

Goal: the whole game (`ELIFOOT.EXE`) and its team editor (`EDITEQ.EXE`) usable in the browser with the original's screens, rules and flow,
verified against the original running under js-dos.

Done: rules core (match, season, finance, market, cup, hot seat, unemployed humans), the classic interface (every main-menu command, the 25
dialogs the game opens), save/load, PWA. Recorded seasons of the original (`tools/capture/season.sh`, `analysis/captures/`) cover the end of a
season, the sack list, the save gauge and an unemployed human (see `docs/matchday.md`). Left out by decision: registration (no key check),
printing.

## 1. Small game gaps: done (2026-10-27)
- Sound: the original's beeps (`1068:11c7`) via `Request::Beep` and WebAudio; the cup draw's pace (1500 ms only while humans' clubs are left).
- *Ver os desempates por penalties...*: every shootout kick by kick (`World::real_shootouts`).
- Market refusals in the original's words ("Tem demasiados jogadores", "... estrangeiros").
- Found to be registered-only, so hidden like the original's unregistered copy: the options' *Equipas com empréstimos / endividadas*
  (the *Controle* page) and the referee aid with its balance picture (`1008:2427`).
- Forced substitution with only a goalkeeper left: the port behaves as the original (list, OK disabled until a row is chosen).

## 2. Screens that need the original first (capture, render with `classic_shot`, diff with `tools/ui/regions.py`)
- New game: *Selecção de Países* and *Dinheiro* done (captures in `analysis/captures/newgame/`). The currency record (`economy::CURRENCIES`,
  `World::currency`) gives `G`, the loan step, the prizes and the word after amounts; kept in saves, read from `.e98` headers. Open: the ticket
  price's display for small units (`1018:11f3` uses `+75` and the factor), the player-search value `+67`.
- Invitation of a human who has a club (`TinviteCoachDlg`).
- Stadium: checked at 10 000 to 25 000 seats (`analysis/captures/stadium/`); the user may build whenever the money is there. The stands'
  geometry is still approximate (about 500 pixels per picture).
- Registered-only in the original's free copy, hidden in the port too (`1018:11f3`, features of `1050:30bc`): *Gestão de treinadores*,
  *Pedir/pagar empréstimo*, *Sair sem gravar*, *Jogadores sob observação*, *Procurar* (`TCheckPlayerDlg`, the spin edits), *Aquisições*;
  also the options' *Controle* page and the referee aid.

## 3. Rules: until played days match
- Record days of the original from `ABCDEEEE` with the recorder (its inputs are fixed), replay the same inputs in the port from
  `original/saves/snap2.e98`, and diff the *Finanças* page day by day (the renderer is pixel-exact, so equal numbers give equal pixels).
- In order: gate receipts `1040:35e1`, salaries (`7ff4` / `+0x31`), market tiers `63fa`, the coach-swap side effect (`core/src/season.rs`),
  half-time logic, the sort direction of `1068:0e0d`; candidates in `docs/decompilation-map.md`.
- The referee: decode whether `TRefereeAttr` affects a game; replace the display-only choice (`ui/src/classic/main_win.rs`).
- Done when cash, receipts and wages match for one season (14 rounds) within rounding.

## 4. Hot-seat leftovers
- Done: one coach profile per human; the unemployed state kept in saves; the turns in the order of the day's games (`13e1` / `3e9c`).
  Open: the auctions interleaved with the main windows per club (`3e9c` calls `5b37` before the human's window); a new game's first day.

## 5. Team editor (`EDITEQ.EXE`)
- Decompile it with the Ghidra scripts in `tools/ghidra/`; map the menu of `TEditor_Main` (Ficheiro, Jogador, Equipa) and the save checks.
- `core/src/eft.rs`: a writer, round-trip tested on every file of `EQUIPAS`.
- `ui/src/editor/`: the editor's windows on the `win31` renderer and the `Dialog` model; skin pack with EDITEQ's forms; native and web hosts;
  edited teams in browser storage, used by the game's new-game flow.
- Verified window by window against the original under js-dos; a team edited in the port opens in the original.

## 6. Browser product and close-out
- A full season in the browser (Playwright) at every speed; autosave, load, *Sair*, service-worker update; speed against the original.
- Final run: two humans, three seasons, native and browser, no panics; CI green; README "Not modelled" and this file updated.

## Size
| Step | Sessions |
|---|---|
| 1 | 1 |
| 2 | 1-2 |
| 3 | 2-3 |
| 4 | 1 |
| 5 | 3-4 |
| 6 | 1 |
