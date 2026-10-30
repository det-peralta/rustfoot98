# Random numbers

Every chance in Elifoot 98, from a shot on goal to the cup draw, comes from one function: Delphi 1's `System.Random`. The game
has no other source of randomness. Knowing exactly how it works matters for two reasons: a faithful port must draw the same
numbers in the same order, and it explains why the original's games cannot be replayed.

## The generator

Delphi's generator is a linear congruential generator with a 32-bit state, the **seed**. Each call first advances the seed
(`10c0:218a`):

```
seed = (seed * 0x08088405 + 1) mod 2^32
```

and then turns the new seed into the result:

```
Random(n)  = (seed * n) >> 32             # an integer in 0 .. n-1   (10c0:212d)
Random(0)  = 0
Random     = (seed_as_signed_i32 + 2^31) / 2^32    # a real in [0, 1)   (10c0:2167)
```

`Random(n)` takes the top 32 bits of the 64-bit product `seed * n`, so it is uniform on `0 .. n-1` for all practical purposes.
The argument is a 16-bit word in the original, so `n` is at most 65 535. The real-valued form (`Random` without argument) is used
where the game needs a fraction, for example in the economy index and in the attendance of a game.

The port's version is `core/src/rng.rs` (`DelphiRng`). A test translates the original 16-bit assembly literally and checks it
against the formula above over 100 000 seeds.

A short example, starting from seed 1:

| call | new seed | result |
|---|---|---|
| `Random(10)` | `0x08088406` | 0 |
| `Random(10)` | `0xdc6dac1f` | 8 |
| `Random(10)` | `0x33dc589c` | 2 |
| `Random(6000)` | `0x45de2b0d` | 1637 |
| `Random(90)` | `0xabf18b42` | 60 |

Because every call advances the same seed, each result depends on **every call made before it**. One extra or missing call
anywhere shifts all later results.

## Where the game uses it

| Routine | What is random | Calls |
|---|---|---|
| `1010:a6eb` | the rare "primary" chance of each side, every minute | `Random(6000)` home, `Random(7000)` away, `Random(3)` for a penalty |
| `1010:46df` | the main chance generator, every minute | `Random(90)` / `Random(180)` home, `Random(120)` / `Random(270)` away, `Random(45)` |
| `1010:81ed` | the shot duel of a primary chance or a penalty kick | `Random(attacker force)`, `Random(keeper strength)`, `Random(4)` |
| `1010:100b` | the roulette that picks a scorer, a sent-off or an injured player | `Random(total weight)` |
| `1010:aa11` | red cards | `Random(1000)`, ban `Random(4)` |
| `1010:ac72` | injuries | `Random(500)`, length `Random(Random(20))` |
| `1010:aea7` | annulled goals | `Random(200)` |
| `1010:afff` / `49e2` | the players' rating accumulators, every minute | `Random(squad size)` per team |
| `1010:4a41` | strength evolution after every round | `Random(2)` |
| `1068:0f68` | shuffles (line-ups, the cup draw, the referee list): `3n` times a random element moves to the end | `Random(n)` |
| `1020:0c41` | the economy index, once per round | three `Random` reals |
| `1010:b0a1` | attendance of a game | one `Random` real |
| `1010:3aad` | a computer club's 1-in-20 decision to sell | `Random(20)` |
| `1028:0373` | starting strengths of a new game | `Random(7)` per player |
| `1028:0b26` | the human's starting club | `Random(6)` |
| `1020:02a6` | the two pool clubs promoted at the season end | `Random(pool size)` |
| `1058:30ba` | the installation id written to `eli.cod` | `Random(20000)` |

The **hidden player attributes** are not random, although the routine that computes them (`1010:7516`) looks as if it calls a
random function: the helper it calls (`10c0:1882`) is a 32-bit `mod`. Talent, aggression and injury proneness are fixed
functions of the player's name (see [The match engine](match-engine.md#players-and-their-hidden-attributes)). The referees'
bonus values are not random either: they are computed from the referee's and the club's names (see
[Coaches and referees](coaches.md#referees)).

## Why the original's games cannot be replayed

The original seeds its generator when the program starts, and no seed has been found in the decoded save file. Loading the same
save twice therefore gives two different continuations: different goals, different sales, a different cup draw.

!!! note "Not decoded"
    The sources only establish that the original seeds its generator at start-up, and that no seed sits in any decoded part of
    the `.e98` save (the whole file has been walked to the last byte, see [Files](files.md)). How the seed is obtained (for
    example from the clock) is not documented in this project's notes.

This shaped how the port was checked. Because a played day of the original cannot be reproduced number for number, the rules
were verified in three other ways:

* **Rules without randomness** were checked byte for byte: the hidden attributes against 15 of 15 players of a real save, the
  referees' bonus values against all 288 games of 18 day saves, the ticket price against 43 of 43 division-rounds, the wage bill
  against every club.
* **Money rules** were checked on pairs of the original's own saves one round apart (option *Gravar o jogo: Todas as jornadas*):
  whatever the random events were, cash must change by exactly the sum of the ledger's entries.
* **Random rules** were checked statistically: for example the number of transfers per season of the original against the
  port's average over many seeds.

## The port's use of the seed

The port keeps the generator's state as part of the game. Its own save format stores the seed, so a game loaded from a port save
continues exactly as it would have without the save. An imported `.e98` save of the original gets a fresh seed.

The port also needs repeatability for a different reason. It computes a whole round in one go, but a human can intervene in the
middle of their own game (a substitution at the break, the shooter of a primary chance). The port then **replays** the round from a
copy of the world with the decision added. To keep every other game of the round unchanged, the generator's state is recorded at
fixed checkpoints (the start of the round, before every game, the start of the cup draw, before every cup tie) and restored at
the same checkpoints in the replay (`season::Control`). A decision at minute `m` can therefore only change the human's own game
after minute `m`. A shooter picked by the human changes the shot duel, but not how many numbers are drawn.
