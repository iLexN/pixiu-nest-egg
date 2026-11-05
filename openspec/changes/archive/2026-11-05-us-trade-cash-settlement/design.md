# Design

## Context

`usd_cash` on the IBKR card is the `amount` of the `broker`-role USD row in `cash_accounts`. Today only `PATCH /api/ibkr` and USD dividend receipts write it; `routes::trades` calls `store::trades` directly with no service composition. The codebase already has two cash-movement patterns to follow: `service::ibkr::create_transfer` (one transaction: log row + `cash_accounts::add_amount` on both sides) and the receipt crediting rule (stores `credited_account_id`/`credited_amount` for exact reversal). The domain fact driving the scope: US trades settle inside IBKR; HK trades settle at the HS bank, which is why they are month-stat 活期 suggestions.

## Goals / Non-Goals

**Goals:**
- US trade writes move the USD broker account atomically, with exact reversal via a recorded amount.
- Settlement lands in `service::trades`, keeping routes thin per the layer rules.

**Non-Goals:**
- HK trades, month items, forecast trade_flows, the UI, and the API surface — all unchanged.
- Modelling in-app HKD→USD FX conversions or a dated cash ledger — the balance stays an undated figure reconciled by PATCH.

## Decisions

- **Record `settled_amount` per trade, don't recompute.** A nullable `trades.settled_amount` (migration `0036`) holds the signed delta applied (BUY −total, SELL +total — `calc::trade_amount`'s convention). Update reverses the stored amount then applies the new one; delete reverses it. Alternative considered — recompute the old row's signed total at update/delete time — needs no migration but breaks the moment a pre-change row (which never moved cash) is edited or deleted, and loses the receipt-style audit trail. NULL = unsettled, which is also what makes pre-existing rows inert for free.
- **`股數 > 0` is the settlement condition.** Zero-share rows are 加權平均買入單價 corrections, not cash events — so a settled trade edited to 0 shares unwinds its recorded amount and records none. Alternative — settle them as cash adjustments — contradicts how the rows are actually used.
- **Reject market-changing `stock_id` repoints.** With a trade's market fixed at creation, settlement always targets one account and there is no cross-market unwind path to design or test. Same-market repoints stay allowed.
- **No balance check.** `cash_accounts::add_amount` already adjusts without the non-negative guard (receipt reversals rely on this); IBKR margin makes negative USD cash a real position, not an error.
- **Service wrappers mirror `service::ibkr::create_transfer`**: `pool.begin()` → store write → `add_amount` on the USD broker row (resolved via `cash_accounts::broker`) → commit. Routes keep doing resolve/validate and swap the `store::trades` calls for the service functions; `record_market` stays in the route, after the committed transaction.

## Risks / Trade-offs

- Pre-change trades stay inert forever (edit/delete included) → documented behaviour; delete + recreate settles a trade if ever needed.
- USD cash still drifts from the app's real figure (interest, FX conversions) → unchanged from today; PATCH reconciles. `vs_now_value` remains the visible gap check.
- Deleting a settled trade can push the balance negative → accepted; the recorded reversal is still exact.
- `insert_trade` is also called by test fixtures/importers → settlement lives in the service layer, so direct store calls (tests, `import_prices`-style tooling) never move cash unexpectedly.
