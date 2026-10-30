# Coaches and referees

Coaches in Elifoot 98 are objects of their own (`TCoach`), not properties of a club. Each has a score that grows with good
results; a coach whose club is in bad form can be sacked, and a sacked coach **swaps clubs** with another coach, so coaches (human
ones included) travel around the world. Referees are a list of names read from a file; the referee of a game, through the referee's name
and country, decides the game's **bonus values**, which govern its rarest events.

The working notes are in [season.md](../season.md) (coach sacking, swaps, history) and [match-engine.md](../match-engine.md)
(the bonus values); the port is `World::replace_coach`, `core/src/history.rs` and the referee functions of `core/src/world.rs`.

## The coach

A coach object holds, among other fields: a flag for a human coach (`coach+0x07`), a holiday flag (`+6`), the number of games
coached at the current club (`+0x0c`), the **score** (`+0x0e`), league and cup titles (`+0x16`, `+0x18`) and a five-byte
profile (`+0x1a`, see [The transfer market](market.md#the-humans-side)). A save's coach list holds one coach per club plus the
humans' coaches (54 entries for 52 clubs and two humans in the sample).

## The score

After every league game (`1010:9396`):

| result | home coach | away coach |
|---|---|---|
| home win | +2 | 0 |
| draw | +1 | +2 |
| away win | 0 | +3 |

After a cup tie the winner's coach gets +3. Both coaches' game counters go up by one after every game. The score travels with the
coach to any new club.

## Sacking

Right after a game, before the morale update, `1010:8b5d` checks the coach of the **loser** (both coaches after a draw; the loser
of a cup tie too):

```
sacked if  games coached at this club >= 10  and  club morale K <= 0.65
```

`K` falls toward 0 with defeats (`K = 0.85 K` after each loss), so a coach who has had ten games is in danger after a bad run.
Starting from `K = 1.0`, three straight defeats give `0.614`, below the limit. (A human coach whose club is in the outside pool
is dismissed at once.)

## Who replaces a sacked coach

The replacement is chosen by `1010:5343`, called from `1010:4f40`. The candidates are coaches of other clubs:

* the coach of a league club whose **morale is at least 1.25** (a constant at `1010:565d`) and whose team rating is **11 to 19
  below** the sacked club's (`1010:5661`); in other words, a successful coach of a clearly weaker club;
* if the sacked club's rating is **below 15**, also the coaches of the **outside pool**, who count as unemployed (no morale or
  rating condition).

The candidates are sorted with a stable bubble sort (`1068:0e0d`). For a club rated below 15, computer coaches come before human
ones (`1050:1e3b`); otherwise the comparer `1050:1d59` ranks them by titles and then **by score, lowest first**. The head of the
list is taken.

A human coach can be a candidate. The original then asks that human in a dialog ("Convite", `TInviteCoachDlg`, `1010:3bf9`) whether they
accept the invitation; a human coach is never a candidate in round 14 or for a sacking after a cup tie. A human who is at the pool without a
club takes the job without being asked.

**If there is no candidate, nothing happens**: the coach stays, and the check repeats after the coach's next lost game.

## The swap

The two clubs **exchange coaches**:

* the sacked coach goes to the candidate's club and the candidate comes to the sacked club, each taking their score with them;
* both coaches' game counters restart at 0;
* both clubs' morale `K` is reset to 1.0.

The round's list of sackings (*Chicotadas Psicológicas*) shows each one in three rows: "*coach* foi despedido do *CLUB*", then
"Troca com *coach* (*CLUB*)" when the incoming coach had a club, or "Para o seu lugar foi contratado *coach*" when the incoming coach came from
the pool, then an empty row. A line naming a human coach is bold. In a synthetic world of the port about 3.5 swaps happen per
season.

## Human coaches and the swap

Because coaches swap rather than disappear, the human coach moves with their coach object:

* A **sacked human** takes over the candidate's club: a weaker club, but one in good form.
* A human who **accepts an invitation** takes over the club that sacked its coach. The port declines invitations by default
  (`World::accept_invitations`); declining replays the round without the swap.
* If the sacked human is replaced by a coach from the pool, that human takes that coach's place at the pool and is **unemployed**: no
  main window and no turns; the old club plays on under its new coach, and the other humans play the days. The human comes back when a
  club rated below 15 sacks its coach and takes them from the free coaches, with no dialog. In one recorded run of the original a
  human sacked at BAHIA had no turn for rounds 1 to 3 of the next season and returned at CRUZEIRO EC (4th division) right after the
  round-3 table. The port does this only when another human still has a club; a single human stays where they are.
* When a human leaves a club to a computer coach, that club's wages are capped at the players' values and a negative cash
  balance is lifted to zero (`1010:6a08`).
* At the season end, a human whose club dropped into the pool is given a club among the bottom six of the 4th division.

!!! note "Not verified"
    The unemployed-human behaviour was seen in one case only, and the port does not keep it in its saves (loading a game gives
    the human a club).

## The coach ranking

The *Ranking* dialog sorts all coaches with the comparer `1050:1d59` and the stable bubble sort: by **league titles**, then
**cup titles**, then **score**, best first. Coaches level on all three keep their order in the list of coaches, where the humans'
coaches come last (checked on the original's ranking of a sample save). The champion's coach counts a league title (`coach+0x16`),
the cup winner's coach a cup title (`+0x18`).

!!! note "Unclear"
    The working notes describe the same comparer `1050:1d59` as "lowest score first" when it ranks replacement candidates and,
    after a correction, as "highest first" for the ranking dialog. The port follows both descriptions (lowest first for
    candidates, highest first for the ranking); whether the original passes a direction or the candidate rule is misread is not
    settled.

## Titles and histories

The game keeps records for its information dialogs (*Últimos vencedores*, *Historial*, *História*, the coach profile):

**Champions** (`TLastWinners`): for every season, the year, the champion and its coach, the cup winner and its coach, appended at
the season end.

**Club history** (`TTeamEvent`: kind, year, coach):

| kind | event |
|---|---|
| 1 | league title |
| 2 | cup |
| 3 | double (instead of 1 and 2 when the champion also won the cup) |
| 4 | second place in the 1st division |
| 5 | third place in the 1st division |

**Coach history** (`TCoachEvent`: kind, year, club):

| kind | event |
|---|---|
| 1 | hired by a club |
| 2 | sacked |
| 3 | league title |
| 4 | cup |
| 5 | "Ingresso no *club*", written for every coach when a new game starts (`1020:0148`) |
| 6 | "Despromovido para o *club*" (written for the coaches of a swap, `1010:4f40`) |
| 7 | "Promovido para o *club*" (likewise) |
| 8 | double |
| 9, 10 | second and third place |

## The coach profile and holidays

The profile (`TCoachOptionsDlg`, five bytes at `coach+0x1a`) says which players' wages the computer renegotiates for a human
coach (domestic, foreigners under the Bosman rule, other foreigners), the coach's auction mode and whether others may see the coach's offers. A
new game of the original starts with the wage switches off and auction mode 1. A coach **on holiday** lets the computer pick the
line-up and run the club in the market. See [The transfer market](market.md#coaches-on-holiday).

## Referees

### The list

The referees come from `REFEREE.TXE`: 182 entries of a country code and a name, numbered in file order (`TReferee`, list `2706`).
Each referee has a *free* flag.

### Picking the referee of a game

When the games of a match day are made (`1010:0875`, called for every division of a league day and once for the ties of a cup
day):

1. The referee list is shuffled (`1068:0f68`: `3n` times a random referee moves to the end).
2. Every game, in the order of the day's list, takes the first free referee **whose country is both clubs' country**; if there
   is none, the first free referee **whose country is neither club's** (`1010:1916`, two passes).
3. The referee stays busy until the game ends (`1010:9396`).

This was checked on the last batch of games of every day save of the original (72 of 72). Two consequences: a country with few
referees runs out of them (with 11 Brazilian referees, a Brazilian league's twelfth game of the day gets a foreign referee), and a
game between clubs of two countries never has a referee from either. If no referee fits, the game has none and its bonus values
are 0 and 0.

### The bonus values

The match object stores two bonus values (`1010:8db2`):

```
homeBonus (match+0x4c) = c95c(referee, home club) + 2
awayBonus (match+0x4e) = c95c(referee, away club)
```

and `1010:c95c` is a checksum of two strings:

```
A = the referee's name                 (string[40])
B = the club's short name              (string[20] of the club's .EFT object)
n = max(length A, length B)
S = sum over i = 1..n of (A[i] - B[i])     # byte values; A counts as 0 past its length
value = |S mod 11|                         # signed remainder, then absolute value
      + 3 if the referee's country is the club's country
```

`B` is read as it lies in memory: past the short name's 20 characters the sum runs on into the club's colours, country code and
level. The formula reproduces the values of all 288 games of the original's 18 day saves byte for byte (`matchsim::referee_value`).

*Worked example* (hypothetical strings). Referee "Silva", club short name "SPORT", same country:

```
S = ('S'-'S') + ('i'-'P') + ('l'-'O') + ('v'-'R') + ('a'-'T')
  = 0 + (105-80) + (108-79) + (118-82) + (97-84) = 103
103 mod 11 = 4        value = 4 + 3 = 7        as the home side: 7 + 2 = 9
```

The checksum part is spread evenly over 0..10 and the `+3` applies to every domestic game, so a domestic league game has a
home bonus of 5 to 15 and an away bonus of 3 to 13; a game with a foreign referee loses the `+3`.

### What the bonus values do

| event | rule | effect of a high bonus |
|---|---|---|
| primary chance, home | `Random(6000) < homeBonus` each minute | more chances |
| primary chance, away | `Random(7000) < awayBonus` each minute | more chances |
| red card | `r = Random(1000)`, `r < 10` and `bonus < r` | none at all from 9 up |
| annulled goal | `r = Random(200)`, `r < 10` and `homeBonus < r` | none at all from 9 up |

So a high-bonus game is calm and full of primary chances, a low-bonus game is stormy: cards, penalties after them, disallowed
goals. The home side's `+2` both gives it more primary chances and protects it from red cards; the annulment rule reads the home
side's value for either side's goals.

In the original's registered copy a click on the referee's name in the half-time dialog opens a *referee aid* (`TRefereeAidDlg`)
whose two gauges show the bonus values; the free copy ignores the click (`1008:2427`), and the port shows it only in its
"registered" mode.

!!! note "Approximated"
    A world built without the game's `REFEREE.TXE` (the port's synthetic test worlds) has no referees; its games use `c95c` with
    a stand-in string (the division name, or "TACA" for the cup) and treat every club as domestic.
