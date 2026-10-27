# Tasks

## 1. Backend delete cascade

- [x] 1.1 Add `months::delete_auto_items_by_key(conn, auto_key)` in `backend/src/store/months.rs` deleting matching `month_items` and `month_item_dismissals` rows in any month; verify via the service tests in 1.3
- [x] 1.2 Change `store::deposits::delete_deposit` to take `&mut SqliteConnection`; verify `cargo build` passes
- [x] 1.3 Add `service::deposits::delete(pool, id)` (NotFound, Conflict when received, one transaction removing `dep-start:<id>` items/dismissals and the row) with inline tests: cascade + unrelated items kept, moved start date, received → Conflict with row and `dep-end` intact, unknown id → NotFound; verify `cargo test` passes
- [x] 1.4 Route `routes::deposits::remove` through the service and add the 409 response to its `#[utoipa::path]`; verify `cargo test` (openapi pin test) and `cargo clippy --all-targets -- -D warnings` pass
- [x] 1.5 Document the delete cascade and received block in `docs/deposits.md`; verify the doc names the `dep-start` cleanup and the 409

## 2. Frontend

- [x] 2.1 Add optional `removable` prop (default true) to `RowActions.vue` hiding 刪除, and pass `:removable="!deposit.received_at"` from `DepositTable.vue`; verify `pnpm build && pnpm exec vue-tsc --noEmit` pass

## 3. Integration

- [x] 3.1 Run `cargo fmt --check`, restart the backend per AGENTS.md, and confirm the API answers (no deletes against the live DB)

## Workflow follow-up

- Archive the change after review.
