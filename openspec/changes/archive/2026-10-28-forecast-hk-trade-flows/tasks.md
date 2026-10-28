# Tasks

## 1. Backend calculation

- [x] 1.1 In `backend/src/calc/forecast.rs` add `trades: &[SuggestionTrade]` to `ForecastInput`, a signed-amount helper (BUY −total, SELL +total), `trade_flows` on `ForecastMonthDerived` (+0.0 normalised), and add it to `cash` (not `locked`); verify with new unit tests: BUY lowers cash, SELL raises it, trade outside the window has no effect, a stock plan and a trade in the same month both count, empty month's `trade_flows` is positive zero

## 2. Backend API

- [x] 2.1 In `backend/src/models/forecast.rs` add `ForecastTrade` (`trade_id`, `code`, `trade_type`, `trade_date`, `amount`, ToSchema) and `trade_lines` on `ForecastMonth`; in `backend/src/routes/forecast.rs` load HK trades for `[first, first + 7 months)` via `store::trades::hk_suggestion_trades`, pass them into `ForecastInput`, and emit each month's `trade_lines`; verify `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass

## 3. Frontend

- [x] 3.1 Add `ForecastTrade` and `trade_lines` to `frontend/src/api.ts`; in `frontend/src/views/OverviewView.vue` make the 股票 cell figure Σ stock items + Σ trade amounts (`—` only when both are empty) and list trade lines read-only marked `已交易` in the cell editor; verify `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` pass

## 4. Docs and integration

- [x] 4.1 Update the 預測 section of `docs/overview.md` (股票 row now includes HK trade lines; 月初 caveat; plans are not suppressed); verify the doc names the new `trade_lines` field
- [x] 4.2 Restart the backend per AGENTS.md and verify `GET /api/forecast` returns trade #37 (匯豐, −54063.17) in the `2026-10` column's `trade_lines`, with that column's `cash` 54063.17 lower than before the change

## Workflow follow-up

- Archive the change after review, syncing the delta into `openspec/specs/overview/spec.md`.
