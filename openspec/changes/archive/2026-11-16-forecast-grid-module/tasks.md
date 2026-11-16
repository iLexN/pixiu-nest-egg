# Tasks

## 1. Baseline

- [x] 1.1 Capture `GET /api/forecast` from the running backend on :8787 to `/tmp/forecast-before.json` before any code change. Verify the file is non-empty JSON with 7 `months`.

## 2. Plan-item facts

- [x] 2.1 Change `ForecastItemFacts` to `ForecastItemFacts<'a> { item: &'a ForecastItem, month, return_month }` (design D2), and add `store::forecast::facts_of(&ForecastItem) -> Result<ForecastItemFacts<'_>, StoreError>` with today's `Corrupt` messages. Add store tests for a good row, a bad `month` and a bad `return_month`. Verify with `cargo test -p wealth-backend store::forecast`.

## 3. The grid module

- [x] 3.1 Add `calc::forecast_grid(&ForecastInput) -> Vec<ForecastMonth>` (design D1). It groups each column's events once and derives the chained values, the cells and the `interest_components` from that grouping, preserving today's line order and −0.0 normalisation. Make `forecast_months`, `forecast_cells` and `ForecastMonthDerived` private, or fold them in.
- [x] 3.2 Port every existing `forecast_months` and `forecast_cells` test onto `forecast_grid`, asserting on the returned `ForecastMonth` columns with the same fixed expected values, and delete the old function-level tests rather than keeping both. Verify with `cargo test -p wealth-backend calc::forecast`.
- [x] 3.3 Add the reconciliation test: a grid with plan items of every kind, an HS and an SC placement, an HK trade and a 轉入 row, spread over two columns. For each column, check that every non-bill, non-interest cell's `figure` equals the sum of its lines, and that `cash − start = salary + spend + deposit_finish + interest + bill + deposit_return + Σ(non-bill, non-interest cell figures)`. Verify with `cargo test -p wealth-backend forecast_grid_reconciles`.

## 4. Service

- [x] 4.1 Reduce `service::forecast::build` to loading the inputs, mapping items through `facts_of`, and calling `calc::forecast_grid`. Delete `same_month`, `parse_stored_ym` and the per-column closure. Verify that the two `service::forecast` tests and the `routes::forecast` convert tests pass unchanged.

## 5. Docs

- [x] 5.1 Update the 預測 wiring block in `docs/overview.md` so it says `service::forecast::build` loads the inputs and `calc::forecast_grid` assigns events to columns once and builds the chained figures and each cell's lines and figure. Verify that `forecast_cells` is no longer named there.

## 6. Verify

- [x] 6.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`.
- [x] 6.2 Run `cd frontend && pnpm gen:api` and confirm the `api-types.ts` diff is empty, then run `pnpm build && pnpm exec vue-tsc --noEmit`.
- [x] 6.3 Restart the backend detached on :8787 (per AGENTS.md), capture `GET /api/forecast` to `/tmp/forecast-after.json`, and verify that `diff /tmp/forecast-before.json /tmp/forecast-after.json` is empty. Take both captures on the same day.

## Workflow follow-up

- Archive with `openspec-archive-change` (no delta specs).
- Run `trail check` before committing.
- Next change: `cash-movement-module`.
