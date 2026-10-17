# Installation

Tested on Arch Linux (Omarchy). Adjust package commands for other distros.

## 1. Prerequisites
- Node.js 18+ (`node -v`; developed with 26.x)
- `ps` from procps (REA uses it for process-ownership checks on Linux)
- Ghidra (brings a JDK)

```bash
sudo pacman -S --needed ghidra
```

## 2. REA
Official sources only: <https://github.com/morluto/rea>, <https://rea.tools>, npm `rea-agents`.

```bash
npx rea-agents setup          # interactive; review each proposed change
# or: npm install --global rea-agents && rea --help
rea doctor --provider ghidra --json
```
`setup` edits your agent's MCP config (with backups). Restart the agent afterwards.

## 3. Original game files (not in this repo)
```bash
mkdir -p original && cd original
curl -LO https://marmota.org/elifoot98/elifoot98.zip
unzip elifoot98.zip -d elifoot98
```
Contents: `ELIFOOT.EXE`, `EDITEQ.EXE` (16-bit Windows NE), `BIVBX11.DLL`, `GAUGE.VBX`,
`EQUIPAS/*.EFT` (teams), `FLAGS/*.BMP`, `*.TXE` (text/config), `JOGOS/` (saves `.e98`).

Reference build for differential testing: <https://github.com/elifoot98web/elifoot98web>
(js-dos bundle `src/assets/elifoot/elifoot98.jsdos`, Git LFS).

## 4. Out of scope
The registration key generator (`crack.exe`) offered on some mirrors is **not** used.

## 5. Running the analysis
REA 6.5.0 `analyze` rejects 16-bit NE executables (`unsupported NE executable in MZ image`),
so use Ghidra headless directly (its NE loader works):

```bash
/opt/ghidra/support/analyzeHeadless analysis/ghidra-proj elifoot \
  -import original/elifoot98/ELIFOOT.EXE -overwrite
/opt/ghidra/support/analyzeHeadless analysis/ghidra-proj elifoot \
  -process ELIFOOT.EXE -noanalysis -scriptPath tools/ghidra \
  -postScript ExportSummary.java analysis/out
```
Outputs (git-ignored): `analysis/out/{memory,functions,strings,symbols}.txt`.
