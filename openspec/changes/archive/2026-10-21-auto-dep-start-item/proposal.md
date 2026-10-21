## Why

Creating a 定期 only produces a `dep-start` *suggestion* in the start month's 調整 — the user must open the month and accept it by hand. Every other cash event (dep-end 收訖, coupon 收訖, bond principal 收訖) records its month item atomically, so the start of a deposit is the one event that still needs a second step.

## What Changes

- `POST /api/deposits` records the `dep-start:<id>` `adjustment` item of −`principal` in the `start_date` month inside the same transaction as the insert — the same semantics as the dep-end item on 收訖 (skipped when the item is already stored or the start month has no row).
- The `dep-start` suggestion stays as the catch-all: a deposit whose start month had no row at create time still surfaces the suggestion once the month exists.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `time-deposits`: deposit creation additionally records the start month's `dep-start:<id>` adjustment item, matching the dep-end receipt-item rule.

## Impact

- Backend: `service::deposits` gains a create path wrapping `insert_deposit` + `record_receipt_item` in one transaction; `routes::deposits::create` calls it. No schema migration, no new endpoints, no API shape change.
- `month-stat` spec unchanged: the `dep-start` suggestion requirement remains accurate as the fallback.
- Docs: `docs/deposits.md` flow note updated.
