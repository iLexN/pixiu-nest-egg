# Tasks

## 1. Backend

- [x] 1.1 In `xlsx.rs`, extend `OverviewCached` with `avg_total_change` (G4), `avg_month_spend` (G5), `avg_living_spend` (G6), `living_budget` (H6), `avg_saved` (G7), `avg_interest` (G8), `pool_balance` (G10); parse in `parse_overview`; extend the workbook test
- [x] 1.2 In `calc.rs`, add `TrailingAverages` + `trailing_averages(rows, items, current_month)`: latest 12 rows with `month < current`, per-column skip-empty averages (interest uses the `start_cash.is_some() || interest != 0` rule), `living_budget = ceil(avg × 1.05 / 100) × 100`; unit tests for the window, skip-empty, and ROUNDUP edge cases
- [x] 1.3 In `models.rs`, add `TwelveMonthAverages` (five `Option<f64>` averages, `living_budget: Option<f64>`, `living_budget_low: bool`, `living_budget_floor: Option<f64>`, `pool_balance: f64`) and `averages` on `OverviewResponse`; in `routes/overview.rs`, load months + items, run `month_derived` + `trailing_averages`, compute the flag from `totals.liquid_assets`, `pool_balance = input.pool_balance`; `cargo test`

## 2. Parity

- [x] 2.1 In `parity.rs` `check_overview`, compare the seven cached cells as informational outcomes; verify `check_parity` reports them

## 3. Frontend

- [x] 3.1 In `api.ts`, add `TwelveMonthAverages` + `averages` on `OverviewResponse`
- [x] 3.2 In `OverviewView.vue`, add the "過去 12 個月平均" card (rows 總數增加/支出/生活支出/生活預算/存/利息/開心 Pool); 生活預算 red when `living_budget_low`, floor in a tooltip; verify `vue-tsc --noEmit`

## 4. Docs and verification

- [x] 4.1 Update `docs/DATA_FLOW.md` (averages block + parity), `docs/MONTH_STAT_OVERVIEW.md` implemented note, `AGENTS.md` roadmap
- [x] 4.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, then `import_xlsx` + `check_parity`; the new cells match or report informational drift
