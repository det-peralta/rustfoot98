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

## Mapped, not decoded
- **Transfer market AI**: `1010:3e9c` (11.8 KB, per team after each game: `5de1`, `5a6a`, `5b37`, `66dc`, `7ff4`).
  Transfers happen by **auction** ("Venda de jogador por leilão", offers in "milhares de contos"), a team cannot sell below
  squad minimums ("Não pode vender mais jogadores / guarda-redes / jogadores de campo"), a player cannot be put up again
  until the next game, pending transfers execute when the next team is opened, and a player refusing a raise "será posto à
  venda em leilão". The options dialog lets the computer manage salaries or make offers to all/only stronger players,
  decline auctions, or hide one's offers.
- **Gate receipts**: `1040:35e1` (called from `1010:0875` when games are created) clamps attendance revenue using `G` and
  the team economic factor; ticket price is a user setting ("Preço dos bilhetes"), stadium data in `TStadium`.
- **Per-round coach finances**: `1010:6e48` (sums `team+0x71` over the division, rescales `team+0x79` with `1068:115e`).
- **Initial money and the factor `F`**: `1028:0373` initialises `team+0x71..0x7d` and `team+0x79` from the list position `i`;
  `1010:0875` also derives a division-wide factor. The save header holds `0x0007A120` (500 000) next to the currency name.
- **Currency**: `TcurrencyDlg` (name pair such as `Reais` / ` reais`, scale `G`).

## Why it matters
Long runs of the demo drift weaker than the real game (see `season.md`). The decoded evolution pulls squads toward their
window, so something else must counter it; the transfer market (clubs buying/selling players, auctions) is the likely candidate.
