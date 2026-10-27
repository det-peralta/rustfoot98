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
`cargo run --release -p rustfoot-ui --bin classic_shot -- original/saves/snap2.e98 out.bmp 'ev=click:190,90;key:F5;tick:500'` replays events
(`click:x,y move:x,y key:Name tick:ms`, separated by `;`) on a saved game and writes the screen; `win31shot FORM out.bmp Control=text ...` renders a
bare form. `tools/ui/regions.py golden.png out.bmp x y w h` lists the differing regions against a capture of the original
(`analysis/shots/`, local); `tools/capture/classic_web.mjs` drives the browser build in headless Chromium.

## Dialogs and where they live
| Kind | form | file |
|---|---|---|
| About | TABOUTDLG | `dialogs/about.rs` |
| SelectGame, CoachName, Progress | TSELECTGAMEDLG, TGETTEAMSDLG (`CoachName`: the players' names and the draw of the club), TPROGRESSDLG | `dialogs/newgame.rs` |
| Options, Stadium, Loan, Message, Balance sheet | TOPTIONSDLG, TSTADIUMDLG, TLOANDLG, message boxes, TBALANCEDLG | `dialogs/misc.rs` |
| Standings, Scorers, Calendar, Winners, AllTimeScorers, Transactions, Acquisitions | TCLASSIFICATIONDLG, TPRINTLISTDLG | `dialogs/tables.rs` |
| Sell, Contract, Search, Observed, Auction, Plantel | main window pages, TCHECKPLAYERDLG, TOBSERVEDPLAYERSDLG, TSELLPLAYERDLG, TPLANTELDLG | `dialogs/market.rs` |
| CoachHistory, CoachManager, CoachRanking, CoachProfile, TeamHistory (TPRINTLISTDLG, TCOACHMANAGERDLG, TCOACHOPTIONSDLG) | `dialogs/coach.rs` |
| Halftime, ForcedSubstitution, Penalty, Shootout, CupDraw, EndOfSeason, RefereeAid, Invitation, Lashes | TSUBSTITUTIONDLG, TFORCEDSUBSTITUTIONDLG, TPENALTYDLG, TPENALTIESDLG, TSORTEIODLG, TENDOFSEASONDLG, TREFEREEAIDDLG | `dialogs/matchday.rs` |

**Main window on a cup day**: `main_win::next_opponent` returns the club of the human's cup tie while `World::cup_day_due()` (division 4 = the
outside pool; use `World::club_of`), and `main_win::view` writes `textOpponent` as ` NAME (nª Divisão|Distrital)` / `CASA|FORA TAÇA - <stage>` and an
empty `ListBoxClassification`. The round button plays the cup day (`Classic::play_round` dispatches on `cup_day_due`).

`dialogs::open_for_menu` / `open_for_button` map menu entries and main-window buttons to dialogs. The match day (round view, break, injuries, penalties, cup, end of season) is described in `docs/matchday.md`.

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

