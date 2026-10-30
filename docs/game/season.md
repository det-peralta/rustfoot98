# The season

A season of Elifoot 98 is 14 league rounds in four divisions of eight, with a knock-out cup for every club of the world played
alongside. This page covers the fixtures, how results are booked and tables sorted, how player strengths evolve between rounds,
the cup, the season end, and how a new game is set up. The working notes are in [season.md](../season.md); the port is
`core/src/season.rs`, `cup.rs`, `evolution.rs` and `world.rs`.

## The structure of a season

The round driver `1020:11b8` loops over the league rounds 1 to 14. Each round runs the league day (`1020:0c41`); when the round
number is even it then runs a cup day (`1020:0ed4`). So the calendar alternates: league, league + cup, league, league + cup, and
so on, with the cup final on the day after league round 14.

## Fixtures

Each division plays a double round robin from one fixed table (`DS:0x1838`, read by `1010:1d20`). The table has a row for each of
the eight team slots of a division and a column for each round. For team slot `t` and round `r`, the entry `v = TABLE[t][r-1]`
names the opponent `|v| mod 8` (8 stands for slot 0), at home when `v > 0` and away when `v < 0`:

| slot | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | -7 | 5 | -3 | -1 | -4 | 6 | -2 | 7 | -5 | 3 | 1 | 4 | -6 |
| 1 | 5 | -4 | 3 | -2 | 8 | -6 | 7 | -5 | 4 | -3 | 2 | -8 | 6 | -7 |
| 2 | -8 | 6 | -4 | 1 | 7 | 3 | -5 | 8 | -6 | 4 | -1 | -7 | -3 | 5 |
| 3 | 7 | -5 | -1 | 8 | -6 | -2 | 4 | -7 | 5 | 1 | -8 | 6 | 2 | -4 |
| 4 | -6 | 1 | 2 | -7 | 5 | 8 | -3 | 6 | -1 | -2 | 7 | -5 | -8 | 3 |
| 5 | -1 | 3 | -8 | 6 | -4 | -7 | 2 | 1 | -3 | 8 | -6 | 4 | 7 | -2 |
| 6 | 4 | -2 | 7 | -5 | 3 | 1 | -8 | -4 | 2 | -7 | 5 | -3 | -1 | 8 |
| 7 | -3 | 8 | -6 | 4 | -2 | 5 | -1 | 3 | -8 | 6 | -4 | 2 | -5 | 1 |

*Reading it.* In round 1, slot 0 has `2`: it plays slot 2 at home; slot 2's entry is `-8`, that is slot 0, away. The four home
games of round 1 are 0-2, 1-5, 3-7 and 6-4. Every pair meets twice, once at each ground, and every team has seven home games.

The table works on fixed slots (the club's place in its division's list), not on the current league position.

## Booking a result

After each game, `1010:9396` books the result:

* **Points 3-1-0**: 3 to the winner (`team+0x20`), 1 to each side after a draw; wins, draws and losses (`+0x16/+0x18/+0x1a`) and
  goals for and against (`+0x1c/+0x1e`).
* **Money**: the gate receipts to the home club, then each club pays its wage bill and its loan interest (see
  [Money](economy.md)).
* **Coach scores**: home win +2 for the home coach, away win +3 for the away coach, draw +1 for the home coach and +2 for the
  away coach; the loser's coach gets nothing.
