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
copies, and the renderer (`ui/src/win31/`, `cargo run -p rustfoot-ui --release --bin win31shot -- TABOUTDLG out.bmp`) reads it at run time.

## The reference: frames of the real thing
`tools/capture/driver.mjs` drives the reference web build in headless Chromium; its `guest` command saves the **emulator's own 800x600 frame**
(`ci.screenshot()`; the page screenshot is resampled and useless for pixel work). Display colours are 5-6-5 without replication (white is
248,252,248; yellow 248,252,0), which `Fb` reproduces. `tools/ui/cmp.py` and `tools/ui/regions.py` compare a render with a capture and list the
differing regions.

### Driving the original locally (no network, our own saves)
`analysis/jsdos/` (git-ignored) holds the js-dos v7 player (downloaded from the reference site) and `game.jsdos`, a bundle built from
`original/bundle` plus our saves. `tools/capture/boot.sh` starts a headless Chromium on it with `tools/capture/driver.mjs`
(`ELI_URL=http://127.0.0.1:8123/index.html`), clears the player's mouse-lock notice, waits for Windows 3.1 and the game, and presses OK on the About box;
then `bash tools/capture/c.sh '{"op":"key","key":"Tab"}'` and `{"op":"guest","path":"analysis/shots/x.png"}` drive and capture it. Mouse input is relative
(pointer lock), so use the keyboard (Tab, Space, Enter; a button shows its focus rectangle only when reached by keyboard; the game quits if Enter
hits Cancel). The saved game `ABCDEEEE` (a 2-round Brazil/Belgium league) lets "Quero continuar um jogo" reach the main screen.

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

## Widgets measured on captures (options, profile, stadium, calendar, scorers, ranking dialogs)
* **Sunken box** (edit, list, combo): gray line over the top and left (`x-1..x+w`, `y-1..y+h`), black frame, white line under and right of the black frame (starting at `x` / `y`).
* **Combo box**: the frame is a pixel shorter than the form's `Height` (the closed box is `Height - 1`), a black divider 17 pixels from the right, a 15-pixel arrow button (white top/left, 2 grey lines bottom/right) with a down arrow and bar glyph; focused (`csDropDownList`): navy highlight inset by one pixel, white text, a *solid* inverted rectangle; open: the list continues the frame, rows `ItemHeight` high, the selected row navy.
* **Scroll bar** (list box, Windows 3.1): 16 wide inside the list's black border, black line left of the bar; the up and down buttons are 15x15 button faces with arrow glyphs, separated from the track by black lines; the thumb is a fixed 17x17 box (black outline, button face) at `first + (len - 2) * top / range` (floor; the end of the range maps to the last position); the track is silver.
* **Check box**: 13x13 black frame with silver inside, a one-pixel cross when checked, caption at +18; focus rectangle around the caption (text rectangle from `-2` to `+tw+2`, 19 pixels tall from the line above the box), clipped to the control. **Radio button**: ring and a 7-pixel disc; radio group buttons start at `tmHeight + 2` below the group top and are `(Height - tmHeight - 5) / items` high.
* **Tabbed notebook**: tabs of `(Width - 4) / TabsPerRow` (3 by default) pixels, the page frame black / white-grey miter bevel 3 pixels outside the page; selected tab bold, 1 pixel higher bevel and open to the page; its focus rectangle is XORed with the tab text colour.
* **Dotted focus rectangle over other colours**: lit pixels invert what is below them (navy -> (248,252,120)).
* **Buttons**: the caption's odd pixel goes left (MS Sans Serif: right); thin-framed buttons raise captions of regular raster fonts by one pixel; the default button is the `Default`/`bkOK` button unless another button has the focus.
* **Stadium picture** (`PaintBoxStadiumPaint`, `1030:2cee`): up to three rings of stands (grey, 2-pixel light yellow outline, `Pie` wedges around the top covering `n/6`, `n/8`, `n/10` of the circle for levels 1..6, 7..14, 15..24; a full ring is an `Ellipse`) drawn first, then the pitch ellipse (green, same outline) with white lines. Ring and pitch geometry is fitted to a level-2 capture.

