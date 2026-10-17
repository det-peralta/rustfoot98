# Season structure (verified vs assumed)

## Verified in the original
- **Divisions of 8 teams, 14 league rounds.** `1020:11b8` (round driver) loops `round = start..=14`.
  Per round: `1020:0c41` (play + end-of-round), and on **even** rounds `1020:0ed4` (the **cup draw**), so the knock-out cup
  alternates with the league. After the loop `1020:038e` applies the morale update for the user's game.
- **Fixture table** (`DS:0x1838`, 8 × 14 signed bytes, read by `1010:1d20`): for team index `t` and round `r`,
  `v = TABLE[t][r-1]`; opponent index = `|v| mod 8` (8 means team 0); `v > 0` home, `v < 0` away.
  Verified by tests: every pair meets twice (once at each ground), each team has 7 home games, 4 games per round.
  The table is `SCHEDULE` in `core/src/season.rs`. Indices are fixed team slots, not the sorted standings position.
- **Round setup**: `1010:0875(teams, nPairs, ...)` creates games by pairing consecutive teams (`i`, `i+1`) and adds them
  to the day's game list; the cup draw first shuffles the entrants with `1068:0f68` (3n random swaps).
  Game finalisation: `1010:13e1 -> a67f -> 3e9c` (per team post-game: finances and transfer AI, 11.8 KB, **not decoded**).
- **Standings order**: the division's team list is sorted with comparer `1050:1a61`, which compares a per-round byte
  array at `team+0x21+round` (points). Only that value is compared, so ties are not separated by goal difference.
  (Assumed: the sort `1068:0e0d` is stable.) The sorted rank feeds the evolution clamp `level = base − 2·rank`.
- **Evolution** after every round (see `match-engine.md`).

## Assumed in the Rust port (`core/src/season.rs`)
- **3-1-0 points** — the write to the per-round byte array was not located.
- **4-4-2 auto line-up** — the original (`1010:4b73`) chooses a formation; ours is a stand-in.
- Ranks for evolution are taken after the round's results (the original sorts at `0c41` start; order not confirmed).
- **Division base level** (`division+4`) is unknown; the example takes it as an argument (default 12).

## What the simulation shows (`cargo run --release -p elifoot-core --example season -- 12 2000`)
Eight clubs of mean strength 9, 8, 7, 6, 6, 5, 4, 3, 2000 seasons: the strongest wins the title in 37 % of seasons,
the weakest in 0.9 %; mean squad strength falls by roughly 2 points over a season (starters drift down, reserves up,
clamped by `base − 2·rank ± 5`). The drift magnitude depends on the unknown base level, so treat it as indicative only.

## Still to decode
Division base level; points rule and the `+0x21` array writes; finances and transfers (`3e9c`, `6e48`, team `+0x71..+0x79`);
cup details (round naming, ties, extra time/penalties); the global `Real` at `10c8:2b46`; promotion/relegation at season end.
