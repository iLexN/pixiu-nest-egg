# Design

## Context

`service::deposits::create` records `dep-start:<id>` via `months::record_receipt_item`; `service::deposits::receive` records `dep-end:<id>` and a cash credit. `routes::deposits::remove` calls `store::deposits::delete_deposit` directly on the pool, with no service layer and no transaction. Bond deletion (`routes::bonds::remove`) already deletes a parent plus children in one transaction, which is the pattern followed here.

## Goals / Non-Goals

**Goals:**
- Deposit delete leaves no stored trace in `month_items` / `month_item_dismissals`.
- Received deposits stay consistent by refusing deletion.

**Non-Goals:**
- Reversing cash credits or adding an "unreceive" flow.
- Bonds, dividends, coupons.
- A migration to clean existing orphans.

## Decisions

- **New `service::deposits::delete(pool, id)`** composes the deposit store and the months store, keeping SQL in `store/` and the route thin. It loads the deposit (NotFound → 404), returns `ServiceError::Conflict` when `received_at` is set, then in one transaction deletes the auto items and the deposit row.
- **Match by `auto_key` in any month** (`months::delete_auto_items_by_key(conn, auto_key)`), not by the current start month: editing `start_date` does not move the stored item, so a month-scoped delete would miss it. Alternative (delete by current start month via the existing `delete_auto_item`) rejected for that reason.
- **Dismissals cleared too**, so the tombstone table holds no keys for non-existent deposits. AUTOINCREMENT ids mean a stale key could never collide with a new deposit, but leaving it is pointless clutter.
- **`store::deposits::delete_deposit` takes a `&mut SqliteConnection`**, like `bonds::delete_with_coupons`, so it can run inside the service transaction.
- **UI**: `RowActions` gets an optional `removable` prop (default `true`); `DepositTable` passes `!deposit.received_at`. The backend 409 remains the authority; existing view handlers already surface `ApiError` messages.

## Risks / Trade-offs

- [A wrongly 收訖'd deposit can no longer be deleted] → accepted by the user; a future unreceive feature would address it.
- [Bonds keep the same orphan gap] → noted out of scope.
