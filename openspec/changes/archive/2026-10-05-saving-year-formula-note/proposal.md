# Proposal

## Why

The 目標 card footnote now expands the 3rd/Final formulas, but the `存/年` footer figure — the pace behind the milestones' years/ETA — still hides its `AVERAGE` over five saving sources. Expanding it with live values matches the other footnotes and lets the user verify the projection without opening the workbook.

## What Changes

- The `goals` block on `GET /api/overview` additionally carries the running averages `saved_avg` and `interest_avg` (`'Month Stat'!L8`/`N8`, monthly — absent while no qualifying month exists), the two `L12` terms not already exposed elsewhere on the response.
- The 目標 card's footnote expands `存/年` as `AVERAGE` over its five yearly terms, each with its live contribution in parentheses: `近12月存×12`, `近3年invested`, `MM月均×12`, `歷年存×12`, `(歷年存+息)×12` — `—` while a term is absent.

## Capabilities

### New Capabilities

### Modified Capabilities

- `overview`: the `Goals block` requirement gains the two running-average fields; the `目標 card` requirement gains the `存/年` expansion in its formula footnote.

## Impact

- `backend/src/models.rs` — two new `Option<f64>` fields on `OverviewGoals`.
- `backend/src/calc.rs` — `overview_goals` echoes them; a unit test asserts the echo.
- `frontend/src/api.ts`, `frontend/src/views/OverviewView.vue` — interface fields plus a third muted footnote line using the existing `note-formula`/`note-line` layout.
- `docs/overview.md` — the 目標 paragraph notes the `存/年` expansion.
- OpenAPI schema updates automatically via `ToSchema`; no endpoint or migration changes.
