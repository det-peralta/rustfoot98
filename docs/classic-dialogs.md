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
| SelectGame, CoachName, Progress | TSELECTGAMEDLG, TGETTEAMSDLG (`CoachName`: the players' names and the draw of the club), TPROGRESSDLG | `dialogs/newgame.rs` |
| Options, Stadium, Loan, Message | TOPTIONSDLG, TSTADIUMDLG, TLOANDLG, message boxes | `dialogs/misc.rs` |
| Standings, Scorers, Calendar, Winners, AllTimeScorers | TCLASSIFICATIONDLG, list dialogs | `dialogs/tables.rs` |
| Sell, Contract, Search, Observed, Transactions, Acquisitions, Auction, Plantel | main window pages, TCHECKPLAYERDLG, TOBSERVEDPLAYERSDLG, TSELLPLAYERDLG, TPLANTELDLG | `dialogs/market.rs` |
| CoachHistory, CoachManager, CoachRanking, CoachProfile, TeamHistory, Invitation | TCOACHMANAGERDLG, TCOACHOPTIONSDLG, TINVITECOACHDLG... | `dialogs/coach.rs` |
| Halftime, ForcedSubstitution, Penalty, Shootout, CupDraw, EndOfSeason, RefereeAid | TSUBSTITUTIONDLG, TFORCEDSUBSTITUTIONDLG, TPENALTYDLG, TPENALTIESDLG, TSORTEIODLG, TENDOFSEASONDLG, TREFEREEAIDDLG | `dialogs/matchday.rs` |

`dialogs::open_for_menu` / `open_for_button` map menu entries and main-window buttons to dialogs.

## Start, save and exit (`Classic::start`, `dialogs/newgame.rs`, `options.rs`)
`Classic::start(skin, forms, data, teams, seed)` begins with no game: the desktop, the About box (OK leads on), `TSelectGameDlg`, a `TProgressDlg` while
the world is built (`World::from_teams`, which already applies the start rule `pick_human`) or while the host answers `Request::Load(name)`, then for a
new game `TGetTeamsDlg` ("Sortear equipas" shows the club the rule gave, OK enters the main window). Cancel in the choice quits.
Host API: `cl_start`, `Classic::load_text(text)` (answer to `Request::Load`), `Classic::save_text()` (answer to `Request::Save`), `Classic.clock`
(`dd/mm/yy hh:mm`, for the description of saves) and `Data::set_saves("NAME\tfirst line of the text\n...")`. The first line of a saved text is a header
(`#elifoot-classic|coach=..|stamp=..|opts=..`) holding the coach and the options; texts without it load with defaults.
The game saves after a round according to the option "Gravar o jogo" (`Options::saves_after`), on "Sair" (and then quits) but not on "Sair sem gravar".
`Classic.opts` (`Options`) is read by the other dialogs: `show_lashes(human)`, `show_cup_draw(humans_in_cup)`, `view_computer_penalties`; the match speed and the
half-time substitutions are already applied in `input.rs`. Options texts: combos use `<name>.items/.sel/.text/.open`, the notebook `TabbedNotebook1.tabs` and `.page`.
`dialogs::paint_under` lets a dialog paint under its controls (the stadium's `TPaintBox`).
