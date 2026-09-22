# Tasks

## 1. Migration

- [x] 1.1 Create `backend/migrations/0017_itemize_interest.sql`: rebuild `month_items` with `'interest'` added to the category CHECK (0016 rebuild pattern), copy existing rows, insert one `interest` item per `month_stats` row where `interest <> 0` and `interest − auto ≠ 0` (label `其他利息`, note `匯入差額`; `auto` = deposits.interest by `end_date` in month + received `bond_coupons.received_amount` + received HK `dividends.received_amount` by `pay_date` in month), drop/rename, recreate `idx_month_items_auto`, and `ALTER TABLE month_stats DROP COLUMN interest`. Verify: run the backend once, then `sqlite3 data/wealth.db "SELECT month, amount FROM month_items WHERE category='interest'"` shows 2026-08 → 2698 and 2026-10 has no row.

## 2. Backend — model & calc

- [x] 2.1 `models.rs`: add `MonthItemCategory::Interest` (`as_str`/`parse` → `'interest'`); remove `interest` from `MonthStatPatch`; add `InterestComponent { source, label, amount }`. Verify: compiles; `MonthItemCategory::parse("interest")` returns `Some`.
- [x] 2.2 `calc.rs`: add `interest` to `MonthItemSums` and sum `Interest` items in `month_item_sums`; replace `suggested_interest` with `interest_components(month, &SuggestionEvents) -> Vec<InterestComponent>` + an `auto_interest` sum helper (same predicates: deposit `interest` by `end_date`, received coupons and received HK dividends by `pay_date`). Update/extend the existing `suggested_interest_*` test to assert the component list. Verify: `cargo test calc` passes.
- [x] 2.3 `routes/months.rs`: add `load_all_events` (unbounded deposits/coupons/HK-dividends queries); make `to_stat_rows` take events + items and set `interest = auto_interest + sums.interest`; update `list`, `summary`, `present_month`, `show` accordingly; in `MonthDetailResponse` replace `suggested_interest` with `interest_auto: Vec<InterestComponent>`; drop `interest` from `upsert`/`validate_figures`/`StoredMonth`/`MONTH_COLUMNS`. Verify: `cargo test` month routes pass; `GET /api/months/2026-08` returns `interest_auto` with 4 entries totalling 16603.37 and `month.interest` 19301.37.
- [x] 2.4 `import.rs`: `import_months` stops inserting `interest` into `month_stats`; after inserting a row, insert an `interest` item with `residual = sheet N − auto` when `≠ 0` (load events once per import run). Verify: fresh-import into a temp DB reproduces the same derived totals as the migrated DB.
- [x] 2.5 `parity.rs`: `check_months` drops `interest` from the stored-column query and compares derived interest (auto + items) vs sheet N. Verify: `cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"` — month 利息 rows match except blank-cell months (2026-10 derived 1278 vs sheet 0) which report as informational/expected diff.

## 3. Frontend

- [x] 3.1 `api.ts`: `MonthItemCategory` += `'interest'`; remove `MonthStatPatch.interest`; replace `suggested_interest` with `interest_auto: { source, label, amount }[]` on `MonthDetailResponse`.
- [x] 3.2 `MonthStatView.vue`: replace the 利息 input with a read-only derived total + breakdown list (auto lines labeled 定期/派息/券息 with label+amount, plus `interest` items rendered with the existing item edit/delete actions); remove `useSuggestedInterest`, the 建議/使用 hint, and `interest` from `MonthDraft`/`startEdit`/`saveMonth`; add `利息` to the 新增項目 類別 options. Verify: `pnpm exec vue-tsc --noEmit` passes; 2026-08 shows the four auto lines + 其他利息 2698.

## 4. Docs & verification

- [x] 4.1 Update `AGENTS.md` and `docs/DATA_FLOW.md` Month Stat sections: 利息 = auto events + `interest` items, derived on read; breakdown in the detail response. Verify: wording matches implemented behavior.
- [x] 4.2 Full verification: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, and `check_parity` per 2.5.

## 5. Deposit 收訖 lifecycle

