# Proposal

## Why

The owner tracks a per-year performance table in a spreadsheet/Google Sheet (columns: yearly invested, year-end 成本, year-end 總市值, 派息, yields, and year-over-year changes). Because the sheet recalculates live, the owner must manually copy raw values at the end of each year to freeze `end of year total 成本` and `end of year value 總市值`. The app can already recompute most of these columns from stored trades and dividends, but has no price history, so past year-end market values still need a place to be stored. Migrating this table removes one more manual sheet section and replaces the copy-paste freeze with an explicit snapshot.

## What Changes

- Add a per-market yearly summary table to 總覽 (SummaryView), one table under each HK/US market tab, one row per year from the earliest trade/dividend year through the current year.
- New `year_snapshots` SQLite table storing per `(market, year)` user-frozen values: `market_value` (required for past years), optional `cost` override, optional `invested` override, and `frozen_at`.
- New API surface:
  - `GET /api/summary/yearly?market=HK|US` — yearly rows with computed and derived columns.
  - `PATCH /api/summary/yearly/:market/:year` — store/update frozen values (manual paste of sheet figures).
  - `POST /api/summary/yearly/:market/:year/freeze` — snapshot the currently computed year-end 成本 and 總市值 into the row.
- Extend the workbook importer to seed snapshots where the workbook holds year-end figures (`港股`/`美股` year blocks and `YearInReview` 股票 cost/"now value" rows), never overwriting an existing snapshot.
- Computed columns per year and market: `invested` = Σ BUY − Σ SELL total for trades dated in the year; `成本` = cumulative Σ BUY total through Dec 31 (frozen snapshot value wins when present); `總市值` = snapshot for past years, live totals for the current year; `派息` = Σ `received_amount` with `pay_date` in the year (received only, estimates excluded).
- Derived columns: 報酬率 1 = 派息 ÷ 成本, 報酬率 2 = 派息 ÷ 總市值, 月均派息 = 派息 ÷ 12, 派息 YoY = (J − J_prev) ÷ J_prev, invested YoY = (F − F_prev) ÷ F_prev where F is the cumulative 成本 column. Sold P/L column is reserved but empty for now.

## Capabilities

### New Capabilities
- `stock-yearly-summary`: per-market, per-year rollup of invested/cost/market value/dividends with year-end snapshot storage and derived yield/YoY columns.

### Modified Capabilities
<!-- none: the change adds a new view and endpoints; existing summary/trade/dividend requirements are unchanged -->

## Impact

- `backend/`: new `year_snapshots` migration; new route module `routes/yearly.rs` (or extension of `routes/summary.rs`); rollup helpers in `calc.rs`; importer additions in `import.rs`/`xlsx.rs`; possible parity coverage in `parity.rs`.
- `frontend/`: `SummaryView.vue` gains a year table per market tab with inline edit for frozen values and a freeze action; `api.ts` gains response types.
- `docs/DATA_FLOW.md`: new data-flow section for the yearly table and snapshot freeze.
- No external dependencies; all calculations stay local in the backend per project conventions.
