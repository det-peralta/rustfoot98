# `.e98` save format (decoded from the writers `1020:1796` and the loader `1020:1d8d`)

Produced with `tools/ghidra/ListFunctions.java` + `tools/map/io_spec.py` (the raw operation lists of every serializer are regenerated from the
binary, not stored here) and checked against two real saves (`core/src/e98.rs` reads them). All integers are little-endian; "pstr" is the
game's delta-ciphered Pascal string (`docs/findings.md`); `DS:xxxx` are globals of the original that the header stores.

## File layout (order of the writes in `1020:1796`)
1. **Header** (`124d` / `1a01`): description pstr (double-delta, `"joga1 e joga2\r17/10/26 21:03"`), then `u16` (round counter `2b40`), `u16 271e`,
   `u8 271a`, `u16 2722`, `u32 year` (`2b42`), `ext E` (economy index, `2b46`), `ext 1.0` (ticket price rounding `2b50`/`2d9b`), and a **101-byte
   currency record** (`2d40..`): currency name pstrs (`Reais`, ` reais`), loan step `u32 @0x7b`, `u32 4`, `G u32 @0x83`, `u32 1`, prizes
   `3 x u32` (20 000 000 / 6 000 000 / 3 000 000 for `G = 5`).
2. Lists, each written as `u16 count` followed by the elements (`1010:0751` and friends), in this order: players `26fe`, teams `2702`, `2706`
   (games of the calendar), master team list `26f6` (flag 1), cup list `26f2` (flag 0), divisions `26fa`, current games `270a`, then the
   news lists (`2716`, `270e`, `2712`: `1020:1513`, `1661`, `16eb`).

## Team record (`1010:305a`, loader `2bca`)
`u16 id` (+8) · the `.EFT` object (`1058:1694`: 50-byte header `EFa`+zeros, full and short name pstrs, colours `u32 x2`, country pstr, level `u8`) ·
`55 bytes @+0x16` (level byte `+0x15`, then standings `wins, draws, losses, goals for, goals against, points` as `u16`, the points-per-round
array, the **ledger**: `+0x31` gate, `+0x35` sales, `+0x39` prizes, `+0x3d` purchases, `+0x41` wages, `+0x45`, `+0x49` interest as `i32`) ·
`14 bytes @+0x4d` (substitutions left `u16`, goals this game, sector strengths GK/DEF/MID/ATT, annulled) · `Real48 K` (`+0x5b`) · `u16 coach id` ·
`28 @+0x89` · `28 @+0xa5` · `i32 loan` (`+0x6d`) · `i32 cash` (`+0x71`) · `i32 start-of-season cash` (`+0x75`) · `ext F` (`+0x79`) · `6 @+0x83`
(players per position, foreigners `+0x87`, squad `+0x88`) · `u32 @+0xc1` · `u16 pending signings` (`+0xc7`) · `u16 pending foreigners` (`+0xc9`) ·
three lists (`2e6b`, `2f10`, `2fb5`: fixture entries) · the squad list (`1c26`: `u16 count`, players) · the stadium (`bf74`: `u8 level`,
`Real48 filling`).

## Player record (`1010:738d`, loader `7184`) — 48 bytes after the player's id, nationality and name pstrs
| tail offset | size | field (object offset) |
|---|---|---|
| 0 | 1 | position (class object `+0x19`) |
| 1 | 1 | star flag (class object `+0x1a`) |
| 2 | 2 | strength (`+0x0a`) |
| 4 | 8 | `+0x0c..0x13`: rating accumulator (`+0x0c`), `+0x10` status (0 out, 1 starter, 2 substitute) at tail 8, `+0x12` flag |
| 12 | 3 | `+0x14` goals scored (`u16`), `+0x16` *fresh* flag (set by a wage renegotiation or a transfer) |
| 15 | 2 | `+0x27` games of suspension |
| 17 | 2 | `+0x2d` aggression |
| 19 | 2 | `+0x29` games of injury |
| 21 | 2 | `+0x2f` injury proneness |
| 23 | 2 | `+0x2b` talent `g` |
| 25 | 4 | `+0x31` wage (`i32`) |
| 29 | 1 | `+0x35` nationality class (0 same country, 1 same group, 2 foreign) |
| 30 | 2 | `+0x36` pending buyer (a purchase waiting for the end of the round) |
| 32 | 16 | `+0x17..0x26` history: appearances/goals/red cards/injuries counters |

