# Proposal

## Why

The 預測 rules live in three places. `routes::forecast::forecast` sequences eight loaders and builds each column's line lists in the route, against the layer rule that routes stay thin. `OverviewView.vue` then re-derives which plan row a placement lands in (`HS` → 定期 HS, anything else → SC高息馬拉松) and sums every cell itself (`cellPlacements`, `cellSum`), even though the backend is meant to own financial figures. The Month book has a similar leak. Its sequencing is private, but its parts are public (`stored`, `stat_rows`, `items`, `live`, `tail`), so four callers pass them into calc functions by hand. Each caller has to know which calc functions take the tail: `month_year_summaries` and `month_running_averages` do, `pool_balances` and `trailing_averages` don't. `routes::months::list` also keeps its own copy of `present_one`. The 預測 module is the hottest area in recent history and it is one of the Month book's callers, so both are deepened together.

## What Changes

- **New `service::forecast` module** behind `build(pool) -> ForecastResponse`. It owns loading, derivation, and cell assembly for the 預測 grid. `routes::forecast::forecast` becomes a one-call adapter.
- **BREAKING (API shape, sole consumer is the app's own frontend)**: each `GET /api/forecast` month entry replaces the flat `plan_items`, `deposit_placements`, `trade_lines` and `ibkr_lines` lists with a `cells` array. There is one cell per plan row (`hs_deposit`, `sc_deposit`, `interest`, `tax`, `stock`, `bill`, `other`), and each carries its plan items, placements, trade lines, IBKR lines, and the `figure` the grid shows. The backend now decides which bank goes to which row and computes the cell's figure.
- **OverviewView renders cells**: `cellItems`, `cellPlacements`, `cellTradeLines`, `cellIbkrLines` and `cellSum` are removed, and the grid and editor read `month.cells`.
- **The Month book is sealed**: `MonthBook`'s fields become private, and the book answers queries instead: presented months, one presented month, year summaries, running averages, trailing averages, pool balance, and pool rate. The book loads pool rates and HK sold P/L itself. `service::months::present` becomes private, and `routes::months::list`'s copy of `present_one` is deleted.
- **All Month book callers migrate**: `routes::months::{list,summary,show}`, `service::overview::build`, `service::year_review::build`, and the new `service::forecast`.
- **Docs**: `docs/overview.md` and `docs/DATA_FLOW.md` wiring; `GLOSSARY.md` gains *Forecast cell*.

The figures themselves don't change. Every number on `/api/months*`, `/api/overview` and `/api/year-review` stays the same, and so does every forecast figure. Only the shape of the forecast lines changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview`: the forecast month entry carries per-row `cells` (lines plus figure) instead of the flat line lists. The placement-to-row rule and the cell figure move from the 總覽 page requirements to the forecast response requirements.

## Impact

- Backend: new `backend/src/service/forecast.rs`; `backend/src/routes/forecast.rs`, `backend/src/calc/forecast.rs` (pure cell grouping), `backend/src/models/forecast.rs` (`ForecastCell`, placement-row mapping), `backend/src/service/months.rs`, `backend/src/routes/months.rs`, `backend/src/service/{overview,year_review,mod}.rs`.
- Frontend: `frontend/src/views/OverviewView.vue`; `frontend/src/api-types.ts` and the `api.ts` aliases regenerate via `pnpm gen:api`.
- API: `GET /api/forecast` response shape (breaking for the in-repo frontend only). No other endpoint changes.
- No schema migration.
