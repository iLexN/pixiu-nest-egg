# Proposal

## Why

The MPF page already derives 上月 and 最高 figures automatically from recorded history, but the HK/US portfolio summaries only ever show the current 未實現金額 and 未實現報酬率 — there is no way to see last month's numbers or the all-time highs without the spreadsheet. Recording a per-market history of the totals lets the app derive the same last-month and max figures for stocks, with zero extra input from the owner.

## What Changes

- New `market_history` table recording one row per market per day: `market`, `recorded_on`, `buy_cost_priced`, `market_value`, `synthetic` flag, `UNIQUE(market, recorded_on)` — rebuilding the summary again on the same day replaces that day's row instead of appending.
- Recording flow (whenever a market summary is computed with at least one priced stock):
  1. Upsert today's history row with the current `buy_cost_priced` / `market_value`.
  2. Month-rollover backfill: for every calendar month fully elapsed since the previous record that has no rows, insert a synthetic month-end row carrying the last-recorded values forward, so 上月 never shows a gap when the owner skips a month.
  3. `prices::apply` additionally triggers a summary build for every market it updated, so bulk/CLI price imports are captured even when the market's summary is never viewed.
- Derived on read, never stored: per-market 上月 (latest history row in the previous calendar month) and 最高 (maxima over the imported seed marks, every history row, plus the current values) for 未實現報酬率 `(market_value − buy_cost_priced) ÷ buy_cost_priced` and 未實現金額 `market_value − buy_cost_priced`. The two maxima are tracked independently — they may come from different moments.
- Summary API: `GET /api/summary?market=HK|US` gains `last_month` and `max` figure fields, each carrying `percent` and `amount`.
- 持倉總覽 totals strip gains 上月 and 最高 cells next to 未實現報酬率, showing `percent / amount` pairs colored against the current values, matching the MPF cards.
- `import_xlsx` seeds synthetic Dec-31 history rows from `year_snapshots` (HK 2023–2025 year-end 成本/總市值), so 最高 reflects real history immediately and January's 上月 resolves to the prior year-end. The workbook attributes no year-end figures to US, so the US year-end history starts empty.
- `import_xlsx` additionally seeds each market's cached `last month` rate as a synthetic previous-month-end history row (components reconstructed from the market's Σ BUY total), and the cached `max Balance %` / `max net` cells as per-market maxima marks in `app_meta` that floor the derived 最高 — the same seed pattern MPF uses.
- `check_parity` compares the market's derived 上月/最高 figures against the sheet's cached cells, loosely — real records may legitimately diverge from stale sheet marks.
- Not in this change: per-stock last-month/max (no per-stock price history is kept), a market-history viewer or row deletion UI, and any charting.

## Capabilities

### New Capabilities

### Modified Capabilities
- `stock-portfolio-summary`: the summary records a per-market totals history (daily upsert + synthetic month-end backfill), derives 上月/最高 figures for 未實現報酬率 and 未實現金額 on read over seed marks + history + current, and the import seeds synthetic year-end history rows from `year_snapshots`.
- `spreadsheet-trade-import`: the import command also seeds `market_history` year-end rows from `year_snapshots` idempotently, plus each market's cached `last month` row and `max Balance %` / `max net` maxima marks.

## Impact

- Backend: new migration `0011_market_history.sql`; new `market_history.rs` module; additions to `models.rs`, `routes/summary.rs`, `prices.rs`, `import.rs`, `xlsx.rs` (cached cells), `parity.rs` (figure comparisons).
- Frontend: `api.ts` types; `SummaryView.vue` totals cells; shared figure/compare helpers lifted from `MpfView.vue` into `format.ts`.
- Docs: `docs/DATA_FLOW.md` gains the market history flow; `AGENTS.md` notes the new derived figures.
- No new dependencies; `財富分析報告.xlsx` stays read-only and unchanged.
