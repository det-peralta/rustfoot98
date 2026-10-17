# Findings

## Binary
- `ELIFOOT.EXE` 836,356 B, `EDITEQ.EXE` 472,532 B, both MS-DOS stub + NE 6.01 (Windows 3.x GUI).
- Delphi 1 evidence: `TGauge`, `TGaugeKind`, `TBiGauge`, `gameClock`, `Thunder` strings.
- External deps: `BIVBX11.DLL`, `GAUGE.VBX`, `CTL3DV2.DLL`, `CARDS.DLL`.
- Gauges in UI: gaugeHomeAid, gaugeAwayAid, gaugeMoral, gaugeOpponentMoral, gaugeProgress, gaugeSkill.

## Data
- `EQUIPAS/*.EFT`: ~300–600 B per team. Format TBD.
- `FLAGS/*.BMP`: country flags.
- `COUNTRY.TXE`, `REFEREE.TXE`, `CTRGROUP/*.TXE`: data/text, format TBD.

## Existing web port
Boots Windows 3.1 from `hdd.img` inside `elifoot98.jsdos`; game at `D:\ELI98`, saves `d/eli98/jogos/*.e98`.

## Ghidra import (ELIFOOT.EXE)
- Loader: New Executable (NE); language `x86:LE:16:Protected Mode`. Auto-analysis ~34 s.
- 25 code segments (~16 KB each, Code3 56 KB), 1 data segment (12.7 KB), many resource blocks.
- 3007 functions, 1111 defined strings, 662 named symbols (mostly imports: KERNEL, BIVBX11 ordinals).
- Delphi RTTI/names present in strings: `ExecuteTransfers`, `TtransferPlayerDlgX`,
  `JogadorTransferirClick`, `ultimastransferencias1Click`, `Golos`, registration tiers
  (registada / VIP / super-VIP / amigo / experimentador / autor).
- Uses Jan Goyvaerts' freeware component (1996) and Ctl3d.
- REA `analyze` does not support NE; use Ghidra headless (see installation.md).

## File formats (decoded, verified on all shipped files)
See `tools/efdecode.py` (reference decoder). Summary:
- **String cipher:** Pascal string `[len][bytes]`, delta-encoded: `enc[i]=enc[i-1]+plain[i]`, `enc[-1]=len`.
- **`.TXE`:** same, applied twice. `BOSMAN.TXE` = title + country codes under the Bosman rule;
  `PLOP.TXE` = Portuguese-speaking countries; `COUNTRY.TXE` = 217 `CODE Name` entries;
  `REFEREE.TXE` = 182 `CODE Name` referees.
- **`.EFT` (195/195 files parse to exact file length):** magic `EFa`, 49-byte zero header, full name,
  short name, two kit colours (RGB+pad), country code, strength 1..20, squad size 14..20,
  squad (nationality, name, position 0 GK/1 DEF/2 MID/3 ATT, 0), coach.
  Players carry **no skill value** in the file; skill must be derived at runtime (to confirm in code).
- **`JOGOS/DRLIST.ELI`:** 102-byte list, record layout TBD. `.e98` saves: format TBD
  (need a save produced by the game).

## Save game (`JOGOS\*.e98`) — static findings so far
- Delphi inline string literals sit **before** each routine; the real entry is the next `55 89 E5` prologue.
  Pointing the decompiler at a string address yields garbage.
- Save routine: `1020:1796` (load counterpart near `1020:1d8d`). It writes the game state as a sequence of
  lists, in this order, from globals `DAT_10c8_26fe, 2702, 2706, 26f6, 26f2, 26fa, 270a` plus three
  special blocks (`1020:1513`, `1020:1661`, `1020:16eb`):
  `1010:0e33` (teams, trivial stub), `1010:11c3`, `1010:189f`, `1010:0751` (x2, flag 1/0 — squad lists),
  `1010:0cfd`, `1010:1342`.
- List serializer (`1010:0751`): writes `u16 count`, then each element via a per-class method
  (`1010:305a` for the flag=1 variant) or a raw 2-byte id at `elem+8` (flag=0). Binary stream via
  `10c0:0f67` (BlockWrite-like). Not yet decoded field by field.
- Other files referenced: `\equipas.ef2` (bundled teams), `\treinad.ef2` (coaches), `drlist.eli`,
  `elif98.ini` (sections System/Options), `win.ini` section `[Elias]` (registration).
- Ghidra's decompiler is weak on 16-bit far-call Delphi code; expect to read disassembly for field layouts.

## Method note
Static decompilation of every serializer is slow. A real `.e98` save from the original game gives a
ground truth to decode black-box and to validate each static finding.

## `.e98` save — decoded from a real save (snapshot 1: new game, Figueirense F.C., Brazil 4th div, 2026)
Source: original game running in native DOSBox (`tools/run/run-original.sh`), file `JOGOS\ABCDEEEE.E98`, 83,676 bytes.
Saves are written to the host folder `original/run/d/eli98/jogos/` (git-ignored).