## Smaller records
* **Fixture entry** (`1010:2031`, 16 bytes, read order of `1f12`): `u16 +8` opponent team id, `u16 +0xe` venue (0 home, 1 away, 2 neutral), `u16 +0xa` and
  `u16 +0xc` the goals of the club and of the opponent (`FFFF` while unplayed), `u32 +0x10` attendance, `u8 +7` cup flag, `u8 +6` result kind (0 normal,
  1 extra time, 2 won and 3 lost on penalties), `u16 +4` round (the first entries of a season are the 14 league games, rounds 1..14).
* **Lists of the information dialogs**: champions `ca77` (`u16 year`, champion team id, its coach id, cup winner id, its coach id), transfers `cf1f` (`u16 +6` player id,
  `u16 +0xa` team id of the buyer, `i32 +0xc` fee, `u16 +8` strength, `u8 +5`, `u8 +4` a user's club is involved), team history `TTeamEvent` (`u8` kind, `i32` year, `u16` coach id)
  and coach history `TCoachEvent` (the same with a team id); the coach record after its score has `u16 +0x16`/`+0x18` titles and the five profile bytes of `+0x1a`.
* **Game record** (`1010:91cf`): `u8 type` (0 league, 1 cup, 2 ...), `41 bytes` pstr (competition), ids of the two teams, `ext price`
  (`+0x36`), `i32 attendance` (`+0x40`), lines, `u16 +0x4c/+0x4e`, `u8 +0x50`.
* **Stadium** (`1010:bf74`): `u8 level`, `Real48 filling`.
* **Division** (`1010:6ccf`): `u32 +0x1f` (cash total), `u16 +4` (base), `21-byte name pstr`, then the list of team ids.
* News / events (`b5b3`, `ba8f`, `be31`, `c816`, `cb23`, `d002`): small fixed records of 3-14 bytes (type, ids, amounts); the text of every
  message is generated at display time from them.

`core/src/e98.rs` implements the pieces needed to continue a game (teams, squads with status/goals/wage/fresh/ban/injury, standings, `K`,
cash/loan/`F`, ledger, year, economy). Not imported yet: the fixture/calendar lists, the cup list, the news lists and the second human.

## Verified end-to-end
`tools/e98/strict.py <file.e98>` walks a whole save with exactly the structure above and prints each section's offsets. On all three local
saves (new game, after two rounds, a two-human game) it consumes the file **to the last byte**. Final layout after the header:

| list | loader | element |
|---|---|---|
| `2702` | `1106` | `8382`: `u16 id`, pstr, `u8`, `u8`, `u16`, `i32`, `i32`, 5 bytes, `u16 n` + n x 7 bytes `(u8, i32, u16)` (54 entries; id/year/event history) |
| `2706` | `17e2` | `c770`: `u16 id`, `u8`, pstr (name), pstr (country) (182 entries) |
| `26f6` | `0651(1)` | team (record above); `26fe` is not read: it is the global player list filled while the squads load |
| `26f2` | `0651(0)` | `u16` team ids (cup entrants) |
| `26fa` | `0c40` | division (`u32`, `u16`, 21 bytes, `u16 n` + n ids) |
| `270a` | `1285` | game: `u8`, 41 bytes, `u16`, `u16`, 10 bytes, `i32`, `u16`, `u16`, `u16`, `u8` (20 entries = the current round) |
| news | `1b1b` | repeated `(u16 team id != 0, u16 n, n x u16)`, terminated by `0` |
| `270e` | `1c18` | `u16 n` + n x 10 bytes (`ca77`) |
| `2712` | `1cc1` | `u16 n` + n x 12 bytes (`cf1f`) |
