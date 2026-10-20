# Proposal

## Why

The backend was migrated from the workbook one sheet at a time, so state that several sections share ended up under whichever section came first:

- The global USD→HKD rate is `aia.usd_hkd_rate`.
- The IBKR cash balances are TEXT values in `app_meta`.
- Cash sits in three places, and the coupon and dividend receipt flows find their bank-in target by the hard-coded row label `HS`.
- The four 收訖 flows (deposit, bond principal, coupon, dividend) each re-implement the same credit / reverse / month-item logic.
- A half-finished move from routes to services left duplicate builders and transaction logic inside route handlers.

The migration is complete, so this is the moment to give these shared concepts a real home while every user-visible function stays the same.

## What Changes

- **New `fx_rates` table.** The current rate per foreign currency is stored as `rate_to_hkd`. It replaces `aia.usd_hkd_rate`, and every rate consumer (AIA, IBKR, live totals, suggestions, year review) reads from it.
- **New `cash_accounts` table.** Each account carries `currency` (`HKD`/`USD`), `role` (`bank` = 活期, `broker` = IBKR cash), `amount`, and `is_default_receipt` (at most one default per currency). It absorbs the `manual_assets` cash rows (HS, 渣打) and `ibkr.hkd_cash`/`ibkr.usd_cash`. `manual_assets` keeps only `asset` rows.
- **One receipt rule.** Each receipt credits the request's account, or else the default account for the receipt's currency, unless bank-in is turned off. All four receipt kinds record `credited_account_id` + `credited_amount`, so a reversal returns money to the account that was actually credited. This fixes reversals following a renamed or changed `HS` row.
- **Typed settings.** Scalar settings stay in `app_meta`, but under domain names: `ledger.salary`, `strategy.semi_liquid_target`, `goals.*`, `ibkr.reported_value`. A single typed settings module is the only reader/writer. The per-year 開心Pool rate moves to a `pool_rates` table.
- **Settings endpoint renamed.** **BREAKING (API):** `GET/PATCH /api/months/settings` becomes `GET/PATCH /api/settings`, with the same fields.
- **FX endpoints.** **BREAKING (API):** `PATCH /api/aia/rate` is removed in favour of `GET /api/fx-rates` and `PATCH /api/fx-rates/{currency}`. `GET /api/aia/summary` and `GET /api/overview` still report the USD rate.
- **Cash-account endpoints.** **BREAKING (API):** new `GET/POST /api/cash-accounts` and `PATCH/DELETE /api/cash-accounts/{id}`. `/api/manual-assets` serves `asset` rows only (no `kind`).
- **Receipt request shape.** **BREAKING (API):** deposit and bond receive take `bank_in` (default true) plus an optional `account_id` instead of `credit_asset_id`. Coupon and dividend receipt patches accept an optional `account_id` beside `bank_in`.
- **IBKR endpoint.** `PATCH /api/ibkr` keeps its fields. `hkd_cash`/`usd_cash` now write the IBKR broker accounts in the same transaction as the transfer and `now_value`.
- **Layering cleanup.**
  - Remove the duplicate builders and helpers in `routes/` (`yearly::build`, `year_review::build`, `credit_ibkr_usd`, `resolve_stock`, `today`, `now_timestamp`).
  - Move receipt, settings and IBKR transaction logic into services.
  - Share one patch-merge helper.
  - Split `models.rs` by domain.
- **Frontend.** Updated to the new endpoints. Every page keeps the same functions:
  - the AIA rate field
  - 總覽 inline cash/asset edits
  - the IBKR card
  - the receipt forms, where the account picker preselects the currency default
  - the 月結 settings card

## Capabilities

### New Capabilities
- `fx-rates`: stored exchange rates to HKD, their API, and the rule that rate-dependent figures are absent while a needed rate is unset.
- `cash-accounts`: currency-aware cash accounts (bank 活期 and broker cash), default receipt accounts per currency, and the shared receipt crediting/reversal rule.

### Modified Capabilities
- `aia-policies`: the USD→HKD rate requirement moves to `fx-rates`; the summary and page read and edit the rate there.
- `overview`:
  - The aggregation's rate source, `cash_rows` from bank accounts, and IBKR cash from broker accounts.
  - `ibkr.now_value` is renamed `ibkr.reported_value`.
  - Cash rows are edited inline via cash accounts.
  - Goal, bill, and semi-liquid settings move to `/api/settings` under renamed keys.
  - The liquidity tiers' rate reference.
- `month-stat`:
  - Manual balances split into cash accounts and asset rows.
  - Salary moves to `/api/settings` (`ledger.salary`).
  - Live totals read bank/broker accounts and the fx rate.
  - The pool rate moves to `pool_rates`.
  - Money Master settings move to `/api/settings`.
- `stock-dividends`: receipt credits via the cash-account receipt rule (HKD default for HK dividends, USD default for US dividends) with an optional account override.
- `bonds`: coupon and principal receipts credit via the cash-account receipt rule.
- `time-deposits`: deposit 收訖 credits via the cash-account receipt rule.
- `year-in-review`: the blended rates read the USD rate from `fx-rates`.

## Impact

- **Database:** new migrations add `fx_rates`, `cash_accounts`, and `pool_rates`. They rebuild `manual_assets` without `kind`, add or rename credited-account columns on `deposits`, `bonds`, `bond_coupons`, and `dividends`, and rename/move `app_meta` keys. Existing data in `data/wealth.db` is carried over by the migrations.
- **Backend:**
  - `store/` gets new `fx`, `cash_accounts`, `settings`, and `ibkr` modules.
  - `service/` gets a new `receipts` module, and settings and IBKR logic moves into services.
  - Most of `routes/` is touched, along with the `models` split and the OpenAPI operation list test.
- **Frontend:** `src/api.ts`, plus the Overview, AIA, Deposits, Bonds, Dividends (receive form), MonthStat views and IbkrCard.
- **Docs:** `docs/database.md`, `docs/DATA_FLOW.md`, and the topic docs.
- **Out of scope** (later changes):
  - Theme 4: `GET /api/summary` still records `market_history`.
  - Theme 5: merging `year_snapshots`/`year_review`, cascades on old tables, and timestamp normalisation.
