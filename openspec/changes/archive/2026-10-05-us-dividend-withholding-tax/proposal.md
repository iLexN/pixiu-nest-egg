# Proposal

## Why

When a US dividend arrives, the broker shows the gross 派息 and the withholding tax on separate lines; the app only accepts the net 實收派息, so the user subtracts by hand on every receipt and the tax figure is lost afterwards. The backend owns financial calculations, so it should derive the net amount from what the user actually sees.

## What Changes

- New nullable `dividends.withholding_tax` column (migration `0027`). Existing and imported rows keep it NULL; no backfill.
- `PATCH /api/dividends/:id` accepts two new fields:
  - `gross_amount` (input-only, never stored): when supplied, the backend sets `received_amount = gross_amount − withholding_tax` (tax taken from the request, else the stored value, else 0).
  - `withholding_tax` (nullable): stored as given.
- New validation: tax finite and non-negative, not greater than gross, and only allowed on a record that has (or is getting) a `received_amount`; `gross_amount` together with an explicit `received_amount` in one request is rejected as ambiguous.
- Clearing `received_amount` (un-receive) also clears `withholding_tax`.
- Dividend responses gain `withholding_tax` and a derived `gross_amount` (= `received_amount + withholding_tax` when both are present, else null).
- `received_amount` keeps meaning the net amount, so banking into `ibkr.usd_cash`, yields, yearly rollups, parity, and 利息 are unchanged. `variance` stays `received_amount − estimated_amount`.
- UI, US dividends only: the 收訖 form and the edit form for a received record take 派息 (gross, prefilled from 預期派息 on receipt) and Tax, with a display-only 實收派息 preview; the received table shows Tax. HK forms are unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-dividends`: the registry gains `withholding_tax`; validation gains the tax/gross rules; derived fields gain `gross_amount`; receiving a dividend can supply gross + tax instead of the net amount, and un-receive clears the tax.

## Impact

- `backend/migrations/0027_dividend_withholding_tax.sql` — new column.
- `backend/src/models.rs` — `Dividend`, `DividendPatch`.
- `backend/src/calc.rs` — dividend validation and the net-from-gross helper, with unit tests.
- `backend/src/routes/mod.rs` — `DIVIDEND_SELECT`, `row_to_dividend`.
- `backend/src/routes/dividends.rs` — `update()` resolves gross/tax, persists tax, clears it on un-receive.
- `backend/tests/api.rs` — receipt-with-tax coverage.
- `frontend/src/api.ts`, `frontend/src/components/DividendReceiveForm.vue`, `DividendForm.vue`, `DividendTable.vue`.
- `docs/database.md`, `docs/stocks.md`, `docs/DATA_FLOW.md`.
- No new endpoint; the OpenAPI operation list is unchanged. Import and parity are unaffected.
