# The match engine

A game of Elifoot 98 is simulated minute by minute. Every minute each side can get a chance, a player can be sent off or
injured, and a goal from the minute before can be disallowed. No ball moves and no positions on the pitch are modelled. What
decides a game is how strong each side is in four **sectors** (goalkeeper, defence, midfield, attack), the side's **morale**,
and the **bonus values** the game's referee gives each side.

This page follows the order in which things happen: the players, the line-up, the sectors, then the minute loop and each event
it can produce, and finally what a game changes after the final whistle. The working notes are in
[match-engine.md](../match-engine.md); the port is `core/src/matchsim.rs`, `chance.rs`, `outcome.rs`, `sector.rs` and
`lineup.rs`.

## Players and their hidden attributes

A player has a **position** (0 goalkeeper, 1 defender, 2 midfielder, 3 attacker), a **nationality** and a **strength** from 1
to 50. The team files (`.EFT`) store no strength at all: it is given when a new game is set up (see
[The season](season.md#setting-up-a-new-game)) and changes from round to round.

Each player also has three **hidden attributes** that the game never shows. They are not random: they are computed from the
player's name (`1010:7516`). Let `H` be the sum of the bytes of the name (one byte per character, the game's single-byte
encoding):

```
g (talent)          = H mod 10 + 1                      # 1 .. 10
aggression          = 5 - trunc(sqrt(H mod 36))         # 0 .. 5
                      for a defender: (aggression + 2) mod 6
injury proneness    = H mod 11                          # 0 .. 10
star ("craque")     = g >= 8 and position >= 2          # midfielders and attackers only
```

A star is shown with `*` after the player's name. The formulas were checked on all 15 players of a real save.

*Worked example.* "Sandro Ventura", a midfielder: the bytes of the name add up to `H = 1388`. Then `g = 1388 mod 10 + 1 = 9`,
`aggression = 5 - trunc(sqrt(1388 mod 36)) = 5 - trunc(sqrt(20)) = 5 - 4 = 1`, `proneness = 1388 mod 11 = 2`. With `g = 9` and
the midfield position the player is a star. Rename the player and all three values change.

The attributes are used as follows: talent raises a player's attacking force and the chance of scoring; aggression is the player's weight
in the red-card roulette; proneness is the player's weight in the injury roulette. Talent, aggression and proneness also enter the player's market
value (see [The transfer market](market.md#the-asking-price)).

## The line-up

A club fields eleven **starters** and names up to five **substitutes**; everyone else is out of the matchday squad. A human
coach picks the line-up in the main window (one of twelve formations, from 4-4-2 to 6-4-0, or any choice of players). The
computer's line-up (`1010:4388` -> `1010:4b73`) works like this:

1. The squad list is shuffled (`3n` times a random player moves to the end of the list).
2. Players are ranked by `strength + goals scored this season`, best first. The sort is stable, so the shuffle decides ties.
3. A first pass fills a core with quotas of 1 goalkeeper, 3 defenders, 3 midfielders and 2 attackers.
4. A second pass with quotas of 1 / 10 / 10 / 10 completes the eleven with the best remaining players of any position (never
   more than ten outfield players) and names the bench: one reserve goalkeeper if there is one, then outfield players until five
   substitutes are named.

Injured and suspended players are never picked. The number of substitutions a side may make in the game is
`min(3, number of fit substitutes)` (`team+0x4d`). The rule reproduces the line-ups of 28 of 29 clubs of a real save (the 29th
had signed two players after its line-up was made).

## Sector strengths

Before kick-off, and again after every substitution or dismissal, each side adds up its starters by position (`1010:397d`):

```
sector[position] += strength + 2 * star + (3 if the player's nationality is the club's country)
```

This gives four numbers per side: `GK`, `DEF`, `MID`, `ATT`. Bench players count for nothing. The domestic bonus of 3 is large
next to low strengths: a 4th-division side of eleven domestic players of strength 6 has sectors `GK 9, DEF 36, MID 36, ATT 18`
in a 4-4-2.

A separate number, the **team rating** (`1010:4ec3`), is the rounded mean strength of the whole squad, bench and reserves
included. The match itself does not use it, but attendance, the cup's automatic shoot-out, the market and coach swaps do.

## The bonus values of the game

Each game has a **referee**, and the referee gives each side a small number, the **bonus value** (`match+0x4c` for the home side,
`+0x4e` for the away side). It is computed from the referee's name and country and the club's short name and country
(`1010:c95c`, see [Coaches and referees](coaches.md#referees)): typically 5 to 15 for the home side and 3 to 13 for the away
side in a domestic league game. The home side's value includes `+2`. A game for which no referee was free has bonus values 0
and 0.

The bonus values gate four rare events: the primary chances, red cards, annulled goals and, through red cards, penalties. A
high value means more primary chances and a calmer game; a low value means few primary chances but more cards and disallowed
goals.

## The minute loop

The round view's timer (`1048:069b`) runs the minutes of all games of the day together. For each game, every minute does, in
this order:

1. **Chances** (`1010:a6eb`): the primary rolls, and if neither side got one, the main chance generator (`1010:46df`), home first.
2. If no goal was scored: **red card** check (`1010:aa11`).
3. If still nothing happened: **injury** check (`1010:ac72`).
4. If still nothing happened and the minute is not 46 or 91: **annulment** check (`1010:aea7`) for a goal of the previous
   minute.
5. Always: the **rating accumulators** of one random player per team change (`1010:afff`).

A league game has 90 minutes. A drawn cup tie continues to minute 120 (extra time). At minute 45 (and 90 before extra time) the
original opens the half-time dialog for every human club when the option *Substituições ao intervalo* is on; substitutions can
be made there.

The speed of the clock depends on a profile chosen in the options: without a human coach in the day's games a minute takes
100 ms, with humans `p3 + (n - 1) * p1` ms for `n` humans and a profile `(p1, p3)` from Lento `(200, 1000)` to Ultrassónico
`(0, 50)`; *Médio* `(150, 800)` is the default (`1048:0ba3`).

## Primary chances

Each minute the home side gets a **primary chance** when `Random(6000) < homeBonus`, the away side when
`Random(7000) < awayBonus` (the away roll is ignored if the home side already has one). With a home bonus of 10, the home side
gets on average `90 * 10 / 6000 = 0.15` primary chances a game.

A primary chance is a shot. The shooter is the starter with the highest **attacking force** (`1010:818c`), the later one in the
squad list on a tie; when a human coach's side gets a primary chance, the original asks that coach to choose the shooter from the side's
starters (the *Penalty* dialog, `1038:328b`). The defender is the opponent's goalkeeper (the starter of lowest position,
strongest on a tie).

```
force = round_half_even( (position + 1) * strength * (g / 10 + 1) / 2 )
```

An attacker (position 3) of strength 8 and talent 10 has force `4 * 8 * 2 / 2 = 32`; a goalkeeper of strength 5 and talent 1 has
force `round(1 * 5 * 1.1 / 2) = 3`. Position matters a lot: the same strength is worth four times as much in attack as in goal.

The **shot duel** (`1010:81ed`) then decides the outcome:

```
a = Random(force)
d = Random(keeper strength)
if Random(4) == 0:                      # 1 in 4: the shot may miss outright
    a < 4  -> wide        (Ao lado)
    a < 10 -> over        (Por cima)
    a < 12 -> crossbar    (Na barra)
    a < 13 -> post        (Ao poste)
otherwise, or if a >= 13:
    a < d  -> saved       (Defendeu)
    else   -> GOAL        (GOLO)
```

A strong shooter rarely misses outright: with force 32, only `a < 13` (13 of 32 values) in the 1-in-4 branch leads to a miss.
A weak shooter against a strong keeper is usually saved. A primary chance counts as a goal only when the duel returns *GOLO*.

## The main chance generator

Most goals come from the secondary, or main, generator (`1010:46df`). It is tried for the home side and, if the home side did
not score, for the away side, in every minute where no primary chance occurred and no penalty is pending.

First it computes a **threshold** `T` from the sectors of both sides and the side's morale `K`:

```
ownAtt = own.ATT + own.MID / 2
oppDef = 3 * opp.GK + opp.DEF + opp.MID / 2
R      = 4 * ownAtt / (oppDef + 1)

# three "lines", with running sums that are not reset between lines
lo = 1 ; hi = 1 ; S = 0
lo += own.DEF ; hi += opp.ATT              ; S += 1 * lo / (lo + hi + 1)
lo += own.MID ; hi += opp.MID              ; S += 2 * lo / (lo + hi + 1)
lo += own.ATT ; hi += 3 * opp.GK + opp.DEF ; S += 1 * lo / (lo + hi + 1)

Y = ln(R + 1) * S
T = (K / 2)^(1/4) * Y
```

`R` measures attack against defence; `S` adds up how the side compares line by line (its defence against their attack, midfield
against midfield, its whole team against their defence and goalkeeper), with midfield counting double. The goalkeeper's sector is
multiplied by 3 wherever it defends.

Then three random stages decide whether the side scores this minute:

1. **Candidate.** Home: `Random(90) < T - (goals + annulled)` or `Random(180) == 0`. Away: the same with `Random(120)` and
   `Random(270)`. Here `goals` is the number of goals the side has scored in this game and `annulled` the number of its goals
   disallowed so far.
2. **Warm-up gate.** The candidate stands only if `Random(45) < minute`. In minute 1 it passes 1 time in 45, in minute 22 about
   half the time, and from minute 45 on always. Early goals are rare.
3. **Scorer.** The scorer is drawn with the weighted roulette (below, weight `position² × strength × g`). If the roulette picks
   nobody, there is no goal.

Two consequences follow from stage 1. Away sides are handicapped twice: `Random(120)` instead of `Random(90)` against the same
threshold, and a smaller bonus value. And the threshold is reduced by **every goal already scored**: once a side has scored about
`T` goals, it can only score through the 1-in-180 (1-in-270 away) lucky roll. This keeps scores low and makes big wins rare.

*Worked example.* Two equal 4th-division sides as above (`GK 9, DEF 36, MID 36, ATT 18`), both at the starting morale `K = 1`:

```
ownAtt = 18 + 36/2 = 36            oppDef = 27 + 36 + 18 = 81
R = 144 / 82 = 1.756               ln(2.756) = 1.014
S = 37/57 + 2*73/129 + 91/210 = 0.649 + 1.132 + 0.433 = 2.214
Y = 2.245                          T = (1/2)^(1/4) * 2.245 = 0.841 * 2.245 = 1.888
```

At 0-0 the home side's candidate test `Random(90) < 1.888` passes for the values 0 and 1, that is 2 times in 90, plus the
1-in-180 roll. After one home goal only the value 0 passes (`< 0.888`); after two goals only the lucky roll is left. For an
unequal game, a side with sectors `GK 8, DEF 32, MID 32, ATT 16` facing one with `GK 13, DEF 52, MID 52, ATT 26` has a
threshold of about 1.09, while the stronger side's threshold is about 3.0.

## The weighted roulette

Scorers, sent-off players and injured players are drawn by one routine, `1010:100b`. Each kind of event has its own weight
(`1010:0f2e`):

| kind | event | weight of a player |
|---|---|---|
| 1 | (strength) | `strength` |
| 2 | scorer | `position² × strength × g` |
| 3 | red card | aggression |
| 4 | injury | injury proneness |

The routine adds up the weights of **every** player in the squad, bench and players outside the matchday squad included, draws
`r = Random(total)`, and then walks **only the starters**, choosing the first whose running sum exceeds `r`. A draw that falls in
the share of the non-starters chooses nobody, and the event does not happen.

So a deep squad dilutes its own goals, cards and injuries. Goalkeepers (position 0) have scorer weight 0 and never score from the
main generator; an attacker weighs nine times as much as a defender of the same strength and talent.

## Red cards

Every minute in which nobody scored, `1010:aa11` draws `r = Random(1000)`:

* if `r < 10` and `homeBonus < r`, the home side gets a red card;
* otherwise, if `r < 10` and `awayBonus < r`, the away side gets one.

A side with a bonus value of 9 or more can therefore never be sent off, and a side with bonus `b` below 9 has the chance of
`(9 - b)` in 1000 per minute (less for the away side when the home side's range overlaps). The player is drawn with the
aggression roulette; the player leaves the pitch and is suspended for `Random(4) + 1` games (1 to 4; the counter drops by one at the end
of every round, and the player cannot be picked while it is above zero).

A sent-off **goalkeeper** is replaced automatically if the side has substitutions left: the first substitute goalkeeper comes on
and the **last starter** in the squad list goes off. A sent-off outfield player is not replaced. A human coach is asked instead.

## Penalties

A red card can give the other side a **penalty**: in the minute after a sending-off, the opponents get a forced primary chance if
the sent-off player was a goalkeeper (always) or a defender (`Random(3) == 0`, one time in three), provided `minute mod 45 > 1`.
The penalty is a primary chance like any other: the best attacker (or the human's choice) against the goalkeeper in the shot
duel. A pending penalty also blocks the main generator for that minute.

## Injuries

In a minute where nobody scored and nobody was sent off, `1010:ac72` draws `Random(500)`: 0 means the home side has an injury,
1 the away side, anything else nothing (so about one injury per side every 500 minutes before dilution). The player is drawn with
the proneness roulette. The player leaves the pitch, is out for `Random(Random(20)) + 1` games (short injuries are much more common than
long ones), and **loses 10 points of strength** (to a minimum of 1).

If the side has substitutions left, the first designated substitute of the same kind (goalkeeper for goalkeeper, outfield for
outfield) comes on automatically (`1010:67f7`). The original asks a human coach to choose (*TForcedSubstitutionDlg*).

## Substitutions

Apart from the automatic replacements above, substitutions come from the human coach: at half time, after an injury or a
sending-off, or planned in advance in the port (minute, player off, player on; at most three, valid for one game). A goalkeeper can
only be replaced by a goalkeeper. Each substitution uses one of the side's `min(3, bench)` substitutions, and the sectors are
recomputed at once.

## Annulled goals

In a minute where nothing else happened (except minutes 46 and 91), `1010:aea7` draws `r = Random(200)`. If `r < 10` and
`homeBonus < r`, and the last event of the game was a goal scored in the previous minute, that goal is **annulled**: the scoring
side loses the goal, its scorer loses it from their tally, and the side's annulled counter (`team+0x59`) goes up, which lowers its
threshold in the main generator just as a goal would.

The check uses the **home** side's bonus value whichever side scored.

## The rating accumulator

Every minute, after everything else, `1010:afff` picks one random member of each squad (home first): if that player is a starter their
**rating accumulator** (`slot+0x0c`) goes down by 1, otherwise (bench or not in the matchday squad) up by 1. The accumulator does
nothing during the game. After the round, its sign decides whether the player's strength tends to fall or rise (see
[The season](season.md#strength-evolution-between-rounds)): starters drift down, reserves drift up.

## After the final whistle: morale `K`

Every club has a morale value `K` (`team+0x5b`), shown by the game's *Moral* gauges. It starts at 1.0 and is updated after every
game (`1010:9396`, `1068:10d1`) as an exponential moving average:

```
K = 0.85 * K + 0.15 * r
```

| result | home side `r` | away side `r` |
|---|---|---|
| win | 2.0 | 2.5 |
| draw | 0.8 | 1.3 |
| loss | 0.0 | 0.0 |
| cup tie, advances | 1.5 | 2.0 |
| cup tie, eliminated | 0.0 | 0.0 |

`K` therefore stays between 0 and 2.5 during a season. At the season end the 1st division's champion gets `K += 1.0` and the
cup winner `K += 0.5`. When a coach takes over a club, its `K` is reset to 1.0.

`K` enters the main generator as `(K/2)^(1/4)`:

| `K` | 0.3 | 0.65 | 1.0 | 1.25 | 2.0 | 2.5 |
|---|---|---|---|---|---|---|
| factor | 0.62 | 0.76 | 0.84 | 0.89 | 1.00 | 1.06 |

A side on a winning run scores more, a side on a losing run noticeably less. Two defeats from `K = 1` give
`K = 0.85² = 0.7225`; this value was read from a real save after two lost rounds. `K` also drives attendance, the bank's loan
limit, and coach sacking (`K <= 0.65`).

## What a game changes

When the game ends, what persists is copied back to the clubs: the players' rating accumulators, goals, strength losses from
injuries, bans and injury counters, and appearances. Points, money, coach scores, the sacking check and the morale update are
booked per game by `1010:9396` (see [The season](season.md#booking-a-result)).

## What the numbers look like

The port's simulation of 100 000 games per pairing (4-4-2 with a bench of five, domestic league game, bonus values from random
strings) gives:

| home vs away strength | home goals | away goals | home wins | draws | away wins | red cards | injuries |
|---|---|---|---|---|---|---|---|
| 6 vs 6 | 1.15 | 0.87 | 42.6 % | 32.2 % | 25.2 % | 0.15 | 0.24 |
| 8 vs 5 | 1.46 | 0.82 | 54.4 % | 28.1 % | 17.5 % | 0.15 | 0.24 |
| 5 vs 8 | 1.08 | 1.12 | 32.9 % | 32.4 % | 34.7 % | 0.15 | 0.24 |
| 10 vs 3 | 1.85 | 0.51 | 74.9 % | 19.0 % | 6.1 % | 0.15 | 0.24 |

Home advantage is strong: equal sides give the home side 42.6 % of wins against 25.2 % for the away side.

!!! note "Not decoded"
    * The half-time routine (`1010:44a1`) is only partly read; the port models half time only as the point where the human may
      substitute.
    * The interactive path of a human's primary chance (`1038:328b`) is decoded far enough to choose the shooter; the foul roulette
      of that path is not modelled.
    * The port plays a whole round in one go and replays it when a human decides something (see [Random numbers](random.md#the-ports-use-of-the-seed)),
      where the original stops the clock and asks.