- **Header:** double-delta pstr (like TXE): `"joga1 e joga2\r17/10/26 21:03"` = save description
  (the two manager names + timestamp). Followed by fixed binary state (season year `0xEA07`=2026 at
  offset ~0x27, etc.), plain (non-encoded) Pascal strings of fixed width, e.g. currency `"Reais"` / `" reais"`.
- Strings elsewhere are single-delta pstrs (same cipher as `.EFT`).
- **Player record** (verified on all 15 Figueirense players against an in-game screenshot):
  `[id u16 LE][nat pstr (4 B)][name pstr][48 fixed bytes]`, ids sequential per team (737, 738, …).
  Offsets relative to the end of the name:

  | off | size | meaning |
  |---|---|---|
  | +0 | u8 | position: 0 GK, 1 DEF, 2 MID, 3 ATT |
  | +1 | u8 | star flag ("craque", shown as `*` after the name) |
  | +2 | u16 | **strength** (1..50) |
  | +8 | u8 | 1 or 2 (unknown) |
  | +17 | u8 | 0..5 (unknown; maybe behaviour/discipline) |
  | +21 | u16 | 0..10 (probably contract games left — to verify) |
  | +23 | u16 | 1..9 (unknown) |
  | +25 | u16 | **price = 500 × strength** (e.g. 5 → 2500) |
  | +27.. | | zeros to the end; next record's id follows |
- Team record for the managed side starts around `0xCDA0`: pstr name `FIGUEIRENSE F.C.` followed by fixed fields.
- Two human managers are supported (`joga1`, `joga2`).
- The 0-byte `.E98` appears as soon as a name is chosen; content is written on explicit save.
- The "List index out of bounds" error on first new game went away after using a letters-only name
  and selecting the radio button first (not fully diagnosed).

## Snapshot 1 vs snapshot 2 (after 2 league rounds; save made via Elifoot > Sair)
- **Saving:** the game has no separate "save" command. `Elifoot > Sair` saves on exit; `Sair sem gravar` discards.
  (From the `TPrepareGameDlg` main menu: Opções, Sair sem gravar, Sair, Acerca.)
- File grew 83,676 → 83,968 bytes. Header description timestamp updated (`...21:03` → `...21:57`).
- Header counter right after the description: `01` → `03` (round/day counter; to confirm).
- 6 bytes at header offset ~0x29 were zero, now non-zero: a Pascal `Real` (6-byte real48) — likely money. To confirm.
- **Teams are written in standings order**, not fixed order: the managed team's record moved from
  `0xCDA0` to `0x22A1`. A parser must not rely on offsets.
- **Team record** (starts with name pstr, then kit colours, country pstr, level…): after
  `level(05) 00 00` comes a block of u16 counters, zero at start, `01 00 01 00 01 00 02 00 01 00`
  after 2 rounds (played/won/drawn/lost/goals-style stats), then the lineup bytes
  `02 07 06 03 04 08 05 01 08 05 04 03 07 06` (indices of the selected squad).
- **Player tail, changed offsets** (relative to end of name; 48 B):
  - `+2` strength **changed**: Gilberto 5→4, Dionisio 8→7, Nass 6→4, Nilson 7→5, Marquinhos 4→3, Petry 2→1,
    Sandro Ventura 3→1, Vinicius 8→7, Marcelinho 7→6, Nando 8→7, Luciano 2→3.
    Strength moves after a couple of rounds (evolution/form), by more than 1 in some cases.
  - `+25` price (u16) is **not** recomputed for most players (stays 500 × old strength); Luciano's changed
    (1000 → 1750). So price is a separate stored value, updated by other rules.
  - `+8` was 1 or 2 for everyone, now 0 (one-shot "new player" style flag/counter).
  - `+32` 0 → 1 or 2: appearances (games played) this season.
  - `+12` and `+36` became 1 only for Marcelinho (an attacker): goals this season and goals total (hypothesis).
  - `+14` became 1 only for Luciano (a card or injury; to confirm with the in-game Historial).

## Correction to the player record notes
In the save, the tail field at `+23` (u16, values 1..9) is the hidden talent `g`; a player is a star when
`g >= 8` and position >= 2 (matches the single `*` player). The field at `+2` is the strength.

## Player tail fields, final map (verified 15/15 with name-derived formulas)
`+0` position, `+1` star, `+2` strength (u16), `+17` aggression (u8), `+21` injury proneness (u16),
`+23` talent `g` (u16), `+25` price (u16), `+32` appearances. `+8` (1 or 2 -> 0 after a round) is unidentified.
The earlier guess that `+21` was "contract games" is wrong. Aggression, proneness and `g` are not stored
independently of the name: they are recomputed from it by the game (`1010:7516`).
