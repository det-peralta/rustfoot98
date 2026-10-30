# Money

Every club has cash, a stadium, possibly a loan, and a ledger of where its money came from. Money comes in from gate receipts,
player sales and prizes, and goes out on wages, purchases, stadium works and interest. All amounts are scaled by the currency
chosen for the game and by a global **economy index** that drifts from round to round. The working notes are in
[finance.md](../finance.md); the port is `core/src/economy.rs` and `core/src/finance.rs`.

## Currencies and the scale `G`

The *Dinheiro* dialog of a new game offers five currencies (`TCurrencyDlg`; records at `DS:0062`, the chosen one copied to
`DS:2d40` and into the save header). Each record carries the currency scale `G`, the loan step and the three season prizes:

| Currency | `G` | loan step | prizes: champion / cup winner / top scorer |
|---|---|---|---|
| Escudos (contos) | 1 | 100 000 | 3 000 000 / 1 000 000 / 500 000 |
| Euros | 4 | 500 000 | 15 000 000 / 5 000 000 / 3 000 000 |
| Dólares | 5 | 500 000 | 20 000 000 / 6 000 000 / 3 000 000 |
| Reais | 5 | 500 000 | 20 000 000 / 6 000 000 / 3 000 000 |
| Libras | 4 | 250 000 | 10 000 000 / 4 000 000 / 2 000 000 |

`G` multiplies player values, wages, ticket prices and new-game cash. The examples on this page use reais (`G = 5`), the currency
of the sample saves.

## The economy index `E`

A global index `E` (`10c8:2b46`, saved in the `.e98` header) scales prices and values. It is **2.0** in a new game. Once per
round (`1020:0c41`) it takes a small random step, with three `Random` reals `r1, r2, r3` drawn in that order:

```
d = r1 + r2 - r3      if year mod 100 < 50
d = r1 - r2 - r3      otherwise
E = clamp(E + 0.01 * d, 0.5, 10)
```

The expected step is `+0.005` a round in the first case and `-0.005` in the second. So the index **drifts upward in the years
00-49 of a century and downward in 50-99**: a game started in 1998 deflates, a game started in 2026 inflates. This looks like a
year-2000 quirk of the original. After two rounds of the sample game (year 2026) the index read 2.0214.

## The economic factor `F`

Each club also has an economic factor `F` (`team+0x79`), 1.0 in a new game. It is a per-division factor, smoothed every round
by a division routine (`1010:6e48`):

```
ratio = (cash of the division's computer-managed clubs) / (a reference total of the division),  at least 0.1
F     = 0.8 * F + 0.2 * ratio        for every club of the division
```

`F` enters player values (as `sqrt(F)`) and the ticket price (as `1 / sqrt(mean F)`). A division whose clubs get richer pays
higher wages and charges lower ticket prices.

!!! note "Approximated"
    The reference total (a long at `division+0x1f`) is not decoded. The port uses the division's total cash remembered at the
    previous round, which could not reproduce the sample's 4th division exactly (after two rounds the sample shows
    `F` = 1.02097 / 1.01196 / 1.00244 / 0.93451 for divisions 1 to 4).

## Player value and wage

One formula (`1010:7ff4`) gives a player's **value**, shown as the "price" column of the squad list, and the player's **wage**:

```
value = G * 50 * round( sqrt(F) * E * 35 * 1000^0.04 * strength^1.04 / 50 )
value = clamp(value, 50 * G, 50 000 * G)
```

The value is a whole multiple of `50 G` (250 reais). In a new game (`E = 2`, `F = 1`, `G = 5`):

| strength | 1 | 11 | 12 | 13 | 34 | 46 |
|---|---|---|---|---|---|---|
| value | 500 | 5 500 | 6 000 | 6 750 | 18 000 | 24 750 |

The values for strengths 1, 12, 13 and 34 were read from a new-game save. For an injured player the original keeps the previous
stored value if it is higher.

**Wages.** Each player has a stored wage (`slot+0x31`), set to the player's value when the player joins a club or the contract is renegotiated.
Every game a club plays, league or cup, it pays the **sum of its players' stored wages** (`1010:5992`, booked by `1010:5a0a`),
which was verified to the unit on every club across consecutive saves. Because the stored wage lags behind the current value,
wages are renegotiated before match days: a computer club reprices its most overpaid player (stored wage more than 1.2 times
the value) and its most underpaid one (less than 0.8 times) each day (see [The transfer market](market.md#before-every-match-day)).

**A human's wage proposal** (`newSalaryBtnClick`, `1018:46ee`). A human coach can offer a player a new wage, between `50 G` and
`50 000 G`. An offer above 80 % of the player's value is accepted; otherwise the player asks for 1.25 times that value (rounded to
`50 G`), which the coach may accept. An accepted contract marks the player *fresh*, so the player cannot be renegotiated or sold again
this season. When the coach's profile lets the computer manage a class of players (domestic, Bosman-covered foreigners, other
foreigners), the computer renegotiates their wages as for its own clubs.

## Gate receipts

The home club of a league game earns `attendance * ticket price` (`1010:9396`).

