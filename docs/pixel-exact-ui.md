# Pixel-exact UI: how the original's look is rebuilt

Goal: the screens of the port look *identical* to the original running on Windows 3.1 (800x600, 16-bit colour). Everything the original
drew comes from **data we can read**, so it can be reproduced by a renderer, not copied:

| ingredient | where it lives | how we get it |
|---|---|---|
| window and control layouts, captions, fonts, colours | the 49 Delphi forms (RCDATA, `TPF0`) in `ELIFOOT.EXE` | `tools/ne_resources.py` + `tools/dfm.py` -> `analysis/out/forms/*.json` |
| BitBtn glyphs | `Glyph.Data` inside the forms, and `BITMAP` resources `BBOK`, `BBCANCEL`, ... of the exe | `tools/ui/skin.py` -> `glyphs.bin` |
| TrueType fonts (Arial, Times New Roman) | `WINDOWS\SYSTEM\*.TTF` of the Windows 3.1 install in the js-dos image | `tools/ui/fat16.py` (reads the FAT16 disk image) |
| raster fonts (System, MS Sans Serif, MS Serif) | `*.FON` on the Windows 3.1 disks (SZDD-compressed NE files) | `tools/ui/fon.py` |
| widget drawing (buttons, frames, focus rectangle) | Windows 3.1 USER/Delphi VCL code | measured on captures and reimplemented in `ui/src/win31/draw.rs` |

**None of these files may be committed** (they are the game's and Microsoft's): `tools/ui/skin.py` writes `analysis/out/skin/` from the user's own
copies, and the renderer (`ui/src/win31/`, `cargo run -p elifoot-ui --release --bin win31shot -- TABOUTDLG out.bmp`) reads it at run time.

## The reference: frames of the real thing
`tools/capture/driver.mjs` drives the reference web build in headless Chromium; its `guest` command saves the **emulator's own 800x600 frame**
(`ci.screenshot()`; the page screenshot is resampled and useless for pixel work). Display colours are 5-6-5 without replication (white is
248,252,248; yellow 248,252,0), which `Fb` reproduces. `tools/ui/cmp.py` and `tools/ui/regions.py` compare a render with a capture and list the
differing regions.

## What was learned (About dialog: 220 420 pixels, 229 differ)
* **Frame**: `bsSingle` form = 1-pixel black border, an 18-pixel navy caption bar, a 1-pixel black rule, then the client area; centred on the
  screen with `x = (800 - width) / 2`. Caption text is the *System* raster font, not bold.
* **TrueType text**: FreeType's hinter (the bytecode interpreter, mono target) on the *original* Arial/Times files, then **our own fill** of the hinted
  outline by pixel-centre sampling (non-zero winding, no dropout control) reproduces GDI. `tools/ui/ttraster.py`. The GDI ascent is FreeType's
  hinted ascender **plus one**. Underline: top at `round(-underlinePosition * ppem / upem)` below the baseline, thickness `round(underlineThickness * ppem / upem)`
  (`post` table). Remaining difference: about 0.5 % of glyph pixels (single pixels on curves).
* **Font mapping quirk**: a request for Arial at 9 pixels is not drawn with TrueType but with the raster font **MS Serif 7 pt** (`SMALLE.FON`);
  bold of a raster font is GDI's smear (one pixel to the right). Sizes 11, 13, 16, 19, 48 are TrueType.
* **Buttons** (`TBitBtn`): the default/focused button has a 2-pixel black frame, the others 1 pixel; cut corners in the face colour; white
  2-pixel bevel top/left, grey bottom/right with mitred corners; glyph centred, `Margin` below the 2-pixel bevel; caption's cell bottom
  `Margin + 2` above the bottom edge; thin-framed buttons shift the caption one pixel right and up; `DrawFocusRect` dots where `(x + y)` is even,
  corners omitted, around the caption of the focused button.
* **Banner** (`TBanner`): the scrolling text clipped to its rectangle, drawn at `x + width - scroll` with the parent's font (scroll position is run-time state).
* **Not reproduced**: the mouse cursor in captures (126 pixels in the About capture); the exact scroll position of the banner at any moment (state).

## Plan
1. Done: About dialog. 2. Widgets still to measure: list boxes, combo boxes, edits, check/radio buttons, group boxes, bevels, panels, notebooks,
gauges, scroll bars, menus (capture each dialog of the original with `driver.mjs` and iterate with `regions.py`).
3. Bind forms to the game: each form is data; its controls get their text and state from `World` by control name, events call the same
`World` methods as the shared UI. 4. Host the renderer in the SDL and web front ends (blit the 800x600 buffer) and load the skin pack from the user's
own files (SDL: a directory; web: a file the user picks, kept in IndexedDB).
