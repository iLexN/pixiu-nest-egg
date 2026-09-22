# Design

## Context

The workbook `Month Stat` sheet is a balance-difference ledger: one row per month since 2023-12, where the user types the 活期 bank total at payday (F, already including the just-paid salary) and a sum-expression of non-spending bank flows (G 調整); the sheet then derives 月尾 `=F(next) − salary`, 月支出 `=F+G−H`, 生活支出 `=I − inline extras`, and 存 `=salary − I (+ extras)`. Columns B/D (總數/流動資產) link live to `Overview!B1`/`H1` on the newest row and are paste-valued into history. Rows 2–4 hold per-year aggregates; the M column chains the 開心Pool balance with a per-year rate kept in `Overview!N8`. See proposal.md and `docs/MONTH_STAT_OVERVIEW.md` for the full analysis and the confirmed workflow.

Existing conventions to follow: deposits-style REST routes (`list`/`create`/`update`/`remove` + `summary`), `app_meta` key-value storage (`mpf.note`, `aia.usd_hkd_rate`), sqlx migrations numbered sequentially (next is `0014`), `REAL`/`f64` money with display-only formatting, derived fields computed on read, market-history-style month boundaries via `chrono`.

## Goals / Non-Goals

**Goals:**
- Store only what the user enters (payday balance, salary snapshot, interest/entertainment/pool figures, frozen totals, items); derive everything the sheet derives, with identical formula shapes so parity is meaningful.
- Replace the opaque G/J/L sum formulas with labeled items, pre-filled from events already in the DB.
- Reproduce `Overview!B1`/`H1` live totals inside the backend so the current month's row works without an Overview page — the same aggregation the later Overview change reuses.
- Import history verbatim (frozen values, formula text kept as item notes) and keep the whole thing parity-checkable.

**Non-Goals:**
- The Overview dashboard, forecast grid, targets, YearInReview/回報率 generation (later change).
- `HappyPool`/`Mum`/`Dad`/`香港年金`/`FIRE`/`ref1` sheets.
- Transaction-level spending detail — the whole point of the sheet is that it isn't tracked.

## Decisions

### Schema (migration `0014_month_stat.sql`)

```sql
CREATE TABLE month_stats (
    month          TEXT PRIMARY KEY,          -- 'YYYY-MM-01'
    start_cash     REAL,                      -- F 月初(出糧後)
    salary         REAL,                      -- salary in effect that month
    total_assets   REAL,                      -- B 總數; NULL = live
    liquid_assets  REAL,                      -- D 流動資產; NULL = live
    interest       REAL NOT NULL DEFAULT 0,   -- N 利息
    entertainment  REAL NOT NULL DEFAULT 0,   -- O 娛樂支出
    pool_input     REAL NOT NULL DEFAULT 0,   -- P Irene + 開心 Pool
    end_cash_override REAL,                   -- hand-frozen H 月尾; NULL = derive
    note           TEXT,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

CREATE TABLE month_items (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    month      TEXT NOT NULL REFERENCES month_stats (month) ON DELETE CASCADE,
    category   TEXT NOT NULL CHECK (category IN ('adjustment','extra_spend','income')),
    label      TEXT,
    amount     REAL NOT NULL,
    auto_key   TEXT,                          -- NULL = manual or imported
    note       TEXT,                          -- imported formula text / origin
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_month_items_auto
    ON month_items (month, auto_key) WHERE auto_key IS NOT NULL;

CREATE TABLE month_item_dismissals (
    month      TEXT NOT NULL,
    auto_key   TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (month, auto_key)
);

CREATE TABLE manual_assets (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    label      TEXT NOT NULL,
    kind       TEXT NOT NULL CHECK (kind IN ('cash','asset')),
    amount     REAL NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE deposits ADD COLUMN start_date TEXT;   -- optional; feeds dep-start suggestions
```

- `interest`/`entertainment`/`pool_input` stay scalar columns — the sheet tracks each as one number per month and the user confirmed they only need items for 調整/extra-spend/income. Alternative — itemize interest too: rejected for now, it would complicate import (history has single sums) for marginal benefit; the auto-suggested interest *figure* is shown in the UI instead (see below).
- `salary` lives on the row, not in a separate salary-history table: each row snapshots the current setting at creation, which reproduces the sheet's per-era literals (`=F14 - 45500`, `=F16 - 47850`) with one column. Alternative — effective-dated salary table: rejected, more machinery for a value edited once a year.
- `manual_assets` covers Overview's four manual balance cells with one table: `cash` rows are the 活期 behind `start_cash` and the liquid totals (B16 HS, B17 渣打); `asset` rows feed only `total_assets` (B7 Irene, B8 HS人壽). Chosen over `app_meta` keys because balances are a list the user may grow, and over a per-month balance history because the sheet stores one current value — the per-month memory is `start_cash` itself.

### Derived math (`calc.rs`)

Per month `m`, letting `n` = the next stored month:

