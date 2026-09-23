# Tasks

## 1. Migration & models

- [x] 1.1 Create `backend/migrations/0021_year_review.sql`: `CREATE TABLE year_review (year INTEGER PRIMARY KEY, income REAL, invested_adjustment REAL, bond_principal REAL, bond_interest REAL, deposit_principal REAL, deposit_interest REAL, updated_at TEXT NOT NULL)` and `ALTER TABLE year_snapshots ADD COLUMN sold_pl REAL`. Verify: `cargo test db` passes and a fresh in-memory DB has both.
- [x] 1.2 `models.rs`: add `sold_pl: Option<f64>` to `YearSnapshot` and `YearlyPatch`; add `YearReviewRow` (all display fields + `manual` flags) and `YearReviewPatch` (`income`, `invested_adjustment`, `sold_pl`, `bond_principal`, `bond_interest`, `deposit_principal`, `deposit_interest` — all `Option<Option<f64>>` via `nullable` so `null` clears). Verify: compiles.

## 2. Workbook reader & import

- [x] 2.1 `xlsx.rs`: extend `parse_year_review` to also return `Vec<SheetYearReview>` — per block year: `income` (收入 row I), `invested` (F of invested row), `sold_pl` (F of 投資P/L row), `bond_principal`/`bond_interest` (債券 row I/J), `deposit_principal`/`deposit_interest` (定期 row I/J), plus cached cells needed for parity; add `year_review` to `WorkbookData`. Verify: a unit test reads the workbook and finds 2024 income `634830`, sold_pl `-14991.49`, bond `130000`/`5728.26`.
- [x] 2.2 `import.rs`: `import_year_review` — seed `income` and `invested_adjustment` (= sheet `invested` − HK net invested computed from trades) for all block years, `sold_pl` into HK `year_snapshots` (INSERT … ON CONFLICT keep existing, like `import_year_snapshots`), and the four bond/deposit overrides only for `year < current_year`; report counts in `ImportReport`. Verify: fresh scratch-DB import seeds 2024/2025 overrides and 2026 has NULL overrides with income/adjustment set; re-import is a no-op.

## 3. Calculation

- [x] 3.1 `calc.rs`: `year_review_rows(...)` assembling per-year rows — reuse `month_year_summaries`/`pool_balances` (÷12 averages per the sheet), HK yearly figures (cost/invested/dividends/now value, snapshot-aware), bond figures (held-in-year rule + received coupons), deposit figures (end_date in year and ≤ today), blended rates, saved/存% derivations, and YoY deltas (empty on missing/zero prior). Verify: `cargo test calc` — seeded fixtures reproduce the sheet's 2025 block numbers.
- [x] 3.2 `routes/months.rs` `summary`: load HK `sold_pl` from `year_snapshots` and pass it to `month_year_summaries` so `net_investment` populates. Verify: api test — storing HK sold_pl makes the year's `net_investment` = interest + sold_pl.

## 4. Routes

- [x] 4.1 `routes/yearly.rs`: accept `sold_pl` in `update` (validated finite, like `invested`), store it on the snapshot, and return it in each year row. Verify: api test round-trips `sold_pl` via PATCH.
- [x] 4.2 New `routes/year_review.rs`: `GET /api/year-review` (assembles all rows) and `PATCH /api/year-review/{year}` (upserts the six `year_review` fields — `null` clears to derived — plus `sold_pl` onto the HK snapshot; validates finite, non-negative where required). Register in `routes/mod.rs`. Verify: api test — PATCH income then GET reflects the change and derived cells reprice.

## 5. Parity

- [x] 5.1 `parity.rs`: `check_year_review` — per block year compare the row's figures against the sheet's cached cells; hand-frozen legitimately-divergent cells (e.g. the sheet's stale dividend totals, the missing 2026 bond) report informational. Verify: `check_parity` on the real workbook lists year-review rows with expected outcomes.

## 6. Frontend

- [x] 6.1 `api.ts`: `YearReviewRow` interface, `yearReview()`, `updateYearReview()`; add `sold_pl` to `YearRow`/`updateYearly` body. Verify: `pnpm exec vue-tsc --noEmit` compiles.
- [x] 6.2 `views/YearReviewView.vue`: one block per year reproducing the sheet's three groups with YoY columns; inline-edit `income`, `invested_adjustment`, `sold_pl`, and the four overrides (clear → derived), following `SummaryView`'s `saveYearEdit` pattern; reload after mutations. Verify: `pnpm build` clean; the page renders 2024–2026 matching the sheet.
- [x] 6.3 `App.vue`: new `年結` group with a `回顧` tab rendering `YearReviewView`. Verify: nav shows the new group and the page loads.
- [x] 6.4 (optional) `SummaryView.vue`: expose `sold_pl` editing in the yearly table. Verify: `vue-tsc` clean.

## 7. Docs & verification

- [x] 7.1 `docs/DATA_FLOW.md`: new Year-in-Review section (UI action → API → table → calc path). Verify: wording matches behavior.
- [x] 7.2 `AGENTS.md`: move YearInReview to the completed list; note `YYYY回報率` sheets remain unmigrated. Verify: roadmap text updated.
- [x] 7.3 Full check: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, scratch-DB `import_xlsx` + `check_parity` — all pass with expected parity output.
