## Context

`TradeTable.vue` renders the 交易記錄 history rows. Its parent `TradesView.vue` already loads the market's stocks via `api.listStocks(market)` — which includes `manual_price` (現價) — and passes them to `TradeTable` as the `stocks` prop (used today only by the edit-row stock `<select>`). Each `Trade` row already carries `stock_id` and the backend-computed `unit_price_incl_fee`. The green/red styles exist globally as `.positive`/`.negative` in `style.css`, applied via the `signClass()` helper in `format.ts` — the same helper 持倉總覽 uses for 未實現金額.

## Goals / Non-Goals

**Goals:**
- Show each row's stock 現價 beside 平均單價（含 fee）, sign-colored for BUY rows.
- Zero backend/API/schema changes; reuse existing data already on the page.

**Non-Goals:**
- Coloring SELL rows, the 平均單價 cell, or any other column.
- Fetching live market data; 現價 remains the manually stored `manual_price`.
- Changing `POST`/`PATCH /api/trades` responses or `TRADE_SELECT`.

## Decisions

- **Source 現價 from the `stocks` prop, not the trades API.** `TradesView` already has every stock's `manual_price` loaded; a `stock_id → manual_price` map inside `TradeTable` answers the lookup with no new field on `Trade`. Alternative considered: add `s.manual_price` to `TRADE_SELECT` and the `Trade` model — rejected because it changes the API surface and every consumer's payload for data the frontend already holds.
- **Color only the new 現價 cell, only for BUY rows** (user decision). SELL rows, zero-share rows (empty 平均單價), stocks with no 現價, and equal values render uncolored — `signClass(current − avg)` already returns empty for zero/missing inputs.
- **Column placement**: directly after `平均單價（含 fee）` so the two compared values sit adjacent. Table grows 10 → 11 columns; the empty row, inline error row, and footer `colspan`s are adjusted to match, and the editing row gets a matching cell showing the draft-selected stock's 現價 (colored against the live `preview` 平均單價 when available).
- **No recalculation in the frontend**: 平均單價（含 fee）remains the backend-derived `unit_price_incl_fee`; the table only formats and compares display values, consistent with the project's data-flow convention.

## Risks / Trade-offs

- [現價 is stock-level, not trade-level, so a sold-out stock still shows its last stored 現價 on every historical row] → Accepted; it matches the user's request and mirrors how 持倉總覽 treats 現價.
- [A trade whose `stock_id` is absent from `stocks` (e.g. inconsistent state) shows an empty cell] → Same handling as missing 現價; no error path needed.
