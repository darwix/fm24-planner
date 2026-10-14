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

**Layout**: the repo root is the SaveLens clone (Rust in `src/`, laid out as in `ARCHITECTURE.md`: `domain/`, `application/`, `adapters/` = http + persistence + parser, `infra/`; `tests/`; desktop app in `src-tauri/`: runs the server in-process on a free port and serves `web/dist/client` embedded in the exe (Tauri assets) via `infra::app::serve_web`; tagging `v*` builds the portable exe into a draft release (`.github/workflows/release.yml`); `npm run desktop` / `npm run build:desktop` from the root), patched by us (no licence, never publish). Our React app is `web/` (replaces the original SaveLens frontend; served by Vite, the Rust server is API-only and no longer embeds the build). Run web commands from `web/`. The old `savelens-src/` paths below now mean the repo root; its upstream `.git` and original `web/` are backed up outside the repo. `mock/` and `DESIGN.md` stay at the root.

`web/src/` follows `../react-template` (React 18 + TS, react-router, recoil, swr, axios; `pages/`, `components/`, `atoms/`, `services/`, `models/`, `utils/`, glob routes). Kept our own CSS (no antd). Dev server proxies `/api` to a SaveLens server on :4242.

The shipped SaveLens exe does not expose contract data, so the SaveLens clone (now the repo root) is patched: `src/adapters/parser/records/contract.rs` reads weekly wage (GBP, before tax), contract end and value (GBP) from the save; the search API returns `wage`, `contractEnd`, `value`. Build and run it instead of the shipped exe: `cargo build --release --bin fm-squad-planner-server` (from the root), then `target/release/fm-squad-planner-server.exe --no-open --port 4242`. Renamed from SaveLens: crate `fm-squad-planner`, env `FM_SQUAD_PLANNER_*`, APP_ID `io.github.darwix.FMSquadPlanner` (new data dir, saves must be re-imported). Tests: `cargo test --release` (fixtures in `tests/fixtures`, real bytes of players). Contract, wage, expiry and club rules are in `RULES.md`; value = u32 right after the 54-byte attribute block. Check any parser change against a full in-game export with `scripts/check-contracts.py` (RULES.md, "Checking a parser change"); at `fm24-rust-47-fm` expired, end and wage all match the test save's 54,308 players. Transfer and loan listed are decoded too (RULES.md; search filters `transferListed=1`, `loanListed=1`, which like the game leave out players on loan and the managed club). Not decoded: on loan from, sales value.

Frontend: save list, import, club squad, player page, shortlist (CA match, tags contract expiring <=12 months and cheap <=GBP 5M) and scouting all use live backend queries (the backend filters in SQL, max 250 per page). Positions use the backend's labels (AMC, not AM). Money shows in GBP. Run (in `web/`): `npm run dev`, `npm test`, `npm run typecheck`.

### Backend search API (`src/adapters/http/dto/player_query.rs`, `src/adapters/persistence/player.rs`)

