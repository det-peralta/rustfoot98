# Classic interface: how a dialog is built

The classic interface (`ui/src/classic/`) draws the original's own Delphi forms with the pixel-exact renderer (`ui/src/win31/`, see
`pixel-exact-ui.md`) and binds them to the rules in `core/`. The host (SDL or `web/classic.js` via `ui-web/src/classic_host.rs`) only
forwards events (`Classic::event`, `Classic::tick`) and blits `Classic::fb` after `Classic::frame()`.

## State
`Classic` (`classic/mod.rs`) holds the `World`, the skin, the forms, `screen` (`Main` or the running `Round`), a stack of modal
`dialogs`, a `queue` of dialogs to show later (end of round messages, auctions), `requests` for the host and `dirty` (needs a frame).

## A dialog
`dialog::Dialog { kind, form, texts, lists, focus, bg, hits, st }`:
* `form` is the key in `Forms` (upper-case class name, `TABOUTDLG`...), drawn centred (frame by `BorderStyle`: `bsDialog` thick, else plain).
* `texts` are the run-time overrides of the renderer, keyed by control name: `<name>` caption, `<name>.color`, `.bgcolor`, `.pad`, `.pos=x,y`,
  `.visible=0|1`, `.enabled=0|1`, `.value`, `.progress`, `.focus`, `.sel`, `.page`, `.index`, `.tabs`, `.image=FORM/NAME` (flags are `FLAGS/BRA`),
  `.checked=0|1` (check boxes, radio buttons), `.list` / `lists[name]` (rows `a|b|c`), `@caption`, `@color`, `@client=WxH`, `@dx`, `@dy`; an edit's
  text is `texts[name]`. See `docs/pixel-exact-ui.md` and `win31/draw.rs`.
* `st` is the behaviour's own state (strings by name; `num`/`set_num` for numbers).
* The behaviour is chosen by `Kind` and implemented in `classic/dialogs/<group>.rs` through four functions the dispatchers in
  `dialogs/mod.rs` call: `bind(&Classic, &mut Dialog)` (runs before every frame: put the game's data into `texts`/`lists`), `click(&mut Classic,
  &mut Dialog, name) -> Outcome` (a control was pressed; list rows arrive as `ListName#row`), `key` (keys the generic handling does not use) and
  `tick` (time passed: timers, animations).
* `Outcome`: `Keep`, `Close`, `Open(dialog)` (on top of this one), `Replace(dialog)`.
* Generic handling (`dialog::generic_key`): Enter presses the default button (`Default` or `Kind = bkOK`), Escape the cancel button, Tab moves
  the focus, typing edits the focused `TEdit` (`MaxLength` honoured).
* To show something later (after the current dialogs close): `classic.queue.push_back(dialog)`.
* Game actions call the same `World` methods the other front ends use (`human_buy`, `human_sell`, `human_borrow`, ...); never duplicate rules in the UI.

## Verifying
`cargo run --release -p elifoot-ui --bin classic_shot -- original/saves/snap2.e98 out.bmp 'ev=click:190,90;key:F5;tick:500'` replays events
(`click:x,y move:x,y key:Name tick:ms`, separated by `;`) on a saved game and writes the screen; `win31shot FORM out.bmp Control=text ...` renders a
bare form. `tools/ui/regions.py golden.png out.bmp x y w h` lists the differing regions against a capture of the original
(`analysis/shots/`, local); `tools/capture/classic_web.mjs` drives the browser build in headless Chromium.

## Dialogs and where they live
| Kind | form | file |
|---|---|---|
| About | TABOUTDLG | `dialogs/about.rs` |
| SelectGame, CoachName, Progress | TSELECTGAMEDLG, TCOACHNAMEDLG, TPROGRESSDLG | `dialogs/newgame.rs` |
| Options, Stadium, Loan, Message | TOPTIONSDLG, TSTADIUMDLG, TLOANDLG, message boxes | `dialogs/misc.rs` |
| Standings, Scorers, Calendar, Winners, AllTimeScorers | TCLASSIFICATIONDLG, list dialogs | `dialogs/tables.rs` |
| Sell, Contract, Search, Observed, Transactions, Acquisitions, Auction, Plantel | main window pages, TCHECKPLAYERDLG, TOBSERVEDPLAYERSDLG, TSELLPLAYERDLG, TPLANTELDLG | `dialogs/market.rs` |
| CoachHistory, CoachManager, CoachRanking, CoachProfile, TeamHistory, Invitation | TCOACHMANAGERDLG, TCOACHOPTIONSDLG, TINVITECOACHDLG... | `dialogs/coach.rs` |
| Halftime, ForcedSubstitution, Penalty, Shootout, CupDraw, EndOfSeason, RefereeAid | TSUBSTITUTIONDLG, TFORCEDSUBSTITUTIONDLG, TPENALTYDLG, TPENALTIESDLG, TSORTEIODLG, TENDOFSEASONDLG, TREFEREEAIDDLG | `dialogs/matchday.rs` |

`dialogs::open_for_menu` / `open_for_button` map menu entries and main-window buttons to dialogs. The match day (round view, break, injuries, penalties, cup, end of season) is described in `docs/matchday.md`.

