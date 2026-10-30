# Files

Elifoot 98 keeps its data in a handful of small binary files: one file per team (`.EFT`), text tables (`.TXE`) for countries,
referees and country groups, and one save file per game (`.e98`). All of them share one simple string cipher. This page explains
each format at the level needed to understand what the game stores; the byte-by-byte layout of the save is in
[e98-format.md](../e98-format.md), and the history of how the formats were decoded is in [findings.md](../findings.md). The
port's readers are `core/src/cipher.rs`, `eft.rs`, `txe.rs` and `e98.rs`.

## The string cipher

Every text in the game's files is a **Pascal string**: one length byte followed by that many bytes. The bytes are delta-encoded:
each stored byte is the previous stored byte plus the plain character (mod 256), and the length byte serves as the "previous
byte" of the first character:

```
enc[-1] = length
enc[i]  = enc[i-1] + plain[i]   (mod 256)

decoding: plain[i] = enc[i] - enc[i-1]
```

The characters are in the game's single-byte (Windows Latin-1 style) encoding. `.TXE` tables and the save's description apply
the transform **twice**.

## Team files (`.EFT`)

Each team is one file in the `EQUIPAS` folder, about 300 to 600 bytes. All 195 shipped files parse to their exact length with
this layout:

| part | content |
|---|---|
| header | 50 bytes: the magic `EFa` and zeros |
| full name | ciphered string, for example `FIGUEIRENSE F.C.` |
| short name | ciphered string (at most 20 characters); this is what the table shows and what the referee's checksum reads |
| colours | background RGB + 1 pad byte, foreground RGB + 1 pad byte |
| country | ciphered string, a country code such as `BRA` |
| level | 1 byte, 1 to 20 |
| squad size | 1 byte, 14 to 20 in the shipped files |
| pad | 1 byte |
| players | for each player: nationality (ciphered string), name (ciphered string), position (1 byte: 0 GK, 1 DEF, 2 MID, 3 ATT), pad byte |
| coach | ciphered string, the coach's name |

A team file holds **no strength** for its players. Strength is assigned when a new game starts, from the team's place in the
master list, which in turn comes from the file's **level** (see [The season](season.md#setting-up-a-new-game)). The hidden
attributes come from the players' names (see [The match engine](match-engine.md#players-and-their-hidden-attributes)). So the
only things that make a team strong in a new game are its level and the countries chosen; within a level, directory order breaks
ties.

The team editor (`EDITEQ.EXE`) writes the same layout; the port's editor does too (`eft::write`). A team must be able to field at
least one of the game's twelve formations to be used.

## Text tables (`.TXE`)

A `.TXE` file is a sequence of double-ciphered Pascal strings. The first is a title, the rest are rows:

| file | rows |
|---|---|
| `COUNTRY.TXE` | 217 entries `CODE Name` |
| `REFEREE.TXE` | 182 entries `CODE Name`: the referees, with their country code |
| `CTRGROUP/BOSMAN.TXE` | the country codes under the Bosman rule (title `Bosman`) |
| `CTRGROUP/PLOP.TXE` | the Portuguese-speaking countries |

