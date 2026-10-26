# Tasks

## 1. Backend totals

- [x] 1.1 Add `PoolTotals { ilex_input, irene_input, irene_payable }` and a pure `pool_totals(&[MonthYearSummary])` in `backend/src/calc/months.rs`, with an inline test covering the 2024–2026 figures (≈64976.27 / 25319.64 / 7168.50) and the empty case; verify with `cargo test`
- [x] 1.2 Add `pool_totals` to `MonthSummaryResponse` in `backend/src/routes/months.rs` and fill it in `summary`; verify with `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`

## 2. Frontend HappyPool tab

- [x] 2.1 Add a `PoolTotals` interface and `pool_totals` on `MonthSummary` in `frontend/src/api.ts`
- [x] 2.2 Create `frontend/src/views/HappyPoolView.vue` with the four cards and the per-year table (all-zero years hidden), and register the `happyPool` tab after 年結 in `frontend/src/App.vue`; verify with `pnpm build && pnpm exec vue-tsc --noEmit`
- [x] 2.3 Document the HappyPool tab wiring in `docs/overview.md` and `docs/DATA_FLOW.md`
- [x] 2.4 Add a per-row `copy` button to the HappyPool table that writes `year<TAB>結餘<TAB>iLex input<TAB>Irene input<TAB>支出` (toFixed(2) values) to the clipboard and briefly shows `copied`, and mention it in `docs/overview.md`; verify with `pnpm build && pnpm exec vue-tsc --noEmit`
- [x] 2.5 Add `avg_spend` and `avg_input` (averaged over active years) to `PoolTotals`, with tests, plus the 平均支出 and 平均 input cards and docs; verify with `cargo test`, clippy, fmt, `pnpm build` and `vue-tsc`

## 3. Integration

- [x] 3.1 Restart the backend and confirm that `/api/months/summary` returns `pool_totals` matching the spec scenario and that the HappyPool tab renders

## Workflow follow-up

- Archive the change with `/openspec-archive-change` after review.
