# External references (leads, not ground truth)

Everything here is third-party research. Treat as hypotheses; **verify each claim in our own
Ghidra project** before relying on it. Addresses in those sources are runtime selectors
(`17fb:`, `3e3c:`, `DS=0x8484`) and do not match our static ones (`1010:`, `10c0:`, ...).
Do not copy their code (the match-engine repo has no license); re-derive and re-implement.

| Source | License | Covers |
|---|---|---|
| [andrelcmoreira/libelifoot](https://github.com/andrelcmoreira/libelifoot) (Python, `docs/equipa_format.txt`) | LGPL-3.0 | `.EFT` team format. Independently **confirms our decode** (header 50 B, cipher, field order). |
| [andrelcmoreira/elf98](https://github.com/andrelcmoreira/elf98) | none stated | CLI to view/patch `.EFT`; same format doc; provider data (transfermarkt/espn JSON) |
| [trsthales/desvendando-elifoot-98](https://github.com/trsthales/desvendando-elifoot-98) + TabNews series | none stated | Ghidra + Cheat Engine research on match engine, team strength, evolution (see below) |
| [TabNews part 2: 16-bit lab](https://www.tabnews.com.br/trsthales/desvendando-o-elifoot-98-parte-2-como-montar-um-laboratorio-de-engenharia-reversa-de-16-bits-no-debian-12-com-ghidra-winevdm-e-cheat-engine) | | winevdm (otvdm) to run Win16 under Linux; Ghidra "Decompiler Parameter ID"; ints are 2-byte `SmallInt` |
| [TabNews: engine analysis](https://www.tabnews.com.br/trsthales/engenharia-reversa-como-o-motor-do-elifoot-98-decide-os-gols-e-por-que-o-5-0-5-quebra-o-jogo) | | match engine summary |

## Corrections to our own notes (from libelifoot)
- The "strength 1..20" byte in `.EFT` is the team **level**; the "squad size" byte is the player count.
- The two colours are **background** then **foreground** (text).

## Claims to verify in Ghidra (match engine, from trsthales)
- Pre-match sector strength (GK/DEF/MID/ATT) = sum over starters of `force*2 + (same_country ? 3 : 0) + position_adjust`.
- Per-minute loop 1..90 (`minute` counter at object offset `0x18e`); chance generation uses
  `Random(6000) < attack` (home) vs `Random(7000) < attack` (away).
- Secondary generator: home `Random(90) < threshold - (shots+goals)` or `Random(180)==0`;
  away `Random(120)` / `Random(270)==0`.
- Duel: `Random(attackerForce)` vs `Random(defenderForce)`.
- Shot outcomes: GOLO, Defendeu, Ao poste, barra, Para fora (strings confirmed in the binary).
- Evolution: after games, +1 force if rating good (keeper needs extra `Random(2)==0`), capped at 50 and
  at `teamLevel+5`; -1 on poor rating with 50% chance, floor `teamLevel-5`.
- Player in-memory struct: force 1..50, position 0..3, nationality text, contract games left,
  salary/value, discipline/injury state. No hidden attributes.
- Behaviour (aggressiveness) only weights the foul/card roulette; contracts are a countdown.
- A runtime file `eli.cod` exists (deleting it makes the game recreate it).

## Saves
No `.e98` sample or format description was found online (searched GitHub, TabNews, general web).
We must produce our own saves from the original game.
