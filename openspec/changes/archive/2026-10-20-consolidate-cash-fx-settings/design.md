# Design

## Context

See proposal.md (Why). The current state that shapes the approach:

- **`app_meta` is free-text key/value.** Callers read and write it directly through `store::meta::{meta_get, meta_put, meta_f64}`, with key constants spread over `store/{aia,overview,months,mpf,market_history,family}.rs`.
- **Cash lives in two stores.**
  - `manual_assets` rows with `kind='cash'`. Live: id 2 `HS`, id 3 `渣打`. Id 1 `HS 人壽` is an `asset`.
  - `app_meta` `ibkr.hkd_cash` / `ibkr.usd_cash`.
- **Receipt flows live in route handlers.**
  - `routes/deposits.rs::receive`, `routes/bonds.rs::{receive, update_coupon}`, `routes/dividends.rs::update`.
  - Coupons and HK dividends find their target through `store::months::hs_cash_asset_id` (`label = 'HS'`).
  - Live data has 2 deposits with `credited_asset_id = 3`, 3 HK dividends with `credited_amount` set, and no credited coupons or US dividends.
- **The layering rules in AGENTS.md** (routes thin, no SQL outside `store/`, `calc/` pure) already hold for most modules. The duplicates are leftovers.
- **sqlx migrations are checksummed.** The live DB has applied 0001–0027, so new work must come as 0028+.

## Goals / Non-Goals

**Goals:**
- Every derived number the API returns today stays bit-for-bit identical after migration. Only field and endpoint shapes listed in the proposal change.
- After this change, `app_meta` is touched only by `store::settings`.
- One code path for receipt credit/reverse.

**Non-Goals:**
- Moving `market_history` recording out of GET.
- Merging `year_snapshots` with `year_review`.
- Normalising timestamps or adding cascades on old tables.
- Converting `month_items.auto_key` strings to foreign keys.
- Multi-currency bank (活期) accounts. `bank` accounts are HKD-only by constraint.

## Decisions

### D1. `cash_accounts` keeps the manual_assets ids of the cash rows

Migration inserts each `kind='cash'` row with **the same id**. The IBKR broker rows then take fresh autoincrement ids.

`deposits.credited_asset_id` / `bonds.credited_asset_id` therefore become `credited_account_id` with values unchanged. There is no remap table and no chance of mis-pointing historical credits.

*Alternative:* fresh ids plus a mapping CTE. Rejected: it adds risk for no benefit.

Schema:

```sql
CREATE TABLE cash_accounts (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  label              TEXT    NOT NULL UNIQUE,
  currency           TEXT    NOT NULL CHECK (currency IN ('HKD','USD')),
  role               TEXT    NOT NULL CHECK (role IN ('bank','broker')),
  amount             REAL    NOT NULL DEFAULT 0,
  is_default_receipt INTEGER NOT NULL DEFAULT 0 CHECK (is_default_receipt IN (0,1)),
  sort_order         INTEGER NOT NULL,
  updated_at         TEXT    NOT NULL,
  CHECK (role = 'broker' OR currency = 'HKD')
);
CREATE UNIQUE INDEX idx_cash_accounts_default ON cash_accounts (currency) WHERE is_default_receipt = 1;
CREATE UNIQUE INDEX idx_cash_accounts_broker  ON cash_accounts (currency) WHERE role = 'broker';
```

Seeding:
- Cash rows are copied as `bank` / `HKD`, keeping `sort_order`. The row labelled `HS` becomes the HKD default, matching today's implicit behaviour.
- `IBKR HKD` and `IBKR USD` are inserted as `broker` with `CAST(value AS REAL)` from the meta keys (0 when absent). `IBKR USD` becomes the USD default.
- Fresh databases also get the two broker rows, so the IBKR block always has its accounts.

*No `amount >= 0` CHECK:* user edits are validated non-negative in the API layer, as they are today. A receipt reversal can currently drive a balance negative (`add_manual_asset_amount` does not check), and a DB CHECK would turn that into a new failure mode. So the constraint lives in API validation only.

### D2. `manual_assets` is rebuilt without `kind`

SQLite cannot drop a CHECK'ed column in place, so the migration:
1. creates `manual_assets_new (id, label, liquidity, amount, sort_order, updated_at)`
2. copies the `kind='asset'` rows with their ids
3. drops the old table and renames the new one

### D3. Credited-account columns