## Widgets and their text keys (`win31/widgets.rs`, `classic/widget_input.rs`)
All keys are in `Dialog::texts` (`<name>` is the control's name in the form; list contents are in `Dialog::lists`). Anything not set keeps
the form's design-time value. The renderer only draws what the texts say; `classic/widget_input.rs` implements the generic mouse and key
behaviour (below) so that a dialog's `click` just reacts to the new state.

| widget | keys |
|---|---|
| any control | `.visible=0\|1`, `.enabled=0\|1` (disabled look: dithered text for check/radio buttons, embossed text for labels, buttons and speed buttons, grey text for edits, combos and lists), `.pos=x,y`, `.size=WxH` (run-time resize), `.color=r,g,b` (font colour) |
| `TCheckBox`, `TRadioButton`, `TSpeedButton` | `.checked=0\|1` (speed button: pressed); caption `texts[name]`; the focus rectangle appears when the control is `focus` |
| `TEdit` | text `texts[name]`; `.bgcolor=r,g,b`; `.caret=N` (caret before character N, drawn when focused); `.selstart=N` and `.sellen=N` (highlighted characters) |
| `TComboBox` | shown text `<name>.text`, else the selected item; items `<name>.items` (separated by `\n`) or `lists[name]` or the form's `Items.Strings`; `<name>.sel` highlighted/selected row; `<name>.open=1` draws the drop-down (up to `DropDownCount` rows, scroll bar beyond); `<name>.top` first visible row; `<name>.cols` / `<name>.align` as for lists, a cell `@FORM/NAME` is a skin bitmap (flags) |
| `TListBox`, `lbOwnerDrawFixed` lists | rows `lists[name]` (a row is `a\|b\|c`), `<name>.sel` selected row (navy, white text; dotted focus rectangle when focused), `<name>.top` first visible row, `<name>.caret` (row of the focus rectangle while nothing is selected, default 0), `<name>.itemheight`, `<name>.bold=1` (all rows bold); a vertical scroll bar appears when the rows do not fit. **Columns**: `<name>.cols=x1,x2,...` are x offsets of the cells from the list's left edge (cell text starts at `x + offset`) and `<name>.align=lrc...` the alignment of each cell (`l` text starts at the offset, `r` ends at it, `c` centred on it; an upper-case letter draws the cell bold); a cell `@FORM/NAME` draws that skin bitmap. **Row styles**: `lists["<name>.style"][i]` = `fg=r,g,b bg=r,g,b bold` (all optional) for row `i`. Without `cols` the whole row is drawn at `x + 3` |
| `TMemo` | lines `lists[name]` or `texts[name]` (split at `\n`); `<name>.top`; a bordered memo is a sunken box with a scroll bar when `ScrollBars` asks for it |
| `TSpinEdit` | `<name>.value` (approximate look: no capture of the original) |
| `TRadioGroup` | `<name>.index` selected item (items: `lists[name]` or the form's `Items.Strings`) |
| `TTabbedNotebook` | `<name>.page` (visible `TTabPage`), `<name>.tabcount=N` (only the first N pages get a tab: the options dialog hides its third, debugging page); the tab strip, page frame and the focus rectangle (when the notebook is `focus`) are drawn; page controls are drawn like the form's own |
| `TColorGrid` | `<name>.index` (foreground colour, `F`), `<name>.bgindex` (`B`); approximate look |
| `TPaintBox` | only the stadium's: `PaintBoxStadium.level=0..24` (stands; approximate, ~430 px off in the level-2 capture) |
| `TGauge` / `TBiGauge` | `.progress` / `.value`, `.barcolor` |

Mouse hits recorded in `Dialog::hits` (`Hit::class`): `TCheckBox`, `TRadioButton`, `TComboBox`, `TSpinEdit`, `TEdit`, `TListBox`,
`TSpeedButton`; `ListRow` (rows `Name#i`, `i` the index in the whole list), `ComboItem` (rows `Name#i` of an open drop-down), `ScrollUp`,
`ScrollDown`, `ScrollThumb`, `ScrollPage` (`Name#up`, `Name#down`, `Name#thumb`, `Name#pgup`, `Name#pgdn`), `NotebookTab` (`Name#i`),
`RadioItem` (`Group#i`), `SpinUp` / `SpinDown` (`Name#up`, `Name#down`), `ColorCell`. A `ScrollInfo` record (`Name#info`, `w` = rows that fit,
`h` = rows) accompanies every scroll bar; it is not a control.

**Generic behaviour** (`widget_input::mouse` / `key`, called by `input.rs` and `dialog::generic_key` before the dialog sees the event):
a press on a check box toggles `.checked`; on a radio button sets `.checked=1` and clears its siblings; on a radio group item sets `.index`;
on a combo box toggles `.open` (closing others); on a combo item sets `.sel` (and `.text` if the dialog keeps one) and closes the list; on a
notebook tab sets `.page`; on a list row sets `.sel`; on scroll arrows / track changes `.top`; on a spin button changes `.value` within
`MinValue`/`MaxValue`. The dialog's `click(name)` is then called with the control's name (`List#row` for list rows) except for a combo's
own press and scroll bars. Keys: Space toggles a check box / selects a radio button, Up/Down move the selection of a list, combo
(closed: changes it and calls `click`; open: moves the highlight, Enter accepts, Escape closes, F4 toggles the list), radio group and spin
edit, Left/Right change the notebook page. Dragging the thumb and the mouse wheel are not handled.

**Frame sizes.** The thick `bsDialog` frame surrounds a client area 4 pixels larger each way than the form's `ClientWidth/Height`
(measured on the options, profile, stadium, progress, calendar and scorer dialogs), so `dialog::draw` uses `draw::form_client`
(`ClientWidth + 4`, `ClientHeight + 4`; the new-game dialog is the one exception). A dialog the program resizes passes `@client=WxH`
in those units (e.g. the scorers list: `@client=378x286`). `form_frame` draws the system menu box (the dash box at the left of the
caption) unless the form's `BorderIcons` leave it out (the list dialogs of the game have it, the options and profile dialogs do not).

