# The transfer market

Players change clubs through **auctions**. Before every match day each computer-managed club decides whether to put a player up
for sale; if it does, every other club that can afford the player places one sealed bid, and the highest bid wins. Human coaches can bid
in these auctions and put their own players up. Computer clubs trade with each other too: in the sample save, after two rounds,
Palmeiras had booked a sale of 5 171 000 and São Paulo a purchase of exactly 5 171 000.

The working notes are in [finance.md](../finance.md) (sections on the computer-managed market) and [matchday.md](../matchday.md);
the port is `core/src/market.rs` and the market functions of `core/src/world.rs`.

## When the market runs

The market is not a separate phase: it is part of the routine that prepares each club for its next game. Before every match day,
`1020:13e1` walks the day's list of games and calls `1010:3e9c` for the **home club and then the away club** of each game
(`1010:a67f`):

* on a league day, the round's games division by division;
* on a cup day (after the draw, before the ties), the drawn ties, pool clubs included.

For a computer-managed club `3e9c` runs the computer's decisions (`1010:3aad`). For a club with a human coach it opens the
**main window** (`TPrepareGameDlg`), where the human prepares their game. The main windows and the auction dialogs of the day are
therefore interleaved in the order of the day's games.

`3e9c` reads the day counter (1 to 14, the day about to be played): on **day 1**, which runs right after the season change,
nobody renegotiates or sells.

## Before every match day

For each computer-managed club, in this order:

