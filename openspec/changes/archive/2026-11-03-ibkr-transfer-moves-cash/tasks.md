# Tasks

## 1. Schema and store

- [x] 1.1 Add `backend/migrations/0034_ibkr_transfer_accounts.sql` adding nullable `from_account_id` and `broker_account_id` (`REFERENCES cash_accounts(id) ON DELETE SET NULL`) to `ibkr_transfers`; document both columns and the migration in `docs/database.md`; verify `cargo test` applies it on a memory DB and existing rows read NULL
- [x] 1.2 In `store/ibkr.rs`, extend `insert_transfer` to take both account ids and return the new id; add `list_transfers` (joined with the source label, newest `transfer_date` then id first), `load_transfer` (`NotFound` for unknown ids), `delete_transfer`, and a month-range loader of cash-moving transfers; verify with inline store tests for ordering, NotFound, and the legacy-row NULLs
- [x] 1.3 Add `cash_accounts::bank_by_label` (role `bank` only) in `store/cash_accounts.rs`; verify an inline test finds `HS` and ignores broker rows

## 2. Transfer service and API

- [x] 2.1 Add `IbkrTransfer` and `NewIbkrTransfer` to `models/overview.rs` and remove `transfer_hkd` / `transfer_date` from `IbkrPatch` (backend model and `IbkrPatch` doc comment); verify `cargo build`
- [x] 2.2 Add `service/ibkr.rs::create_transfer` per the overview spec (source resolution by id or label `HS`, HKD bank check, amount > 0, date ≤ today defaulting to today, overdraft check inside the transaction, debit source, credit IBKR HKD broker, `record_receipt_item` with `ibkr:<id>` / `IBKR 轉入` / `−amount`); verify inline tests: HS debit + broker credit + item recorded, explicit 渣打 source, no-HS 400, broker source 400, unknown id 404, amount ≤ 0 / NaN 400, bad and future date 400, insufficient balance 400 with nothing changed, missing month row writes no item
- [x] 2.3 Add `service/ibkr.rs::delete_transfer` (reverse recorded sides, skip deleted/NULL accounts, `delete_auto_items_by_key("ibkr:<id>")`, delete row); verify inline tests: exact reversal with item gone, legacy row moves nothing, deleted source reverses only the broker side, unknown id 404
- [x] 2.4 Remove the transfer branch (and the `transfer_hdk` typo) from `service::overview::update_ibkr`; replace `update_ibkr_appends_a_dated_transfer` in `routes/overview.rs` with a test that a PATCH appends no transfer row; verify `cargo test`
- [x] 2.5 Add `list_ibkr_transfers`, `create_ibkr_transfer` (201), and `delete_ibkr_transfer` (204) handlers with `#[utoipa::path]` in `routes/overview.rs`, register them via `utoipa_axum::routes!` in `routes/mod.rs`, and add `/api/ibkr/transfers` get+post and `/api/ibkr/transfers/{id}` delete to `openapi_documents_every_api_operation`; verify that test and `cargo test` pass

## 3. 月結 candidate fallback

- [x] 3.1 Add `SuggestionIbkrTransfer` and `SuggestionEvents.ibkr_transfers` in `calc/suggestions.rs`; emit `ibkr:<id>` `adjustment` candidates labelled `IBKR 轉入` of `−amount` in `build_suggestions`; load cash-moving transfers for the month in `service::months::load_events`; verify inline calc tests for emission and stored/dismissed filtering, and a service test that a transfer recorded before its month row exists is suggested once the row is created while a legacy row is not
- [x] 3.2 Update the 月結 section of `docs/overview.md` / `docs/DATA_FLOW.md` with the `ibkr:<id>` auto item; verify the doc names the key, label, and fallback

## 4. 預測 股票 row

- [x] 4.1 Add `ForecastInput.ibkr_transfers`, `ForecastMonthDerived.transfer_flows`, and its term in the `cash` chain (not `locked`) in `calc/forecast.rs`; verify an inline test that a month-1 transfer lowers `cash` in months 1+ by the amount, leaves `locked` unchanged, and keeps plan items
- [x] 4.2 Add `ForecastIbkrTransfer` to `models/forecast.rs` and `ForecastMonth.ibkr_lines` (date then id order) in `routes/forecast.rs`, loading the window's transfers; verify a route test that an in-window transfer appears with `amount` `−amount_hkd` and a pre-window one appears nowhere
- [x] 4.3 Document `ibkr_lines` in the 預測 section of `docs/overview.md`; verify the cash formula there includes it

## 5. Frontend

- [x] 5.1 In `frontend/src/api.ts` add `IbkrTransfer`, `NewIbkrTransfer`, `ForecastIbkrTransfer`, `ForecastMonth.ibkr_lines`, and `listIbkrTransfers` / `createIbkrTransfer` / `deleteIbkrTransfer`; drop `transfer_hkd` / `transfer_date` from `IbkrPatch`; verify `pnpm exec vue-tsc --noEmit`
- [x] 5.2 In `IbkrCard.vue`, remove the 轉入 inputs and preview from 編輯; add a 「轉入」 form (amount, date default today with `max` today, HKD bank account `<select>` preselecting label `HS`, before→after preview of the source and IBKR HKD), with no transfer history on the card; after create refetch `api.ibkr()` and emit `saved`; verify `pnpm build` and `vue-tsc`
- [x] 5.3 In `OverviewView.vue`, include `ibkr_lines` in the 股票 cell sum and list them read-only as `IBKR 轉入` `已轉帳` in the selected-cell editor; verify `pnpm build` and `vue-tsc`
- [x] 5.4 Update the IBKR card wiring in `docs/overview.md` / `docs/stocks.md` (UI action → `POST`/`DELETE /api/ibkr/transfers` → tables touched); verify the docs no longer mention 轉入 via `PATCH /api/ibkr`

## 5b. 轉入記錄 tab

- [x] 5b.1 Add `frontend/src/views/IbkrTransfersView.vue` listing `api.listIbkrTransfers()` (date, amount, source label or `—`, 累計 total, empty placeholder) with a confirmed 刪除 per row calling `api.deleteIbkrTransfer` then reloading; verify `pnpm build` and `vue-tsc`
- [x] 5b.2 In `App.vue`, add a `transfers` tab labelled 轉入記錄 to the 股票 group shown only while `market === 'US'`, render `IbkrTransfersView` for it, and switch to 股票 → 總覽 when 港股 is selected on it; verify `pnpm build`, `vue-tsc`, and in the browser that the tab appears/disappears with the market toggle
- [x] 5b.3 Document the tab in `docs/stocks.md` / `docs/overview.md` (replace the "API-only" wording) and verify no doc still says listing/deleting is API-only

## 6. Integration

- [x] 6.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; all pass
- [x] 6.2 Exercise the flow against a scratch DB (`WEALTH_DB=/tmp/wealth-test.db`, spare `WEALTH_ADDR`): create a transfer, confirm HS/IBKR HKD balances, the 月結 item, and the 預測 `ibkr_lines`; delete it and confirm everything reverts
- [x] 6.3 Restart the real backend per AGENTS.md and confirm `GET /api/ibkr/transfers` lists the two legacy rows and `GET /api/forecast` answers

## Workflow follow-up

- Archive the change with `openspec-archive-change` after the user verifies a real 轉入 in the UI.
