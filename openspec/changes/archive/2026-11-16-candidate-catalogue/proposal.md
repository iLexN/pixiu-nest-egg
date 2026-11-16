# Proposal

## Why

Every Candidate (the event-derived month item behind 月結's 建議 rows) is defined twice: once in `calc::build_suggestions` and again in each write path that records it on 收訖, placement or 轉入. Each copy builds its own `auto_key`, month, sign and label, and the copies have already drifted:

- An unlabeled deposit's suggestion stores a `null` label, but its write path stores "定期".
- Clearing a coupon or dividend 收訖 deletes the item from the *new* `pay_date` month, which can leave the original item orphaned.
- Bond 收訖 records the principal even though the spec says it records the credited amount.
- Bonds have no bond-end Candidate at all. A bond 收訖'd before its maturity month row exists therefore loses its 調整 for good.

## What Changes

- Define each Candidate once. A per-source builder turns the source's facts into one Candidate: key, month, category, label and amount. Suggestions, 利息 components and every write path use the same builder.
- Store a Candidate through one record operation and remove it through one retire operation. Retire removes the key's item in whichever month it sits, along with its dismissals.
- An unlabeled deposit's dep-start and dep-end Candidates are labelled "定期" in both the suggestion and the stored item.
- Clearing a coupon's or HK dividend's `received_amount` retires its item in whichever month it sits.
- Bond principal 收訖 records the bond-end item at the credited amount, not the principal.
- **New Candidate**: `bond-end:<id>`, an `adjustment` of + the bond's principal in its maturity month. It is suggested before 收訖, with the same lifecycle as dep-end.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `month-stat`: adds requirements for the bond maturity Candidate, the "定期" label for deposit Candidates, recorded Candidates matching their suggestion, and retiring a Candidate item.
- `bonds`: Bond principal 收訖 records the credited amount. Coupon receipt reversal retires the item in whichever month it sits.
- `stock-dividends`: Dividend receipt reversal retires the item in whichever month it sits.
- `time-deposits`: The deposit creation and receipt month items use the "定期" label fallback.

## Impact

- Backend:
  - `calc/suggestions.rs`: Candidate builders and the typed key.
  - `store/months.rs`: record and retire replace `record_receipt_item`, `delete_auto_item` and `delete_auto_items_by_key`.
  - `store/bonds.rs`: a bond-end event source.
  - `service/{deposits,bonds,dividends,ibkr,months}.rs`: all call the shared builders.
- API: `GET /api/months/:ym` can return a new `bond-end:<id>` suggestion with `source` `bond`. No request or response shapes change, so `api-types.ts` doesn't change.
- No schema change. Stored `auto_key` strings keep their exact format.
- Docs: `docs/DATA_FLOW.md`, `docs/deposits.md`, `docs/investments.md`.