```text
end_cash(m)      = end_cash_override(m) ?? (start_cash(n) − salary(m))
                   -- absent without n or an override
month_spend(m)   = start_cash(m) + Σadjustment(m) − end_cash(m)
living_spend(m)  = month_spend(m) − Σextra_spend(m)
saved(m)         = salary(m) − month_spend(m) + Σincome(m)
total_change(m)  = total_assets(n) − total_assets(m)    -- absent without n
liquid_change(m) = liquid_assets(n) − liquid_assets(m)
```

Note `end_cash` uses the row's *own* salary: the sheet's `H25 (2025-03) = F26 − 47850` keeps March's 47850 even though April's L26 shows the raised 50810 — verified across every raise (45500→47850→50810→52700). Salary per row is recovered on import from the leading literal of the row's own L formula (`=47850-I25 + 83895`).

`end_cash_override` exists because the chain is not universal: `H10` (2023-12) is `=78110.32-8000-45500` — the real bank balance typed by hand before the 月初 convention existed — and `H11` is a literal equal to the chain value. The import marks any H cell that is not `=F<next> − …` as frozen and stores its cached value; a blank H stores NULL and derives normally. Clearing the override (UI field left blank → `null`) restores derivation.

`pool_balance(y)` chains yearly: `pool_balance(y−1) + Σinterest(y) × rate(y) − Σentertainment(y) + Σpool_input(y)`, base `pool_balance(<first year−1>) = 0`. The "current" pool balance (used by live `liquid_assets` and the UI) is the running balance through the latest stored month of the current year.

### Live totals

When a row's `total_assets`/`liquid_assets` is `NULL`, derive live (reproducing `Overview!B1`/`H1`):

```text
total_assets   = hk.market_value + us.market_value×rate + deposits.active_total
               + bonds.active_principal + aia.value×rate + mpf.balance
               + Σmanual_assets(asset) + Σmanual_assets(cash)
liquid_assets  = hk.market_value + deposits.active_total + Σcash
               + bonds.active_principal + us.market_value×rate − pool_balance(current)
```

Each term maps to an existing summary computation (the market `buy_cost_priced`/`market_value` totals, deposit active rollup, `債券!B1` equivalent, AIA totals, MPF totals). Components needing `aia.usd_hkd_rate` are omitted when no rate is stored. This goes in a shared `calc` helper so the later Overview endpoint composes the same pieces.

Frozen vs live: importing a row whose B/D cells are formulas stores `NULL` (live); literals store frozen. Creating a month row via `PATCH`/`PUT` snapshots the live values (the paste-values ritual, made explicit); `recapture: true` re-snapshots; patching the fields to `null` returns to live. Alternative — auto-freeze when the next row appears: rejected, the snapshot belongs to the payday moment, not month-end processing.

### Auto-suggested items

`GET /api/months/:ym` merges stored items with computed candidates for months ≥ the current month:

| auto_key | Source | Category | Amount |
|---|---|---|---|
| `dep-start:<id>` | deposits with `start_date` in month | adjustment | −principal |
| `dep-end:<id>` | deposits ending in month | adjustment | +(principal + interest) |
| `trade:<id>` | HK trades dated in month | adjustment | −total (BUY) / +total (SELL) |
| `div:<id>` | HK dividends with `received_amount`, pay_date in month (US ones stay in IBKR) | adjustment | +received_amount |
| `coupon:<id>` | coupons with `received_amount`, pay_date in month | adjustment | +received_amount |
| `aia-pay:<id>` | AIA payment events in month | extra_spend | amount_usd × rate |
| `pool-input` | month's own `pool_input` | adjustment | −pool_input (only while > 0) |

Candidates exclude keys already stored in `month_items` and keys in `month_item_dismissals`. Accepting POSTs an item carrying the `auto_key` (the partial unique index prevents double-accept); dismissing writes a dismissal row. Suggestions are computed, never auto-inserted — stored rows stay the only truth, so re-ordering or deleting events never silently rewrites a month. Alternative — materialize rows on first view: rejected, it writes on GET and needs tombstones for the same dismissal problem anyway.

Cutoff at the current month: imported history already carries its 調整 as single items; suggesting auto items there would double-count. Manual items have no `auto_key` and are unaffected.

The response also carries a suggested `interest` figure (Σ deposit `interest` ending in the month + Σ received coupon amounts + Σ received dividends… — flag in tasks: verify against sheet N-column semantics during implementation; bank savings interest has no source and stays manual) shown next to the editable `interest` field rather than materialized.

### Import (`xlsx.rs`, `import.rs`)

