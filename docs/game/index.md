# How Elifoot 98 works

Elifoot 98 (André Elias and Ricardo Correia, 1998) is a football-manager game for Windows 3.1, written in Delphi 1. This guide
explains its rules: how a match is simulated, how a season runs, where the money comes from, how the computer clubs trade
players and replace their coaches, and what the game writes to disk. The rules were recovered by decompiling `ELIFOOT.EXE`
with Ghidra and checking what was found against the original's own save files and screen captures; the Rust port in `core/`
implements them.

You do not need to read code to follow this guide. Formulas appear in code blocks, and addresses such as `1010:46df` name the
routine of the original program a rule comes from (segment `1010` is the game's main unit). They are references only, for anyone
who wants to check the disassembly or the deeper notes in the parent folder.

## A game of Elifoot 98

**The world.** At the start of a new game you choose countries, and the game takes the teams (`.EFT` files) of those countries:
at least 34 and at most 128 of them. It sorts them by the level written in each team file and splits the list:

* the first 32 clubs form **four divisions of eight** (1st to 4th division);
* the rest form an **outside pool**: clubs that play no league games (the game labels them *Distrital*) but play in the cup and
  can be promoted into the 4th division.

**The season.** Each division plays a **double round robin**: 14 league rounds, every club meets every other club once at
home and once away. Wins give 3 points, draws 1. The **cup** (*Taça*) is a single-leg knock-out that every club of the world
enters, divisions and pool alike. It is played on its own match day after every **even** league round, and the final always
falls on round 14. After the last round and the cup final comes the **season end**: prizes, the top two of each lower division
swap places with the bottom two of the division above, two pool clubs join the 4th division, and the year advances.

**Human coaches.** One or more people play on the same machine, taking turns at the main window (hot seat; the port allows up
to six). Each human coach starts at a random club among the bottom six of the 4th division. Every other club has a computer
coach. Coaches are people in the game: they have a score, they can be sacked, and a sacked coach swaps clubs with another one.
A human coach can therefore be moved to another club, or be left without a club for a while.

**Players.** A player has a position (goalkeeper, defender, midfielder, attacker), a nationality and a **strength** from 1 to
50, the only skill value the game shows. Each player also has three hidden attributes (talent, aggression, injury proneness) that the
game derives from the letters of the player's name. Strength changes after every round: starters tend to lose strength and reserves to
gain it, within a window set by the club's division and league position.

## The life of one match day

Everything in the game happens in this cycle. A league day is numbered 1 to 14; on even days a cup day follows.

1. **The market runs before the day's games** (`1020:13e1` -> `1010:3e9c`). For both clubs of every game of the day, in the
   order of the day's list, the computer clubs renegotiate wages, may put a player up for sale, and expand their stadiums.
   Every other club bids for a player put up for sale in an auction. For a club with a human coach, this is the moment the
   **main window** opens: the human picks the line-up, buys, sells and sets the ticket price. See [The transfer market](market.md).
2. **Purchases arrive** (`1020:0aae`): players bought directly during the previous step join their new club.
3. **The games are played** (`1048:069b`), minute by minute, all games of the day together. The original shows them in the
   round view with a pie clock, and stops to ask a human coach at half time, after an injury, after the coach's goalkeeper is sent off,
   and at every rare "primary" chance of that coach's game. See [The match engine](match-engine.md).
4. **Results are booked** (`1010:9396`, per game): points, wins, draws, losses and goals; gate receipts to the home club; every
   club's wage bill and loan interest; the coaches' scores; a sacking check for the losing coach; and the morale `K` of both
   clubs. See [The season](season.md), [Money](economy.md) and [Coaches and referees](coaches.md).
5. **The division tables are sorted** and shown to the humans (the classification), together with the list of sacked coaches.
6. **End-of-round work** (`1020:0c41`): ban and injury counters drop by one, every squad's strengths evolve, each division's
   economic factor is smoothed, and the global economy index takes a small random step.
7. **On an even round, the cup day** (`1020:0ed4`): if enough clubs are left, the survivors are drawn into ties, the market runs
   again for the clubs of the drawn ties (step 1), the ties are played (with extra time and penalties when needed), and losers
   leave the cup.
8. **After round 14 and the cup final, the season end** (`1020:038e`, `1020:2ae6`): prizes, titles and histories, the ledger
   moves to "last season", promotion and relegation, the year advances and a new season starts with day 1.

## The pages of this guide

| Page | What it covers |
|---|---|
| [Random numbers](random.md) | The Delphi random number generator, how it is used, and why games of the original cannot be replayed |
| [The match engine](match-engine.md) | Players and their hidden attributes, line-ups, team sectors, the minute loop, chances and goals, cards, injuries, substitutions, penalties, annulled goals, morale |
| [The season](season.md) | Fixtures, points and tables, strength evolution between rounds, the cup, the season end, setting up a new game |
| [Money](economy.md) | Currencies, the economy index, player values and wages, gate receipts, stadiums, loans, prizes, the ledger |
| [The transfer market](market.md) | Who sells, the asking price, auctions and computer bids, the human's side of the market, coaches on holiday |
| [Coaches and referees](coaches.md) | Coach scores, sacking, coach swaps and invitations, the coach ranking and histories, how referees are chosen and what their "bonus values" do |
| [Files](files.md) | The team files (`.EFT`), text tables (`.TXE`), save games (`.e98`) and the port's own save format |

## How sure is this?

Most rules in this guide were read instruction by instruction from the disassembly and then checked against data the original
produced: saves made after every round of a season and a half, screen captures, and the dialogs' own texts. Where a rule is only
approximated, calibrated to fit the data, or not decoded yet, the page says so in a box like this one:

!!! note "Not decoded"
    Boxes of this kind mark what is missing or uncertain. Everything outside them is what the original program does.

The deeper working notes behind this guide are [match-engine.md](../match-engine.md), [season.md](../season.md),
[finance.md](../finance.md), [matchday.md](../matchday.md), [e98-format.md](../e98-format.md), [findings.md](../findings.md),
[classes.md](../classes.md) and [decompilation-map.md](../decompilation-map.md).
