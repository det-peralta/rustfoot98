# Match engine (verified in our Ghidra project)

Addresses are static (`1010:xxxx` = Code3). Functions were decompiled with `tools/ghidra/DecompileAt.java`;
`Random(n)` is Delphi `System.Random` at `10c0:212d` (all call sites: `tools/ghidra/RandomCalls.java`).
Object offsets are Delphi 1 instance offsets (16-bit). Confirms the leads in `references.md`.

## Random call sites (arg = constant)
| function | role | Random args |
|---|---|---|
| `1010:a6eb` | per-minute attack opportunity | 3, **6000**, 3, **7000** |
| `1010:46df` | secondary chance generator | 90, 180 (home) / 120, 270 (away), 45 |
| `1010:81ed` | shot duel / outcome | attacker force, defender strength, 4 |
| `1010:4a41` | post-match strength evolution | 2, 2 |
| `1010:100b` | foul/card roulette | (weights) |
| `1010:397d` | team sector strength | (none) |
| `1010:7516` | squad slot init (`g`, star) | 10, 11 (via `10c0:1882`) |
| `1010:aea7` | goal annulment | 200 |
| `1010:aa11` / `ac72` | cards / injuries (not decoded) | 1000 / 500 |
| `1058:30ba` | `eli.cod` install id | 20000 |

## Squad slot, hidden attributes and star ("craque") — `1010:7516`, `397d`, `818c`
**Corrections:** earlier notes (and third-party write-ups) said a starter adds `strength × 2` and that players have
no hidden attributes. The disassembly shows `slot.strength + 2 × star + (same country ? 3 : 0)` (`397d`), and
three hidden attributes that are **deterministic functions of the player's name** (`7516`, `10c0:1882` is a 32-bit
`mod`, not `Random`). With `H` = sum of the bytes of the name:

| attribute | slot offset | formula | verified |
|---|---|---|---|
| `g` (talent) | `+0x2b` | `H mod 10 + 1` | 15/15 players of a real save (tail `+23`) |
| aggression (red-card weight) | `+0x2d` | `5 - trunc(sqrt(H mod 36))`; defenders `(x+2) mod 6` | 15/15 (tail `+17`) |
| injury proneness | `+0x2f` | `H mod 11` | 15/15 (tail `+21`) |

- **Star** (player `+0x1a`) = `g >= 8 && position >= 2`; the only `*` player in the save has `g = 9`.
- Slot: `+6` player data, `+0x0a` strength (1..50), `+0x10` status (0 out, 1 starter, 2 substitute),
  `+0x27` ban games, `+0x29` injury games. Player data: `+0x19` position, `+0x1a` star, `+0x1b` nationality.
- **Sector strength** (`397d`): each starter adds `strength + 2*star + 3*sameCountry` to `sector[position]`.
- **Attacker force** (`818c`): `round_half_even((position+1) * strength * (g/10 + 1) / 2)`.
- **Shooter for AI chances**: starter with the highest force (ties -> later slot; `0ee6` + comparer `201d`).
- **Defender**: starter with the lowest position, i.e. the goalkeeper, strongest on ties (`0ee6` + comparer `1f88`).
- **Auto line-up** (`4b73`): `team+0x4d` (substitutions left) = `min(3, fit substitutes)`.

## Weighted roulette — `1010:100b(kind, mode, squad)` (scorers, cards, injuries)
Weights (`0f2e`): kind 1 `strength`; **kind 2 `position² × strength × g`** (scorer); kind 3 aggression; kind 4 proneness.
`total` sums the weight of **every** slot (bench and unavailable players included), then `r = Random(total)`;
only starters are eligible and the first one whose running sum exceeds `r` is chosen. A roll that lands in the
bench's share picks **nobody** and the event does not happen, so deep benches dilute goals, cards and injuries.

## Red cards (`aa11`, event kind 4) and injuries (`ac72`, kind 5)
- `aa11`: `r = Random(1000)`; if `r < 10 && home.bonus(+0x4c) < r` home, else if `r < 10 && away.bonus(+0x4e) < r` away.
  `48be`: roulette kind 3; needs aggression >= 1; the player leaves (status 0) and gets a ban `Random(4)+1` games.
- `ac72`: `r = Random(500)`; `0` = home, `1` = away. `4940`: roulette kind 4; needs proneness >= 1; status 0,
  injury `Random(Random(20)) + 1` games, **strength −10 (minimum 1)**.
- AI substitution (`67f7`), if `subs_left > 0`: injury = first designated substitute with the same goalkeeper-ness;
  red card = only for a sent-off goalkeeper: first substitute goalkeeper in, **last starter out**; a red card to an
  outfield player is not replaced. Sectors are then recomputed (`397d`).
- **Penalty** (`a6eb`): if the previous minute ended with a red card of the other side (and `minute % 45 > 1`),
  the offender being a goalkeeper (always) or a defender (`Random(3) == 0`), the side gets a forced primary chance.
  A pending penalty also blocks the main generator that minute.

