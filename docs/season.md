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
  +3 for the away coach, draw +1 home / +2 away) and may sack a coach (`8b5d` -> `4f40`, see *Coach sacking* below).
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
- **Auto line-up**: decoded and ported (`lineup.rs`, see `match-engine.md`); the user's *default* plan uses the same rule, formations stay selectable.
- Ranks for evolution are taken after the round's results (the original sorts at `0c41` start; order not confirmed).

## What the simulation shows (`cargo run --release -p elifoot-core --example season -- 12 2000`)
Eight clubs of mean strength 9, 8, 7, 6, 6, 5, 4, 3, 2000 seasons: the strongest wins the title in 37 % of seasons,
the weakest in 0.9 %; mean squad strength falls by roughly 2 points over a season (starters drift down, reserves up,
clamped by `base − 2·rank ± 5`). The drift magnitude depends on the unknown base level, so treat it as indicative only.

## Still to decode
The per-round byte array at `team+0x21` that the sort compares (the points word is `+0x20`);
finances and transfers (`3e9c`, `6e48`, team `+0x71..+0x79`); cup details (round naming, ties, extra time/penalties);
the global `Real` at `10c8:2b46`; what the new-season routine `2f3c` resets.

## World dynamics (core/src/world.rs, `cargo run --release -p elifoot-core --example world -- 12`)
A new world starts, by the verified rule, with mean squad strength about **43 / 31 / 18 / 7** for divisions 1..4. With the
decoded evolution (starters drift down, reserves up, clamped to `base − 2·rank ± 5`) the means then erode slowly, to about
**34 / 20 / 11 / 2** after twelve seasons. AI clubs never trade players with each other (see `finance.md`), so this erosion
is most likely intended; the user counters it by buying players at auction.

## The user's club (core/src/world.rs, web/)
- **Start**: a random one of the bottom six clubs of the 4th division (`7 − Random(6)`, `1028:0b26`); the demo also lets you pick any club.
- **Controls**: formation (the twelve of the game's menu), the role of every player (starter / bench / out) and up to three planned
  substitutions (minute, who leaves, who comes on; a goalkeeper may only be replaced by a goalkeeper, as in the automatic rule).
  Injuries and red cards still trigger the automatic replacement (`67f7`); the original stops the game and asks the user, the
  demo plays each match in one go. Planned substitutions last for one round.
- **Repairs**: unavailable (injured/suspended) or missing starters are replaced by the best available player of the same
  position before the match, and the repaired line-up is written back into the displayed plan.
- If the user's club ever leaves the league (relegated to the outside pool) the game hands them another one (`4f40`).

## The cup (`1020:0ed4`, implemented in `core/src/cup.rs` and `world.rs`)

- Played after the league games of every **even** round; every club of the world (divisions + pool) enters.
- The draw only happens when the number of survivors N is greater than `p = 2^((14 - r) / 2)`, so the final is always on round 14
  (checked for N = 34..128). Survivors are shuffled (3n remove-and-append swaps); `k` = smallest power of two >= n/2; `n - k`
  pairs are drawn (first of the pair is at home), the other clubs get a bye.
- Single leg. A draw goes to extra time (to minute 120) and then a shootout.
- **Two shootout paths** (`1010:9396` calls `1050:0d46` for a level cup game): *automatic* when no human plays the tie and the round is
  before 14 (and the option at `10c8:2b33` is off): each side sums `3 x Random(team rating)` and the *home* side is eliminated when
  its sum is greater than or equal to the away sum — **inverted** in the original, kept faithfully. Otherwise (a human plays, or
  **always in the final**) the *Penaltis* dialog plays real kicks (`Timer1Timer`, decoded from the raw bytes at `1050:114d`):
  home first, alternating; the taker is the next squad member (cycling the squad list) who is on the pitch; the opponent's best
  goalkeeper on the pitch defends; each kick is the shot duel `81ed(taker force, keeper strength)` (GOLO = goal); the shootout ends
  when a side leads by more than the other's remaining kicks of the first five, or, after five each, when a pair of kicks leaves
  the score apart (sudden death). The side with fewer goals is eliminated (the result flag is `away >= home` = home eliminated).
  The port plays the kicks (`cup::shootout`) and shows them in the Taça card; the user does not choose the direction.
- Morale: the home side that advances gets 1.5, the away side that advances 2.0, the loser 0 (see `match-engine.md`).
- Round names: `FINAL`, `Meias-finais`, `Quartos`, `Oitavos`, then "k avos de final".
- The browser shows the last cup round in a "Taça" card and the winner in the season-end summary. Saves keep the cup state
  (`cup|name`, `cupchampion|name` records); older saves start a fresh cup.

## Coach sacking and swaps (`1010:8b5d` -> `5343` -> `4f40`, `World::replace_coach`)
Coaches are objects (`coach+0x07` user flag, `+0x0c` games coached, `+0x0e` score) that belong to a team and **move between
teams**; the 54 entries of the save's first list are the 52 clubs' coaches plus the two users' coaches.
* **Scores** (`9396`, league games): home win +2 for the home coach; away win +3 for the away coach; draw +1 home / +2 away; the
  loser gets nothing. Cup ties (also `9396`): the winner's coach +3, and both coaches' game counters grow as in the league.
* **Sack check** (`8b5d`) runs right after the game, before the morale update, **only for the loser** (both clubs after a draw;
  the loser of a cup tie with flag `0x400`): the coach has coached at least 10 games and the team's `K <= 0.65`. (A user's coach on a
  club of the outside pool is dismissed at once; the pool never plays here.)