- `Month Stat` parsed like other optional-but-present sheets: monthly rows = rows under the header with a date in A and any data cell; B/D read as cached values, with `worksheet_formula` consulted — a B/D formula (`=Overview!$B$1`) imports `NULL` (live), a literal imports frozen.
- G/J/L handling: the sheet's adjustment sum (G), the extras inside J's formula (`=I−extras`), and the income extras inside L's formula (`=salary−I+extras`) import as single `month_items` carrying the cached delta and the formula text as `note`. Concretely: adjustment item = cached G; extra_spend item = `cached I − cached J` when nonzero; income item = `cached L − (salary − cached I)` when nonzero. This keeps every derived column consistent without parsing expressions.
- `salary` per row recovered from the *previous* row's H formula literal (`=F15 - 47850` → `47850` belongs to row 15's month); the earliest row falls back to the next parsed salary, then `Overview!E1`.
- Settings seeded once (`INSERT OR IGNORE`): `overview.salary` ← `Overview!E1`; `manual_assets` ← B16/B17 (cash) + B7/B8 (asset); `overview.pool_rate.<current year>` ← `Overview!N8`; historical `overview.pool_rate.<y>` solved from imported aggregates so imported pool balances reproduce exactly.
- `deposits.start_date`: the workbook has no start column, so import best-effort parses the first date in `note2` (the SC-marathon rate-schedule text, e.g. `17 Jun 2026 to 02 Aug 2026: 2.60%`) and leaves `NULL` otherwise; new deposits default the field to today in the form. A NULL `start_date` simply produces no `dep-start` suggestion — nothing breaks.
- Idempotency: month row key = `month` (PK); skip if present. Items belong to skipped months, so no item dedup needed.

### Parity (`parity.rs`)

Compare stored `start_cash`/`interest`/`entertainment`/`pool_input`/frozen totals per month against the sheet's cached F/N/O/P/B/D; derived `end_cash`/`month_spend`/`living_spend`/`saved`/`total_change` against cached H/I/J/L/C/E where present; the yearly block against rows 2–4; running averages against row 8; seeded settings against `Overview!E1`/`N8`/B7/B8/B16/B17. The live-linked current row and user-edited cells are informational — same convention as deposits/MPF parity staleness.

### API

Flat routes in `routes/months.rs`, wired in `mod.rs`:

- `GET /api/months?year=` — rows + derived fields, sheet column order.
- `GET /api/months/summary` — per-year aggregates, running averages, current pool balance, pool rate.
- `GET /api/months/:ym` — one row + items + `suggestions` (with `auto_key`) + suggested interest figure.
- `PATCH /api/months/:ym` — upsert: create snapshots live totals + current salary; patch merges fields; `recapture` re-snapshots totals; `null` on `total_assets`/`liquid_assets` restores live derivation.
- `DELETE /api/months/:ym` — removes the row and cascades its items.
- `POST /api/months/:ym/items`, `PATCH /api/month-items/:id`, `DELETE /api/month-items/:id` — item CRUD; posting with an `auto_key` accepts a suggestion.
- `POST /api/months/:ym/items/dismiss` `{ auto_key }` — tombstone a suggestion.
- `GET/POST /api/manual-assets`, `PATCH/DELETE /api/manual-assets/:id` — balance list.
- `GET/PATCH /api/months/settings` — `{ salary, pool_rate }` over `app_meta`.

### Frontend

`App.vue` gains a `月結` group (after AIA) with one 總覽 tab → `MonthStatView.vue`: a yearly aggregate strip (the sheet's rows 2–4 figures for the selected/current year + pool balance + running averages), the month table showing all stored and derived columns with a year selector (like DividendsView), a per-row expander or drawer editing the month's fields and its item list — auto-suggestions rendered as ghost rows with 接受/忽略 and manual add — and a settings card for the manual balances, salary and pool rate. Reload after every mutation per repo convention.

## Risks / Trade-offs

- **Auto-suggestion semantics vs the sheet's G** — the sheet's adjustment column lumps whatever the user typed; the app's candidates are event-derived and may differ in granularity (e.g. a 定期 end credits principal+interest while the sheet may split them). → Parity compares derived `month_spend` per month, so a mismatched suggestion surfaces immediately; suggestions are confirm-only and editable.
- **Interest suggestion is incomplete** — bank 活期 interest isn't tracked anywhere. → It's a displayed hint on the editable field, not an auto-item; no false precision.
- **Frozen totals go stale by design** — captured at creation, they don't move with later edits (same as paste-values). → `recapture` re-snapshots; `null` returns to live; the UI marks live cells.
- **Pool rate fallback** — a year with no stored rate inherits the latest earlier year's, matching "the rate changes at year-end and applies to that year"; a year with no rate anywhere reports no pool income rather than guessing.
- **US-side flows are manual** — US trades settle inside IBKR, so the IBKR transfer is the bank event and stays a manual item; documented in DATA_FLOW.md.

## Migration Plan

1. `0014_month_stat.sql` creates the four tables.
2. `import_xlsx` parses Month Stat + Overview manual cells and seeds settings; `check_parity` reports the new section — both verified against the real workbook before UI work is trusted.
3. API + `MonthStatView.vue` land together behind the `月結` nav group.
4. Rollback: drop the new tables and the `overview.*` meta keys; the workbook is untouched throughout.