- [x] 5.1 `backend/migrations/0018_deposit_received.sql`: `ALTER TABLE deposits` add `received_at TEXT`, `credited_asset_id INTEGER`, `credited_amount REAL`; backfill `received_at = end_date` for `end_date <= today`. Verify: ended deposits read received; Oct/Nov/2027 rows stay NULL.
- [x] 5.2 `models.rs`/`calc.rs`/`routes`: `Deposit.received_at`, `DepositStatus` END once received; `SuggestionDeposit.received`; `InterestComponent.received`; `interest_components` emits unreceived in-month deposits as pending components while `auto_interest` sums received only; `DEPOSIT_COLUMNS`/`row_to_deposit`/`load_deposit_events` carry `received_at`; `list?status=` and `summary.upcoming` filter on receipt; new deposits/imported deposits with past `end_date` record `received_at = end_date`. Verify: `cargo test` passes; `GET /api/months/2026-10` → `interest` 0, both deposits pending.
- [x] 5.3 `routes/deposits.rs`: `POST /:id/receive` (received_at, optional interest correction, optional cash credit to a `kind='cash'` manual asset, auto `dep-end` item with auto_key dedup + dismissal cleanup, month-row guard) and `POST /:id/unreceive` (reverses all three; 409 on wrong state). Verify: new api test — receive credits cash + creates item + interest counts; unreceive reverses.
- [x] 5.4 `import.rs`: residual subtracts all in-month components (unreceived included) so a pre-typed sheet cell isn't double-counted; imported past-dated deposits record `received_at`.
- [x] 5.5 Frontend: `api.ts` (`Deposit.received_at`, `InterestComponent.received`, `receiveDeposit`/`unreceiveDeposit`, `ReceiveDepositBody`); `DepositTable` 收訖/取消收訖 actions + 已到期未收 marker + 收訖日 in history; `DepositsView` receive form (收訖日 / 實收利息 / 存入活期 select defaulting SC→渣打, HS→HS / editable amount); `DepositHistoryView` 取消收訖; `MonthStatView` pending lines muted with 未收; hint card updated. Verify: `pnpm build` + `vue-tsc` clean.
- [x] 5.6 Docs + migration on the live DB (backup first); `check_parity`: Oct's interest/interest_sum/interest_avg/pool_balance INFO diffs become exact matches.

## 6. Bond principal 收訖 lifecycle

- [x] 6.1 `backend/migrations/0019_bond_received.sql`: `ALTER TABLE bonds` add `received_at`/`credited_asset_id`/`credited_amount`; backfill `received_at = maturity_date` for matured bonds.
- [x] 6.2 `Bond.received_at` + `BOND_SELECT`/`row_to_bond`; `insert_bond` records `received_at = maturity_date` for past-dated bonds (covers create + import); active/matured split and `債券!B1` stay date-based.
- [x] 6.3 `POST /api/bonds/:id/receive` + `/unreceive` — same shape as the deposit endpoints; credit defaults to `principal` (interest already comes via coupon receipts); auto item `bond-end:<id>`. Verify: new api test.
- [x] 6.4 Frontend: `Bond.received_at`, `receiveBond`/`unreceiveBond`; BondsView 已到期 section flags 本金未收 + 收訖 form (收訖日 / 存入活期 / 存入金額) and 收訖日 + 取消收訖 for received bonds.
- [x] 6.5 Docs (AGENTS.md, DATA_FLOW.md) + spec delta; migration applied to the live DB (backup `data/wealth.db.bak-0019`).

## 7. Pending coupon/dividend preview in the breakdown

- [x] 7.1 `SuggestionReceipt`: `received_amount: f64` → `amount: Option<f64>` + `received: bool`; `build_suggestions` gates `div:`/`coupon:` on `received` so pending events never become items; `interest_components` emits all in-month coupons/dividends (pending preview with expected/estimated amount, None while 待定); `InterestComponent.amount` → `Option<f64>`; `auto_interest` unchanged (received only).
- [x] 7.2 `load_coupon_events`/`load_dividend_events` drop the received-only filter and carry `per_10k`/`principal`/`estimated_amount`; import residual unwraps `amount` (None → 0). Verify: `GET /api/months/2026-10` shows the 2026-10-23 coupon pending with `amount: null`; Sept lists the 3 pending dividends.
- [x] 7.3 Frontend `InterestComponent.amount: number | null`, pending rows render `—`.

## 8. Auto bank-in + month item on coupon/dividend 收訖

- [x] 8.1 `backend/migrations/0020_receipt_bank_in.sql`: `credited_amount` on `bond_coupons` + `dividends`.
- [x] 8.2 Shared `record_receipt_item` + `hs_cash_asset_id` helpers in `routes/mod.rs`; deposits.rs/bonds.rs refactored onto them (same dedup + month guard + dismissal cleanup).
- [x] 8.3 `update_coupon`: on `received_amount` NULL→set — credit HS cash (skip `bank_in: false`), store `credited_amount`, insert `coupon:<id>` adjustment item in the pay month; set→NULL reverses both. `BondCouponPatch.bank_in`.
- [x] 8.4 `dividends.rs update`: same transition — HK → HS + `div:<id>` item; US → `ibkr.usd_cash` app_meta (no month item, never touches 活期); undo reverses. `DividendPatch.bank_in`.
- [x] 8.5 Frontend: `bank_in` checkbox on the coupon 收訖 form (存入活期 HS) and DividendReceiveForm (存入活期 HS / 存入 IBKR USD cash by market).
- [x] 8.6 Test: `coupon_and_dividend_receive_banks_in_and_records_the_item` covers coupon→HS+item+undo, HK div→HS+item, US div→IBKR USD+no item+undo. Spec deltas (bonds + stock-dividends MODIFIED) + docs.
