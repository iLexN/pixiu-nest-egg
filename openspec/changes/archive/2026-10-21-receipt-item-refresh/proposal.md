## Why

Accepting a pending receipt suggestion stores the expected/estimated amount, and a later 收訖 never corrects it: `record_receipt_item` skips the write when the keyed item already exists, so the 調整 item can stay stale against the real 實收 figure. The same staleness applies to dep-end with corrected interest.

## What Changes

- Receipt writes become upserts by key: when the month's `coupon:<id>` / `div:<id>` / `dep-end:<id>` / `bond-end:<id>` (or `dep-start:<id>`) item already exists, the receipt updates its `label` and `amount` to the actual figure instead of skipping. The write is still skipped only when the month has no row.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `time-deposits`: dep-end/dep-start month items update an already-stored keyed item to the event amount instead of skipping it.
- `bonds`: `coupon:<id>` and `bond-end:<id>` month items update an already-stored keyed item instead of skipping it.
- `stock-dividends`: `div:<id>` month item updates an already-stored keyed item instead of skipping it.

## Impact

- Backend: `store::months::record_receipt_item` updates the keyed item when present. All five receipt paths inherit it; no API or schema change.
- A hand-edited auto item would be overwritten by the next receipt write — accepted.
