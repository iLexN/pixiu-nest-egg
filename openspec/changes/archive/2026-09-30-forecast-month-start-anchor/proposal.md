# Proposal

## Why

Converting a current-month deposit plan with `轉為定期` inflates the 預測 grid. On 2026-09-30, converting the `SC高息馬拉松` `-55000` plan made 活期 / 半流動 / ref check jump by `55000` in every column. The grid mixes two starting points from different moments. `start` is the month's 月初 snapshot (`start_cash`), taken before the money moved. `locked` starts from the deposits active *today*, which now include the new deposit. The deleted plan was the only flow connecting the two. The same mismatch makes `locked` too low after a deposit matures earlier in the current month: it drops out of the live total and is also subtracted as `定期 finish`. It also puts an early-converted future plan into `locked` before its placement month.

## What Changes

- The forecast `locked` chain starts from the deposits held on the window's first day: `start_date` absent or before the 1st, and `end_date` on or after the 1st. It no longer uses the live Σ active principal. The overview `半流動資金` block keeps the live figure.
- A real deposit whose `start_date` falls inside a window month becomes a placement flow in that month: `cash` −principal, `locked` +principal. This mirrors a `hs_deposit`/`sc_deposit` plan.
- Each forecast month returns `deposit_placements` (deposit id, label, bank, principal, start_date) for the deposits placed in it.
- The 預測 card shows each placement as a read-only line in its bank's plan row: HS goes to `定期 HS`, every other bank to `SC高息馬拉松`. The line is included in the cell sum. Converting a plan therefore leaves the grid's totals unchanged.
- Data fix (not a code change): SC-3615's `start_date` moves from 2026-09-01 to 2026-08-31, because its money had already left 活期 before September's 月初 figure was entered.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the forecast projection block (the `cash` formula and the new `deposit_placements`), the locked 定期+SC chain (month-start anchor, placement flow), convert plan to deposit (the grid is unchanged by conversion), and the 預測 grid card (read-only placement lines in the bank rows).

## Impact

- `backend/src/calc.rs`: the `forecast_months` placement flow, a new month-start anchor helper, and tests.
- `backend/src/routes/forecast.rs`, `backend/src/models.rs`: the anchor source and the `deposit_placements` field in the `GET /api/forecast` response (additive).
- `frontend/src/api.ts`, `frontend/src/views/OverviewView.vue`: the placement type and the read-only lines in the plan cells.
- `docs/overview.md`: the 預測 section.
- No schema migration.