- `deposits` and `bonds`: rename `credited_asset_id` to `credited_account_id` (`ALTER TABLE … RENAME COLUMN`). A FK can't be added by ALTER, and rebuilding `deposits`/`bonds` to add `REFERENCES cash_accounts(id) ON DELETE SET NULL` is a larger rebuild than this round's hygiene budget.
- `bond_coupons` and `dividends`: `ADD COLUMN credited_account_id INTEGER REFERENCES cash_accounts(id) ON DELETE SET NULL`. SQLite allows a REFERENCES clause on ADD COLUMN when the default is NULL.
- For deposits/bonds, the store's delete of a cash account nulls their `credited_account_id` in the same transaction. That gives the same observable behaviour as `ON DELETE SET NULL`.
- Backfill: dividends with `credited_amount IS NOT NULL` get the HS id for HK and the IBKR USD id for US. Coupons likewise get the HS id. These are exactly the accounts today's reversal code would hit.

### D4. `fx_rates`

`fx_rates(currency TEXT PRIMARY KEY CHECK (currency = 'USD'), rate_to_hkd REAL NOT NULL CHECK (rate_to_hkd > 0), updated_at TEXT NOT NULL)`.

It is seeded from `aia.usd_hkd_rate` when that parses as a positive number, and the key is then deleted. `store::fx::usd_rate(pool) -> Option<f64>` replaces every `meta_f64(.., aia::RATE_KEY)` call:
- `service/months.rs` live totals and suggestion events
- `service/overview.rs` IBKR block
- `routes/aia.rs` summary
- the year-review path, which receives it through the same inputs

*Alternative:* keep a generic settings key `fx.usd_hkd`. Rejected: the user chose a table, and the CHECK constraints are the point.

### D5. `pool_rates` table instead of a `year_review` column

The plan's draft put the pool rate on `year_review`. Inserting a `year_review` row just to hold a rate would create year records that have no other content and that `year_review_rows` iterates over. A dedicated `pool_rates(year INTEGER PRIMARY KEY, rate REAL NOT NULL CHECK (rate >= 0 AND rate < 1))` keeps the fallback semantics (latest earlier year) without touching year-review behaviour. It is seeded from `overview.pool_rate.<year>` keys, which are then deleted.

### D6. Typed settings over `app_meta`

`store::settings` owns every key constant and exposes:

- `Settings::load(pool) -> Settings`: a typed struct with the stored values (`Option<f64>` / `Option<String>`) for salary, bill amount, semi-liquid target, goals ×5, money master ×6, `ibkr.reported_value`, and the seed marks.
- `patch(tx, &SettingsPatch)`: applies `Option<Option<T>>` fields, where `None` means untouched and `Some(None)` means delete.
- Note helpers `note(pool, NoteKey)` and `set_note(..)` for `aia.note`, `mpf.note`, `family.note.<holder>`, plus `family_note_holders` (the existing `keys_like`).
- Seed-mark readers for MPF and per-market marks.

Effective defaults (bill 2158, target 0.25, goal constants) stay in `service`/`calc` where they are applied today. Settings returns stored values only.

Key renames, done in migration 0031 with `UPDATE app_meta SET key = …`:

| old | new |
|---|---|
| `overview.salary` | `ledger.salary` |
| `overview.semi_liquid_target` | `strategy.semi_liquid_target` |
| `overview.goal1_amount` / `goal2_amount` / `goal3_inflation` / `goal3_years` / `final_replacement` | `goals.goal1_amount` / … / `goals.final_replacement` |
| `ibkr.now_value` | `ibkr.reported_value` |

The following keys are unchanged:
- `forecast.bill_amount`
- `money_master.*`
- `mpf.note`, `mpf.seed_max_*`
- `market.<M>.seed_max_*`
- `aia.note`
- `family.note.<holder>`

`store::meta` becomes private to `store::settings`, enforced by `pub(super)` visibility.

### D7. `service::receipts`

```rust
pub enum Currency { Hkd, Usd }
pub struct Credit { pub account_id: i64, pub amount: f64 }

/// Resolve and apply the credit inside `tx`; returns what was credited.
pub async fn credit(tx, currency: Currency, amount: f64,
                    bank_in: bool, account_id: Option<i64>, now: &str)
    -> Result<Option<Credit>, ReceiptError>;   // ReceiptError::{Store, CurrencyMismatch, AccountNotFound}

/// Subtract a previously recorded credit; no-op when None or account gone.
pub async fn reverse(tx, credit: Option<Credit>, now: &str) -> Result<(), StoreError>;
```

`month_items` writes stay with the callers through the existing `store::months::{record_receipt_item, delete_auto_item}`, because whether an item is written differs per source (US dividends never). Each source's store gets `set_credit(tx, id, Option<Credit>)` / `credit(tx, id) -> Option<Credit>`. Routes map `CurrencyMismatch` to a 400 field error on `account_id` and `AccountNotFound` to 404.

The deposit, bond, coupon and dividend receipt logic moves into `service::{deposits, bonds, dividends}` functions that own the transaction. Routes only validate and call them.

