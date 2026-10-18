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

`dialogs::open_for_menu` / `open_for_button` map menu entries and main-window buttons to dialogs.

## Widgets and their text keys (`win31/draw.rs`)
All keys are in `Dialog::texts` (`<name>` is the control's name in the form). Anything not set keeps the form's design-time value.

| widget | keys |
|---|---|
| any control | `.visible=0\|1`, `.enabled=0\|1` (disabled look: dithered or embossed text, greyed glyphs), `.pos=x,y`, `.color=r,g,b` (font), `.bgcolor=r,g,b` |
| `TCheckBox`, `TRadioButton`, `TSpeedButton` | `.checked=0\|1` (speed button: pressed down); caption `texts[name]` |
| `TEdit` | text `texts[name]`; `.caret=N` (caret before character N, drawn when the control has the focus); `.selstart=N` and `.sellen=N` (highlighted range) |
| `TComboBox` | edit text `<name>.text`; `<name>.items` (items separated by `\n`, or `lists[name]`); `<name>.sel` (highlighted row); `<name>.open=1` shows the drop-down list below the control; `<name>.top` first visible row of an open list |
| `TListBox` | rows `lists[name]` (a row is `a\|b\|c` for several columns), `<name>.sel` (selected row), `<name>.top` (first visible row); a vertical scroll bar appears when the rows exceed the box; `<name>.cols=x1,x2,...` x offsets of the columns inside the box and `<name>.align=lrr...` per column (`l`, `r` or `c`); `<name>.rowcolors` and `<name>.colors` are not used: use `<name>.color` |
| `TSpinEdit` | `<name>.value` |
| `TRadioGroup` | `<name>.index` selected item (items are the form's `Items.Strings`) |
| `TTabbedNotebook` | `<name>.page` (index of the visible `TTabPage`); the tab strip is drawn from the pages' captions |
| `TColorGrid` | `<name>.index` selected colour |
| `TPaintBox` | drawn by the dialog (`Dialog` supplies the content through `lists`/`texts`, see the stadium) |
| `TGauge` | `.progress`; `TBiGauge`: `.value`, `.barcolor` |

Hits recorded for the mouse (`Dialog::hits`, `Hit::class`): `TCheckBox`, `TRadioButton`, `TComboBox`, `TSpinEdit`, `TEdit`, `TListBox`;
`ListRow` (rows named `Name#i`, `i` the index in the whole list), `ComboItem` (rows `Name#i` of an open drop-down), `ScrollUp`, `ScrollDown`,
`ScrollThumb` (named `Name#up`, `Name#down`, `Name#thumb`) and `ScrollPage` (`Name#pgup`, `Name#pgdn`).
