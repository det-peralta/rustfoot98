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
| Standings, Scorers, Calendar, Winners, AllTimeScorers, Transactions, Acquisitions | TCLASSIFICATIONDLG, TPRINTLISTDLG | `dialogs/tables.rs` |
| Sell, Contract, Search, Observed, Auction, Plantel | main window pages, TCHECKPLAYERDLG, TOBSERVEDPLAYERSDLG, TSELLPLAYERDLG, TPLANTELDLG | `dialogs/market.rs` |
| CoachHistory, CoachManager, CoachRanking, CoachProfile, TeamHistory (TPRINTLISTDLG, TCOACHMANAGERDLG, TCOACHOPTIONSDLG) | `dialogs/coach.rs` |
| Invitation | TINVITECOACHDLG | `dialogs/matchday.rs` (the arm in `dialogs/mod.rs` still names `coach`) |
| Halftime, ForcedSubstitution, Penalty, Shootout, CupDraw, EndOfSeason, RefereeAid | TSUBSTITUTIONDLG, TFORCEDSUBSTITUTIONDLG, TPENALTYDLG, TPENALTIESDLG, TSORTEIODLG, TENDOFSEASONDLG, TREFEREEAIDDLG | `dialogs/matchday.rs` |

`dialogs::open_for_menu` / `open_for_button` map menu entries and main-window buttons to dialogs.

## The information dialogs (`tables.rs`, `coach.rs`)
Most commands of the Campeonato, Equipa, Jogador and Treinador menus open **one generic form**, `TPrintListDlg` (a bevel, the owner-drawn list
`listOfItems` and OK); only the standings (`TClassificationDlg`, four lists on a green form), the coach manager and the coach profile have their own.
`tables::Spec { caption, rows, flags }` is what each command builds (`1068:0634` / `1050:3938`, `TPrintListDlg.Execute`) and `tables::bind_list`
lays the form out like the original: the list is as high as its rows but not above the screen (`Min(600 - form height, rows * 16 + 2 - 225)`), `WIDE`
(flag 8) makes it 200 pixels wider, `SELECT_LAST` (0x10) selects the last row and scrolls to it, `NUMBERED` (2) prefixes `1 2 3`, `AUTOCLOSE` (1)
closes after rows + 3 seconds. Texts it sets: `@caption`, `@client`, `listOfItems.size`, `Bevel1.size`, `OKBtn.pos`, `listOfItems.sel|top`.

**Rows are in the original's field encoding** (`win31/printlist.rs`): `[modifier] width text TAB` per field, as the item classes (`TPlayer`,
`TCalendarEntry`, `TLastWinners`, `TLastTransfer`, `TTeam`, `TCoach`, `TTeamEvent`, `TCoachEvent`) build them for `1068:0ad6`; the modifiers are bold
(`\x02`/`\x04`), italic, right-aligned (`\x06` with 10 pixels of extra space for this and the following fields, `\x07` without), blue (`\xFC`);
`\u{1}fg;bg\u{1}` before a row colours it (club colours, the red and blue of the calendar). The selected row is drawn inverted. The renderer
(`draw.rs` `listbox`) takes lists named `listOfItems`, `ListBoxDivision1..4` and `ListBoxOfCoaches` and draws a scroll bar when the rows do not fit
(approximate look; hits `<list>.up|down|pgup|pgdn` and `<list>#row`).

Other keys added to the renderer: `<name>.size=w,h` resizes any control (`draw.rs` `control`), `TTabbedNotebook` (`<name>.page`, tabs hit as
`<name>#page`, class `TabSetTab`) and `TRadioGroup` (`<name>.index`, items hit as `<name>#item`) in `win31/tabbed.rs`. The check boxes of the profile and the
manager bind to `<name>.checked` / `.enabled` and are drawn by whatever renders `TCheckBox`.

Showing the standings from the game flow: `tables::standings_dialog(auto)` (`auto = true` closes it after ten seconds like the original's `Timer1`).
`classic_shot ... dialog=<menu entry name> play=<rounds>` opens a dialog on a saved game, optionally after playing rounds (`dialog=CampeonatoClassificao`,
`menuCalendrio`, `campeonatoMelhoresmarcadores`, `ltimosvencedores1`, `Melhoresmarcadoresdesempre1`, `ltimastransferncias1`,
`EquipaAquisicoes1`, `Historial1`, `treinadorHistoria`, `TreinadorGestao`, `coachRanking`, `Opesindividuais1`, button `opponentCalendarBtn`).
