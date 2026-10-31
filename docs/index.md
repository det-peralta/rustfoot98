# rustfoot98

**Elifoot 98**, the Portuguese football manager by André Elias and Ricardo Correia (1998, Windows 3.1, Delphi 1), reverse-engineered
from its binary and rebuilt in Rust. The rebuild runs in a browser and looks and plays like the original: the same 800x600 screens,
fonts, dialogs and rules, recovered from the program itself rather than imitated.

[Play in the browser](play/classic.html){ .md-button .md-button--primary }
[How the game works](game/index.md){ .md-button }
[Screen-by-screen verification](verification/screens.md){ .md-button }

!!! note "The game's data"
    The game page plays at once: the site carries the original's team files, forms and bitmaps and the Windows 3.1 fonts as
    pre-drawn glyphs, as Elifoot 98's free distribution allows (see [NOTICE](https://github.com/det-peralta/rustfoot98/blob/main/NOTICE.md)).
    The source repository holds none of them; a local build makes them from your own copy (see [Method and tools](installation.md)).

## What is here

| Section | What it holds |
|---|---|
| [How the game works](game/index.md) | The rules of Elifoot 98 explained from the decompiled program: the match engine minute by minute, seasons and the cup, the economy, the transfer market, coaches and referees, the save files. |
| [Verification](verification/screens.md) | 77 screens of the original captured under an emulator and compared with the port's picture of the same game state, 63 of them pixel by pixel; the [gallery](verification/gallery.md) shows them side by side and [the method](verification/method.md) explains how. |
| [Reverse engineering](installation.md) | How the binary was taken apart (Ghidra on the 16-bit NE executable, the Delphi class and form tables) and the raw findings: function map, file formats, notes per subsystem. |
| [The port](architecture.md) | How the Rust code is organised: the rules core, the Windows 3.1 renderer, the game's windows as a state machine, the WebAssembly host. |

## How it was done

1. **Decompile.** Ghidra loads `ELIFOOT.EXE` (16-bit Windows NE). The Delphi 1 runtime type information gives the class table
   (195 classes) and the 49 forms with every control's position, font and caption. The game logic was then read function by
   function; the [decompilation map](decompilation-map.md) lists what each of the ~3000 functions does.
2. **Measure.** The original runs in DOSBox (js-dos) under Windows 3.1. Scripts drive it with the keyboard and mouse, capture
   the screen and read its save files, so every rule is checked against what the original does: money day by day, the
   referees of 288 games, auctions, the clock of a match day.
3. **Rebuild.** `core/` holds the rules in plain Rust (formats, random numbers, match engine, season, finances, market).
   `ui/` draws the interface the way Windows 3.1 and the Delphi VCL draw it, from the forms and fonts of your own copy, so a
   window of the port is the same pixels as the original's.
4. **Verify.** Each screen of the original is captured and compared with the port's render of the same state; the remaining
   differences are listed per screen.
