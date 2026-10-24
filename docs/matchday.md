# Match day in the classic interface

What happens between *Jogar* and the main window coming back: the round view, the dialogs the original asks during the games, and the dialogs
that follow a round. Code: `ui/src/classic/{round_view,session}.rs`, `ui/src/classic/dialogs/matchday.rs`; core support in
`core/src/{matchsim,season,world}.rs`. Verified against `analysis/shots/match1..38` (local captures) as noted below.

## The original's flow (decoded: `1020:11b8`, `1020:0c41`, `1048:*`, `1008:21cb..2bb4`, `1038:328b..`, `1050:0d46..`)
1. `Jogar` -> the round view (`TJourneyDlg`, title `Nª Jornada - ano`), a pie clock (`TGauge` gkPie, `MaxValue` 60) and a timer
   (`MinutePassed`, `1048:069b`) that starts at minute -5, runs the minutes of all games, and closes the window 5 ticks after the last minute.
2. Minute 45: `HalfTime` opens the squad dialog (`TSubstitutionDlg`, mode 4) for every human club (when the option *Substituições ao
   intervalo* is on), then the clock turns red. Injury of a human player: `TForcedSubstitutionDlg` (mode 2), the user must pick who comes on.
   Sending-off of a human goalkeeper: the squad dialog (mode 3). Every primary chance (`Random(6000) < bonus`, penalties after a red card)
   in a game of a human coach: `TPenaltyDlg` (the human attacker picks the shooter from the starters and presses *Chutar*; otherwise the best
   attacker is announced; the result shows after 3 ticks, the window closes after 5).
