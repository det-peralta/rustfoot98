# Roadmap to a finished port

Goal: the whole game playable in the browser with the original's screens, rules and flow, verified against the original running in DOSBox.
Done so far: rules core (match, season, finance, market, cup, hot seat), the classic interface (all windows and dialogs), save/load, PWA.

## Phase 1: record the original (evidence first)
Everything below is checked against captures, so the captures come first.
- Keep captures out of `/tmp` (reboots wipe it): use `analysis/captures/` (git-ignored) with a driver script kept in `tools/capture/`.
- Play one human season of the original in Rápido (about 27 s per day, 80 rounds): per-day frames plus a log of dialog kinds.
- Targets: end-of-season screens (titles, promotion/relegation), invitations, sack list, start-of-day auctions, cup-day market, new season hand-over.
- Exit: a frame list per dialog kind, each tagged with the port dialog it maps to.

## Phase 2: close the screen gaps (by evidence)
1. End of season, invitation and sack-list dialogs: compare with Phase 1 frames, fix with `tools/ui/regions.py`.
2. Auction timing: auctions at the start of a day (inside `3e9c`, before the human's main window) and the cup-day market.
3. Stadium levels above 2, TSpinEdit/speed button/colour grid, TCheckPlayerDlg, observed and acquisitions dialogs.
4. Referee-aid dialog balance drawing; "view computer penalties" option.
5. Referee shown (display-only guess today), turn order after a round with several humans.

## Phase 3: rules still approximated
- Fees calibrated on one save: check against a second save and a played season (market tiers `63fa`, gate receipts `35e1`, salaries).
- Coach swap partner choice (`5343`), half-time logic, core sort comparer `1050:1d59` direction.
- Method: for each item decode the function, add a core test with values taken from the original, then compare a played day with the original.

## Phase 4: browser product
- Play a full season in the browser (Playwright) at every speed setting; check autosave, load, "Sair", service-worker update.
- Performance: a round at "Médio" takes about 3 minutes in the original's timing; check it matches the original rather than being slower.
- Sound: the original only uses `MessageBeep`; add the same beep via WebAudio.
- Mobile/touch is out of scope unless asked.

## Phase 5: close out
- Update docs: analysis-plan checklist (stale items), decompilation-map work queue, README "Not modelled" list.
- Final run: two-human season, three seasons, in native and browser, no panics; CI green.
- Registration features stay hidden (no key check, by decision).

## Order and size
| Phase | Size | Depends on |
|---|---|---|
| 1 record | one background run, a few hours | none |
| 2 screens | about 1 to 2 sessions | 1 |
| 3 rules | open ended; stop when played days match | 1 |
| 4 browser | about 1 session | 2 |
| 5 close | short | all |