- `GET /api/snapshots/<id>/players?...` and `POST /api/snapshots/<id>/players/search` (JSON body, arrays become comma lists; same filters). Response: `total`, `page`, `limit` (max 250), `players`.
- Filters added by us: `contractEndMax` (YYYY-MM-DD), `valueMax` (+ `valueUnknown=1` keeps players with no value), `mainPosition` (comma indices of the best-rated position), `club=-1` (free agents), `ageMin/ageMax`, `page`, `shortlist=1` (only players in the `shortlist` table; empty before the table exists), `excludeClub=<id>` (free agents stay), `orNoContract=1` (with `contractEndMax`, also free agents / no contract). Sorts: `name`, `club`, `age`, `ca`, `pa`, `position` (main_position), `value`, `wage`, `contractEnd`, attributes; `direction=asc|desc`; unknown (NULL) values always last.
- Shortlist table: `shortlist(player_id)` in each snapshot `.sqlite`, created on first use (`src/adapters/persistence/planner.rs`). `GET /api/snapshots/<id>/shortlist` -> `{ids}`, `PUT`/`DELETE /api/snapshots/<id>/shortlist/<player>`. Opening it moves an old planner `kept` list into the table once and deletes the key, so the web reads the list (`getShortlist`) before searching with `shortlist=1`.
- `main_position` column = best-rated position. `positions[0]` (the UI's `p.p`) is NOT the main position.
- A market value of 0 in the save means unknown: stored as NULL (PARSER_VERSION `fm24-rust-8-fm`).
- Rust layout: see `ARCHITECTURE.md`. `lib.rs` still re-exports `parser` and `storage` (= `adapters::persistence`) for the tests.

### Scouting flow in the React app

Filter, swipe, shortlist review, act. Built: filter, swipe, shortlist review.
- `/scouting` = filter screen (`components/Brief.tsx`): pitch position picker (multi), age sliders, max price chips, contract chips (Any, Expired = free agents, <=12, <=6 months), live match count from `total`, "Start scouting". Name search (debounced 300 ms, ignores the filters) sits above Lists in the left column, results drop down and open the player page.
- `utils/player.ts`: `scoutQuery` builds the filter params, `swipeBody` pages it (50 per page), `morePages` says if another page remains.
- `/scouting/swipe` (`pages/scouting/swipe.tsx`): swipe right = sign (adds a row to the shortlist table; `keptState` holds the ids), left = pass (not stored: the user wants passed players to come back on the next scouting run). Shortlisted players are skipped in the stack. Tap the card to open the book with the player's real attributes. Next page loads when 5 cards are left. A Cards | List toggle in the head switches to a table of the same stack (PA, name, pos, age, CA, contract, value, wage, "+ Shortlist"), with "Load 50 more".
- `/shortlist` (Shortlist tab, `pages/shortlist/index.tsx`, logic in `utils/shortlist.ts`): the shortlist table per mock `scout-e-shortlist.html`, minus your own club. The backend filters and sorts (`shortlistQuery` builds the params). Position chips (positions anyone on the list can play; any picked), status chips Transfer listed / Loan listed / Contract expiring <=12 months incl. free agents (all must hold), click a column header to sort (default price ascending, unknown last). No Swipe button. Ability = stars, gold CA + outline PA, fixed scale 200 = 5 (user's choice for now; the game rates against your squad). Add to squad, Drop. No passed count, no role fit, no vs starter (user removed them).
- Not built: Replace… from the shortlist, role-fit panel (needs a role catalog in the web app). Mocks D (swipe), F (filter), G (filter with search) and H (swipe with Cards | List) are in `mock/`.
- Styles: one `.css` per page or component, imported by it; `index.css` holds the shared base only.

### Squad behaviour in the React app

- **Managed club**: the backend snapshot returns `managedClubId`, read from the club name in `save_game_summary.dat` (only clubs that have players are matched; ambiguous or no match = none). The upload page prefills the club box with it.
- **Open squad** shows a loading state while the squad is fetched.
- **Pitch**: drag a starter onto another starter to swap spots, or onto a dashed Empty slot to move there. Removing a starter promotes the best cover who can play that group; the slot is Empty only if nobody can.
- **Position groups**: a player has one main position `p` plus `pos` (every position he can play) and `cover` (extra groups he is counted in). `inGroup`/`need` in `utils/logic.ts` count both. The depth panel shows "+ Add cover" always, even over target.
- **Find replacement** and **Add cover** share `/candidates` (`pages/candidates/`, no tab; Squad tab stays lit). "From your club" = squad players who can play the group (Promote, Swap spots or Use as cover) plus everyone else at the club who can (`club=<id>` search; adding cover skips those already in the group). "Your shortlist" = the shortlist table outside your club (`shortlist=1&excludeClub=<id>`) plus planned signings already in the squad plan from another club (`SquadPlayer.cl` = real club). No search of all players (user removed it). Tag chips hide only tagged players whose tags are all off.
- **Match score**: `50 + 2 x (CA - target CA)`, clamped 1-99, so higher CA ranks first. Add cover target = squad average CA - 10.
- **Persistence**: planner state lives in a `planner(key, value JSON)` table inside each snapshot's `.sqlite` (`src/adapters/persistence/planner.rs`, `GET/PUT /api/snapshots/<id>/planner/<key>`, web `services/planner.ts`). Keys: `squad:<club>` (arrangement only: ids, group, pitch spot, covers, empty slots). The shortlist has its own table (see above). Passed players are not stored. The open club is `session` in `settings.json` (`GET/PUT /api/session`), with the club `id` (needed for `excludeClub`); a session without it is not restored, so the user picks the club again. No local storage. Player data is fetched fresh on restore (`loadSquad`). Only a 404 drops a player; any other error fails the restore. A new save version gets a new snapshot id and starts fresh (no reliable way to link versions). No reset button.

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

Workflow: write tests first and commit them, then implement.

1. **Replace… from the shortlist** (mock button, not built): swap a shortlisted player in for a squad player.
2. Later: role-fit panel (needs role catalog); decode "on loan from" and sales value. (Prettier pre-commit hook and `.github/workflows/pr.yml` already exist.)
3. Housekeeping: uncommitted `.gitignore`, `package.json`, `web/tsconfig.json`, `parser.html` are the user's; the `web/tsconfig.json` change breaks `npm run typecheck` (drops `moduleResolution: Node`, so tsc can't find packages); ask before touching.

## Gotchas

- `agent-browser open` with only a hash change does not reload; use `agent-browser reload`.
- Stop the SaveLens server before `cargo build` (Windows locks the exe). Bump `PARSER_VERSION` in `src/adapters/parser/mod.rs` when parser output changes, or re-import reuses the cached snapshot.
- `agent-browser` is headless by default; use `--headed` to watch. Element refs change after each click, so re-snapshot first.
- Vite can serve a stale module after big edits (blank page, "does not provide an export named"): `touch` the file to refresh.
- Vitest: a `beforeEach` that returns a mock runs it as a cleanup hook; use braces. `src/services/save.test.ts` holds the API client tests, so new service tests go in their own file.
- CSS class `.up` is the delta colour in `a-board.html`; do not reuse it for layout.

## Commits

Use semantic (Conventional Commits) messages: `type(scope): subject`, with type one of `feat`, `fix`, `test`, `refactor`, `docs`, `chore`, `style`, `perf`, `ci`. Imperative subject, no trailing period.
