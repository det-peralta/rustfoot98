# Finances and transfers

Status: **partly decoded.** The verified pieces are implemented in `core/src/economy.rs`; the rest is mapped below so the
next pass can start from the right functions.

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
**Key finding: the market is user ↔ AI.** No routine moves a player between two computer-managed clubs. The auction dialog
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
- **What an AI club accepts** (`1010:63fa`): requires `5a6a == 0`; computes the player's score; the demanded price is tiered with
  thresholds near 30/31, 15/16, 7/8 and 1/2 millions, +country adjustments, and a random rounding step.

## Mapped, not decoded
- The exact price tiers and rounding of `63fa`, the bid acceptance in the auction dialog, the user's own selling dialog
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
  multiple of the setting `DS:2d9b` (sign of the helper direction unverified).
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