* **Sacking check** for the loser's coach (both coaches after a draw), using the morale before this game's update (see
  [Coaches and referees](coaches.md#sacking)).
* **Morale** `K = 0.85 K + 0.15 r` for both clubs (see [The match engine](match-engine.md#after-the-final-whistle-morale-k)),
  and both coaches' game counters go up by one.

## The league table

The division lists are sorted with the comparer `1050:1a61`, which compares only the **points** (a per-round byte array at
`team+0x21+round`). Goal difference is not part of that sort; clubs on equal points keep their order in the list. This sorted
order is what decides the champion, promotion and relegation, and the ranks used by strength evolution.

The classification window the player sees (`SortDivisionLists`, `1050:1ab6`) orders by points, then goal difference, then goals
scored, with clubs equal on all three keeping their previous order.

!!! note "Approximated"
    The sort routine `1068:0e0d` is assumed to be stable (it is a bubble sort). The port also takes the ranks for the evolution
    after the round's results; the exact moment the original sorts relative to the evolution is not confirmed.

## Strength evolution between rounds

Strengths change after **every** round, not only at the season end (`1020:0c41` -> `1010:0d9c` -> `1010:6fb8` -> `1010:4a41`).
First the ban and injury counters of every player drop by one. Then each club gets a **level** from its division and its rank in
the sorted table:

```
level = base(division) - 2 * rank              # rank 0 = top of the table
base  = 52, 38, 24, 10 for divisions 1 to 4   (1010:0b60: division+4 = index * -14 + 52; the pool would be -4, never used)
```

For every player of the squad (starters and bench alike) the sign of the player's rating accumulator decides:

* **positive** (the player was mostly not a starter): for an outfield player, or a goalkeeper who wins `Random(2) == 0`, the
  strength goes up by 1 (at most 50); the step is undone if the new strength exceeds `level + 5`;
* **negative** (the player was mostly a starter): `Random(2)` is drawn first; on 0, if the strength is above 1, it goes down by 1; the step
  is undone if the new strength falls below `level - 5`;
* **zero**: nothing happens.

The accumulators are reset after the evolution once the club's coach has coached more than one game (`1010:3e9c`), so from the
second round on the sign mostly reflects whether the player started that round's game.

The effect is a steady pull of every squad toward a window around its club's level: strong starters slide down until they reach
`level - 5`, reserves climb toward `level + 5`. In the 4th division the top club has `level = 10` (window 5..15) and the bottom
club `level = -4` (window down to 1). In the 1st division the top club's window is 47..57, capped at 50.

*Example from a real save.* After two rounds most starters of a 4th-division side had lost 1 or 2 points (Gilberto 5 -> 4,
Nass 6 -> 4, Nilson 7 -> 5) and a reserve had gained one (Luciano 2 -> 3).

A new world starts with mean squad strengths of about 43 / 31 / 18 / 7 in divisions 1 to 4. In the port's long simulations they
erode slowly (to about 34 / 20 / 11 / 2 after twelve seasons, measured before the market was decoded), and buying players is the
human's counter-measure.

## The cup

The cup (*Taça*) is a single-leg knock-out tournament for **every** club of the world: the 32 league clubs and the whole pool,
34 to 128 clubs. The routine is `1020:0ed4`; the port is `core/src/cup.rs` and `World::draw_cup` / `play_cup_day_ctl`.

**When a round is drawn.** After an even league round `r`, a cup round is drawn only if the number of clubs exceeds

```
p = 2^((14 - r) / 2)        # integer division: 64, 32, 16, 8, 4, 2, 1 for r = 2, 4, ..., 14
```

This rule makes the final fall on round 14 whatever the number of clubs.

**The draw.** The survivors are shuffled (`1068:0f68`: `3n` times a random club moves to the end of the list). With `n`
survivors, let `k` be the smallest power of two that is at least `n / 2`. The first `n - k` consecutive pairs of the shuffled list
play (the first club of a pair at home); the others get a **bye**. After the round exactly `k` clubs are left.

With 52 clubs:

| after league round | `p` | survivors | ties | byes | left | stage |
|---|---|---|---|---|---|---|
| 2 | 64 | 52 | none (52 <= 64) | | 52 | |
| 4 | 32 | 52 | 20 | 12 | 32 | 32 avos de final |
| 6 | 16 | 32 | 16 | 0 | 16 | 16 avos de final |
| 8 | 8 | 16 | 8 | 0 | 8 | Oitavos de final |
| 10 | 4 | 8 | 4 | 0 | 4 | Quartos de final |
| 12 | 2 | 4 | 2 | 0 | 2 | Meias-finais |
| 14 | 1 | 2 | 1 | 0 | 1 | FINAL |

The stage is named after `k`: 1 *FINAL*, 2-3 *Meias-finais*, 4-7 *Quartos de final*, 8-15 *Oitavos de final*, otherwise
"*k* avos de final".

**The cup day.** The draw is shown (by an option: never, when a human is drawn, always), the market runs for the clubs of the
drawn ties, and then the ties are played in their own round view. A human whose club is out of the cup, or has a bye, has no
main window that day.

**A tie.** A tie is one game. If it is level after 90 minutes it goes to **extra time** (to minute 120). If it is still level,
a **penalty shoot-out** decides it. There are two kinds of shoot-out (`1050:0d46`):

* **Automatic**, when no human plays the tie, the round is not the final, and the option *Ver os desempates por penalties nos
  jogos sem treinadores humanos* is off. Each side draws three times `Random(team rating)` and adds them up. The **home side is
  eliminated when its sum is greater than or equal to the away sum**, so the lower total wins. This looks inverted, but it is
  what the program does, and the port keeps it.
* **Real kicks**, when a human plays, always in the final, and for every tie when the option is on (the *Penaltis* dialog). The
  sides kick alternately, home first. The taker is the next member of the squad list who is on the pitch (cycling through the
  list); the opponent's goalkeeper on the pitch defends; each kick is the shot duel of the match engine (taker's attacking force
  against the keeper's strength, *GOLO* scores). The shoot-out ends as soon as one side leads by more than the other side's
  remaining kicks of the first five, or, after five kicks each, when a pair of kicks leaves the scores apart (sudden death). The
  side with fewer goals is eliminated.

**What a tie books.** The ticket price of a cup day is one price for every tie: the league formula applied to the clubs of the
round, times 1.5. Attendance gets `+30 %`. The **gate is split half and half** between the two clubs; both pay their wages and
interest as in the league. The winner's coach scores +3; the loser's coach may be sacked (and a human's coach is never the
replacement in that case). Morale: the winner's `r` is 1.5 if it was the home side and 2.0 if it was the away side, the loser's 0.
Both coaches' game counters go up.

!!! note "Approximated"
    The sources disagree slightly on what is compared with `p`: the working notes say "the number of survivors", the code
    compares the number of clubs of the world. The two give the same rounds in practice; the port follows the code.

## The season end

After round 14 and the cup final the season-end routines run (`1020:038e`, prizes `1020:2ae6`, ledger `1010:38a9`). The port
runs them in this order, which agrees with what the original's saves on both sides of a season change show:

1. **The final table** of each division is sorted by points.
2. **Prizes** (category 3 of the ledger, amounts from the currency, see [Money](economy.md#prizes)): the 1st division's
   champion, the cup winner, and the club of every top scorer of the season (all players tied on the most goals; a club gets the
   prize once per such player). The players' goal tallies then start again from zero.
3. **The ledger rolls over** (`1010:38a9`, run for every club as the new season starts): this season's totals become last
   season's, last season's become the season before's, and the totals restart at zero. Since the prizes are booked first, they
   show in last season's column.
4. **Every player may be sold again**: the *fresh* mark (set by a transfer or a wage renegotiation) is cleared for every player.
5. **Titles and histories** are recorded: the champions list, the clubs' and coaches' histories, coach title counts (see
   [Coaches and referees](coaches.md#titles-and-histories)).
6. **Morale bonus**: the 1st division's champion `K += 1.0`, the cup winner `K += 0.5`.
7. **Promotion and relegation** (`1020:02a6`): the top two of each lower division swap places with the bottom two of the
   division above. The routine moves clubs one by one between lists and appends them at the end, which is why the original calls
   it with list index 6 for the 1st/2nd division swap and index 4 for the 2nd/3rd and 3rd/4th: clubs just relegated into a
   division sit at the end of its list and are not sent further down.
8. **The pool**: two **distinct random clubs of the outside pool** replace the bottom two of the 4th division, which drop into the
   pool.
9. The table is cleared, the **year advances by one**, the cup starts again with every club, and the new season begins with
   day 1.

A human whose club has dropped into the pool is given another club among the bottom six of the 4th division (one no other human
manages).

The season-end money was checked on saves either side of a season change: every club's cash changed by its last round's gate and
wages plus the prizes, and nothing else, for 52 of 52 clubs.

!!! note "Not decoded"
    What the new-season routine `1020:2f3c` resets beyond the table, the cup and the ledger is not fully read.

## Setting up a new game

**The teams** (`1028:002a`, `1028:0544`). You choose countries; the game needs 34 to 128 teams in all (`1028:18d9`). It builds
the master list by looping over the `.EFT` **level from 20 down to 1** and taking that level's team files of the chosen countries
(ties in directory order; under DOSBox that is alphabetical, case-insensitive). Entry `i` of the list goes to division
`min(4, i / 8)`: the first eight to the 1st division, the next eight to the 2nd, and so on; everything from entry 32 on is the
pool. In a real save the 52 teams appear in strictly descending level order.

**Starting strengths** (`1028:0373`). The club at list position `i` gets a base strength

```
base(i) = max(5, round((1 - i/32) * 50) - 1)     for i <= 32
base(i) = 1                                       beyond
```

that is 49, 47, 46, 44, 43, 41, 40, 38 in the 1st division, 37 down to 26 in the 2nd, 24 down to 13 in the 3rd, 11, 10, 8, 7, 5,
5, 5, 5 in the 4th, 5 for the first pool club (`i = 32`) and 1 for the rest of the pool. Every player then gets

```
strength = clamp(base + 3 - Random(7), 1, 50)    # base - 3 .. base + 3
```

In the sample game, Figueirense (list position 29, base 5) had a squad averaging 5.4.

**Money and morale.** Each club starts with cash `base * 50 000 * G` (`G` is the currency scale, 5 for reais: 12 250 000 for the
best club, 250 000 for a pool club of base 1), a stadium of level `max(1, round(base * 0.3))` (5000 seats a level), an economic factor
`F = 1.0`, and every player's wage at the player's value. The global economy index starts at `E = 2.0`. Every club starts at `K = 1.0`. See
[Money](economy.md).

**The humans' clubs** (`1028:0b26`). Each human coach is placed at a random club among the **bottom six of the 4th division**
(list index `7 - Random(6)` within the division), never one another human already manages.

**Coaches.** Every club's coach (the name in its `.EFT` file) gets a first history entry, "Ingresso no *club*"
(`1020:0148`).