The two country groups decide which foreign players count as "same group" in the market (see
[The transfer market](market.md#which-player-a-club-sells)). Flags are bitmaps in `FLAGS/*.BMP`.

## The save file (`.e98`)

Games are saved in `JOGOS\*.e98`. The original has no separate save command: *Elifoot > Sair* saves on exit (*Sair sem gravar*
discards), and the option *Gravar o jogo* saves automatically when the main window appears (for example every round). An empty
file is created as soon as a name is chosen for a new game. A new-game save of the sample was 83 676 bytes.

The save was decoded from the writer `1020:1796` and the loader `1020:1d8d`, and checked on real saves: a structural walker
(`tools/e98/strict.py`) consumes every local save **to the last byte**. Integers are little-endian.

### Layout in order

1. **Header.** A description string (double-ciphered, for example `"joga1 e joga2\r17/10/26 21:03"`: the human coaches' names and
   the time of saving), then fixed fields: the day counter (`2b40`), the id of the human club whose main window was open when the game
   was saved (`271e`: where the interrupted market of the day resumes), the "league day already played" flag (`271a`), the **year**, the **economy index** `E` (an 80-bit extended real; 2.0 in a new game), the
   ticket price rounding unit (1.0), and the 101-byte **currency record** (currency names, loan step, `G`, the three prizes). The
   port reads that club id at offset `0x1f`, the year at `0x24`, `E` at `0x28` and the currency at `0x3c`.
2. **Lists**, each a 16-bit count followed by its elements:
    * the coaches (id, name, flags, score, titles, profile, and each coach's history events);
    * the **referees** (id, free flag, name, country);
    * the **master team list**, the full team records (below);
    * the clubs still in the **cup** (team ids);
    * the four **divisions** (a cash total, the base level, a name, the list of team ids);
    * the **current games** of the round (competition kind and name, the two team ids, the ticket price, attendance, the referee's
      id and the two bonus values);
    * the news lists, the **champions** list and the **transfer log** (player, buyer, fee, strength).

The team order in the file is not the table order and not the division order. A reader must locate records by structure, not
by fixed offsets.

### The team record

A team record (`1010:305a`) holds, in order:

* the team id and the embedded `.EFT` object (names, colours, country, level);
* the **standings** (wins, draws, losses, goals for and against, points) and the points-per-round array;
* the season **ledger** (gate, sales, prizes, purchases, wages, works, interest), and later the ledgers of the last two seasons;
* the per-game state: substitutions left, goals in the current game, the four sector strengths, annulled goals;
* the morale `K` (a 6-byte Real48), the coach's id;
* the **loan**, the **cash**, the cash at the start of the season and the economic factor `F`;
* players per position, foreigners, squad size, pending signings and pending foreigners;
* three lists of **calendar entries** (one per fixture: opponent id, venue home/away/neutral, the goals of both sides or
  `FFFF` while unplayed, attendance, a cup flag, the result kind: normal, extra time, won or lost on penalties, the round);
* the **squad**;
* the **stadium** (level and filling).

### The player record

Each player is an id, nationality and name (ciphered strings) and 48 bytes:

| tail offset | field |
|---|---|
| 0 | position |
| 1 | star flag |
| 2 | strength |
| 4 | rating accumulator, matchday status (out / starter / substitute), a flag |
| 12 | goals this season, the *fresh* flag |
| 15 | games of suspension left |
| 17 | aggression |
| 19 | games of injury left |
| 21 | injury proneness |
| 23 | talent `g` |
| 25 | stored wage (32-bit) |
| 29 | nationality class (same country / same group / foreign) |
| 30 | pending buyer (a sale that completes at the end of the round) |
| 32 | career history: appearances, goals, red cards, injuries |

Aggression, proneness and talent are stored, but they are always the values the name gives; the game recomputes them from the
name.

### What the port imports

`World::from_e98` builds a game from an original save: the clubs, squads (with status, goals, bans, injuries, wages and *fresh*),
standings, morale, cash, loans, `F`, the ledgers, the year and round, the economy index, the cup entrants, the division lists, the
coaches and their histories, the champions list, the transfer log, the referees with each game's referee, and the human coaches (who become the hot-seat players). No random seed has been found
in the save, so an imported game continues with a fresh seed (see [Random numbers](random.md)).

!!! note "Not decoded"
    * Some header fields and flags (for example a byte of each player's tail at offset 8 that is 1 or 2 in a new game and 0
      after a round) have no identified meaning.
    * The news records (small fixed records from which the game generates the text of each message at display time) are walked
      but not interpreted by the port.

## Other files

| file | content |
|---|---|
| `\equipas.ef2` | the bundled teams |
| `\treinad.ef2` | coaches |
| `JOGOS/DRLIST.ELI` | a 102-byte list; record layout not decoded |
| `elif98.ini` | sections `System` and `Options` (for example the last currency used) |
| `win.ini`, section `[Elias]` | the registration |
| `eli.cod` | an installation id drawn with `Random(20000)` (`1058:30ba`) |

## The port's own save format

The port does not write `.e98` files. Its saves are a small versioned **text** format (`core/src/save.rs`, first line
`ELIFOOT98-RS 1`): one record per line, fields separated by `|`. They store everything needed to continue a game identically: the
clubs with squads and standings, the pool, the year, round and economy index, the currency, the humans' plans, coaches and seats,
the market state (open auctions, pending purchases, observed players), the histories, a pending cup draw, and the **random seed**.
The results of the last round and the last season summary are not stored.