3. A click on a club of a game opens its squad dialog (mode 1: events so far, the squad only for the user's own club).
4. After the view of a league day: sack list (*Chicotadas Psicológicas*, `TPrintListDlg`, by the option *Mostrar chicotadas psicológicas*), the
   classification (`TClassificationDlg`, "Classificação - Nª jornada") when a human played or after round 14. On **even** rounds the cup
   has its **own match day** after that (see "The cup day" below): the draw (`TSorteioDlg`, option *Ver sorteio da taça*: never / when a
   human is drawn / always), then the main window again, then *Jogar* plays the cup view ("Taça - <stage> - ano"; drawn ties continue to
   minute 120 with their own break at 90, the others leave the list; 20 ties per page, `<` `>` buttons). Shootouts (`TPenaltiesDlg`)
   come after the cup view, season end (`TEndOfSeasonDlg`) after the final, auctions last.
5. A coach invitation (`TInviteCoachDlg`, "Convite") appears when a computer club sacks its coach and the user's coach is the one picked.

Timer speed (`1048:0ba3`, profiles registered by `1060:3219`): without a human coach in the games 100 ms per minute (50 for *Ultrassónico*);
with humans `p3 + (n - 1) * p1` ms for the profile `(p1, p3)` = Lento (200, 1000), Médio (150, 800, the default), Rápido (100, 600),
Super-rápido (125, 400), Rapidíssimo (50, 250), Ultrassónico (0, 50).

## The cup day (`1020:11b8` -> `1020:0ed4`, decoded; captures `season/or/r7,c1,c3,p2,d5`)
`11b8` loops the rounds: `0c41` (league day) and, when the round number is even, `0ed4` (cup day). The structure, with the functions that
matter for the order of things:
* `0c41`: `01f6` (finishes the games left in the list `270a`, `1010:9396` per game = points, money, scores, sacking check), the factors/evolution
  of each division (`6e48`), `38b8` (sorts the four division lists), then **`13e1(games, round, 0)`**: for every game of the day `a67f` ->
  `3e9c(team, round)` for the home and then the away club. **`3e9c` is the per-club "before the game" routine**: the computer clubs'
  sales and renegotiations (it calls `1010:5b37`, which opens the auction through `1008:2fec`, for the player a club puts up for sale) and, **for a
  human club, the main window (`TPrepareGameDlg`, `1018:11f3`, shown modally through the nested procedure `1010:3a6c`)**; computer-managed
  clubs go through `3aad`. The main window is therefore opened *per game with a human in it*, in the order of the list of games, not once
  per round, and it is the same code for a league game and a cup tie (this is also where the autosave of the option *Gravar o jogo* runs, `1018:11f3`).
  After `13e1`: `0aae` (the purchases arrive), the round view (`1048:03ea` = `TJourneyDlg`, caption "Nª Jornada - ano"), `01f6` again
  (the results are booked), `025a` ("too many foreigners" list), a refresh of the main window, `38b8`, and the classification (`1020:3905`) when a human played
  or in round 14; then the economy random walk (`10c8:2b46`) and the clearing of the per-round flag of every team.
* `0ed4` (cup day; only when the survivors outnumber `2^((14-r)/2)`): `01f6`, the shuffle of the survivors (`1068:0f68`), `1010:0875` (pairs -> list of
  games), the draw dialog (`1020:321a`, by the option), a sort of the games, then **`13e1(games, 0, 1)`** (again `3e9c` per club of each tie: the
  main window of every human who has a tie, after the draw dialog), the cup round view (`03ea` with the stage name as caption), `01f6`
  (the ties are booked here: extra time, the shootout dialogs of `1050:0d46`, sackings), `025a`, `0aae`.
* A human who is out of the cup (or has a bye) is in no tie of the list, so `3e9c` is not run for his club: **no main window for him on that
  day**; with no human tie at all the draw dialog is followed straight by the cup round view.
* The globals `271a` (league day already played) and `271e` (ties left in the list) are what a save made on the main window of a cup day stores:
  a loaded game resumes with `0c41` skipped and the draw not repeated.
* Auctions: `5b37` is called from `3e9c` (and `3aad`), i.e. while `13e1` runs at the **start** of a day, once per listing of a club
  *that has a game that day*, interleaved with the main windows. So a dialog of a day's market appears before the main window of that day
  (clubs earlier in the list than the human's game) or after *Jogar* and before the round view (clubs later in the list); in the captures the cup
  day shows auctions right after *Jogar* and, after the cup view, the auctions (and "Vendido ao X por Y") of the *next* league day before its main window.
  The port approximates this: the market of a day is computed with the league round before it (`World::market_round`, "clubs of the next round's
  games") and the auction dialogs open at the end of the previous sequence, i.e. **after the cup day when a cup day follows the league day**,
  otherwise after the classification. The cup day itself has no market in the port (the original's `3e9c` also runs for the cup clubs).

In the port: `World::play_round_ctl` plays the league day and, on an even round, ends with `World::draw_cup` (the shuffle and the pairing, the
random state is checkpointed first, so a replay of the league day gives the same draw); the result is `World::cup_draw: Option<CupDraw>`
(`round`, `name` of the stage, `pairs` in draw order, first of a pair at home). `World::cup_day_due()`, `human_cup_tie()` (the active human's
`CupTie { opponent, home, stage }`) and `humans_in_cup_tie()` (seats) are what the interface asks. `World::play_cup_day_ctl(&mut Control)` plays the
pairs with the **current** plans of the humans, books the ties (`last_cup`, calendars), clears the planned substitutions, and runs the
season end after the final. `World::play_round()` is `play_round_ctl` followed, when due, by `play_cup_day_ctl` (the one-call API of the other front ends),
so the random numbers are drawn in the same order as before the split. A save keeps a pending draw (`cupdraw|round|stage` and `cuptie|home|away` lines).

Interface (`session.rs`, `dialogs/matchday.rs`, `main_win.rs`): the league session ends with the invitations, sackings, classification, then
`Step::CupDraw` (the draw dialog, reading `World::cup_draw` and the survivors `World::cup`) and `Step::CupDay`: the main window comes back with the
Adversário panel of the tie (below) for the humans in a tie; with none, `Classic::play_cup_day` runs at once. *Jogar* (`round_button`) on a cup day
plays the cup day: a new `Session { cup_day: true }` whose snapshot is the world before the ties (a replay plays `play_cup_day_ctl` from it), the view
`RoundView::of_cup`, then `end_of_cup` (shootouts, lashes of the cup round, end of season, **auctions of the league day's market**, main window).
Hot seat: on a cup day only the humans who play a tie take turns on the main window (`Classic::turn_seats`); the window goes to the first of them when
the league day ends.

**Main window of a cup day** (captures `c1/00`, `c1/01`): the *Adversário* panel keeps the opponent's colours; first line ` NAME (3ª Divisão)`
(`(Distrital)` for a club outside the four divisions), second line `CASA TAÇA - 32 avos de final` (`FORA` away; the stage is `cup::round_name`), the
Jogo page has no mini standings table. Referee, squad, formation, market work as on a league day.

Verified against the binary: the order league view -> classification -> draw dialog -> main window -> cup view, no main window without a tie
(by the structure of `13e1`/`3e9c`), the point where the draw's random numbers are used. Approximated: the order of the humans' windows (seat order),
auction timing as described above, no market on the cup day, the autosave happens when the main window of the cup day appears.

## How the user's decisions are applied: replay
The core computes a round in one go. The interface plays it on a copy of the world and shows it; when the user decides something at a
minute of his own game (a substitution at the break, the replacement of an injured player, the shooter of a primary chance, declining an
invitation) the round is played again from the world before it with the decision added, and the view goes on from the same minute.
* `rustfoot_core::matchsim::Decision` (`Shooter`, `Substitute` (reply to an injury or a sent-off goalkeeper, replacing the automatic one),
  `NoSubstitution`, `Planned` (a substitution made at the break, for this game only)) and `Match::{decisions, primaries}`.
* `season::Control { league, cup, log, frozen }`, `World::play_round_ctl(&mut Control)` (league day + draw) and `World::play_cup_day_ctl(&mut Control)`
  (the ties, with their own `Control`); `World::play_round()` is both with the default control, so a round nobody touches is unchanged (the golden
  tests of the core still hold). `Control::checkpoint` records the
  random state at the start of the league round, before every game, at the start of the cup draw and (cup day) before every tie; in a replay
  (`Control::replay(log, league, cup)`) the same checkpoints restore the recorded state, so **every game the user's decisions do not touch
  plays out exactly as before** (a decision at minute m changes only the user's game after m; tests in `world.rs` and `session.rs`).
* `MatchRecord`/`CupMatch` carry what the dialogs need: `start` (both squads at the kick-off), `primaries` (every primary chance, goal or
  not), `bonus` (the fixture values shown by the referee aid); `CupRound::coach_changes_from` splits the sackings of the league round from
  the cup's; `SeasonSummary::honours` has the coaches of the winners, the top scorers and the prizes (`1020:2ae6`).
* `World` is `Clone`.

## Dialog bindings (`dialogs/matchday.rs`)
| Kind | form | state in `Dialog::st` | notes |
|---|---|---|---|
| `Halftime` | `TSUBSTITUTIONDLG` | `cup src side minute mode next pend sel1 sel2` | caption "HOME, g - AWAY, g"; `textReferee` (a click opens the referee aid); `checkViewRedCards/Injuries.checked` (kept in `md.opts`); `ListBoxOfEvents`, `ListBoxOfSelectedPlayers`, `listBoxOfSubstitutePlayers` rows `G|Name*|strength`; `substituteBtn.enabled` by the rules of `1008:2905`; the form is 470 wide, 377 high for the user's club (squads' box) and 169 high for the others |
| `ForcedSubstitution` | `TFORCEDSUBSTITUTIONDLG` | `cup src side minute player sel` | `Label1`, `ListBoxOfSubstitutePlayers`, `OKBtn.enabled` after a row is picked |
| `Penalty` | `TPENALTYDLG` | `cup src side minute attack sel shot ms` | form painted in the club's colours; keys `LabelChoose`, `labelResult`, `shootBtn`, `ListBoxOfPlayers` |
| `Shootout` | `TPENALTIESDLG` | `src ms human` | the recorded kicks one per second (three with a human); closes 3 s after the last |
| `CupDraw` | `TSORTEIODLG` | `ms human` | 1500 ms per tick with a human drawn, otherwise 400; two ticks per game; reads `World::cup_draw` and the survivors `World::cup`, opened at the end of the league day |
| `EndOfSeason` | `TENDOFSEASONDLG` | `ms` | titles are counted in `Classic::md.titles` (this run of the program) |
| `Invitation` | `TINVITECOACHDLG` | `coach team` | *Não* replays the round without the invitation (`World::accept_invitations`) |
| `Lashes` | `TPRINTLISTDLG` | `lists["lines"]` | caption "Chicotadas Psicológicas", rows of `listOfItems` |
| `RefereeAid` | `TREFEREEAIDDLG` | `cup src` | team labels and the two gauges (`gaugeHomeAid/AwayAid.value` = the fixture bonus values); the balance drawing of `FormPaint` is not drawn |

Options acting on a match day live in `Classic::md.opts` (`session::Options`): `time_profile` 1..=6 (default 2), `halftime_subs`,
`cup_draw` 0..=2 (default 1), `lashes` 0..=2 (default 1), `view_red`, `view_injuries`. The options dialog should write them.

## Hot seat (several human coaches)
`World::human` (with `pending`, `observed`, `sell_tried`) is the *active* human; the others sit in `World::seats` (`Bench`) and are swapped in
by `World::switch_human(seat)`, so every `human_*` method acts for the active human and the single-human code is unchanged.
`human_count`, `humans()`, `human_clubs()`, `seat_of`, `is_human_club`, `for_each_human`, `name_humans` (new game) and `human_moves()` (coach
swaps of the round that moved a human, replacing `human_moved`) complete it. The round plays every human's plan (league and cup), the computer
clubs never bid for / coach-swap with a human, a market sale by the computer lets any human bid, the purchases arrive per human.
* Flow (observed on the original with joga1/joga2): the round button (Enter) on a human's main window shows the next human's window
  (`Classic::round_button`, `turns`); when every human has pressed it the round is played. The Classic header coach follows the active human.
* Decisions are tagged with the home club of their game (`Control::league/cup: Vec<(String, Decision)>`, `Control::decisions`), so each
  human's break substitutions, injuries and shooters replay only his own game. Questions are asked per game and side
  (`Session::handled` has the game and the side).
* The break dialog (`TSubstitutionDlg` mode 4) opens for every human side of the view, in seat order (a game of two humans asks both);
  an injury or a sent-off goalkeeper asks the human of that side; a primary chance asks the attacker if he is human, otherwise the human defender
  is only told.
* Invitations (`Step::Invitations`): one dialog per invited human, looked up when its turn comes; *Não* adds the club to
  `World::seats.declined` and replays the round. Sackings: the lashes list shows a sacking when any human is involved.
* Auctions: each open auction is offered to every human who can bid, human after human, before the next auction (the main window goes to the
  human of each dialog: `Dialog::st["seat"]`).
* Saves: `human|club|formation|starters|bench|subs|coach` once per human in seat order, `activeseat|N` (only with several humans) and a last
  field with the seat on `pending`/`observed`/`selltried` lines; old saves (no seat, no coach) load as one human.
* `.e98` import: the humans of the save (`coach_human`, order of the save's coach list) become the seats, `human_id` the active one.

## Measured against the captures
* Pie clock: 6 degrees per minute; the gauge truncates the progress to whole per cent and GDI draws the pie from the integer end point
  `Round(22 * (1 - cos/sin(pi * (per cent / 50 + 0.5))))`; the radial lines are drawn by the pen (Bresenham, ties to the minor axis). 15 of 18
  captured frames are pixel-exact, three differ by 6 pixels on the radial line. Blue first half, red second, purple extra time.
* Squad dialog: pixel-exact against `match13` (Figueirense) and `match21` (Blumenau, same players) except the texts that differ in the game
  (referee, events, line-up) and the 1-pixel offset of the caption of the focused `OKBtn` (renderer).
* The application window behind the end-of-round dialogs (silver caption bar "Elifoot 98") is pixel-exact against `match35`.

## Not done / differences
* Hot seat: see the section below (the cup day: only humans with a tie take turns). Approximations: the order of the turns after a round (the window stays with the human who had it
  and the others follow in seat order; the original's rule was only observed, not decoded), one coach profile (`History::options`) shared by
  all humans, the standings and the cup draw are the same for everybody.
* Shootouts of ties without a human are shown only when the core played them with kicks (human tie or the final); the option *Ver os
  desempates por penalties nos jogos sem treinadores humanos* is not honoured by the core.
* Mid-game substitutions by clicking on the user's club (mode 1 with `Substituir`) are applied from the next minute by a replay.
* `TSorteioDlg` shows the left list in one column (the form asks for three); the balance picture of the referee aid is missing; no sounds.

## Seen in a played season of the original (recorder `tools/capture/season.sh`, frames in `analysis/captures/`, 2026-10-24)
* **Sack list** (`TPrintListDlg`, 3 rows: the form is 148 high, list 52): `<coach> foi despedido do <CLUB>` (normal), the second line (bold in the
  capture) and an empty row. The lines come from the coach swap `1010:5343` (three `0x3ac` event objects): the second is `Troca com <coach> (<CLUB>)`
  when the coach taken over has a club, **`Para o seu lugar foi contratado <coach>`** when he has none (`1010:d55c` finds no club): in the capture
  a human coach (joga1) without a club was hired by PONTE PRETA. The port always swaps two clubs, so it never writes the second text (open: the
  original keeps unemployed human coaches; when does a sacked human become one?). The bold of the second line is seen only for this text.
* The port's sack list now uses `tables::bind_list` (height from the rows, rows in the field encoding: the first letters were being eaten before).
* **Progress boxes**: the original shows `A gravar o jogo...` (the `TProgressDlg` gauge) while saving, e.g. at the end of the season; the port saves
  without it (`A carregar o jogo...` exists for loading).
* **Cup draw** (`Sorteio dos jogos da taça`) at the round of 32: same form and fonts as ours, the left list is in several columns of 25 rows
  (ours: see above), and the draw takes the clubs out of the list one by one.
* **Injury substitution** (`TForceSubstitutionDlg`) can show a list with a single goalkeeper for an injured midfielder and OK stays disabled until
  a row is selected; the recorder could not select it (keys and the relative mouse do nothing). What the port offers in that case is unchecked.
* The recorder: after an auction the original sometimes stops answering keys on the main form (Windows still answers Alt+Tab); roughly 1 in 300 to
  1 in 2000 steps. `tools/capture/season_retry.sh` restarts from the save.
