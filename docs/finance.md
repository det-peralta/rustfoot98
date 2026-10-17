# Finances and transfers

Status: **partly decoded.** The verified pieces are implemented in `core/src/economy.rs`; the rest is mapped below so the
next pass can start from the right functions.

## Verified and implemented
- **Economy index `E`** (global `Real` at `10c8:2b46`, saved in the `.e98` header, 1.0 in a new game). Every round
  `1020:0c41` does `E += 0.01·d`, clamped to `[0.5, 10]`, with three `Random()` reals `r1, r2, r3` (a real is
  `(seed_as_i32 + 2^31) / 2^32` after advancing the seed): `d = r1 + r2 − r3` if `year mod 100 < 50`, else `r1 − r2 − r3`.
  So the index drifts **up in the years 00–49 and down in 50–99** (a Y2K-style quirk; a 1998 game deflates, a 2026 game inflates).
- **Player value** (`1010:7ff4`), in units of the currency scale `G` (`10c8:2d87`), which cancels out:
  `value = clamp(50 · round(√F · E · 35 · strength^1.04 · 10^0.04 / 50), 50, 50000)`, where `F` is the team's economic factor
  (extended at `team+0x79`). For an injured player the original keeps the previous stored value if it is higher.
  Note: the price column shown in the squad list of a real save is exactly `500 × strength`, so that column is *not* this value;
  `7ff4` feeds the salary/transfer logic (`slot+0x31`).
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
