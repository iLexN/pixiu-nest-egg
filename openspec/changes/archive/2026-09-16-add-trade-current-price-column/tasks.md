## 1. TradeTable 現價 column

- [x] 1.1 In `frontend/src/components/TradeTable.vue`, import `signClass` from `../format` and add a computed `stock_id → manual_price` map from the existing `stocks` prop, plus helpers `currentPrice(trade)` and `priceClassFor(tradeType, avg, current)` that return `''` unless the type is `BUY` and both values are non-null; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 1.2 Add a `<th class="num">現價</th>` header immediately after `平均單價（含 fee）`, and in the read-only row a `<td class="num">` bound to `priceClassFor(trade.trade_type, trade.unit_price_incl_fee, currentPrice(trade))` rendering `fmtPrice(currentPrice(trade))`; verify in the browser that BUY rows show 現價 green/red vs 平均單價 and SELL rows show it uncolored
- [x] 1.3 Add the matching `<td>` to the editing row showing the draft-selected stock's 現價 (colored against `preview?.unitInclFee` when available), and update colspans for the 11-column table (empty row and error row `10` → `11`, tfoot trailing `3` → `4`); verify inline edit alignment and that the footer still spans correctly

## 2. Verification

- [x] 2.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and verify it succeeds
- [x] 2.2 Manually verify in 交易記錄 (HK and US): BUY row with 現價 above 平均單價（含 fee）renders green, below renders red (same colors as 未實現金額 in 持倉總覽), SELL rows uncolored, stocks without 現價 and zero-share rows uncolored, equal values uncolored
