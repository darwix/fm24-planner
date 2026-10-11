# Parser rules: contracts and clubs

How the FM24 save (`game_db.dat`) is read for contract, wage, expiry and club. Fitted on
`Meong Racun - 2052 (v03).fm` and checked against in-game exports. Code: `src/adapters/parser/records/`
(`contract.rs`, `clubs.rs`, `teams.rs`). Bump `PARSER_VERSION` (`src/adapters/parser/save.rs`) when output changes.

## Definitions (match the game)

- **Expired** = no wage and no contract end (`wage IS NULL AND contract_end IS NULL`). The game's
  "Contract Expired" list (16,277 in the test save) = Non-Contract (10,047) + unattached free
  agents (1,719) + amateurs and out-of-contract players at a club.
- **Non-Contract** players are expired. **Amateur** is expired. **Month-to-month** is expired
  (the game shows a salary but no expiry; we drop the wage).
- **Part Time**, **Youth**, **Full Time** (including at £0 wage) and MLS types are contracts.

## Contract rows

Before each person record sits a list of rows for that person:

```
[person id][team][00 x4][wage u32][01 ?? 00][date u32 at +19] .. [type byte at +25]
```

- `team` is a team number (team ordinal + 1), the same numbering as the ability block's team.
- Date at +19: `FF FF FF FF` = signed contract row; `01 00 6C 07` (1900-01-01) = blank.
- **The first row of the list is the contract the game shows** (the parent club's for a loan).
  Wage and end come from it.
- Later rows: FF-dated = older or loan rows; blank-dated paid rows = other clubs' approaches.
- The list can start up to ~3,200 bytes before the person; we read 4,000 (`OWN_WINDOW`).

## Terms block (contract end)

Every contract has a 28-byte block before its row:

```
[04 00 00 00][u32][year or 1900][00 00][x][v1][v2][end day u16][end year u16][start day][start year]
```

- `v1`, `v2` are `FF FF FF FF` when unset, otherwise amounts (clauses).
- The year field is not always a start year (can be later than the end).
- End day is a day of year; mask with 511. So is the start day after it (Breant: `EF 32`).
- The end is read from the block nearest before the first row.

## Special cases

| Case | Bytes | Result |
|---|---|---|
| Free agent | ability team `FFFFFFFF` | expired, clubless |
| Contract ended | end before the game date | expired |
| Month-to-month | `[FF x4][02 00 6C 07][..][start year]` within 450 bytes before the paid row (searched in the wide window) | expired |
| Amateur | first row zero wage, date not blank, no end in an unset-start block | expired |
| Full time at £0 (Sarı) | first row zero wage, block `[1900][00 00][0][FF x4]` with an end | wage 0, that end |
| Non-contract | last own-club row zero wage, date `FF`, and type byte 3, or (more than 800 bytes back, tail `FF FF ?? 00` or `FF x4`, and no paid row before it) | expired |

## Transfer and loan listed

- **Transfer listed**: the date just before the list's first contract row (`[day][year]`, year
  at row - 3) is the listing date; 1900 (`6C 07`) when not listed.
- **Loan listed**: bit `0x02` of the flags byte 35 bytes into the terms block before that row.
- A player without a contract (expired, non-contract, amateur, free agent) is treated as not
  listed: 335 non-contract players carry a stale `0x02` from an old contract. Known miss: Bernd
  Bochmann (Würzburger Kickers II) is non-contract and loan listed in the game, but neither of
  his rows' flags bytes has `0x02` (both `0x40`); where a non-contract listing is stored is not
  decoded.
- Checked against in-game filtered exports (transfer listed 1,052, loan listed 2,321): every
  listed player is flagged (one loan miss). We flag 105 more as transfer listed (55 show `Lst`
  in the full export; the rest are mostly out on loan, which the game's listed searches leave
  out: Antonio Guimaraes, Mohamed Cherkaoui and Aritz Ruiz confirmed listed) and 75 more as loan listed (the
  game's loan-listed search leaves out players already out on loan: José Alberto, Melbourne
  City, is loan listed and on loan).

## Clubs

- Club record: `[id][uid][uid][00][nation][FF x4][nation2][nation2] .. [name len][name][short len][short]`.
  The nation must repeat, or the second one must repeat itself (clubs in another nation's league:
  Toronto FC, Vancouver, CF Montréal, DPMM, Vaduz).
- Each club record lists its teams after its names: `[FF x4 or club id][00 00 00][count 1-4][team numbers]`.
  Preference: the list whose first team's own record names the club, then the FF-marked one,
  then a club-id-marked one. Teams listed by two clubs are dropped.
- A player's club, first rule that answers: club list for the ability team, the team record's
  club, a roster the player is on, the club most contracts of that team name.

## Checking a parser change

Ground truth is an in-game Player Search export of all players (Ctrl+P, Web Page) with
Name, Club, Contract Type (F/PT), Salary, Expires. The test save's export has 54,383 players.

1. Build the server with a separate `CARGO_TARGET_DIR`, run it on another port with its own
   `FM_SAVELENS_24_DATA_DIR`, import the save.
2. `python -I scripts/check-contracts.py <export.html> <new snapshot.sqlite> [<previous snapshot.sqlite>]`
   matches export rows to players by name (or surname-first) + club and counts, per contract
   type, expired agreement, equal ends and wages within the game's display rounding (max of
   £15 or 6%). With a previous snapshot it lists every player whose wage or end changed; each
   must be a fix.

Baseline at `fm24-rust-49-fm` (54,308 matched): expired 100%; end 38,031 of 38,031; wage 38,019
right, 87 month-to-month with no wage (by design), 0 wrong.

## Notes

- In-game filtered exports (expired, unattached) can silently leave players out; the full export does not.
- Wages are stored before display rounding (game shows £550,000 for 538,133).
- Known miss: Carlos Villar's display name ("Carlos").
