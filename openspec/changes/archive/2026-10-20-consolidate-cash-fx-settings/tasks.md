# Tasks

## 1. Baseline capture (before any code change)

- [x] 1.1 Copy `data/wealth.db` to `/tmp/wealth-baseline.db`. Run the current HEAD backend on it (`WEALTH_DB=/tmp/wealth-baseline.db WEALTH_ADDR=127.0.0.1:8797`). Save the JSON of `/api/overview`, `/api/months/summary`, `/api/months` (list), `/api/year-review`, `/api/summary?market=HK`, `/api/summary?market=US`, `/api/summary/yearly?market=HK`, `/api/summary/yearly?market=US`, `/api/aia/summary`, `/api/deposits/summary`, `/api/bonds/summary`, `/api/dividends/summary`, `/api/forecast`, `/api/ibkr`, `/api/months/settings` and the current month `/api/months/<ym>` under `/tmp/wealth-baseline/`. Verify: one non-empty file per endpoint.

## 2. Migrations

- [x] 2.1 Add `backend/migrations/0028_fx_rates.sql` per design D4. Verify: an inline test seeds `aia.usd_hkd_rate = '7.843045'` at 0027 state, migrates, and asserts the `fx_rates` USD row and that the key is gone; a test with no key asserts an empty table.
- [x] 2.2 Add `0029_cash_accounts.sql` per D1/D2 (same ids for cash rows, IBKR broker rows from the meta keys, defaults HS/IBKR USD, `manual_assets` rebuilt without `kind`). Verify with an inline test using fixtures shaped like live data (asset id 1, cash ids 2 `HS` and 3 `渣打`, `ibkr.hkd_cash`/`ibkr.usd_cash`). Assert the ids, labels, currency/role, amounts, the default flags, that the keys are gone, and that `manual_assets` holds only id 1. Also cover a fresh empty DB getting both broker rows.
- [x] 2.3 Add `0030_credited_accounts.sql` per D3. Verify with an inline test: deposits/bonds `credited_asset_id = 3` survives as `credited_account_id = 3`; an HK dividend and a coupon with `credited_amount` get the HS id; a US dividend gets the IBKR USD id; uncredited rows stay NULL.
- [x] 2.4 Add `0031_settings_keys.sql` per D5/D6 (key renames and `pool_rates`). Verify with an inline test: every renamed key moves with its value, `overview.pool_rate.2024..2026` become `pool_rates` rows, and no `overview.%` key remains.
- [x] 2.5 Update `docs/database.md`: new `fx_rates`, `cash_accounts`, `pool_rates`; `manual_assets` without `kind`; credited-account columns; the `app_meta` key list under new names, including the previously undocumented `money_master.*`. Verify: every table in `sqlite_master` of a migrated DB has a section, and no removed key is mentioned.

## 3. Store layer

- [x] 3.1 Add `store::fx` (`usd_rate`, `list`, `set`, `clear`) and switch every `aia::RATE_KEY` reader to it. Verify: `cargo test` passes and `graft grep RATE_KEY` finds nothing.
- [x] 3.2 Add `store::cash_accounts` (CRUD, `set_default` clearing the others in one tx, `default_for(currency)`, `broker(currency)`, `add_amount(tx, id, delta)`; delete nulls `deposits`/`bonds.credited_account_id` and refuses broker rows). Remove `hs_cash_asset_id`, `manual_asset_kind`, and the cash paths in `store::months` asset functions. Verify with inline tests: default uniqueness, broker delete refused, credited ids nulled on delete.
- [x] 3.3 Add `store::settings` per D6. Make `store::meta` reachable only from it, and move every key constant there. Move the `pool_rates` reads and writes into it, or into `store::months::pool_rates` reading the table. Verify: `graft grep "meta::"` outside `store/settings.rs` and `store/meta.rs` returns nothing, and existing settings, seed-mark, note and family-note tests pass.
- [x] 3.4 Move the `ibkr_transfers` SQL to `store::ibkr`. Verify: `cargo test`.

## 4. Services and receipts

- [x] 4.1 Add `service::receipts::{credit, reverse}` per D7. Verify with inline tests covering each scenario of the cash-accounts "Receipt crediting rule" and "Recorded credit and exact reversal" requirements: default, override, mismatch rejected, no default, reversal after the default moved, reversal after the account was deleted.
- [x] 4.2 Move the deposit and bond principal receive logic into `service::{deposits, bonds}` using receipts, with the `bank_in`/`account_id` (and bond `credit_amount`) inputs. Verify: the existing receive tests are adapted and pass, plus a test for the deposit default-account scenario.
- [x] 4.3 Move coupon receipt (`update_coupon` transition) and dividend receipt (`update` transition) into services using receipts with currency HKD / per market. Verify: the existing coupon/dividend receipt tests pass with assertions moved to cash accounts, plus new tests for the "HK receipt into a chosen account" and "US receipt banks into IBKR USD cash" scenarios.
- [x] 4.4 Move IBKR patch handling into `service::overview` (one transaction: transfer row, `ibkr.reported_value`, broker account amounts, `null` → 0). Verify: the `ibkr_block_derives_the_cross_checks` test is updated to seed broker accounts and the fx rate, and a new test covers the "Cash edit is the account balance" scenario.
- [x] 4.5 Move settings GET/PATCH logic (validation and writes) from `routes/months.rs` into `service::settings` on top of `store::settings`. Verify: the round-trip tests for semi-liquid target and goals pass against `/api/settings`.
- [x] 4.6 Update live totals, overview `cash_rows`, and the recapture `start_cash` to read `bank` accounts, IBKR cash from `broker` accounts, and the rate from `store::fx`. Verify: the `calc::overview` and `service::months` tests pass unchanged in their numbers.

