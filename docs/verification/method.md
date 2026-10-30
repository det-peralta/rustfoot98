# How screens are verified

The port is checked against the original program running under emulation, screen by screen and pixel by pixel. The tools are in
`tools/screens/`; the pictures they make stay local (they are drawn with the game's fonts and bitmaps).

## The original under emulation

`tools/capture/boot.sh` starts the original Elifoot 98 under Windows 3.1 in DOSBox (js-dos, driven by headless Chromium through
`tools/capture/driver.mjs`). Commands go in as JSON lines: keys, mouse moves and clicks at guest pixels
(`tools/capture/mouse.py` finds the pointer in the frame and corrects until it is on target), and `guest` writes the emulated
screen to a PNG. The captures are 6 bits per colour channel, so both sides are compared on their top 5 bits.

## The manifest

`tools/screens/manifest.py` lists every screen once, with what is needed on both sides:

- **the original's inputs** from the loaded saved game `ABCDEEEE` (the main window of the coach *joga1*, FIGUEIRENSE): menu keys
  (`Alt`, arrows, `Enter`), clicks, waits, and how to close the screen again without changing anything;
- **or a recorded frame** (`capture`): the start of a new game, the team editor, and the match days, recorded earlier;
- **the port's arguments**: `classic_shot` renders the game from the same save (`original/saves/snap2.e98` is the file the
  emulator loads as `ABCDEEEE`), with a script of events (`ev=click:x,y;key:Enter;tick:ms;season:N:Kind`); `editor_shot` renders
  the team editor with its own script (`do=Command;@x,y;!Button;#key;%x,y`);
- whether the comparison is **exact** (the same state on both sides) or a layout comparison: the original seeds its random
  numbers at start-up, so a match day or a new game's draw cannot be reproduced, and animated screens (the About box's banner)
  depend on the moment of the capture.

## Running it

```bash
bash tools/capture/boot.sh                                   # the original, at the choice of game
for k in Tab Tab Space Tab a Enter; do bash tools/capture/c.sh "{\"op\":\"key\",\"key\":\"$k\"}"; done
analysis/captures/venv/bin/python tools/screens/capture_original.py      # the scripted screens
cargo build --release -p rustfoot-ui --bin classic_shot --bin editor_shot
analysis/captures/venv/bin/python tools/screens/compare.py --publish     # renders, diffs, report and tables
```

`capture_original.py` checks after every screen that the main window is back exactly as at the start, and stops otherwise:
pressing keys blindly in a dialog could change the game (an `Enter` in the stadium dialog builds stands).

`compare.py` writes `analysis/screens/{original,port,diff}/<id>.png`, a side-by-side report `analysis/screens/index.html`,
the table [Screen by screen](screens.md) and the [gallery](gallery.md). The mouse pointer is part of the original's picture
only, so its box is left out of the count. `tools/screens/where.py <id>` lists where a screen still differs.

## What the remaining pixels are

Most screens differ in a few dozen to a few hundred pixels out of 480 000. Almost all of them are single pixels of TrueType
glyphs: the port rasterises the Windows 3.1 TrueType fonts with FreeType's v35 interpreter and its own fill rule
(`tools/ui/ttraster.py`), which agrees with the Windows 3.1 rasteriser on nearly every pixel but not all. Each screen's
notes say what else is known to differ.
