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
- **Points are 3-1-0** (verified): `1010:9396`, the per-game finaliser, adds 3 to `team+0x20` for a win and 1 to both sides
  for a draw; wins/draws/losses are `+0x16/+0x18/+0x1a`, goals for/against `+0x1c/+0x1e`. Goal difference is not stored
  in the sort key. Each game also updates the two coaches' scores (`coach+0x0e`: home win +2 for the home coach, away win
  +3 for the away coach, draw +1 home / +2 away) and may sack a coach (`8b5d` -> `4f40`).
- **Morale `K`** after every game: `K = 0.85·K + 0.15·result` (see `match-engine.md`).
- **Season end** (`1020:038e`, runs after round 14): lists are sorted by points; champion `K += 1.0`, cup winner `K += 0.5`;
  `02a6` swaps teams between lists: **top two of each lower division <-> bottom two of the division above** (D1/D2 at
  indices 6; D2/D3 and D3/D4 at 4 after the lists shift), then **two distinct random clubs from an outside pool replace
  the bottom two of the 4th division**; year +1 and a new season starts (`2f3c`). Events are recorded for titles,
  promotions ("Promovido para o"), relegations ("Despromovido para o") and sackings.
- **Division base levels** (`1010:0b60`: `division+4 = index × −14 + 52`): **52, 38, 24, 10** for divisions 1–4 and −4 for the outside pool (index 4, never evolved). The evolution window of rank `r` is `base − 2r ± 5`.
- **Evolution** after every round (see `match-engine.md`).

## New game (verified against a real save)
- **Division assignment** (`1028:002a`, `1028:0544`): the master team list is built by looping the EFT **level from 20 down to 1**,
  taking the team files of that level (ties in directory order; on the host DOSBox lists files alphabetically, case-insensitive)
  from the selected countries; entry `i` goes to division `min(4, i / 8)`, so the first 32 teams fill the four divisions and
  the rest are the outside pool. The game needs 34–128 teams (`1028:18d9`). In the pre-match save the 52 teams appear in strictly
  descending level order and the 4th division is the fourth group of eight (Sport, Waregem, Blumenau, Ceará, Cruzeiro,
  Figueirense, Goiás, Guarani), which includes the round-1 opponent.
- **Starting strength** (`1028:0373`): team at list position `i` gets `base(i) = max(5, round((1 − i/32)·50) − 1)` for `i ≤ 32`
  and 1 beyond (49, 47, 46, 44, 43, 41, 40, 38 | 37 … 26 | 24 … 13 | 11, 10, 8, 7, 5, 5, 5, 5 | 1…), and every player gets
  `clamp(base + 3 − Random(7), 1, 50)`. Figueirense (index 29, base 5) had a squad averaging 5.4, as observed.
- **The human manager** is placed in a random one of the **bottom six** teams of the 4th division (`1028:0b26`: index `7 − Random(6)`).
- Team finances are initialised from the position too (`team+0x71..0x7d`, ticket price etc.): not decoded.

## Assumed in the Rust port (`core/src/season.rs`)
(3-1-0 points, morale and promotion/relegation are now verified and implemented.)
- **4-4-2 auto line-up** — the original (`1010:4b73`) chooses a formation; ours is a stand-in.
- **Auto line-up**: the strongest of the game's twelve formations (`FORMATIONS`, verified from the menu) by total strength; the original's rule (`1010:4b73`) is not decoded. Real squads are uneven (e.g. 3 defenders), so a fixed 4-4-2 does not work.
- Ranks for evolution are taken after the round's results (the original sorts at `0c41` start; order not confirmed).

## What the simulation shows (`cargo run --release -p elifoot-core --example season -- 12 2000`)
Eight clubs of mean strength 9, 8, 7, 6, 6, 5, 4, 3, 2000 seasons: the strongest wins the title in 37 % of seasons,
the weakest in 0.9 %; mean squad strength falls by roughly 2 points over a season (starters drift down, reserves up,
clamped by `base − 2·rank ± 5`). The drift magnitude depends on the unknown base level, so treat it as indicative only.

## Still to decode
The per-round byte array at `team+0x21` that the sort compares (the points word is `+0x20`);
finances and transfers (`3e9c`, `6e48`, team `+0x71..+0x79`); cup details (round naming, ties, extra time/penalties);
the global `Real` at `10c8:2b46`; coach sacking rule (`8b5d`); what the new-season routine `2f3c` resets.

## World dynamics (core/src/world.rs, `cargo run --release -p elifoot-core --example world -- 12`)
A new world starts, by the verified rule, with mean squad strength about **43 / 31 / 18 / 7** for divisions 1..4. With the
decoded evolution (starters drift down, reserves up, clamped to `base − 2·rank ± 5`) the means then erode slowly, to about
**34 / 20 / 11 / 2** after twelve seasons. The original presumably counters this with mechanics not decoded yet
(transfers, finances), so long runs of the demo drift weaker than the real game would.
