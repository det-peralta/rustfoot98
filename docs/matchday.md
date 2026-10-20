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
4. After the view: sack list (*Chicotadas Psicológicas*, `TPrintListDlg`, by the option *Mostrar chicotadas psicológicas*), the
   classification (`TClassificationDlg`, "Classificação - Nª jornada") when a human played or after round 14, then on even rounds the draw
   of the cup (`TSorteioDlg`, option *Ver sorteio da taça*: never / when a human is drawn / always) and the cup view ("Taça - <round> - ano";
   drawn ties continue to minute 120 with their own break at 90, the others leave the list; 20 ties per page, `<` `>` buttons).
   Shootouts (`TPenaltiesDlg`) come after the cup view, season end (`TEndOfSeasonDlg`) after the final, auctions last.
5. A coach invitation (`TInviteCoachDlg`, "Convite") appears when a computer club sacks its coach and the user's coach is the one picked.

Timer speed (`1048:0ba3`, profiles registered by `1060:3219`): without a human coach in the games 100 ms per minute (50 for *Ultrassónico*);
with humans `p3 + (n - 1) * p1` ms for the profile `(p1, p3)` = Lento (200, 1000), Médio (150, 800, the default), Rápido (100, 600),
Super-rápido (125, 400), Rapidíssimo (50, 250), Ultrassónico (0, 50).

## How the user's decisions are applied: replay
The core computes a round in one go. The interface plays it on a copy of the world and shows it; when the user decides something at a
minute of his own game (a substitution at the break, the replacement of an injured player, the shooter of a primary chance, declining an
invitation) the round is played again from the world before it with the decision added, and the view goes on from the same minute.
* `rustfoot_core::matchsim::Decision` (`Shooter`, `Substitute` (reply to an injury or a sent-off goalkeeper, replacing the automatic one),
  `NoSubstitution`, `Planned` (a substitution made at the break, for this game only)) and `Match::{decisions, primaries}`.
* `season::Control { league, cup, log, frozen }` and `World::play_round_ctl(&mut Control)`; `World::play_round()` is `play_round_ctl` with
  the default control, so a round nobody touches is unchanged (the golden tests of the core still hold). `Control::checkpoint` records the
  random state at the start of the league round, before every game, at the start of the cup draw and before every tie; in a replay
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
| `CupDraw` | `TSORTEIODLG` | `ms human` | 1500 ms per tick with a human drawn, otherwise 400; two ticks per game |
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
* Hot seat: see the section below. Approximations: the order of the turns after a round (the window stays with the human who had it
  and the others follow in seat order; the original's rule was only observed, not decoded), one coach profile (`History::options`) shared by
  all humans, the standings and the cup draw are the same for everybody.
* Shootouts of ties without a human are shown only when the core played them with kicks (human tie or the final); the option *Ver os
  desempates por penalties nos jogos sem treinadores humanos* is not honoured by the core.
* Mid-game substitutions by clicking on the user's club (mode 1 with `Substituir`) are applied from the next minute by a replay.
* `TSorteioDlg` shows the left list in one column (the form asks for three); the balance picture of the referee aid is missing; no sounds.
