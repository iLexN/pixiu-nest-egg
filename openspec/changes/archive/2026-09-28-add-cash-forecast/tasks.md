# Tasks

## 1. Storage and settings

- [x] 1.1 Create `backend/migrations/0026_forecast_items.sql` — `forecast_items` (`id`, `month` TEXT `YYYY-MM-01`, `kind` TEXT, `amount` REAL, `return_month` TEXT nullable, `note` TEXT, `sort_order`, `created_at`, `updated_at`, index on `month`) — and update `docs/database.md`; verify `cargo run` migrates a scratch DB cleanly (`WEALTH_DB=/tmp/wealth-test.db`)
- [x] 1.2 Extend `PATCH /api/months/settings` to accept `bill_amount` (finite, non-negative → `forecast.bill_amount` in `app_meta`, default `2158` when unset); verify via an api test that the value round-trips and rejects negatives

## 2. Forecast derivation and API

- [x] 2.1 Implement `calc.rs` forecast derivation — 7-month window, `start` (first month = current `start_cash` else live 活期 sum; later months chain from `cash`), `salary`, `spend` (−`living_budget`), `deposit_finish`/`interest` (deposits ending + HK dividends + coupons due in month + `interest` items), `bill` (quarter-month default vs `bill`-item override), `deposit_return` (per-item `return_month` else kind lag), `cash`, `locked`, `semi_liquid`, `ref_check`; verify with inline `#[cfg(test)]` cases covering the spec scenarios (anchor, fallback, chain, finish/interest, bill override, both lags, override month)
- [x] 2.2 Add `routes/forecast.rs` with `#[utoipa::path]` handlers — `GET /api/forecast`, `POST /api/forecast/:ym/items`, `PATCH/DELETE /api/forecast-items/:id` (kind/amount validation, `return_month` rejected on non-deposit kinds) — registered via `utoipa_axum::routes!`; verify with `backend/tests/api.rs` tests for CRUD, the multiple-items-per-month sum, and the 400 on bad `return_month`
- [x] 2.3 Add `POST /api/forecast-items/:id/convert` — one transaction creating the deposit (bank from kind, fields from payload) and deleting the item; verify api test: deposit exists, item gone, and the return/lock effects vanish on the next `GET /api/forecast`
- [x] 2.4 Update `routes::tests::openapi_documents_every_api_operation` for the new endpoints; verify `cargo test` passes

## 3. Frontend

- [x] 3.1 Add forecast types and client methods to `frontend/src/api.ts` (`getForecast`, item create/patch/delete, `convertForecastItem`); verify `pnpm exec vue-tsc --noEmit`
- [x] 3.2 Add the 預測 card to `OverviewView.vue` — sheet row labels × 7 month columns, `—` for absent figures, red `ref_check` when negative, inline item add/edit/delete per plan row, and `bill` cell edit writing a `bill` item; verify in the running app that the grid matches the sheet's shape
- [x] 3.3 Add the `轉為定期` action on `hs_deposit`/`sc_deposit` items opening the deposit form prefilled (principal `−amount`, bank from kind) and calling the convert endpoint on save; verify a plan converts end-to-end in the UI

## 4. Docs and verification

- [x] 4.1 Update `docs/overview.md` (forecast flow), `docs/DATA_FLOW.md` (index entry), and `docs/MONTH_STAT_OVERVIEW.md` (mark `A20:H36` migrated); verify the documented endpoints match the implementation
- [x] 4.2 Run the full verification set — `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm build`, `pnpm exec vue-tsc --noEmit` — and fix any failures