## 5. Routes, API and layering cleanup

- [x] 5.1 Add `routes/fx.rs` (`GET /api/fx-rates`, `PATCH /api/fx-rates/{currency}`) and `routes/cash_accounts.rs` (CRUD); remove `PATCH /api/aia/rate`; rename `/api/months/settings` to `/api/settings`; drop `kind` from `/api/manual-assets`; change the receipt request bodies per D8. Update `openapi_documents_every_api_operation`. Verify: `cargo test` and a check at `http://127.0.0.1:<port>/scalar` on a scratch DB.
- [x] 5.2 Delete the duplicates listed in D9 (`routes/yearly::build`, `routes/year_review::build`, both `credit_ibkr_usd`, `routes/mod::{resolve_stock,today,now_timestamp}`). Verify: `graft grep` finds one definition of each surviving symbol, and `cargo test` passes.
- [x] 5.3 Add `routes/patch.rs` merge helpers and replace the per-file `merge`/`merge_text` closures and fns in all 7 route files. Verify: `graft grep "fn merge"` lists only `routes/patch.rs`, and `cargo test` passes.
- [x] 5.4 Split `models.rs` into `models/` per D9, with re-exports. Verify: `cargo build`, and no import path outside `models/` changes.
- [x] 5.5 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`. Verify: all three pass.

## 6. Frontend

- [x] 6.1 Update `frontend/src/api.ts`: add `CashAccount`/`FxRate` types and calls; rename `MonthSettings` calls to `/settings`; remove `updateAiaRate` in favour of `updateFxRate('USD', rate)`; drop `kind` from `ManualAsset`; update the receipt request types (`account_id`, `bank_in`) and `credited_account_id`. Verify: `pnpm exec vue-tsc --noEmit` points only at the views fixed in 6.2–6.5.
- [x] 6.2 AIA view: the rate field saves through the fx endpoint. Overview view: cash rows are edited inline through cash-accounts and asset rows through manual-assets. Verify: the build passes, and on a scratch backend editing the rate and the HS row updates the figures.
- [x] 6.3 MonthStat settings card: list and edit cash accounts (label, amount, default flag) separately from asset rows; salary and pool rate through `/api/settings`. Verify: the build passes, and on a scratch backend creating an account, toggling the default, and editing salary works.
- [x] 6.4 Receipt forms (DepositsView, BondsView principal + coupon, DividendReceiveForm): an account picker filtered by the receipt currency, preselecting the matched or default account, with a 不存入 option sending `bank_in: false`. Verify: the build passes, and on a scratch backend each 收訖 credits the chosen account.
- [x] 6.5 IbkrCard: unchanged fields against `PATCH /api/ibkr`; confirm the response shape still matches. Verify: the build passes, and saving `usd_cash` updates the IBKR USD account.
- [x] 6.6 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`. Verify: both pass.

## 7. Docs

- [x] 7.1 Update `docs/DATA_FLOW.md`, `docs/overview.md`, `docs/stocks.md`, `docs/deposits.md`, `docs/investments.md` wherever they mention `aia.usd_hkd_rate`, `ibkr.*_cash`, manual cash rows, `/api/months/settings`, `/api/aia/rate`, or `credit_asset_id`. Verify: a grep for those strings in `docs/` returns nothing stale.

## 8. Integration checks

- [x] 8.1 Equivalence: copy `data/wealth.db` to `/tmp/wealth-after.db`, run the new backend on it (spare port), capture the same endpoints as 1.1 (settings from `/api/settings`), and diff numeric values against `/tmp/wealth-baseline/`. Verify: the only differences are the documented renames (`cash_rows` element shape, settings path, credited field names, `ibkr` sourced the same). Every number is identical. Report the diff.
- [x] 8.2 Back up `data/wealth.db` to `data/wealth.db.bak-2026-10-20`, restart the real backend per AGENTS.md, and confirm `/api/overview` and `/api/cash-accounts` answer. Verify: the backend log shows migrations 28–31 applied, and the 總覽 totals match the baseline.

## Workflow follow-up

- The user checks every page manually against the live DB.
- Archive the change with `/openspec-archive-change` once accepted.
