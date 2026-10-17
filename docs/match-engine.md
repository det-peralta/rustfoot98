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
| `1058:30ba` | `eli.cod` install id | 20000 |

## Team sector strength — `1010:397d`
`team+0x51..0x57` = four `u16` sectors indexed by position (0 GK, 1 DEF, 2 MID, 3 ATT), zeroed then
for each squad slot with `slot+0x10 == 1` (starter):
```
sector[player.position] += player.strength*2 + (player.nationality == club.country ? 3 : 0) + slot.positionAdjust
```
(`player.position` = byte `+0x19`, `player.strength` = byte `+0x1a`, nationality pstr at `+0x1b`,
club country pstr at `club+0x4a`, `slot.positionAdjust` = u16 `slot+10`.)

## Attack opportunity — `1010:a6eb(match, ..., minute)`
Match object: home team ptr `+0x2e`, away team ptr `+0x32`, event list `+0x44`,
**home attack value `+0x4c`**, **away attack value `+0x4e`** (signed u16).
1. A pending penalty from the previous minute (last event: `minute-1`, kind 4, not yet taken) forces a chance
   (taken immediately, or with `Random(3)==0` on a retake) and requires `minute % 45 > 1`.
2. Otherwise home chance if `Random(6000) < homeAttack`; away chance if `Random(7000) < awayAttack`
   (home advantage: 7000/6000 ≈ +16.7%). A shooter is then picked (`1038:39b6`); if home already
   has a chance the away roll is ignored.
3. If neither primary roll fired, `1010:46df` is tried for home then (only if home didn't) for away.
4. On a chance: team shots counter `team+0x4f`++, player counters (`+0x0e`, `+0x14`, 32-bit `+0x1b`)++,
   and a match event is created with `1010:c1bf` (kind 1 normal, 2 when from a primary roll) and appended.

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

## Primary rolls recap (`a6eb`)
`homeAttack(+0x4c) = c95c(...) + 2` and `awayAttack(+0x4e) = c95c(...)` are set once in the match
constructor `1010:8db2`; `c95c` returns a small number (0..10, plus 3 under a country condition,
derived from comparing byte strings). So `Random(6000) < homeAttack` / `Random(7000) < awayAttack` are
**rare bonus events** (≈0.03–0.2 % per minute), not the main source of chances.

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

## Strength evolution — `1010:4a41(team, teamLevel)`
For each player, with `rating` from `1068:1079(real at player+0xc)` (+1 / 0 / -1):
- `+1`: if not goalkeeper, or `Random(2)==0` for goalkeepers: strength++ (max 50); undone if it exceeds `teamLevel+5`.
- `-1`: if `Random(2)==0` and strength > 1: strength--; undone if it falls below `teamLevel-5` or 1.

## Still unknown (next)
- What `c95c` really compares (byte strings at `calEntry+0x0b+4` vs `team+0x2d`), and the `+3` condition.
- How `K` (`team+0x5b`, the Real48 multiplier) is updated in `1020:038e` (FPU ops missing from the decompile).
- What `team+0x59` counts exactly (goals?) and `team+0x4f` (shots).
- How `rating` (player `+0xc`, a Real) is produced after each match.
- `1010:100b` weights, `1038:39b6` shooter selection, `1010:818c` attacker force.
