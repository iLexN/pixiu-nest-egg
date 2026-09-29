# Proposal

## Why

When a dividend arrives, the stock's 現價 (`manual_price`) has usually already been updated — often via `import_prices`. The 收訖派息 form still asks for 當時現價 by hand, so the user retypes a number the system already knows, or leaves it blank and loses the `yield_on_price` figure entirely.

## What Changes

- On the receipt transition (`received_amount` NULL → set), when the update supplies no `received_price` — field absent or null — the record SHALL store the stock's current `manual_price` as `received_price`. If the stock has no `manual_price`, `received_price` stays NULL (unchanged from today).
- An explicitly supplied `received_price` always wins; clearing `received_price` on an already-received record (no receipt transition) still clears it.
- No frontend changes are required: the 收訖派息 form already sends `received_price: null` for a blank field, which now resolves to the stock's current price. The 同時更新現價 checkbox is unaffected.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-dividends`: the "Receiving a dividend" requirement gains a default rule — a receipt that carries no `received_price` snapshots the stock's current `manual_price` at the moment of receipt.

## Impact

- `backend/src/routes/dividends.rs` — `update()` resolves the default inside the existing receipt transaction.
- `backend/tests/api.rs` — new coverage for the default rule.
- No schema migration, no API shape change, no frontend change.
- `docs/stocks.md` and `docs/DATA_FLOW.md` document the receipt flow and need a one-line update.
