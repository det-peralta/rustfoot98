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
