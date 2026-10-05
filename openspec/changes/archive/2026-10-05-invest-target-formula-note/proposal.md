# Proposal

## Why

The 投資目標 card shows a `target` figure per year but gives no hint how it is derived — the six-term formula lives only in the spec and `docs/overview.md`. A small footnote spelling out the formula with live values (like the 目標 card's existing `年均回報率 N4 = …` note) lets the user sanity-check the projection without leaving the page.

## What Changes

- Each `invest_targets` row on `GET /api/overview` additionally carries `interest` and `pool_spend` — the year-review 利息回報 / 開心Pool支出 figures the target formula reads — so the frontend can expand the formula without a second fetch.
- The 投資目標 card gains a muted footnote expanding the current-year `target` into its six terms, each with its computed value in parentheses, e.g. `目標 = 上年invested (428,635.41) − 上年利息回報 (51,900.65) − 上年Pool支出×0.7 (27,110.65) + 月薪增幅×50%*12 (11,340.00) + 今年利息回報 (62,753.15) + 今年Pool支出×0.7 (6,715.10)`; absent inputs render `—` and the note hides when no current-year row exists.

## Capabilities

### New Capabilities

### Modified Capabilities

- `overview`: the `投資目標 block` requirement gains the two row fields; the `投資目標 card` requirement gains the formula footnote.

## Impact

- `backend/src/models.rs` — two new `f64` fields on `InvestTargetRow`.
- `backend/src/calc.rs` — `invest_targets` populates them; inline unit test extended.
- `frontend/src/api.ts`, `frontend/src/views/OverviewView.vue` — interface fields plus computeds and the muted footnote.
- `docs/overview.md` — one sentence noting the footnote.
- OpenAPI schema updates automatically via `ToSchema`; no endpoint or migration changes.
