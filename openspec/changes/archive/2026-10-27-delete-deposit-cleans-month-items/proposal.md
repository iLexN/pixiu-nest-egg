# Proposal

## Why

Creating a deposit with a `start_date` stores a `dep-start:<id>` adjustment item (−principal) in the start month, but deleting the deposit only removes the deposit row. 預測 drops the deleted deposit because it derives from `deposits` on read, while 月結 調整 keeps the orphaned stored item (seen live as `6129  -140,000.00` in 2026-10), so the user has to delete it by hand.

## What Changes

- Deleting an unreceived deposit also removes its `dep-start:<id>` month item and any dismissal tombstone for that key, in whichever month they sit, in the same write as the deposit row.
- **BREAKING**: deleting a received (收訖) deposit is rejected with `409 Conflict` and changes nothing — its cash credit and `dep-end` item stay consistent. The UI hides 刪除 for received deposits.
- Out of scope: bonds (same orphan gap for `bond-end:`/`coupon:` items), and cleanup of existing orphans (the user removes the one known orphan by hand).

## Capabilities

### New Capabilities

### Modified Capabilities
- `time-deposits`: deleting a deposit now cascades to its `dep-start` month item, and deleting a received deposit is rejected.

## Impact

- `DELETE /api/deposits/{id}` gains a `409` response.
- Backend: `routes/deposits.rs`, `service/deposits.rs`, `store/deposits.rs`, `store/months.rs`.
- Frontend: `RowActions.vue`, `DepositTable.vue`.
- Docs: `docs/deposits.md`.
