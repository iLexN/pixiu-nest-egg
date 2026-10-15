# Proposal

## Why

Banks often quote only the interest amount for a 定期, so the user leaves 利率 blank and the deposit list shows no rate. The rate can be worked out from 本金, 利息 and the 開始日 to end-date span, so the app should calculate it rather than make the user do it by hand.

## What Changes

- When a deposit has no entered rate, the API reports an annual rate derived on read as `interest ÷ principal × 365 ÷ (end_date − start_date)`, using actual days. It is never stored.
- A derived rate is reported only when the principal is positive, the interest is present, the start date is present, and the end date is after the start date. Otherwise the rate stays empty.
- A rate the user entered always wins, and existing stored rates are unchanged.
- The deposit response gains a `rate_derived` flag. The 利率 column shows a tooltip on derived rates. The edit form leaves the rate field blank for a derived rate and shows it only as a placeholder, so saving does not freeze it into storage.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `time-deposits`: "Derived deposit fields" gains the derived rate rule. "Display formatting" gains the marker that distinguishes a derived rate.

## Impact

- Backend: `calc/deposits.rs` (new pure derivation), `store/deposits.rs` (`row_to_deposit`), `models.rs` (`Deposit.rate_derived`). This affects every endpoint returning `Deposit`. The OpenAPI schema gains the field automatically.
- Frontend: `api.ts`, `DepositTable.vue`, `DepositForm.vue`.
- Docs: `docs/deposits.md`.
- No migration. Request bodies are unchanged. `family_deposits` is unaffected because it has no rate.
