# Tasks

## 1. Backend: raise storage and derivation

- [x] 1.1 Add `backend/migrations/0023_year_review_raise.sql` (`ALTER TABLE year_review ADD COLUMN raise REAL`); verify `cargo test` passes with the migration applied
- [x] 1.2 In `models.rs`, add `raise` to the year-review record, `YearReviewPatch` (`Option<Option<f64>>`, finite — negative allowed), and the effective `raise` on `YearReviewRow`'s investment section
- [x] 1.3 In `calc.rs`, extend `year_review_rows` with a `last_salaries: &BTreeMap<i32, f64>` input and derive `raise` = stored override else `max(0, lastSalary(Y) − lastSalary(Y-1))`; unit tests cover derived/override/zero/absent cases
- [x] 1.4 In `routes/year_review.rs`, load the per-year last-salary map in `build` (latest `month_stats` month per year with non-NULL `salary`) and accept `raise` in the PATCH upsert; verify `GET /api/year-review` reports `raise` `1890` for 2026 and a `PATCH … {"raise": 2500}` then `{"raise": null}` round-trips

## 2. Backend: 投資目標 block

- [x] 2.1 In `calc.rs`, add `invest_targets(rows, current_year)` producing `avg_invested` (mean of `invested` over the last three completed years, skip-absent) and per-row `target`/`remain`/`growth` per the spec formula; unit tests cover the 2026 scenario numbers (`target` ≈ `430432.36`), completed-year growth, first-year absences, and the skip-absent average
- [x] 2.2 In `models.rs`, add `InvestTargetRow`/`InvestTargets` and `invest_targets` on `OverviewResponse`; in `routes/overview.rs`, call `year_review::build` and assemble the block; verify `GET /api/overview` returns the four rows with `avg_invested` ≈ `326841.22`

## 3. Import and parity

- [x] 3.1 In `xlsx.rs`, extend `OverviewCached` with the `J22:N27` cells (`avg_invested` J22, per-year `year`/`invested`/`target`/`remain`/`growth` from `J23:N26`); parse in `parse_overview` and extend the workbook test
- [x] 3.2 In `import.rs`, seed `invested_adjustment` from the Overview `J:K` year/invested cells (`K − HK net invested − Σ transfers`) for every listed year, never overwriting; verify a fresh import stores 2023 `+110000`, years covered by `YearInReview` seed identically, and a re-import creates no duplicates
- [x] 3.3 In `parity.rs` `check_overview`, compare `J22`, the `K` cells, and completed-year `N` cells normally, reporting `L`/`M` and the current-year `N` as informational; verify `check_parity` shows `K23` matching `206523.15`

## 4. Frontend

- [x] 4.1 In `api.ts`, add `InvestTargetRow`/`InvestTargets`, `invest_targets` on `OverviewResponse`, and `raise` on the year-review row/patch types; verify `vue-tsc --noEmit`
- [x] 4.2 In `OverviewView.vue`, add the 投資目標 card (近3年平均 header + 年份/invested/target/remain/growth% table, `—` for absent); verify it renders against the dev server
- [x] 4.3 In `YearReviewView.vue`, show the effective `raise` per year with inline edit/clear via `PATCH /api/year-review/:year`; verify editing then clearing restores the salary-derived figure

## 5. Docs and integration

- [x] 5.1 Update `docs/DATA_FLOW.md` (投資目標 block, raise derivation, 2023 seeding) and `AGENTS.md` roadmap
- [x] 5.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, then `import_xlsx` + `check_parity`; the `J22`/`K`/completed-`N` cells match and `L`/`M`/current-`N` report informational