* **Replacement** (`4f40` -> `5343`): candidates are the coaches of clubs with `K >= 1.25` (constant at `1010:565d`, a
  32-bit float) whose rating (`4ec3`) is **11..19 below** the sacked club's (`5661`). Teams rated below 15 also accept unemployed
  coaches (none exist here) and rank computer coaches before the user's (`1050:1e3b`); otherwise the comparator `1050:1d59` ranks
  by `+0x16`, `+0x18` (both 0 in every save seen) and then **by score, lowest first**; the sort is the stable bubble sort `1068:0e0d`.
  The head of the list is taken; the user's coach would first be asked in a dialog (`3bf9`), and is never a candidate in round 14
  or for cup sackings (`0x400`). **Without a candidate nothing happens** and the check repeats after every lost game.
* **Effect**: the two clubs **swap coaches** (score travels, both coaches' game counters reset to 0) and both clubs' `K` resets to
  1.0. A user whose coach is swapped *takes over the other club* (so a sacked user is moved to a weaker club in form; an accepted
  invitation takes them to the sacked club). When the user leaves a club for a computer coach, `6a08` caps that club's wages at
  the players' values and lifts a negative cash balance to 0.
In the port `World::accept_invitations` (default off; `world_set_accept_invitations` in the wasm ABI) decides about invitations;
`coach_changes` / `human_moved` report what happened in the last round. About 3.5 swaps per season occur in a synthetic world.

## History records of the information dialogs (`core/src/history.rs`, `World::history`)
The dialogs *Calendário*, *Últimos vencedores*, *Historial*, *História* (coach), *Ranking* and the coach profile read what the original
keeps in object lists (see `e98-format.md`); the port keeps them by club and coach **name** (two coaches of the same name share a
record), saved as `winner|`, `fixture|`, `clubevent|`, `coachevent|`, `coachtitles|`, `coachopts|` and `transfer|` lines and imported from an
original save (`World::from_e98`).
* **Calendar** (`TCalendarEntry`, 16 bytes in the save: `u16 +8` opponent id, `u16 +0xe` venue, `u16 +0xa`/`+0xc` the club's own goals and
  the opponent's, `u32 +0x10` attendance, `u8 +7` cup flag, `u8 +6` result kind, `u16 +4` round): the 14 league games exist from the
  start of the season with goals `-1`; venue 0 home, 1 away, 2 neutral (the letters `C`, `F`, `N` of the list), result kind 0 normal time,
  1 extra time (`a. p.`), 2 won (`v. pen.`) and 3 lost on penalties (`d. pen.`). Cup ties are added after the cup draw/round. In the port
  `History::calendars` is filled by `record_league_fixtures` / `record_cup_fixtures` after every round (a cup tie is listed right after the
  league game of the round it followed, with a blank round number) and cleared at the end of the season; a club without an entry shows the
  schedule (`World::calendar`).
* **Champions** (`TLastWinners`, 10 bytes: year, champion club id, its coach id, cup winner id, its coach id): appended at the season end
  (`1020:038e`, `History::season_end`).
* **Club history** (`TTeamEvent`, 7 bytes: kind, year, coach id): kind 1 league title, 2 cup, 3 double (instead of 1 and 2 when the
  champion also won the cup), 4 second and 5 third place of the *first division* only. **Coach history** (`TCoachEvent`): 1 hired by a club,
  2 sacked, 3 league title, 4 cup, 5 "Ingresso no <club>" (written for every coach at a new game, `1020:0148`), 6 "Despromovido para o
  <club>" and 7 "Promovido para o <club>" (the two coaches of a swap, `1010:4f40`: the sacked club's coach goes to the other club), 8 double,
  9 and 10 second and third place. The champion's coach also counts a title (`coach+0x16`, the cup winner's `coach+0x18`).
* **Coach ranking** (`1050:1d59` with the stable sort `1068:0e0d`): league titles, cup titles, then score, **highest first** (an earlier
  note in this file read the comparer as "lowest first"). The port's `replace_coach` now also swaps the coaches' *names* with their
  scores, so histories and titles follow the person.
* **Coach profile** (`TCoachOptionsDlg`, 5 bytes at `coach+0x1a`): the computer manages the wages of the domestic players, of the
  foreigners covered by the Bosman rule and of the others (`1010:3e9c` branches on `option[player+0x35]`; the original asks the user in a
  dialog for the classes it does not manage), the auction mode (0 offers for all players, 1 not for the weaker ones, 2 no buying in
  auctions) and "do not let the others see my offers". The port applies the three wage switches in `club_after_round` (only the
  classes switched on are renegotiated for the user) and mode 2 (the user gets no auction listing, `human_may_bid`); mode 1 and the
  hidden offers are stored only. The original's new-game values are all off, auction 1 (an imported save keeps what it stores); a port
  world starts with the wages switched on because the port never had the asking dialog.
* **Holiday** (`coach+6`, `TCoachManagerDlg`): the computer picks the line-up for the user's club (`History::holiday`).
