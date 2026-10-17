# Analysis plan

- [x] Identify target: 16-bit Windows NE, Delphi 1 (TGauge/TGaugeKind classes, VBX gauges)
- [x] Install Ghidra 12.1.2, `rea doctor --provider ghidra` healthy (REA's `analyze` rejects NE, so Ghidra headless is used directly)
- [x] Load `ELIFOOT.EXE` with Ghidra NE loader (x86:LE:16:Protected Mode), auto-analysis done, 3007 functions
- [ ] Recover Delphi RTTI/class names and map event handlers to functions
- [ ] Extract form resources (TForm layouts, strings) → UI spec
- [x] File formats: `.EFT` team, `.TXE` (done, see findings)
- [ ] File formats: `.e98` save, `DRLIST.ELI`
- [ ] Locate RNG and match simulation; document algorithm
- [ ] Finances, transfers, injuries, calendar/competition generation, AI
- [ ] Registration check: understand shareware limits (no keygen use)
- [ ] Implement `core/` in TS with golden tests from the original
- [ ] Build UI, PWA, saves import/export compatible with `.e98`
