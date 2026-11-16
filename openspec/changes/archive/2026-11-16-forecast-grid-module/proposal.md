# Proposal

## Why

Two layers decide which 預測 column each event belongs to:

- `calc::forecast_months` groups deposits, receipts, trades and IBKR 轉入 by column with `in_month` to chain `cash` and `locked`.
- `service::forecast::build` re-filters the same events with its own `same_month`, and matches plan items by string month, to build each column's display lines.

`calc::forecast_cells` then re-sums those lines into each Forecast cell's `figure`. Today the chained amounts and the lines a cell shows agree only because the two filters happen to match. The glossary promises that routing a line to its row and summing the figure happen in the backend only. They happen twice, and the two can drift apart. 預測 is also the most-changed area in recent history.

## What Changes

- One pure 預測 grid function in `calc/forecast.rs` takes the loaded inputs and returns the finished `ForecastMonth` columns. It groups each event into its column once, and from that grouping derives the chained amounts, each Forecast cell's lines and figure, and the 利息 components.
- `forecast_months` and `forecast_cells` become private to that module.
- `service::forecast::build` shrinks to loading the inputs and making one call. Its `same_month`, the three per-column filters and `parse_stored_ym` are deleted.
- `ForecastItemFacts` borrows its stored `ForecastItem`, so plan-item lines come from the same value that feeds the chain. A new `store::forecast::facts_of` parses the stored months and returns `StoreError::Corrupt` on a bad stored date.
- No behaviour change. `GET /api/forecast` returns the same JSON, and `api-types.ts` doesn't change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a pure refactor, so `skip_specs: true` is set.

## Impact

- Backend:
  - `calc/forecast.rs`: the new grid function, plus the existing tests ported onto it.
  - `service/forecast.rs`: shrinks to loading inputs.
  - `store/forecast.rs`: gains `facts_of`.
- Docs: the 預測 wiring block in `docs/overview.md`.
- No schema, API or frontend change.