## Start, save and exit (`Classic::start`, `dialogs/newgame.rs`, `options.rs`)
`Classic::start(skin, forms, data, teams, seed)` begins with no game: the desktop, the About box (OK leads on), `TSelectGameDlg`; a new game then
asks for the countries (`TSelectCountriesDlg`: the pack's countries sorted as Windows 3.1 sorts them, accented initials first; a click chooses or
leaves out a country; *Todas*; OK from 34 teams; above 128 the yellow warning; Cancel quits) and the currency (`TCurrencyDlg`, *Dinheiro*), and the
world is built under `TProgressDlg` *A iniciar o jogo...* (`World::from_countries`: the cut to 128 of `1028:19ac`, teams by level of `1028:002a`;
`World::from_teams` when the pack is empty); a saved game shows *A carregar o jogo...* while the host answers `Request::Load(name)`. Then for a
new game `TGetTeamsDlg` ("Sortear equipas" shows the club the rule gave, OK enters the main window). Cancel in the choice quits.
Host API: `cl_start`, `Classic::load_text(text)` (answer to `Request::Load`), `Classic::save_text()` (answer to `Request::Save`), `Classic.clock`
(`dd/mm/yy hh:mm`, for the description of saves) and `Data::set_saves("NAME\tfirst line of the text\n...")`. The first line of a saved text is a header
(`#elifoot-classic|coach=..|stamp=..|opts=..`) holding the coach and the options; texts without it load with defaults.
The game saves when a day starts (after *Jogar*, with the `A gravar o jogo...` gauge) according to the option "Gravar o jogo"
(`Options::saves_after`, `Classic::play_round`), on "Sair" (and then quits) but not on "Sair sem gravar".
`Classic.opts` (`Options`) is read by the other dialogs: `show_lashes(human)`, `show_cup_draw(humans_in_cup)`, `view_computer_penalties`; the match speed and the
half-time substitutions are already applied in `input.rs`. Options texts: combos use `<name>.items/.sel/.text/.open`, the notebook `TabbedNotebook1.tabs` and `.page`.
`dialogs::paint_under` lets a dialog paint under its controls (the stadium's `TPaintBox`).

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

## Widgets and their text keys (`win31/widgets.rs`, `classic/widget_input.rs`)
All keys are in `Dialog::texts` (`<name>` is the control's name in the form; list contents are in `Dialog::lists`). Anything not set keeps
the form's design-time value. The renderer only draws what the texts say; `classic/widget_input.rs` implements the generic mouse and key
behaviour (below) so that a dialog's `click` just reacts to the new state.

| widget | keys |
|---|---|
| `TLabel` | caption `texts[name]`; `.autosize=1` (the run-time caption sizes the label and its click area); `.wrap=1` (word wrap the form does not ask for: the team editor's long labels); `.bgcolor`, `.pad` |
| any control | `.visible=0\|1`, `.enabled=0\|1` (disabled look: dithered text for check/radio buttons, embossed text for labels, buttons and speed buttons, grey text for edits, combos and lists), `.pos=x,y`, `.size=WxH` (run-time resize), `.color=r,g,b` (font colour) |
| `TCheckBox`, `TRadioButton`, `TSpeedButton` | `.checked=0\|1` (speed button: pressed); caption `texts[name]`; the focus rectangle appears when the control is `focus` |
| `TEdit` | text `texts[name]`; `.bgcolor=r,g,b`; `.caret=N` (caret before character N, drawn when focused); `.selstart=N` and `.sellen=N` (highlighted characters: typing or Backspace replaces them); `.indent=N` (text from the left edge, default 2; 3 in the team editor's edits, whose highlight starts at the text and is a pixel wider and taller). Typing follows the form's `CharCase` |
| `TComboBox` | shown text `<name>.text`, else the selected item; items `<name>.items` (separated by `\n`) or `lists[name]` or the form's `Items.Strings`; `<name>.sel` highlighted/selected row; `<name>.open=1` draws the drop-down (up to `DropDownCount` rows, scroll bar beyond); `<name>.top` first visible row; `<name>.cols` / `<name>.align` as for lists, a cell `@FORM/NAME` is a skin bitmap (flags); `<name>.draw=flag` is the team editor's owner-drawn country combo (`DrawFlagAndCountry`: rows `@FLAGS/CODE\|name`, white, the flag a pixel down, the name in Arial 16 at 44,5; the field is the item height + 6, with a navy frame and the focus rectangle while focused; the list shows `DropDownCount` font lines' worth of rows) |
| `TListBox`, `lbOwnerDrawFixed` lists | rows `lists[name]` (a row is `a\|b\|c`), `<name>.sel` selected row (navy, white text; dotted focus rectangle when focused), `<name>.top` first visible row, `<name>.caret` (row of the focus rectangle while nothing is selected, default 0), `<name>.itemheight`, `<name>.bold=1` (all rows bold); a vertical scroll bar appears when the rows do not fit. **Columns**: `<name>.cols=x1,x2,...` are x offsets of the cells from the list's left edge (cell text starts at `x + offset`) and `<name>.align=lrc...` the alignment of each cell (`l` text starts at the offset, `r` ends at it, `c` centred on it; an upper-case letter draws the cell bold); a cell `@FORM/NAME` draws that skin bitmap. **Row styles**: `lists["<name>.style"][i]` = `fg=r,g,b bg=r,g,b bold` (all optional) for row `i`. Without `cols` the whole row is drawn at `x + 3` |
| `TMemo` | lines `lists[name]` or `texts[name]` (split at `\n`); `<name>.top`; a bordered memo is a sunken box with a scroll bar when `ScrollBars` asks for it |
| `TSpinEdit` | `<name>.value`; `<name>.height` (the height after `AutoSize`); `<name>.style=editor` the team editor's look (measured: black rule, 13-pixel buttons, the value selected 4 pixels in). The default look is approximate (no capture) |
| `TRadioGroup` | `<name>.index` selected item (items: `lists[name]` or the form's `Items.Strings`) |
| `TTabbedNotebook` | `<name>.page` (visible `TTabPage`), `<name>.tabcount=N` (only the first N pages get a tab: the options dialog hides its third, debugging page); the tab strip, page frame and the focus rectangle (when the notebook is `focus`) are drawn; page controls are drawn like the form's own |
| `TColorGrid` | `<name>.index` (foreground colour), `<name>.bgindex` (background), their marks `<name>.fgmark` / `.bgmark` (default `FG` / `BG`; the team editor's `T` / `F`); cells in Delphi's order, sunken, the marks in the inverse colour (measured on the editor's colour dialog) |
| `TPaintBox` | only the stadium's: `PaintBoxStadium.level=0..24` (stands; approximate, ~430 px off in the level-2 capture) |
| `TGauge` / `TBiGauge` | `.progress` / `.value`, `.barcolor`; a gauge with `ForeColor` is a black frame filled from the left in that colour (`PaintAsBar`, a pixel short), text only with `ShowText` |

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


## Hot seat in the dialogs
`TGetTeamsDlg` takes up to six names: "Sortear equipas" (`World::name_humans`) gives every named coach a different club of the bottom six of the
4th division, `playerTeam1..6` show them, OK starts the game on the first human's main window. Everything that acts on "my team" (menus,
shortcuts, buttons, market, finances) acts on the active human (`Classic::switch_human` swaps the window, resets the page and keeps the line-up
marks per human). Bold in the coach ranking, the winners list and the club history marks every human coach; transfers of any human club are blue.
`classic_shot` opens `snap2.e98` with both humans (`seat=N` or `human=CLUB` select the active one; `season:N` presses F5/Enter once per human).

## Menus of the unregistered copy
The main window's refresh (`1018:11f3`) sets the visibility of menu items by registration features (`1050:30bc`), found through the
form's published field table: `TreinadorGestao` (`0x224`, feature `0xe`), `Pediremprestimo1` (`0x278`, `0x13`), `Sairsemgravar1` (`0x288`,
`0x46`), `Jogadoressobobservao1` (`0x2ac`, `0xc`), `JogadorProcurar1` (`0x2fc`, `0xc`), `EquipaAquisicoes1` (`0x30c`, `0x10`). The free copy
shows none of them (checked on the original, all six menus); `menu::model` hides them unless `MarketState::registered`. Shortcuts come from
the form's `ShortCutText` (*Sair* `Alt+F4`, *Classificação* `C`).