## Measured against the original's own dialogs (information dialogs, October 2026)
Captures of the original (the saved game ABCDEEEE = `original/saves/snap2.e98`) opened through its menus and compared with `classic_shot` renders
(`tools/ui/regions.py`): standings, scorers, all-time scorers, calendar, transfers, coach history, ranking, options (both tabs), coach profile and
stadium now differ only by the mouse cursor and the state that cannot be reproduced (the second human's bold row; the referees are the game's since 2026-10-29).
* **Windows below a dialog** have the inactive caption: silver bar (and silver thick frame) with black lettering (`win31::draw::INACTIVE`).
* **Thick frame sizes**: a `bsDialog` form is drawn `ClientWidth + 4` x `ClientHeight + 4` (the frame hides four pixels); a form the program resizes
  itself passes `@client` in real units: the standings window is 592x375 (the DFM size, no +4), the list dialogs' `@client` is the computed size + 4
  and the form is `client + 29` high when it is stretched to the 600-pixel screen (`tables::layout`).
* **Flat list boxes** (`Ctl3D = False`, the four division lists of the standings): a plain black outline over the rows' edges, no focus rectangle.
* **Club colours in tables**: row background = the club's second colour, lettering = the first (as in the round view).
* **Ties** in the standings keep the order of the last sort (`World::tie_order`, loaded from the save's own division lists); scorers equal in goals
  and strength come out in the reverse of the master-list order.
* **`TBitBtn` with the default `Layout = blGlyphLeft`** (the OK button of the list dialogs): glyph and caption side by side, `CalcButtonLayout` inside
  the client rectangle `(x + 2, y + 2, w - 5, h - 5)` (`bitbtn_glyph_left`); the other buttons of the game have `blGlyphTop`.
* **Scroll bars** of every list use `widgets::vscroll` (solid silver track, arrow glyphs with a stem, black separator lines); the list dialogs had a
  dithered copy until measured.
* **Multi-column list boxes** (`Columns`): rows run down a column then on to the next, the selection bar is one column wide, no scroll bar.
* **Notebook focus**: the tab strip is a tab stop (Tab follows the `TabOrder` of the form, among the children of each container) and the arrow
  keys change the page; the options dialog has two tabs (the *Controle* page is for registered copies).
* **New-game dialog**: no radio button is chosen until the user clicks a radio, the file-name field or a saved game; radio buttons are tab stops
  only while chosen (so Tab goes edit -> list -> Cancel); typing a letter in the list selects the next game starting with it.
* **Squad list marks** (the `+` / `-` of starters and substitutes at the left of the rows) appear only after the user chooses a formation for the
  next round (F-keys or the *Seleccionar* menu) and disappear again when the round is played.

## Frames captured so far (analysis/shots, local)
`guest1` About (done), `guest3` new-game dialog (done), `guest11/13` the same with a saved game listed and selected, `guest14` the "A carregar o jogo..."
gauge dialog (`TProgressDlg`, `TGauge`: filled part black, rest white, percentage text inverted), `guest15` the **main screen** (`TPrepareGameDlg`, 632x434, black
form, menu bar, owner-drawn player list with positions/strengths/wages, flag image, notebook with tabs Jogo/Jogador/Finanças/Selecção/Adversário, `TBiGauge` morale bar).

## Plan
1. Done: About dialog. 2. Widgets still to measure: list boxes, combo boxes, edits, check/radio buttons, group boxes, bevels, panels, notebooks,
gauges, scroll bars, menus (capture each dialog of the original with `driver.mjs` and iterate with `regions.py`).
3. Bind forms to the game: each form is data; its controls get their text and state from `World` by control name, events call the same
`World` methods as the shared UI. 4. Host the renderer in the SDL and web front ends (blit the 800x600 buffer) and load the skin pack from the user's
own files (SDL: a directory; web: a file the user picks, kept in IndexedDB).
