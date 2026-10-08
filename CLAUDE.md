# FM Squad Planner

New app built on how FM SaveLens 24 (https://github.com/MSalman5230/FM-SaveLens-24) parses Football Manager 2024 `.fm` saves.
Test save: `Meong Racun - 2052 (v03).fm` (~400 MB). `savelens/FM SaveLens 24/` holds the downloaded v1.4.0 portable release (GUI + local server mode). `fmsave` (Python) does not work: it targets FM26.

## Goals

1. Squad planner: squad screen with each player's abilities.
2. Add / remove players in the squad.
3. Replacement shortlist: players matching the profile of the one being replaced, prioritising contract expiring, transfer listed, cheap.
4. Scouting: preset lists (promising youngsters 18-23, contract expiring) plus custom lists.
5. Upload flow for the `.fm` save.

## Status: design mock only, no real parser or app yet

Mock lives in `mock/`. Chosen design is **A (`a-board.html`)**; `b-ledger.html` and `c-compare.html` are rejected alternatives, `index.html` is the first rejected draft. Shared mock data is in `mock/data.js` (placeholder, not parsed from a save).

### `a-board.html` (single file, hash routing)

- Top bar: club name, tabs Squad / Shortlist (count badge) / Scouting, "Load save" button.
- `#/upload` (landing page): drop zone for `.fm`, simulated 4-step progress (reading, decompressing, parsing, building squad), summary, "Open squad". Rejects non-`.fm` with an inline error.
- `#/squad`: pitch only (4-2-3-1: two DM left and right, one AM). Clicking a pitch player opens the right panel showing only that player's position group, with a Close button.
  - Positions: GK, DR, DC, DL, DM, AM, AMR (right wing), AML (left wing), ST.
  - Depth targets: GK 2, DR 2, DC 4, DL 2, DM 2, AM 2, AMR 2, AML 2, ST 2. Missing players show dashed "+ Add cover" slots (count `n/target`).
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
- Dir is not yet a git repo.

## Gotchas

- `agent-browser open` with only a hash change does not reload; use `agent-browser reload`.
- CSS class `.up` is the delta colour in `a-board.html`; do not reuse it for layout.
