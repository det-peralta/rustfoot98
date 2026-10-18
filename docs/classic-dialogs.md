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

## Market dialogs (`dialogs/market.rs`)
* **Pages of the main window** (`Page::Sell`, `Contract`, `Message`; state in `MainState::market`): Jogador > Vender (`Vender1Click`) checks `World::sale_blocker`
  (original sentences from the data segment, `sale_blocker_text`) and shows the reason on the Mensagem page, otherwise the Vender page (`textSellPrice` in
  thousands, empty = any price) whose button runs `World::human_sell` and shows the auction dialog with its result (`Vendido ao X por Y` / `Não houve ofertas`).
  `ChangeSalaryBtn` opens Contrato for a player who is not `fresh` (spin from `50 G` to `50 000 G`); `Propôr` -> `World::human_propose_wage`: a wage above
  80 % of the player's value is accepted, otherwise `TASKMORESALARYDLG` shows his demand (`1.25 * value` in steps of `50 G`): Sim sets wage and `fresh`, Não puts him
  up for auction at any price; when the club may not sell him only OK. A player nobody bid for cannot be offered again until the next game (`World::sell_tried`).
* **Auction** (`TSELLPLAYERDLG`, `auction_dialog`, `queue_auctions`): one dialog per open listing of `World::board` that the user can take part in
  (`listing_open_to_user`), queued by `matchday::end_of_round`; the offer is in thousands; OK -> `World::human_bid`. The form needs `@client` 4 px larger than declared.
* **Search** (`TCHECKPLAYERDLG`, Jogador > Procurar): filters as in `1048:3223` (name, strength range, position, nationality, price range of `50 000 G`... and `Qualquer` = for sale,
  not my team) through `World::search_players` (50 at most, sorted by name); **Equipa...** opens the squad viewer (`TPLANTELDLG`) on the player's club.
  The viewer is also reached from the opponent page (button visible only when `MainState::market.registered`: the original hides it in unregistered copies, feature `0x10`),
  buys with `human_buy` (the player arrives after the round) and toggles the observed state (`World::observe`, saved as `observed|club|player`, 15 at most).
  **Lista** / Jogador > Jogadores sob observação opens `TOBSERVEDPLAYERSDLG` (Remover, Limpar, Equipa...).
* Lists emit rows `a|b|c` with `<name>.cols` / `<name>.align`; combos use `lists[name]`, `<name>.sel`, `<name>.text`, `<name>.open`; hits `Name#i` for rows.
* `classic_shot` options: `open=search|observed|plantel|auction|auction_demo|sell|contract|message`, `page=sell|contract|message`, `registered=1`.
