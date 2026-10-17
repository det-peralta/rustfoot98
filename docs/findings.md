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
