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

## Secondary generator — `1010:46df(team, side, ..., minute)`
Candidate if `Random(N1) < T - (team.shots(+0x4f) + team.goals(+0x59))` or `Random(N2) == 0`
with `N1,N2` = `90,180` for home and `120,270` for away; `T` is computed with Pascal `Real`/FPU math
(`1010:45b8`, `455d`, `44f5`, constants at `1010:46d3..46db`) — **still to decode**.
Then the chance only stands if `Random(45) < minute`-style gate; at minute 45 a half-time hook runs.

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
- Who computes `match+0x4c / +0x4e` (the attack values fed to the 6000/7000 rolls) from the sectors.
- The `Real` arithmetic of `T` in `46df`.
- How `rating` (player `+0xc`, a Real) is produced after each match.
- `1010:100b` weights, `1038:39b6` shooter selection, `1010:818c` attacker force.
