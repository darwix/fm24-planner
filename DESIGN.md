# Design

Source of truth: `mock/a-board.html` (removed; in git history at `011a622`). Warm "paper" look, editorial headings, one dark accent. Keep it quiet: no gradients, no heavy shadows.

## Colour tokens

| Token       | Value                      | Use                                          |
| ----------- | -------------------------- | -------------------------------------------- |
| `--bg`      | `#f3efe6`                  | page background                              |
| `--card`    | `#fbf8f1`                  | cards, selected rows, filter bar             |
| `--line`    | `#e2dccd`                  | dividers, borders                            |
| `--ink`     | `#1d2320`                  | text, primary buttons, active chips          |
| `--mut`     | `#7b7a70`                  | secondary text, labels                       |
| `--acc`     | `#1f6f4a`                  | positive: match %, PA, upward delta, "cheap" |
| `--warn`    | `#b5651d`                  | contract expiring, depth below target        |
| `--bad`     | `#b23a2e`                  | negative delta, remove, expired contract     |
| listed blue | `#2b5fa8`                  | "transfer listed" tag only                   |
| pitch       | `#e7e2d3`, lines `#c9c2ad` | pitch surface and markings                   |

Rules: one accent (green). Orange and red are status only. Greys stay warm.

## Type

- Headings, big numbers (match %, CA/PA, logo): **Fraunces** 600/700, tight tracking (`-.02em`).
- Everything else: **Geist** 400/500/600, 14px base. Labels 11px muted.
- All numbers use `font-variant-numeric: tabular-nums`.
- Section labels: 11-12px uppercase, wide tracking, muted. Use sparingly.
- Sentence case for UI copy.

## Layout

- Top bar: sticky, blurred `--bg`, 1px bottom border. Club name left, tabs, "Load save" right. Active tab has a 2px underline; no pill.
- Page padding 40px, max width 1400px.
- Squad: pitch (max 560px) beside a position panel. Pitch is centred alone until a player is clicked.
- Shortlist: 320px sticky target card + list. Scouting: 260px lists column + table. Player detail: 1100px centred, three attribute columns. Upload: 640px centred.
- Lists are separated by hairlines, not boxes. Cards only where they carry meaning (target card, filter bar, selected row).

## Components

- **Pitch player**: 44px rounded square (radius 14) showing CA, name and position below. Selected = ink fill. Contract expired = red inset ring.
- **Row**: radius 12, hover `--card`, selected = `--card` + 1.5px ink inset ring. Selected row shows "Find replacement" and "Remove".
- **Depth slot**: dashed 1.5px `--line` border, centred muted text "+ Add cover · POS". Section label shows `n/target`, orange when below target.
- **Buttons**: radius 8. Primary = ink fill. Default = text only, hover tint. Destructive = red text.
- **Chips** (filters): radius 8, off = faint tint, on = ink fill.
- **Tags**: coloured text, no background (expiring orange, listed blue, cheap green).
- **Delta**: `+n` green, `-n` red, `·` muted for equal.
- **Attribute value**: 14+ green tint, 11-13 orange tint, below that plain.
- **Drop zone**: dashed 2px, radius 20, hover/drag = ink border + `--card`.
- **Stepper** (upload): hollow circle, pulsing ink for current, green check when done.

## Motion

- 150ms transitions on hover/selection. Buttons scale to `.97` on press.
- Pitch players lift 3px on hover.
- Page switches are instant (hash routes); no page transitions.

## Content

- Money: `€1.2M`, `€900k`; wage `€38k/wk`; contract as "7 months left" when within 12 months, otherwise the end year.
- Positions shown by code (GK, DR, DC, DL, DM, AMC, AMR, AML, ST); section titles spell them out.
- Errors are plain and inline, no `alert()`.

## Accessibility to do in the real app

- Visible focus rings (mock has none).
- Pitch players and rows must be keyboard reachable (mock uses click handlers on divs).
- Contrast of `--mut` on `--bg` is borderline for 11px labels; darken if needed.
