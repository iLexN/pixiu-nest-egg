# Tasks

## 1. Backend schema and models

- [x] 1.1 Add `backend/migrations/0024_family_deposits.sql` creating `family_deposits` (holder NOT NULL, label, bank, principal, interest, start_date, end_date NOT NULL, received_at, note, sort_order, created_at, updated_at) with an index on `(holder, end_date)`; verify `cargo test` boots a fresh DB with the migration applied.
- [x] 1.2 Add `FamilyDeposit` (derived `total`, `status`, `end_year`, `end_month`), `NewFamilyDeposit`, and `FamilyDepositPatch` (nullable-patch pattern as `DepositPatch`) to `backend/src/models.rs`; add `FAMILY_DEPOSIT_COLUMNS` and a row mapper in `routes/mod.rs` next to `DEPOSIT_COLUMNS`; verify `cargo clippy --all-targets -- -D warnings` passes.

## 2. Backend routes

- [x] 2.1 Create `backend/src/routes/family.rs` with `list` (status/holder/year/order filters), `create`, `update`, `remove`, reusing `calc::validate_deposit` (rate `None`) plus a trimmed-non-empty holder check; mount under `/api/family/deposits` in `api_router`; verify with API tests covering create → list filtered by holder, blank holder → 400 naming `holder`, `2026/13/45` end date → 400, negative principal → 400, edit note/interest, delete.
- [x] 2.2 Add `receive` (`{received_at?, interest?}`, 409 if already received) and `unreceive` (409 if not received) that only touch `family_deposits` — no `manual_assets`, `month_items`, or `record_receipt_item`; verify with API tests: receive sets `received_at` + corrected interest and status `END`, second receive → 409, unreceive restores `ACTIVE`, unreceive again → 409.
- [x] 2.3 Add `PUT /api/family/holders/:holder/note` storing `family.note.<holder>` via `mpf::meta_put` (`null`/empty clears) and `GET /api/family/deposits/summary` returning per-holder `{holder, note, upcoming, active_principal}` (holders = distinct deposit holders ∪ note keys) plus `history_years`, using `calc::active_totals`; verify with API tests: note round-trips through summary, holder with note but no deposits still listed, clearing removes it, active principal counts only future end dates.
- [x] 2.4 Add the isolation test in `backend/tests/api.rs`: snapshot `GET /api/deposits/summary` active principal, `GET /api/overview` 總數/流動資產, and `GET /api/months/:ym` `interest_auto` for the current month; create a family deposit (300000, interest 920.29, ending this month) and receive it; assert all three snapshots are byte-identical. Verify `cargo test` passes and `rg family_deposits backend/src` matches only `routes/family.rs` and `routes/mod.rs` column constant.

## 3. Frontend

- [x] 3.1 Add `FamilyDeposit`, `FamilyHolderSummary`, `FamilyDepositSummary` types and `listFamilyDeposits`, `createFamilyDeposit`, `updateFamilyDeposit`, `deleteFamilyDeposit`, `receiveFamilyDeposit`, `unreceiveFamilyDeposit`, `familyDepositSummary`, `updateFamilyHolderNote` (URL-encoding the holder) to `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 3.2 Create `frontend/src/views/FamilyDepositsView.vue`: holder chips filter; per-holder section with 活躍本金, click-to-edit note (textarea, 儲存/取消), upcoming table (label, bank, principal, interest, total, start/end, note, 已到期未收 flag) with 收訖 dialog (收訖日 + 利息 only) and 取消收訖, inline edit/delete via `RowActions`; 新增 form modelled on `DepositForm.vue` with holder `<datalist>`, no rate, note textarea, `DateInput` for both dates (start defaults to today); year-filtered history table. Verify `pnpm build` passes and, in the browser, entering Dad's SC-9024 and receiving it moves it from upcoming to history with interest 920.29.
- [x] 3.3 Add the 家人 group (after 年結) with a 定期 sub-tab rendering `FamilyDepositsView` in `frontend/src/App.vue`, market toggle hidden; verify in the browser that the group appears last, opens on 定期, and the 港股/美股 toggle is not shown.

## 4. Docs and integration

- [x] 4.1 Update `AGENTS.md` (data-flow section for 家人 定期 + holder note; roadmap "Completed" list; remove `Mum`, `Dad` from the unmigrated list) and `docs/DATA_FLOW.md` with the new page, endpoints, table, and `app_meta` keys; verify the documented endpoints match `api_router`.
- [x] 4.2 Run the full gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; then manually confirm Overview 總數/流動資產 and 定期 → 總覽 figures are unchanged after adding a family deposit in the UI.