## Attack opportunity — `1010:a6eb(match, ..., minute)`
Match object: home team ptr `+0x2e`, away team ptr `+0x32`, event list `+0x44`,
**home attack value `+0x4c`**, **away attack value `+0x4e`** (signed u16).
1. A pending penalty from the previous minute (last event: `minute-1`, kind 4, not yet taken) forces a chance
   (taken immediately, or with `Random(3)==0` on a retake) and requires `minute % 45 > 1`.
2. Otherwise home chance if `Random(6000) < homeAttack`; away chance if `Random(7000) < awayAttack`
   (home advantage: 7000/6000 ≈ +16.7%). A shooter is then picked (`1038:39b6`); if home already
   has a chance the away roll is ignored.
3. If neither primary roll fired, `1010:46df` is tried for home then (only if home didn't) for away.
4. On a goal: `team+0x4f` (**goals scored**) ++, the scorer's counters (`+0x0e`, `+0x14`, 32-bit `+0x1b`) ++,
   and a match event is created with `1010:c1bf` (kind 1 normal, 2 when from a primary roll) and appended.
   The primary path counts as a goal only if the duel `81ed` returns outcome 1 (`GOLO`).

## Secondary (main) chance generator — `1010:46df(minute, opp, shooterOut, side, team)`
Fully decoded from the disassembly (`tools/ghidra/ListRange.java`). Stack order is Pascal: the *last* source
parameter sits at `bp+6`, so Ghidra's `param_1` is the own team, `bp+0xa` is `side` (0 home / 1 away),
`bp+0xc` the shooter out-pointer, `bp+0x10` the opponent (pointer-to-pointer), `bp+0x14` the minute.

RTL helpers used (all in the Delphi system unit, `10c0`): `04b2` = `ST1/ST0`, `04af` = `ST0/ST1`,
`1295` = `Ln`, `1299` = `Exp`, `1285` = `Sqrt`, `11af` = Real48→Extended, `1068:1194` = `Power(base, exp)`.

Sector values live at `team+0x51 GK, +0x53 DEF, +0x55 MID, +0x57 ATT` (from `397d`). Let `own` = this team, `opp` = opponent:
```
ownAtt  = own.ATT + own.MID / 2                         # 455d
oppDef  = 3*opp.GK + opp.DEF + opp.MID / 2              # 44f5
R       = 4*ownAtt / (oppDef + 1)
# 45b8: running sums, NOT reset between lines (start lo=1, hi=1, S=0)
 line 1: lo += own.DEF ; hi += opp.ATT          ; S += 1 * lo/(lo+hi+1)
 line 2: lo += own.MID ; hi += opp.MID          ; S += 2 * lo/(lo+hi+1)
 line 3: lo += own.ATT ; hi += 3*opp.GK+opp.DEF ; S += 1 * lo/(lo+hi+1)
Y       = Ln(R + 1) * S            # Power(Y, 1.0) is the identity
K       = Real48 at own.+0x5b      # starts at 1.0 (exponent byte 0x81, mantissa 0); modified by 1020:038e
T       = (K / 2)^(1/4) * Y        # two Sqrt calls
```
Stage 1 (candidate): `Random(90) < T - (own.shots(+0x4f) + own.goals?(+0x59))` **or** `Random(180) == 0` for home;
`Random(120)` / `Random(270)` for away.
Stage 2 (warm-up gate): the candidate stands only if `Random(45) < minute` — early minutes are suppressed
and from minute 45 on it always passes. At minute 45 a half-time hook (`44a1`) also runs; `44a1` picks the shooter
via `1010:100b` (weights) into `shooterOut`.
Stage 3: needs a shooter (`*shooterOut != nil`). Then `8c67` is checked twice and `1068:11c7` runs (side effect, unknown).

## Per-fixture bonus values — `1010:c95c` (decoded)
The match constructor `1010:8db2` stores `homeBonus (+0x4c) = c95c(division, home) + 2` and
`awayBonus (+0x4e) = c95c(division, away)`. Exactly:
```
A = pstr at (division+7)->+4        B = pstr at teamdata+0x2d         n = max(len A, len B)   (1068:0fed = Max)
S = sum_{i=1..n} A[i] - B[i]         (bytes past a string's length count as 0)
value = |S mod 11|  (signed remainder, then abs)  + 3 if  pstr(division+7 -> +0x2d) == teamdata country(+0x4a)
```
`|S mod 11|` is uniform on 0..10, and the `+3` applies to every domestic league fixture, so a typical league match has
home bonus 5..15 and away bonus 3..13. The bonus gates four things: the rare primary rolls
(`Random(6000) < homeBonus`, `Random(7000) < awayBonus`), red cards and annulments (both need `bonus < r` with `r < 10`,
so a bonus of 9 or more disables them), which makes a high-bonus fixture calm and a low-bonus one stormy.
Which exact strings these are (division/country names vs team strings) is not identified; for simulation any strings do.

## Shot outcome — `1010:81ed(attacker, defenderSlot)`
```
a = Random(attackerForce);  d = Random(defender.strength)
code = 0
if Random(4) == 0:                    # 25%: miss lottery, decided by a
    a < 4  -> 5 ; a < 10 -> 6 ; a < 12 -> 4 ; a < 13 -> 3
if code == 0: code = (a < d) ? 2 : 1
```
Codes map to the on-screen strings (order found in the binary): 1 GOLO, 2 Defendeu, 3 Ao poste,
4 (barra) crossbar, 5 Ao lado, 6 Por cima.

## Strength evolution and the rating accumulator (decoded)
**Rating accumulator** (`slot+0x0c`, int): `1010:afff` runs every minute of every game and calls `49e2` for both teams:
pick one random squad member (`Random(squad.count)`, home team first); if it is a starter its rating goes **−1**,
otherwise (bench or not in the matchday squad) **+1**. Starters therefore drift negative and reserves positive.

**Evolution** runs after **every round**, not only at season end: `1020:0c41` (end-of-round routine, also updates a
global `Real` random walk at `10c8:2b46` clamped to [lo, hi] — role not decoded) -> `1010:0d9c` (the 4 divisions)
-> `6fb8` (each team, in list order) -> `4a41(team, level)` with **`level = round(division.base − 2 × rank)`**
(`rank` = index of the team in the division's list, 0 = best; `division.base` = division object `+4`).
For every slot of the squad (starters and bench), with `rating = sign(slot+0x0c)` (`1068:1079`, threshold 0.0):
- `+1`: if not a goalkeeper (or a goalkeeper winning `Random(2)==0`), strength++ (max 50); undone if it exceeds `level+5`.
- `-1`: `Random(2)==0` is **always drawn first**, then if strength > 1 it drops by 1; undone if below `level−5` or 1.
- the rating is **not reset** between rounds (only when a coach takes over a team, `4f40`), so the sign keeps pushing:
  strong starters keep sliding down until they reach `level−5`, strong-by-position reserves climb to `level+5`.
  The effect is a steady pull of every squad toward its team's level window. (Matches the save: after 2 rounds most
  starters had lost 1–2 points and a reserve had gained 1.)

## Morale `K` (`team+0x5b`, Real48) — `1020:038e`, `1010:4f40`
`K` is the team's "moral" (the game has Moral gauges). It is 1.0 at creation and when a coach takes over (`4f40`).
After a processed game `038e` adds **+1.0 to the user's team and +0.5 to its opponent**. It enters the goal threshold as
`(K/2)^(1/4)`, so a long-serving coach's team scores more. `038e` addresses the user's game pair (list index 0), so
AI-vs-AI games leave `K` untouched.

## Still unknown (next)
- What `c95c` really compares (byte strings at `calEntry+0x0b+4` vs `team+0x2d`), and the `+3` condition (it gates cards, annulments and the 6000/7000 rolls).
- Role of the global `Real` at `10c8:2b46` that `0c41` random-walks every round (likely a finance/economy index).
- Which of the two teams of a game is `puVar8` (+1.0) in `038e`; whether AI games ever change `K`.
- What `team+0x59` counts exactly (goals?) and `team+0x4f` (shots).
- `1010:100b` weights, `1038:39b6` shooter selection, `1010:818c` attacker force.

## Minute loop — `1048:069b`
Per minute (`match+0x18e`), for every game of the round in turn: `a6eb` (goal chances); if no goal, `aa11`
(`Random(1000)`), then `ac72` (`Random(500)`); if still nothing and minute is not 46 or 91, `aea7`
(annulment: with `Random(200) < 10 && match.homeBonus < roll`, a goal from the previous minute is cancelled:
`team+0x59`++, `team+0x4f`--, scorer counters --). Then `afff`. At minute 45 a half-time routine runs; at 90 a cup
tie may extend the length to 120.

## Rust port status (`core/src/matchsim.rs`)
Implemented: bonus rolls, main goal generator, weighted scorer roulette, red cards, injuries, automatic
substitutions, penalties after red cards, goal annulment (30 unit tests, clippy clean).
Not yet: half-time/extra time, interactive (human) paths, foul roulette of the UI path, and
the identity of the two `c95c` strings (the formula itself is implemented as `fixture_value`).
100k games per pairing (`cargo run --release -p elifoot-core --example sim`, 4-4-2, bench of 5, `c95c` with random strings,
domestic league match):

| home/away strength | home goals | away goals | home W | draw | away W | reds | injuries |
|---|---|---|---|---|---|---|---|
| 6 vs 6 | 1.15 | 0.87 | 42.6 % | 32.2 % | 25.2 % | 0.15 | 0.24 |
| 8 vs 5 | 1.46 | 0.82 | 54.4 % | 28.1 % | 17.5 % | 0.15 | 0.24 |
| 5 vs 8 | 1.08 | 1.12 | 32.9 % | 32.4 % | 34.7 % | 0.15 | 0.24 |
| 10 vs 3 | 1.85 | 0.51 | 74.9 % | 19.0 % | 6.1 % | 0.15 | 0.24 |
