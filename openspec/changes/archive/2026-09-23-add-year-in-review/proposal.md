# Proposal

## Why

The `YearInReview` sheet is the owner's annual cross-domain rollup — ledger aggregates, investment returns, and per-asset-class yields per year — and it is still maintained by hand in the workbook. Almost every figure it shows is already derivable from data the app stores (month ledger, trades, dividends, deposits, coupons, pool chain); only a handful of hand-entered cells (income, invested adjustments, sold P/L, pre-app bond/deposit figures) need storage. Migrating it removes one of the last hand-maintained summary sheets.

## What Changes

- New `GET /api/year-review` endpoint returning one row per year (every year with a month row or seeded data) covering the sheet's three column groups: ledger aggregates, investment summary, and per-asset-class returns with blended rates, 收入/存%, and YoY deltas.
- New `year_review` table storing per-year manual inputs (`income`, `invested_adjustment`) and nullable overrides for figures whose history was deleted from the workbook (`bond_principal`, `bond_interest`, `deposit_principal`, `deposit_interest`); NULL derives live, matching the `year_snapshots` convention.
- `year_snapshots` gains a `sold_pl` column (per market-year, manual) — defining the column `stock-yearly-summary` reserved — which feeds YearInReview's 投資P/L, the yearly summary table, and Month Stat's 投資純利 (currently always empty).
- `PATCH /api/year-review/:year` edits the per-year manual figures and clears/sets overrides; `PATCH /api/summary/yearly/:market/:year` also accepts `sold_pl`.
- Import seeds `income`/`invested_adjustment`/`sold_pl` for all sheet years and bond/deposit overrides for past years (current year derives live, matching `import_year_snapshots`).
- Parity check gains a `check_year_review` section comparing each year block against the sheet's cached cells; hand-frozen/divergent cells report informational.
- New top-level 年結 nav group with a 回顧 view reproducing the sheet's three-group block layout with inline editing of the manual cells.
- The `YYYY回報率` projection sheets stay unmigrated (out of scope).

## Capabilities

### New Capabilities

- `year-in-review`: the per-year review report — derivation rules for every cell of the sheet's three groups, the stored manual inputs and overrides, the API surface, the import seeding, and the frontend view.

### Modified Capabilities

- `stock-yearly-summary`: the reserved `sold_pl` column is now defined — a stored manual per-(market, year) figure editable through the yearly PATCH.
- `month-stat`: the yearly block's 投資純利 now reports `Σ interest + stored HK sold_pl` instead of always being absent.
- `app-navigation`: a new top-level group 年結 with a 回顧 tab is added to the nav order.

## Impact

- `backend/migrations/0021_year_review.sql` — new `year_review` table + `year_snapshots.sold_pl`.
- `backend/src/xlsx.rs` — `parse_year_review` extended to read the full block (income, invested, 投資P/L, bond/deposit cells) into `SheetYearReview`.
- `backend/src/import.rs` — `import_year_review` seeding.
- `backend/src/calc.rs`, `models.rs` — `year_review_rows` assembly, `YearReviewRow`/`YearReviewPatch`, `sold_pl` on yearly rows.
- `backend/src/routes/year_review.rs` (new), `yearly.rs` (`sold_pl` in patch/response), `months.rs` (pass real `hk_sold_pl` map), `mod.rs`.
- `backend/src/parity.rs` — `check_year_review`.
- `frontend/src/api.ts`, `views/YearReviewView.vue` (new), `App.vue`, `views/SummaryView.vue` (optional sold_pl editing).
- `docs/DATA_FLOW.md`, `AGENTS.md`.
