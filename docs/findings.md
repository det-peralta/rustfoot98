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
