## Why

The 持倉總覽 summary does not show the dividend-adjusted position figures the owner tracks on the workbook's 港股 sheet: 累計派息, 累計派息%, 淨投入總本金, 淨攤薄單價, and 實質動態總回報%. Migrating them removes another reason to keep maintaining that sheet by hand.

## What Changes

- Extend the per-stock summary (`GET /api/summary`) with five derived fields, for both HK and US markets:
  - 累計派息 — sum of the stock's **received** dividend amounts only. The workbook's K column sums every J–O row including not-yet-received ones; the app deliberately counts only money actually received.
  - 累計派息% — 累計派息 ÷ 總買入成本; empty when 總買入成本 is 0.
  - 淨投入總本金 — 總買入成本 − 累計派息 (mirrors the sheet's `F − K`).
  - 淨攤薄單價 — 淨投入總本金 ÷ 股數 held (the sheet's `U ÷ D`); empty when holdings are 0.
  - 實質動態總回報% — (當前總市值 − 淨投入總本金) ÷ 淨投入總本金 (the sheet's `(H − U) ÷ U`); empty when the stock has no 現價/market value or 淨投入總本金 is 0.
- Extend market totals with aggregate 累計派息, 淨投入總本金, and 實質動態總回報% — the return computed over the priced subset (mirroring `buy_cost_priced`).
- Render the five new columns and three new totals cards in 持倉總覽.
- Extend the parity check to compare the workbook's cached K/P/U/V columns on the 港股 sheet against recomputed values. Because the sheet counts pending dividend rows, parity compares against the *effective* amount (received else estimated) rather than the received-only figure shown in the UI. W (實質動態總回報%) is not compared: the sheet's market value uses live market-data prices while the app uses manual 現價, so it would diverge for reasons unrelated to correctness.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-portfolio-summary`: the per-stock summary and market totals gain dividend-adjusted figures; defines the received-only 累計派息 semantics and the empty-cell rules.
- `spreadsheet-trade-import`: the parity command gains comparisons for the 港股 sheet's 累計派息, 累計派息%, 淨投入總本金 and 淨攤薄單價 cached columns, computed with effective dividend amounts to match the sheet's semantics.

## Impact

- `backend/src/calc.rs` — `summarize` gains a dividend-total parameter; `StockSummary`, `RollupInput` and `MarketTotals` gain fields.
- `backend/src/routes/summary.rs` — one grouped query sums `received_amount` per stock and feeds it into `summarize`.
- `backend/src/xlsx.rs` — `SheetSummary` gains the HK-only cached columns.
- `backend/src/parity.rs` — new comparisons in the market check.
- `frontend/src/api.ts` — `SummaryStock` and the totals interface gain fields.
- `frontend/src/views/SummaryView.vue` — five columns and three totals cards.
- `docs/DATA_FLOW.md` — the summary data-flow section documents the new fields.
- No schema migration needed: everything derives from existing `trades` and `dividends` rows.
