# Finances and transfers

Status: **partly decoded.** The verified pieces are implemented in `core/src/economy.rs` and `core/src/finance.rs` (wages, gate,
ticket price, new-game cash, stadium, ledger; the browser shows the user's finances); the rest is mapped below so the next pass can
start from the right functions.

## Verified and implemented
- **Economy index `E`** (global extended at `10c8:2b46`, saved in the `.e98` header at offset `0x28`; **2.0** in a new game, read from a real
  save — an earlier version of this document said 1.0). Every round
  `1020:0c41` does `E += 0.01·d`, clamped to `[0.5, 10]`, with three `Random()` reals `r1, r2, r3` (a real is
  `(seed_as_i32 + 2^31) / 2^32` after advancing the seed): `d = r1 + r2 − r3` if `year mod 100 < 50`, else `r1 − r2 − r3`.
  So the index drifts **up in the years 00–49 and down in 50–99** (a Y2K-style quirk; a 1998 game deflates, a 2026 game inflates).
- **Player value / wage** (`1010:7ff4`, stored in `slot+0x31`; the "price" column of the squad list):
  `value = G * 50 * round(sqrt(F) * E * 35 * 1000^0.04 * strength^1.04 / 50)`, clamped to `[50 G, 50000 G]`, with `G` the currency
  scale (`10c8:2d87`; **5** for the `Reais` saves) and `F` the team's economic factor. (The earlier text used `10^0.04` and `G` cancelling;
  the corrected formula reproduces every strength of a new-game save: 1 -> 500, 12 -> 6000, 13 -> 6750, 34 -> 18000 at `E = 2`, `F = 1`.)
  For an injured player the original keeps the previous stored value if it is higher.
  The same number is also the player's **wage**: every game each team pays `sum(slot+0x31)` (over slots without a sale cooldown,
  `5992`) as ledger category `0x16` (`5a0a`), verified to the unit on two teams after two rounds.
- **Loans**: interest is **5 % of the outstanding capital per game played** (help text of the loan dialog); the maximum loan
  depends on "the bank's confidence"; debt can be reduced at will or increased if allowed.

## Transfer market: how it is built (decoded in outline)
**Superseded (see the last section): AI clubs do trade with each other.** The auction dialog
(`TsellPlayerDlg`, `1008:2fec`, "Venda de jogador por leilão", offers in "milhares de contos") is where the *user* bids for a
player an AI club has put up; the "pending transfers" message belongs to the team editor. The strength erosion seen in long
simulations (see `season.md`) is therefore most likely intended: buying players is the user's counter-measure.

Money fields of a team object: `+0x71/0x73` cash (long), `+0x6d/0x6f` outstanding loan (long), `+0xc5` team rating (`4ec3`,
level `L = rating / 10`), `+0x79` economic factor (extended), `+0x83..0x86` players per position, `+0x88` squad size.
Slot fields: `+0x31/0x33` stored value (long, from `7ff4`), `+0x35` status flag (0/1/2), `+0x36` cooldown after a failed sale.

