# Tasks

## 1. Backend

- [x] 1.1 Remove `unreceive` handlers + `#[utoipa::path]` blocks from `routes/deposits.rs`, `routes/family.rs`, `routes/bonds.rs`; drop the three registrations in `routes/mod.rs` and the three entries in the `openapi_documents_every_api_operation` path list. Verify `cargo test openapi_documents_every_api_operation` passes.
- [x] 1.2 Delete dead store helpers — `credited_of`/`clear_receipt` in `store/deposits.rs`, `clear_receipt` in `store/family.rs`, `credited_of`/`clear_receipt` in `store/bonds.rs` — and any now-unused imports in the route files (`months_store` stays: receive still uses it). Verify `cargo clippy --all-targets -- -D warnings` is clean.
- [x] 1.3 Trim the unreceive assertions in `backend/tests/api.rs`: deposit test (~L807-821), bond test (~L881-894), family test (~L1858-1878); keep the re-receive 409 checks. Verify `cargo test` passes.

## 2. Frontend

- [x] 2.1 Remove `unreceiveDeposit`, `unreceiveFamilyDeposit`, `unreceiveBond` from `frontend/src/api.ts`.
- [x] 2.2 Remove the `取消收訖` `v-else` button and `unreceive` emit from `DepositTable.vue`; remove `unreceive()` and `@unreceive` wiring in `DepositHistoryView.vue`; remove `unreceive()` + button in `FamilyDepositsView.vue` (update the section comment); remove `unreceiveBond()` + both buttons in `BondsView.vue`. Verify `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes and no view references `unreceive`.

## 3. Docs

- [x] 3.1 Remove 取消收訖/`unreceive` flow lines from `docs/deposits.md` (~L51, ~L95) and `docs/investments.md` (~L52); update the `docs/DATA_FLOW.md` summary line (~L39).
- [x] 3.2 Reword `credited_asset_id`/`credited_amount` entries in `docs/database.md` (~L58, ~L185): columns record which cash row the 收訖 bank-in went to — no longer "stored so 取消收訖 reverses exactly". Verify `grep -n "取消收訖\|unreceive" docs/` returns nothing outside archived material.

## 4. Verification

- [x] 4.1 Run `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — all clean.
- [x] 4.2 Restart the backend (`lsof -ti :8787 | xargs kill`, then `nohup cargo run -p wealth-backend --bin wealth-backend > /tmp/wealth-backend.log 2>&1 < /dev/null & disown`); confirm the 定期記錄, 家人定期, and 債券 views render with no 取消收訖 action and 收訖日 still shows for received rows.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
