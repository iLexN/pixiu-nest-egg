# Proposal

## Why

The 目標 card's 3rd/Final rows show formula-derived targets (`O14`/`P14`) with no hint how they are computed — the formulas and their constants live only in the spec, docs, and the 月結 settings. Extending the card's footnote to spell out both formulas with live values matches the new 投資目標 footnote and lets the user verify the targets at a glance.

## What Changes

- The `goals` block on `GET /api/overview` additionally carries the effective formula constants `goal3_inflation`, `goal3_years`, and `final_replacement` (the defaults `1.07`/`30`/`0.7` while unset) so the frontend can expand the formulas without fetching month settings.
- The 目標 card's footnote expands both formulas with live values: `3rd = 生活預算 (…) × 通脹 (…) × 12 × 年期 (…)` and `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)`; absent inputs render `—`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `overview`: the `Goals block` requirement gains the three constant fields; the `目標 card` requirement gains the formula footnote.

## Impact

- `backend/src/models.rs` — three new `f64` fields on `OverviewGoals`.
- `backend/src/calc.rs` — `overview_goals` echoes the effective constants; a unit test asserts the echo.
- `frontend/src/api.ts`, `frontend/src/views/OverviewView.vue` — interface fields plus a second muted footnote line.
- `docs/overview.md` — the 目標 paragraph notes the expanded footnote.
- OpenAPI schema updates automatically via `ToSchema`; no endpoint or migration changes.
