# FM Squad Planner

New app built on how FM SaveLens 24 (https://github.com/MSalman5230/FM-SaveLens-24) parses Football Manager 2024 `.fm` saves.
Test save: `Meong Racun - 2052 (v03).fm` (~400 MB). `savelens/FM SaveLens 24/` holds the downloaded v1.4.0 portable release (GUI + local server mode). `fmsave` (Python) does not work: it targets FM26.

## Goals

1. Squad planner: squad screen with each player's abilities.
2. Add / remove players in the squad.
3. Replacement shortlist: players matching the profile of the one being replaced, prioritising contract expiring, transfer listed, cheap.
4. Scouting: preset lists (promising youngsters 18-23, contract expiring) plus custom lists.
5. Upload flow for the `.fm` save.

## Status: React app on a patched SaveLens backend (real wage, contract end, value)

**Layout**: the repo root is the SaveLens clone (Rust in `src/` (`domain/`, `adapters/` = parser + storage, `services/`, `error.rs`), `tests/`; no desktop app, browser only), patched by us (no licence, never publish). Our React app is `web/` (replaces the original SaveLens frontend; served by Vite, the Rust server is API-only and no longer embeds the build). Run web commands from `web/`. The old `savelens-src/` paths below now mean the repo root; its upstream `.git` and original `web/` are backed up outside the repo. `mock/` and `DESIGN.md` stay at the root.

`web/src/` follows `../react-template` (React 18 + TS, react-router, recoil, swr, axios; `pages/`, `components/`, `atoms/`, `services/`, `models/`, `utils/`, glob routes). Kept our own CSS (no antd). Dev server proxies `/api` to a SaveLens server on :4242.

The shipped SaveLens exe does not expose contract data, so the SaveLens clone (now the repo root) is patched: `src/adapters/parser/contract.rs` reads weekly wage (GBP, before tax), contract end and value (GBP) from the save; the search API returns `wage`, `contractEnd`, `value`. Build and run it instead of the shipped exe: `cargo build --release --bin fm-savelens-24-server` (from the root), then `target/release/fm-savelens-24-server.exe --no-open --port 4242`. Tests: `cargo test --release` (fixtures in `tests/fixtures`, real bytes of four players). Layout notes: wage = u32 after `[eid][team][00 x4]`, contract end = `(day,year)` 12 bytes after the u32 before an `FF FF FF FF` run, value = u32 right after the 54-byte attribute block. Not decoded: transfer-listed, loan-listed, sales value. Coverage is high for top players and thin for low-rated or unsigned ones.

Frontend: save list, import, club squad, player page, shortlist (CA match, tags contract expiring <=12 months and cheap <=GBP 5M) and scouting all use live backend queries (the backend filters in SQL, max 250 per page). Positions use the backend's labels (AMC, not AM). Money shows in GBP. Run (in `web/`): `npm run dev`, `npm test`, `npm run typecheck`.

### Backend search API (`src/adapters/storage/mod.rs`, `src/services/mod.rs`)

- `GET /api/snapshots/<id>/players?...` and `POST /api/snapshots/<id>/players/search` (JSON body, arrays become comma lists; same filters). Response: `total`, `page`, `limit` (max 250), `players`.
- Filters added by us: `contractEndMax` (YYYY-MM-DD), `valueMax` (+ `valueUnknown=1` keeps players with no value), `mainPosition` (comma indices of the best-rated position), `excludeIds` (max 5000), `club=-1` (free agents), `ageMin/ageMax`, `sort=pa`, `page`.
- `main_position` column = best-rated position. `positions[0]` (the UI's `p.p`) is NOT the main position.
- A market value of 0 in the save means unknown: stored as NULL (PARSER_VERSION `fm24-rust-8-fm`).
- Snapshots: one SQLite file per import in `<data>/snapshots/<id>.sqlite`, id = sha256(path|size|mtime|PARSER_VERSION). A successful import deletes the older snapshots of the same save path (best effort; a file still open on Windows goes on the next import).
- Rust layout: `src/domain` (roles, rating_systems, hybrid), `src/adapters` (parser, storage), `src/services`, `src/error.rs`; `lib.rs` re-exports the old module names.

### Scouting flow in the React app

Filter, swipe, shortlist review, act. Built: filter and swipe.
- `/scouting` = filter screen (`components/Brief.tsx`): pitch position picker (multi), age sliders, max price chips, contract chips (Any, Expired = free agents, <=12, <=6 months), live match count from `total`, "Browse as list", "Start scouting".
- `utils/player.ts`: `scoutQuery` builds the filter params, `swipeBody` pages it (50 per page) and excludes passed ids.
- `/scouting/swipe` (`pages/scouting/swipe.tsx`): swipe right = sign (kept in `keptState`, not persisted), left = pass (ids in local storage `fm:passed:<snapshotId>`, skipped next time; "Reset passed" clears). Tap the card to open the book with the player's real attributes. Next page loads when 5 cards are left.
- Not built: shortlist review (mock `scout-e-shortlist.html`), add-to-squad from the kept list, role-fit panel (needs a role catalog in the web app), persisting kept players. Mocks D (swipe) and F (filter) are in `mock/`.
- Styles: one `.css` per page or component, imported by it; `index.css` holds the shared base only.

### Squad behaviour in the React app

- **Managed club**: the backend snapshot returns `managedClubId`, read from the club name in `save_game_summary.dat` (only clubs that have players are matched; ambiguous or no match = none). The upload page prefills the club box with it.
- **Open squad** shows a loading state while the squad is fetched.
- **Pitch**: drag a starter onto another starter to swap spots, or onto a dashed Empty slot to move there. Removing a starter promotes the best cover who can play that group; the slot is Empty only if nobody can.
- **Position groups**: a player has one main position `p` plus `pos` (every position he can play) and `cover` (extra groups he is counted in). `inGroup`/`need` in `utils/logic.ts` count both. The depth panel shows "+ Add cover" always, even over target.
- **Find replacement** and **Add cover** share `/shortlist`. "From your club" lists squad players who can play the group (Promote, Swap spots or Use as cover); "Shortlist" is the outside search. Squad players already in "From your club" are left out of the outside list; others stay so they can cover several groups.
- **Match score**: `50 + 2 x (CA - target CA)`, clamped 1-99, so higher CA ranks first. Add cover target = squad average CA - 10.
- **Persistence**: local storage keeps only the arrangement (ids, group, pitch spot, covers, empty slots) as `fm:session` and `fm:arr:<snapshotId>:<club>`. Player data is fetched fresh on restore (`loadSquad`). Only a 404 drops a player; any other error fails the restore and leaves storage untouched. A new save version gets a new snapshot id and starts fresh. No reset button: clear the `fm:` keys in the console.

Mock lives in `mock/`. Chosen design is **A (`a-board.html`)**; `b-ledger.html` and `c-compare.html` are rejected alternatives, `index.html` is the first rejected draft. Shared mock data is in `mock/data.js` (placeholder, not parsed from a save).

### `a-board.html` (single file, hash routing)

- Top bar: club name, tabs Squad / Shortlist (count badge) / Scouting, "Load save" button.
- `#/upload` (landing page): drop zone for `.fm`, simulated 4-step progress (reading, decompressing, parsing, building squad), summary, "Open squad". Rejects non-`.fm` with an inline error.
- `#/squad`: pitch only (4-2-3-1: two DM left and right, one AMC). Clicking a pitch player opens the right panel showing only that player's position group, with a Close button.
  - Positions: GK, DR, DC, DL, DM, AMC, AMR (right wing), AML (left wing), ST.
  - Depth targets: GK 2, DR 2, DC 4, DL 2, DM 2, AMC 2, AMR 2, AML 2, ST 2. Missing players show dashed "+ Add cover" slots (count `n/target`).
  - Selected row has "Find replacement" and "Remove".
- `#/short`: two modes.
  - Replacement: left card for the player replaced, right list ranked by match %, filter chips (contract expiring, transfer listed, cheap), attribute deltas vs the player replaced, "Swap in".
  - Add cover (from a "+ Add cover" slot): same list for that position, "Add to squad", returns to squad.
- `#/scout`: Lists panel (presets "Promising youngsters" 18-23 all positions, "Contract expiring" <=12 months; custom lists via "+ New list"). Filters: position, age range, max value, contract expiring. Table ranked by potential, "+ Add" adds to squad.
- `#/player`: clicking a scouting row opens player detail (facts, technical/mental/physical attributes, "+ Add to squad"). Attributes are generated placeholders.

## Decisions and preferences

- Light "paper" style, Fraunces + Geist, single dark accent, green for positive.
- No top-right "Remove selected" / "Find replacement" buttons (moved to the selected row).
- "From" (nationality) filter removed from scouting; the table column remains.
- Per-position depth targets are the user's; "X remove 1" meant lowering that target by one.

## Next

- Write tests first and commit them, then implement (per global workflow).
- Build the parser from the SaveLens approach; replace mock data.
- Per global rules: pre-commit Prettier hook and a GitHub Actions PR workflow (lint/format/tests) for the real project.
- Open bug: some contract ends decode to a past date (Mirco Soldi, id 621367, shows 2050-03-20 in a 2053 save, so 0 months left). Dump his record and compare with a correct player.
- Dir is a git repo; the squad/persistence work above is uncommitted.

## Gotchas

- `agent-browser open` with only a hash change does not reload; use `agent-browser reload`.
- Stop the SaveLens server before `cargo build` (Windows locks the exe). Bump `PARSER_VERSION` in `src/adapters/parser/mod.rs` when parser output changes, or re-import reuses the cached snapshot.
- `agent-browser` is headless by default; use `--headed` to watch. Element refs change after each click, so re-snapshot first.
- Vitest: a `beforeEach` that returns a mock runs it as a cleanup hook; use braces. `src/services/save.test.ts` holds the API client tests, so new service tests go in their own file.
- CSS class `.up` is the delta colour in `a-board.html`; do not reuse it for layout.

## Commits

Use semantic (Conventional Commits) messages: `type(scope): subject`, with type one of `feat`, `fix`, `test`, `refactor`, `docs`, `chore`, `style`, `perf`, `ci`. Imperative subject, no trailing period.