- **Who can be sold** (`1010:5a6a`, returns a reason code, 0 = fine): 1 squad below 12 players; 2 on cooldown ("só pode tentar vender
  este jogador novamente no próximo jogo"); 3 / 4 flagged; 5 the last goalkeepers (fewer than 2); 6 outfield count too low
  (`squad − goalkeepers ≤ 10`).
- **Sale priority score** (`1010:5de1`; the AI puts up the sellable player with the **lowest** score):
  `score = (5/(aggression+1))^(1/4) · T[L][pos]/(count[pos]+1) · T[L][5]/(squad+1) · strength / (rating+1)`, times a factor
  for slot status 0 or 1. `T` is a table at `DS:0x300`, rows for `L = 0..5` as `[GK, DEF, MID, ATT, 0, total]`:
  `L0 [3,8,7,3,0,22]  L1 [3,8,7,4,0,22]  L2 [3,8,8,5,0,23]  L3 [4,8,8,5,0,24]  L4 [4,9,8,5,0,25]  L5 [4,9,8,5,0,26]`
  (exact bytes of the binary: target squad shape, growing with the level).
  So overstaffed positions, large squads, weak and aggressive players go first.
- **When an AI club sells** (`1010:3aad`, after each game, and only from round 2): when its cash is below the summed stored
  values of its sellable players (`5992`), or with probability 1/20, or when the squad is over 20 and the club is not in the
  default state; the asking price comes from `1040:371a` (`≈ strength^1.04` scaled) and `5b37` opens the auction.
  The same routine repays loan steps (`668d`: `cash −= step`, `loan −= step`) and handles the maximum loan (`3899`).
- **What an AI club accepts** (`1010:63fa`): see *Direct purchases* below (decoded; only the registered copy's *Procurar* reaches it).

## Mapped, not decoded
- The bid acceptance in the auction dialog, the user's own selling dialog
  (`TsellPlayerDlg` from the menu *Jogador > Vender*), and salary renegotiation (`7ff4` vs the stored `+0x31`).
- **Gate receipts**: `1040:35e1` (called from `1010:0875`) clamps attendance revenue using `G` and the economic factor; ticket
  price is a user setting ("Preço dos bilhetes"), stadium data in `TStadium`. **Per-round coach finances**: `1010:6e48`.
- **Initial money and `F`**: `1028:0373` initialises `team+0x71..0x7d` and `+0x79` from the list position; the save header holds
  `0x0007A120` (500 000) next to the currency name. **Currency**: `TcurrencyDlg`.

## Consequence for the port
Because AI clubs never trade with each other, the demo's world is faithful without a market; the user's market (buy via AI
auctions, sell, loans, tickets, salaries) is the missing gameplay layer and needs the pieces listed above.

## Findings from the saves (session 2) — ledger, cash, F, gate, stadium
**Team ledger** (`1010:58bf(team, amount, category)` adds `amount` to cash `+0x71` and to a per-category total):
`+0x31` category 1 gate receipts, `+0x35` category 2 player sales, `+0x39` category 3, `+0x3d` category `0x15` player purchases,
`+0x41` category `0x16` wages, `+0x45` `0x17`, `+0x49` `0x18` loan interest. In the file they are `long`s at (standings-block start
`level byte` + `mem - 0x15`). Other team fields: `+0x6d` loan, `+0x71` cash, `+0x75` cash at the start of the season (long),
`+0x79` economic factor `F` (extended), `+0x83..0x88` players per position + squad size.
- **Initial cash** (`1028:0373`): `base_strength(i) * 50000 * G` where `i` is the position in the master list (so 12 250 000 for the
  best club, 250 000 from `base 1`, the fourth division 1 250 000..2 750 000). `F` starts at 1.0 (`27fb`).
- **`F` is a per-division factor, smoothed each round** (`1010:6e48`, a division method): `ratio = (sum of the AI teams' cash) / ref`
  (`ref` = a long of the division, `+0x1f`), floor 0.1; each team's `F = 0.8 * F + 0.2 * ratio` (`1068:115e`). `ref` itself is not
  decoded (D1..D3 start at exactly 1.0, D4 at 219/224). After two rounds the sample saw 1.02097 / 1.01196 / 1.00244 / 0.93451.
- **Gate** (game record `+0x40` attendance, `+0x36` ticket price): `gate = attendance * price` to the home club (cup games: split).
  `price` for a division comes from `1040:35e1(avgF, flag, avgRating)` called by `1010:0875`:
  `a = max(avgF, 0.1)`; `price = round( (G*E/sqrt(a)) * avgRating / 5 * (1 + 0.5*[flag==1]) ) / 2`, floored at `G/2` and rounded to a
  multiple of the setting `DS:2d9b`. Against the sample save the flag-less price is within 3 % for divisions 1-3 (44 / 32 / 18
  recovered from the gates) and ~15 % low for division 4 (7 against ~8; the sample shows per-game variation there, so the price
  may also depend on the game); the `flag == 1` factor 1.5 overshot by 30 % when applied to the user's division, so the port omits it.
  `attendance` (`1010:b0a1`): `w = 3*rating(home) + rating(away)`; `p = (w + 3*rand) * sqrt(K_home) * sqrt(K_away) * 200`;
  `att = round(p^1.04 * (1 + 0.3*[cup]))`, then clamped to the stadium capacity.
- **Stadium** (team `+0x12` object): `+4` level (0..24), capacity = level * 5000, initial level `max(1, round(base * 0.3))`; `+5` Real48 is
  an exponential average (weight 0.2) of the filling ratio, level increases when it is high (`c030` / `c081`, resets to 0.6).
- **Coach sacking** (see `season.md`).
- **Correction — AI clubs do trade with each other.** In the sample after two rounds Palmeiras booked a sale of 5 171 000 and
  São Paulo a purchase of exactly 5 171 000; other clubs booked sales (Sport 1 352 000, Ceará 592 000) with no league buyer. So
  the market is **not** user-only: AI clubs buy through the auction too (the earlier conclusion came from not finding a direct
  AI-AI routine). The routines that decide who buys (`3aad`, `5b37`, `63fa`) still need to be decoded for the port to
  reproduce the strength drift this causes.

### Implemented in the port (`core/src/finance.rs`, `world.rs`, `season.rs`)
Every club has cash, ledger totals, a stadium level and `F`. Per league game (order of `1010:9396`): attendance (one `Random` real), gate to
the home club, then each club pays its wage bill. Cup ties split the gate. `F` is smoothed each round with
`ratio = cash of the AI clubs / division total at the previous round` — this is an approximation of `6e48` (the sample's fourth division
could not be reproduced exactly). Not modelled: transfers (purchases/sales), loans, stadium expansion, the user's price setting, prizes.
New games and imported `.e98` saves start with the original's cash (the import also reads cash, loan, `F` and the season ledger).
Checked against the sample save: first-division gates 2.0-2.9 M (sample 2.2-2.6 M), wages -532 k per game for the best club (sample -471 k).

## The computer-managed transfer market (decoded this session, `core/src/market.rs`)
Everything below was read from the disassembly and checked against the original's day saves (`tools/capture/days.sh`).
- **Who sells** (`1010:3e9c` after every round, per club of the next round's games, home then away; `3aad` inside it). Needs the
  round >= 2. Each club: (a) if its coach has coached more than one game: renegotiates the wage of the **most overpaid** player
  (stored wage / current value > 1.2) and of the **most underpaid** one (< 0.8) to the current value, marking both `fresh` (`slot+0x16`);
  (b) if its cash is negative: puts up the lowest-scoring sellable player at any price; (c) puts a player up for sale if
  `3 * cash < wage bill`, else with probability 1/20 (`Random(20) == 0`), else when it has exactly five foreigners and a squad of 21+.
  The player is the sellable one with the **lowest** `5de1` score (only a foreigner when the club has five foreigners).
- **Who is sellable** (`5a6a`): squad >= 12, not `fresh`, at least 2 goalkeepers kept, outfield count stays above 10. `fresh` is set
  when a wage is renegotiated or a player changes club, and **the season change clears it for every player**: in the original's day
  saves the flagged players grow from 15 to 252 over 2026 and are 0 in the first save of 2027 (no routine writes the byte back to 0,
  so the season change probably rebuilds the slot records; Donizete, sold on the cup-final day, was free again in 2027 and sold twice).
  So every player can be sold once a season; the simulation keeps 11-27 transfers a season over twelve seasons (`examples/market.rs`;
  before the clearing the market died out, ~1 transfer a season after ten).
- **Asking price** (`3aad` -> `5b37`): `round(371a * [2b50] * 1.6)` floored to a multiple of 1000, the *Preço base* of the auction dialog
  (`2b50` is the header's extended 1.0 in every save seen). `371a` = `round(strength^1.04 * G * E * f * 2000)`,
  `f = 1 - 0.2 aggr/5 - 0.2 prone/10 + 0.2 g/10`. The two sales of `3e9c` itself (a club in debt) ask 0. Checked on the dialog: the
  prices shown (Pierkaski, strength 46: 1 554 000; Maurício Pantera, 11: 319 000) need the factor 1.6 (without it `f` would be above its
  maximum 1.2).
- **Auction** (timer loop `1008:3600` / `373e` / `351a` / `39c7`): every club except the seller, in master-list order, that has
  squad + pending < 24, a foreigner quota left (5, the Bosman / PLOP country groups count as domestic), and cash >= the asking
  price bids `5f77`; the highest floored bid >= the asking price wins (the earlier club on a tie). The winner pays its bid to the seller,
  the player gets the wage `7ff4` computed in the seller's economy, and is marked `fresh`. The dialog shows only the asking price and
  the result ("Vendido ao X por Y"), not the bids.
- **Bid** (`5f77`, checked instruction by instruction with `5bb1`, `371a`, `4ec3` and `5992`): `0` when the position is already at its
  target (`SHAPE[rating/10][pos]`); `X = 371a * need`, with
  `need = (strength/(R+5) * (T+1)/(count+1) * Ttotal/(squad+1))^(1/4) * R^0.2` times 1.5 for a lone goalkeeper, times
  `1.5 + 0.05 foreigners` (same country), `1.15 + 0.05 foreigners` (same group) or 1 (foreigner), times 1.1 for a star (`5bb1`);
  then `r = (cash - X) / (wages + value + 1)` (`value` = `7ff4` of the player in the seller's economy): `r < 3 -> X = cash - 3 (wages + value)`,
  `r > 5 -> X *= ln(r) / 1.6`; a club with wages > 100 and `cash - X > 10 wages` multiplies by `1 + ln((cash - X)/wages - 9)/10`; the bid
  is `max(0, min(cash - X - 5 wages, X))`, floored to 1000 (`351a`). The wages are `5992`, the sum of `slot+0x31` over the slots whose
  `+0x36` is 0. (The injured/banned branch at `626f` writes into the result variable, which the final `min` overwrites: dead code.)
- **Calibration** (`core/examples/auctions.rs` over the 16 day saves of `tools/capture/days.sh`, 36 computer-to-computer auctions; the
  clubs as the later save has them with the sale undone, the player at the strength of the transfer record): with the binary's 1.6 the
  buyer's bid is 24 % above the fee on average (spread 21 %). Two variants fit equally well: the divisor **2.2** with the spare-cash
  factor (fee / buyer's bid 1.005, spread 6.7 %, the buyer has the best bid in 23 of 36), or 1.6 without that factor (1.008, 7.1 %, 23).
  Their residuals cannot be told apart in this data (the spare cash of the buyers covers too narrow a range), so the port keeps the
  decoded structure with 2.2 (`BID_DIVISOR`). The gap is not explained: the routines and their constants are the same in the
  decompiled build and the one the captures ran (see `docs/installation.md`), and neither the buyers' cash before the round, the day's
  order of sales, the ask or the market value as `value`, nor a larger wage bill fits better. On the sample save
  (`core/examples/bidcheck.rs`) 2.2 picks the three winners with fees within 5 % (4.99 M against 5.17 M, 565 000 against 592 000,
  1.42 M against 1.35 M).
- **Rating accumulators**: the same `3e9c` pass clears `slot+0xc..0x13` for every club whose coach has coached more than one game.
  In the sample all but two clubs had zero accumulators after round 2 although round 2's evolution still used them, so the port
  clears them after the evolution (`end_of_round`). (This corrects `match-engine.md`, which said they are never reset.)
- Not modelled: the human taking part in auctions (the original asks the user), loans, stadium expansion, wages demanded by human
  coaches, the second `3e9c` branch for human coaches.

## The user's side of the market (port design, `World::human_bid` / `human_sell`)
The original stops the game with an auction dialog and lets the user raise bids while the computer clubs bid in a timer loop. The
port keeps the same rules but makes it asynchronous, because every game runs in one go:
- **Buying**: when a computer-managed club puts a player up (same triggers and asking price as above) and the user's club could take
  part (cash >= asking price, squad and foreigner limits of `633d`), the auction stays open as a *listing* with the computer's best
  bid. The user may bid any multiple of 1000 above both the asking price and that bid; a winning bid settles at once (fee, wage = value,
  player marked `fresh`). Open listings are saved and settle at the start of the next round with the computer's best bid.
- **Selling**: the user names a price; the computer clubs bid at once (the original `5a6a` rules apply: >= 12 players, 2 goalkeepers,
  more than 10 outfield players, not `fresh`); the best bid >= the price wins.
- Not done: the original's refusal cooldown (`slot+0x36`), user-side loans, the user's price/salary negotiation.

## Loans and stadium works (decoded; `finance.rs`, `World::human_borrow/repay/expand`)
- **Interest**: every game each club pays `round(0.05 * loan)` (`5a37` -> `3865`, category `0x18`).
- **Loan step** `S` (`10c8:2d7f`): the saves' header holds 500 000 next to the currency name; the port uses `100 000 * G`. Borrowing adds `S` to
  cash and loan (`665e`), repaying subtracts it (`668d`). **Limit** (`3899`): `rating * K * E * G * 20000`, rounded down to a multiple of `S`
  (at least one step).
- **Computer clubs never borrow**: the loop in `3aad` only borrows when the cash already covers interest plus wage bill, which the entry
  test rules out (dead code); they repay while `cash - wages - interest > S`. In the sample every loan is 0.
- **Stadium**: `stadium+5` is an exponential average of `attendance / (capacity + 1)` with weight 0.6 on the old value (`c0c0`). At >= 0.8
  and level < 24 (`c030`) a computer club builds 5000 seats when `cost + 2 * wage bill < cash` (`3aad`); the filling resets to 0.6
  (`c081`) and `cost` (category `0x17`) is `max(50000, 50000 * round(sqrt(bracket) * E * 35000 * G / 50000))` with bracket 1 / 2 / 3 for level
  < 6 / < 14 / >= 14. The user does the same from the finance card.

## Direct purchases, prizes (decoded in the second pass)
- **Buying from a computer club** (`1010:63fa`, the *Procurar jogadores* dialog `1048:29f0`): a club sells a player only when it is sellable
  (`5a6a`), its coach is not human and the player's sale score is at most 1.95 (a surplus player). Price: `market value * 3.5 * score`;
  for a buyer of the same country `(p + 30000) * 1.6`, of the same group `(p + 15000) * 1.3`; rounded down to 100 000 (>= 2 M),
  50 000 (>= 1 M), 25 000 (> 500 000), 10 000 (> 100 000) or 1000. The user pays at once (seller `+`, buyer category `0x15`), the
  player joins with the end of the round (`1020:0aae`) with the wage `7ff4` of the buyer's economy, `fresh` set; `team+0xc7/0xc9` count
  the pending arrivals against the 24-player and 5-foreigner limits.
- **Season-end prizes** (`1020:2ae6`; the amounts live in the currency record at `DS:2d8f..2d97`, loaded from the save header — in the sample
  `20 000 000`, `6 000 000`, `3 000 000` with `G = 5`): the first division's champion, the cup winner and the club of every top scorer
  (all players tied on the most goals; the tally `slot+0x14`, which also ranks the line-up, starts again each season in the port).
  Header record (offsets of the `.e98`): `0x7b` loan step 500 000, `0x7f` 4, `0x83` `G`, `0x87` 1, `0x8b` the three prizes, `0x32` the ticket
  rounding unit (extended 1.0), `0x28` economy index.


## Currencies (`TCurrencyDlg`, `economy::CURRENCIES`)
The five records at `DS:0062` (101 bytes; the chosen one is copied to `DS:2d40` and into the `.e98` header at `0x3c`):

| Currency | word | loan step | `+67` | `G` | `+75` | prizes (champion / cup / scorer) | factor |
|---|---|---|---|---|---|---|---|
| Escudos (contos) | contos (` esc.`) | 100 000 | 1 | 1 | 1000 | 3 000 000 / 1 000 000 / 500 000 | 0.5 |
| Euros | euros | 500 000 | 4 | 4 | 1 | 15 000 000 / 5 000 000 / 3 000 000 | 2.0 |
| Dólares | dólares | 500 000 | 4 | 5 | 1 | 20 000 000 / 6 000 000 / 3 000 000 | 2.0 |
| Reais | reais | 500 000 | 4 | 5 | 1 | 20 000 000 / 6 000 000 / 3 000 000 | 2.0 |
| Libras | libras | 250 000 | 4 | 4 | 1 | 10 000 000 / 4 000 000 / 2 000 000 | 2.0 |

The loan step is what `1010:665e` adds to the cash and the debt; `G` scales values, wages and new-game cash; the word is written after
amounts by `1040:335a`. The dialog shows the last currency used first (`elif98.ini`, `[Options] currency`).

## Checked on consecutive saves of the original (2026-10-27, `core/examples/daycheck.rs`, `tools/capture/days.sh`)
The original's own saves one league round apart (option *Gravar o jogo: Todas as jornadas*; cup days are not saved separately):
- **Wages**: the ledger's wage change is exactly the squad's wage bill per game (32 of 32 clubs on a clean round; differences only
  where a wage was renegotiated between the games of a multi-day interval).
- **Ledger**: seven totals at `team+0x31..0x49` (gate, sales, prizes, purchases, wages, works, interest); the cash change equals their
  sum for every club. (The import read purchases from `+0x39` until this check.)
- **Ticket price**: always even, `2 * round(round(x) / 4)` with `x = G E round(meanRating) / (5 sqrt(meanF))` from the division's
  state as the round starts: 43 of 43 division-rounds of the 2026 season (prices 44/32/18/8 at the start, down to 34/24/16/6). An
  intermediate fit with a lagged F and a whole-number price (15 of 16) was a rounding artifact.
- **Cup ties**: one price for every tie of a cup day, the league formula over that round's clubs times 1.5 (`35e1`'s flag): 30 with 32
  clubs, 36 with 16; the gate is split half and half between the two clubs.

## The season change (checked 2026-10-27, `core/examples/seasonend.rs`)
Two saves of the original, the last before the 2026 season's final round and the first of 2027:
- **No other season-end money**: per club the cash changed by the old season's last round (its gate and wages) plus the prizes
  (`1020:2ae6`: the champion 20 000 000, the cup winner 6 000 000, the top scorer's club 3 000 000) and nothing else: 52 of 52 clubs.
- **The ledger rolls over** (`1010:38a9`, called for every club of the five lists by `1020:0002` as the new season starts): the season's
  seven totals (`+0x31`) move to last season's (`+0x89`), those to the season before (`+0xa5`), and the totals, with the table, start at
  zero. The prizes are booked before, so they show in last season's column.
- **Balance sheet** (`TbalanceDlg`, `1028:3e64`; rows by `TThreeBalances` `1050:2bf1`/`21d0` drawn by `1068:0ad6`): a click on the
  Finanças list (`ShowBalance`, `1018:3c45`) or the squad window's balance button (`1048:1bb7`) shows the three ledgers side by side under
  the club's name, in the club's colours. The Finanças page draws the same rows with only this season's column.

