## 1. Backend calculation core

- [x] 1.1 Extend `StockSummary` in `backend/src/calc.rs` with `dividends_received: f64`, `dividend_return: Option<f64>`, `net_invested: f64`, `net_diluted_price: Option<f64>`, `real_total_return: Option<f64>`; add a `dividend_total: f64` parameter to `summarize` implementing 累計派息% `= dividends ÷ 總買入成本` (None when cost is 0), 淨投入總本金 `= cost − dividends`, 淨攤薄單價 `= net_invested ÷ shares_held` (None when held is 0), 實質動態總回報% `= (market_value − net_invested) ÷ net_invested` (None when market_value is None or net_invested is 0)
- [x] 1.2 Add `dividends_received: f64` to `RollupInput` and `dividends_received: f64`, `net_invested: f64`, `net_invested_priced: f64`, `real_total_return: Option<f64>` to `MarketTotals` in `market_totals` — priced subset mirrors `buy_cost_priced`
- [x] 1.3 Update existing `summarize` test call sites and add unit tests: 中國銀行-style case (cost 208746.64, held 68000, price 5.91, dividends 54023.87 → net_invested 154722.77, diluted 2.275335, return 1.597420, yield 0.258801), no-dividend case, zero-holdings, zero-cost, and no-price edge cases; verify `cargo test` passes

## 2. Summary route

- [x] 2.1 In `backend/src/routes/summary.rs::build`, query `SELECT d.stock_id, SUM(d.received_amount) FROM dividends d JOIN stocks s ON s.id = d.stock_id WHERE s.market = ? AND d.received_amount IS NOT NULL GROUP BY d.stock_id` into a map, pass each stock's sum (default 0) to `summarize`, and populate `RollupInput.dividends_received`; verify the API response includes the five new fields per stock and the totals aggregates

## 3. Frontend

- [x] 3.1 Extend `SummaryStock` and the totals interface in `frontend/src/api.ts` with the new fields
- [x] 3.2 In `frontend/src/views/SummaryView.vue` append five columns after 未實現報酬率 — 累計派息 (`fmtMoney`), 累計派息% (`fmtPercent`+`signClass`), 淨投入總本金 (`fmtMoney`), 淨攤薄單價 (`fmtPrice`), 實質動態總回報% (`fmtPercent`+`signClass`) — and add 累計派息, 淨投入總本金, 實質動態總回報% totals cards; verify `pnpm build && pnpm exec vue-tsc --noEmit` passes and both HK/US tabs render the columns

## 4. Parity check

- [x] 4.1 Extend `SheetSummary` in `backend/src/xlsx.rs` with `cumulative_dividends` (col index 10), `dividend_return` (15), `net_invested` (20), `net_diluted_price` (21), parsed for `Market::Hk` only
- [x] 4.2 In `backend/src/parity.rs::check_market`, compare those cached HK columns against figures recomputed with effective dividend amounts (`SUM(COALESCE(received_amount, estimated_amount))` per stock); do not compare W; verify `cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"` reports matches including stocks with pending dividends

## 5. Docs and verification

- [x] 5.1 Document the five new fields and the received-only 累計派息 semantics in `docs/DATA_FLOW.md` under the summary section
- [x] 5.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — all clean
