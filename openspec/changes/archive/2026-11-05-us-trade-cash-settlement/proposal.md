# Proposal

## Why

Recording a US trade today leaves the IBKR USD 現金 untouched: the figure on the IBKR card is a hand-entered broker `cash_accounts` amount that only moves via `PATCH /api/ibkr` or USD dividend receipts, so every trade silently makes it stale. US trades settle inside IBKR (HK trades settle at the HS bank and keep their separate 活期 model), so a recorded US trade should move the broker's USD cash by itself.

## What Changes

- A US trade creation debits/credits the USD broker `cash_accounts` amount by the trade total (BUY −total, SELL +total) in the same transaction as the insert.
- Editing a US trade reverses its recorded settlement and applies the new signed total; deleting one reverses it. A new `trades.settled_amount` column records the applied delta so reversal is exact.
- Trades with no recorded settlement (all rows written before this change) never move cash, even on edit or delete.
- Settlement applies only to rows with 股數 > 0; zero-share rows stay cost-basis corrections for 加權平均單價 and move no cash.
- The USD broker amount may go negative (IBKR margin is legitimate); no balance check is added.
- **BREAKING**: `PATCH /api/trades` rejects a `stock_id` whose market differs from the trade's recorded market — a trade's market is immutable once recorded (previously allowed silently).
- HK trades are unchanged: they remain month-stat 活期 suggestions and move no broker cash.

## Capabilities

### New Capabilities

### Modified Capabilities

- `stock-trades`: add the US settlement requirement, the recorded-amount reversal rule, the 股數 > 0 condition, and market immutability on edit; correct the zero-share scenario wording that claimed those rows exist for brokerage cash adjustments (they exist to correct 加權平均單價 against the IBKR app).
- `cash-accounts`: acknowledge that US trades adjust the USD broker account balance on create/update/delete, alongside the existing receipt crediting rule.

## Impact

- `backend/migrations/0036_trade_settlement.sql` — new nullable `trades.settled_amount` column.
- `backend/src/service/trades.rs` — new create/update/delete wrappers that settle the USD broker row in one transaction.
- `backend/src/store/trades.rs` — read/write `settled_amount`.
- `backend/src/routes/trades.rs` — call the service layer; reject market-changing repoints.
- `docs/database.md`, `docs/stocks.md`, `docs/overview.md` — schema and flow docs.
- No API surface changes (no new endpoints, no request/response shape changes); no frontend changes.