### D8. API shapes

| Endpoint | Change |
|---|---|
| `GET /api/fx-rates`, `PATCH /api/fx-rates/{currency}` | new |
| `PATCH /api/aia/rate` | removed. `AiaSummary.rate` stays (now sourced from fx) |
| `GET/POST /api/cash-accounts`, `PATCH/DELETE /api/cash-accounts/{id}` | new; `CashAccount { id, label, currency, role, amount, is_default_receipt, sort_order, updated_at }` |
| `/api/manual-assets` | `kind` removed from request and response |
| `GET/PATCH /api/months/settings` | becomes `/api/settings`; same `MonthSettings` fields, renamed `Settings` |
| `POST /api/deposits/{id}/receive` | `credit_asset_id` becomes `account_id`; adds `bank_in` (default true) |
| `POST /api/bonds/{id}/receive` | same, plus `credit_amount` kept |
| `PATCH /api/coupons/{id}`, `PATCH /api/dividends/{id}` | add optional `account_id` beside `bank_in` |
| Deposit/Bond/Coupon/Dividend responses | expose `credited_account_id` (replacing `credited_asset_id` where present) |
| `OverviewResponse.semi_liquid.cash_rows` | element type becomes `CashAccount` |
| `GET/PATCH /api/ibkr` | field names unchanged; cash reads and writes the broker accounts; whole patch in one transaction |

`routes::tests::openapi_documents_every_api_operation` is updated to match.

### D9. Layering cleanup

- Delete:
  - `routes/yearly.rs::build` and `routes/year_review.rs::build` (callers use the `service` versions)
  - `routes/dividends.rs::credit_ibkr_usd`, which is replaced by receipts
  - `service/dividends.rs::credit_ibkr_usd`, which is dead
  - `routes/mod.rs::{resolve_stock, today, now_timestamp}` (callers use `service::trades::resolve_stock` / `store::{today, now_timestamp}`)
- Add `routes/patch.rs` with `merge_num`, `merge_text`, `merge_opt`, replacing the closures and fns re-declared in 7 route files.
- Move `ibkr_transfers` SQL from `store/overview.rs` to `store/ibkr.rs`.
- Split `models.rs` into `models/{mod,stocks,trades,dividends,deposits,family,bonds,mpf,aia,months,overview,forecast,cash,settings}.rs`, with `pub use` re-exports from `models/mod.rs` so imports elsewhere stay `crate::models::X`.

## Risks / Trade-offs

- **[Migration on real data]** A mistake corrupts `data/wealth.db`. Mitigations:
  - Each migration gets an inline test that builds a 0027-state DB with fixtures resembling the live data (cash ids 2/3, IBKR keys, HK credited dividends, pool rate keys) and asserts the post-state.
  - The equivalence run is done on a copy.
  - The user's DB is backed up to `data/wealth.db.bak-<date>` before the first real start.
- **[Numbers drift]** Mitigation: a before/after JSON capture of every computed GET on the same DB copy, diffed by value. Expected differences are only renamed fields and `cash_rows` element shape. The numbers must be identical. Run the "before" capture first, then migrate the copy. Theme 4's write-on-read means each capture must start from a fresh copy taken at the same instant.
- **[`ALTER TABLE RENAME COLUMN`]** needs SQLite ≥ 3.25. The bundled sqlx SQLite is newer, so this is acceptable.
- **[No DB-level FK on deposits/bonds `credited_account_id`]** Mitigated by the store nulling it on account delete. A real FK comes with the round-2 rebuild.
- **[Frontend churn]** Seven views change. Mitigated by `vue-tsc` catching type drift and a manual pass over each page's functions after restart.

## Migration Plan

1. `0028_fx_rates.sql`: create the table, seed from `aia.usd_hkd_rate`, delete the key.
2. `0029_cash_accounts.sql`:
   - create `cash_accounts`
   - copy the cash rows with their ids
   - insert the IBKR HKD/USD rows from the meta keys and delete those keys
   - set the defaults
   - rebuild `manual_assets` without `kind` (asset rows only)
3. `0030_credited_accounts.sql`:
   - rename the `deposits`/`bonds` columns
   - add the `bond_coupons`/`dividends` columns
   - backfill the coupons and dividends
4. `0031_settings_keys.sql`:
   - rename the keys
   - create `pool_rates`, move `overview.pool_rate.%` into it, delete those keys

Rollback: restore the file backup. Migrations are forward-only, and the app has a single user and a single file.

## Open Questions

- None blocking. Which cash account each pre-existing dividend credit went to is inferred (HS for HK). That is the account today's reversal logic would use, so behaviour is preserved.