**Ticket price** (`1040:35e1`, called when a day's games are made, `1010:0875`). One price applies to all games of a division,
computed from the division's state as the round starts:

```
x     = G * E / sqrt(max(mean F, 0.1)) * round(mean rating) / 5
price = 2 * round(round(x) / 4)          # always even
price = max(price, G / 2)
```

where `rating` is each club's team rating (the rounded mean strength of its squad). In a new game's 1st division
(`E = 2`, `G = 5`, `F = 1`, mean rating 43): `x = 10 * 43 / 5 = 86`, `price = 2 * round(21.5) = 44`. The formula reproduced the
prices of 43 of 43 division-rounds of a season of the original (44 / 32 / 18 / 8 at the start, falling to 34 / 24 / 16 / 6). On a
cup day one price applies to every tie: the same formula over the clubs of the round, times 1.5 (30 with 32 clubs, 36 with 16).

**Attendance** (`1010:b0a1`), with one `Random` real `u` in `[0, 1)`:

```
w          = 3 * rating(home) + rating(away) + 3 * u
p          = w * sqrt(K_home) * sqrt(K_away) * 200
attendance = round( p^1.04 * (1.3 if cup tie else 1) )
attendance = min(attendance, stadium capacity)
```

Both clubs' morale counts: a game between two sides in form draws a bigger crowd. *Example*: two 1st-division sides of rating 43
at `K = 1`, `u = 0.5`: `w = 173.5`, `p = 34 700`, attendance about 52 700; at 44 reais a ticket that is about 2.3 million, in
line with the 2.0-2.9 million gates of the sample's 1st division. Two 4th-division sides of rating 7 draw about 8 300.

A cup tie's gate is **split half and half** between the two clubs.

## The stadium

A stadium has a **level** from 0 to 24; its capacity is `level * 5000` seats. A new game gives each club level
`max(1, round(base * 0.3))` from its base strength, for example 15 (75 000 seats) for the best club and 2 for a weak 4th-division
club.

After every home game the stadium's **filling** moves toward the ratio of attendance to capacity (`1010:c0c0`):

```
fill = 0.6 * fill + 0.4 * attendance / (capacity + 1)
```

When the filling is at least 0.8 and the level below 24 (`1010:c030`), a computer club builds 5000 more seats, provided
`cost + 2 * wage bill < cash` (checked in `1010:3aad` before match days). The filling then falls back to 0.6 (`1010:c081`). The
works cost (`1010:c172`, ledger category `0x17`):

```
bracket = 1 if level < 6, 2 if level < 14, 3 otherwise
cost    = max(50 000, 50 000 * round( sqrt(bracket) * E * 35 000 * G / 50 000 ))
```

At `E = 2`, `G = 5` that is 350 000 for a small stadium. A human coach can build from the finance page whenever the club has the money;
the 80 % rule is the computer's.

## Loans

A club can borrow from the bank in fixed **steps** of the currency's loan step `S` (500 000 reais). Borrowing adds `S` to both
cash and debt (`1010:665e`); repaying subtracts it from both (`1010:668d`).

```
interest per game = round(0.05 * loan)                              # 1040:3865, category 0x18
limit             = rating * K * E * G * 20 000, rounded down to a multiple of S, at least S    # 1040:3899
```

The interest is **5 % of the debt for every game played**, league or cup, which is very expensive over a 14-round season. The
limit depends on the team rating and the morale ("the bank's confidence").

Computer clubs never borrow: the borrowing branch in `1010:3aad` can only run when the cash already covers interest plus wages,
which the entry test rules out. They repay steps while `cash - wages - interest > S`. Every loan in the sample saves is 0.

In the original's unregistered copy the loan dialog (*Pedir/pagar empréstimo*) is hidden; the port hides it too.

## Prizes

At the season end (`1020:2ae6`) the currency's three prizes are paid as ledger category 3: to the 1st division's **champion**, to
the **cup winner**, and to the club of **every top scorer** (every player tied on the most goals of the season). With reais:
20 000 000, 6 000 000 and 3 000 000. The season change was checked on the original's saves: apart from the last round's gate and
wages, the prizes are the only money that moves (52 of 52 clubs).

## The ledger

Every money movement goes through one routine (`1010:58bf(team, amount, category)`), which adds the amount to the cash
(`team+0x71`) and to a per-category season total:

| total | category | what |
|---|---|---|
| `+0x31` | 1 | gate receipts |
| `+0x35` | 2 | player sales |
| `+0x39` | 3 | prizes |
| `+0x3d` | `0x15` | player purchases |
| `+0x41` | `0x16` | wages |
| `+0x45` | `0x17` | stadium works |
| `+0x49` | `0x18` | loan interest |

On consecutive saves of the original, every club's cash change equals the sum of its ledger changes. As a new season starts the
totals move to "last season" (`+0x89`), last season's to "the season before" (`+0xa5`), and the totals restart at zero
(`1010:38a9`). The balance sheet (`TbalanceDlg`) shows the three seasons side by side; the *Finanças* page shows the current one.
The club also keeps its cash at the start of the season (`+0x75`).

## New-game money

A new game gives the club at master-list position `i` (see [The season](season.md#setting-up-a-new-game)) cash of
`base(i) * 50 000 * G`: 12 250 000 reais for the best club, 1 250 000 to 2 750 000 in the 4th division, 250 000 for the weakest
pool clubs. `F` starts at 1.0, `E` at 2.0, every wage at the player's value.

!!! note "Not decoded"
    * The reference total behind `F` (see above).
    * A ticket-price display detail for small currency units (`1018:11f3` uses the currency record's `+75` field and factor).
    * The value `+67` of the currency record, used by the player-search dialog.