1. **Stadium.** If the stadium is at least 80 % full and the club can pay the works plus two wage bills, it builds 5000 seats
   (see [Money](economy.md#the-stadium)).
2. **Wages** (from day 2, once the coach has coached more than one game). The club reprices the **most overpaid** player (stored
   wage divided by current value above 1.2) and the **most underpaid** one (below 0.8) to their current value, and marks both
   *fresh*.
3. **Forced sale.** If its cash is negative, it puts up its least needed sellable player (below) with an asking price of 0.
4. **Regular sale** (from day 2). It puts up its least needed sellable player at the normal asking price when any of these holds:
    * `3 * cash < wage bill` (it is short of money);
    * otherwise, `Random(20) == 0` (one time in twenty);
    * otherwise, it has exactly five foreigners and a squad of 21 or more (then only a foreigner may go).

The same pass also clears the players' rating accumulators of clubs whose coach has coached more than one game, and repays loan
steps where the cash allows (computer clubs never borrow).

## Who can be sold

A player may be put up for sale (`1010:5a6a`, which returns a reason code) only when all of these hold:

| code | condition that blocks the sale |
|---|---|
| 1 | the squad has fewer than 12 players |
| 2 | the player was already offered without success before this game ("só pode tentar vender este jogador novamente no próximo jogo"; the port reports it as code 4) |
| 3 / 4 | the player is flagged; in the port, code 3 is *fresh*: the contract was renegotiated or the player changed club this season |
| 5 | the player is a goalkeeper and the club has fewer than two |
| 6 | the player is an outfield player and the club would keep 10 or fewer outfield players |

The *fresh* mark (`slot+0x16`) is set by a wage renegotiation and by a transfer, and **cleared for every player at the season
change**. In the original's day saves the marked players grew from 15 to 252 over the 2026 season and were 0 in the first save of
2027. So every player can be sold at most once a season, and a player who just changed club stays put until the season ends.

## Which player a club sells

The club puts up the sellable player with the **lowest sale score** (`1010:5de1`), a measure of how much it needs the player:

```
L     = min(5, rating / 10)                 # the club's level, from its team rating
score = (5 / (aggression + 1))^(1/4)
      * SHAPE[L][pos] / (count[pos] + 1)    # how short of this position the club is
      * SHAPE[L][5]   / (squad + 1)         # how short of players the club is
      * strength / (rating + 1)             # how good the player is for this club
      * 1.5 if the player is of the club's country, 1.2 if of the same country group, 1 otherwise
```

`SHAPE` is the target squad shape, a table in the binary (`DS:0x300`), growing with the level:

| `L` | GK | DEF | MID | ATT | total |
|---|---|---|---|---|---|
| 0 | 3 | 8 | 7 | 3 | 22 |
| 1 | 3 | 8 | 7 | 4 | 22 |
| 2 | 3 | 8 | 8 | 5 | 23 |
| 3 | 4 | 8 | 8 | 5 | 24 |
| 4 | 4 | 9 | 8 | 5 | 25 |
| 5 | 4 | 9 | 8 | 5 | 26 |

So the first to go are weak players for the club, aggressive ones, players at overstaffed positions, and foreigners; a large
squad lowers every score.

**Country groups.** A player's nationality class (`slot+0x35`, `1058:0a2f`) is 0 when the player is of the club's country, 1 when club
and player are both in one of two groups of countries, 2 otherwise:

* the **Bosman** group (`CTRGROUP/BOSMAN.TXE`): POR, ESP, FRA, ITA, ALE, AUT, GRE, ING, ESC, WAL, ILN, IRL, DIN, BEL, HOL, LUX,
  FIN, SUE;
* the **PLOP** group of Portuguese-speaking countries (`PLOP.TXE`): POR, BRA, ANG, MOC, CAV, STP, GBI, GNE, TIM.

Only class 2 counts as a foreigner for the limit of five.

## The asking price

The asking price (the *Preço base* of the auction dialog) starts from a market value (`1040:371a`) that, unlike the wage, takes
the hidden attributes into account:

```
f            = 1 - 0.2 * aggression / 5 - 0.2 * proneness / 10 + 0.2 * g / 10      # about 0.6 .. 1.2
market value = round( strength^1.04 * G * E * f * 2000 )
asking price = round(market value * 1.6), rounded down to a multiple of 1000         # 1010:3aad -> 1010:5b37
```

(The binary also multiplies by a header value at `2b50`, 1.0 in every save seen.) The factor 1.6 was confirmed on the dialog:
Pierkaski, strength 46, was offered at 1 554 000 and Maurício Pantera, strength 11, at 319 000; without the 1.6, `f` would have to
exceed its maximum of 1.2.

## The auction

The auction (`1008:3600`, `373e`, `351a`, `39c7`; the dialog `TsellPlayerDlg`, "Venda de jogador por leilão") visits every club
except the seller in master-list order. A club takes part (`1010:633d`) when:

* its squad plus its pending signings is below 24;
* the player would not be its sixth foreigner (class 2);
* its cash is at least the asking price.

Each taking part places one bid (`1010:5f77`, below). The **highest bid at or above the asking price wins**; on a tie the club
earlier in the master list wins. The winner pays **its own bid** (not the second-best one) to the seller (ledger categories 2 and
`0x15`). The player joins the buyer with a new wage, the player's value (`1010:7ff4`) computed in the seller's economy, and is marked
*fresh*. The dialog shows only the asking price and the result ("Vendido ao *X* por *Y*"), never the bids.

## How a computer club bids

A bid starts from how much the buyer **needs** the player (`1010:5bb1`) and is then cut to what the buyer can afford
(`1010:5f77`). With `R` the buyer's team rating, `L = min(5, R / 10)`, `count[pos]` its players in the player's position, `squad`
its squad size, `wages` its wage bill and `value` the player's value (`7ff4`) in the seller's economy:

```
if count[pos] == SHAPE[L][pos]: bid = 0                 # the position is already at its target

need  = ( strength / (R + 5) * (SHAPE[L][pos] + 1) / (count[pos] + 1) * SHAPE[L][5] / (squad + 1) )^(1/4) * R^0.2
need *= 1.5                       for a goalkeeper when the buyer has only one
need *= 1.5  + 0.05 * foreigners  if of the buyer's country
        1.15 + 0.05 * foreigners  if of the same country group
        1                         otherwise
need *= 1.1                       for a star

X     = market value * need
ratio = (cash - X) / (wages + value + 1)                # spare cash, in wage periods
if ratio < 3:  X = cash - 3 * (wages + value)
if ratio > 5:  X = X * ln(ratio) / D                    # D: see the note below
if wages > 100 and cash - X > 10 * wages (and > 1):
               X = X * (1 + ln((cash - X) / wages - 9) / 10)
bid   = max(0, min(cash - X - 5 * wages, X)), rounded down to a multiple of 1000
```

A rich club with room in its squad bids more than the market value; a club that is short of money bids little or nothing; a
club whose position is full does not bid at all. The final `min` keeps at least five wage bills in the bank.

!!! note "Calibrated"
    The binary divides by `D = 1.6` (`CS:5f57`). With 1.6 the computed bids come out about 24 % above the fees the original
    actually booked (36 computer-to-computer auctions over a season and a half of day saves). Two variants fit the data equally
    well: `D = 2.2` with the spare-cash factor (fee over the buyer's bid 1.005 on average, spread 6.7 %), or 1.6 without that
    factor (1.008, spread 7.1 %). The port keeps the decoded structure with **`D = 2.2`** (`market::BID_DIVISOR`). The gap is not
    explained: the routines and constants are the same in the decompiled build and the one the captures ran, the conditions of the
    spare-cash factor were re-read and hold for these buyers, and neither the buyers' cash before the round, the order of the day's
    sales, the second-best bid as the fee, nor a different wage bill fits better. On the sample save the divisor 2.2 picks the
    three winners with fees within 5 % (4.99 M against 5.17 M, 565 000 against 592 000, 1.42 M against 1.35 M).

**Pace.** From the original's first day save, the original sold 36 players over the 17 rounds its saves span; by the rules about
33 sale attempts were expected. The port, started from the same save, sells 30 on average over 60 seeds.

## The human's side

**In the original**, an auction stops the game with a dialog. A human coach whose club may take part (cash, squad and foreigner
limits as above) raises bids while the computer clubs bid in a timer loop.

**In the port** every game of a round is computed in one go, so the auction becomes asynchronous with the same rules:

* **Buying at auction.** When a computer club puts a player up and a human's club could take part, the auction stays open as a
  *listing* showing the computer's best bid. The human may bid any multiple of 1000 above both the asking price and that bid; a
  winning bid settles at once (fee, wage at the value, player *fresh*). Open listings are saved, and settle with the computer's
  best bid at the start of the next match day.
* **Selling.** The human names a price; the computer clubs bid at once with the rules above; the best bid at or above the price
  wins. The sale rules of `5a6a` apply. A player for whom nobody bid cannot be offered again before the next game.
* **Direct purchases** (`1010:63fa`, the *Procurar jogadores* dialog `1048:29f0`). A computer club sells a player directly only
  when the player is sellable, its coach is not human, and the player's sale score is at most 1.95 (a surplus player). The price is
  `market value * 3.5 * score`; for a buyer of the same country `(p + 30 000) * 1.6`, of the same country group
  `(p + 15 000) * 1.3`; then rounded down to a multiple of 100 000 (from 2 million), 50 000 (from 1 million), 25 000 (above
  500 000), 10 000 (above 100 000) or 1000. The buyer pays at once; the player joins at the end of the round (`1020:0aae`) with the
  wage of the buyer's economy, *fresh*. Pending arrivals count against the limits of 24 players and five foreigners
  (`team+0xc7`, `+0xc9`). In the original's unregistered copy this dialog is hidden, and the port hides it too.

**The coach profile** (`TCoachOptionsDlg`) sets how the human takes part: whether the computer manages the wages of the human's domestic
players, of foreigners covered by the Bosman rule and of other foreigners; an **auction mode** (0 offers for all players, 1 not
for the weaker ones, 2 no buying in auctions); and "do not let the others see my offers". The port applies the three wage
switches and mode 2 (no listings for that human); mode 1 and hidden offers are stored only.

**The human's club after they leave it.** When a human coach moves to another club and a computer coach takes over the old one,
`1010:6a08` caps every wage of that club at the player's value and lifts a negative cash balance to zero.

## Coaches on holiday

A human coach can be put **on holiday** (`coach+6`, set in *Gestão de treinadores*, `TCoachManagerDlg`). A coach on holiday counts
as the computer everywhere the market looks (`1010:8c67` tests "human and not on holiday"): that coach's club goes through the computer's
`3aad` instead of the main window, all its wages are renegotiated, it bids in auctions with the computer's formula, and the
computer picks its line-up. The free copy hides *Gestão de treinadores*, so only a save can bring a holiday.

!!! note "Not modelled"
    * The original's interactive bidding loop (the port's listings replace it).
    * Wages demanded by human coaches and the second `3e9c` branch for human coaches.
    * The auction mode 1 and the hidden-offers option of the coach profile (stored, not applied).
    * The exact timing of the auction dialogs: the port computes a day's market at the end of the previous match sequence and
      opens the dialogs there, while the original opens each one when its club's turn comes in the day's list.
