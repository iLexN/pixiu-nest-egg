# Design

## Context

The workbook `MPF` sheet holds two Manulife accounts (rows 5–7: label, trustee, 總供款額, 帳戶結存, derived 回報率, hand-copied `last month`/`max` rates, plan/contract/member metadata), a totals block (A1:D2, F1:H2), a fund-details table (not migrated), and a remark line. `Overview!B6 = MPF!B2` reads the sheet's `now` total. See proposal.md for motivation and scope.

Existing conventions to follow: `deposits`-style REST routes (`list`/`create`/`update`/`remove` + a `summary` handler), sqlx migrations numbered sequentially (next is `0009`), `crate::routes::today()` for the current date, `REAL`/`f64` money with display-only formatting, derived fields computed on read.

## Goals / Non-Goals

**Goals:**
- Replace the sheet's manual copy ritual: last-month and max figures come out of recorded history automatically.
- Keep corrections cheap: same-day edits replace, stale rows can be deleted, nothing is polluted permanently.
- Import + parity follow the established pattern so the migration stays verifiable against the workbook.

**Non-Goals:**
- Fund-details table, contribution history as a separate ledger (the sheet has a single 總供款額 figure, and history rows capture its evolution implicitly).
- Charts/sparklines, projections, per-fund allocation.
- 債券 / AIA / 香港年金 / Month Stat / Overview sheets.

## Decisions

### Schema (migration `0009_mpf.sql`)

```sql
CREATE TABLE mpf_accounts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    label        TEXT NOT NULL,
    trustee      TEXT,
    contributions REAL NOT NULL DEFAULT 0,
    balance      REAL NOT NULL DEFAULT 0,
    plan_name    TEXT,
    member_no    TEXT,
    sort_order   INTEGER NOT NULL,
    seed_max_rate REAL,
    seed_max_gain REAL,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE TABLE mpf_history (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id    INTEGER NOT NULL REFERENCES mpf_accounts(id),
    recorded_on   TEXT NOT NULL,          -- YYYY-MM-DD
    contributions REAL NOT NULL,
    balance       REAL NOT NULL,
    synthetic     INTEGER NOT NULL DEFAULT 0,
    UNIQUE(account_id, recorded_on)
);

CREATE TABLE app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

- `seed_max_rate` / `seed_max_gain` are the imported high-water marks — written only at import, never touched by updates.
- `app_meta` is a generic key-value table (key `mpf.note` for the section note; `mpf.seed_max_rate`/`mpf.seed_max_gain` for the portfolio-level seeded maxima, which cannot be reconstructed from per-account data). Chosen over a single-row `mpf_meta` table because 債券/AIA will likely want the same facility later.

### Update flow (`PATCH /api/mpf/accounts/:id`, one transaction)

1. Update the account row (contributions/balance and/or metadata).
2. If contributions or balance changed: `INSERT ... ON CONFLICT(account_id, recorded_on) DO UPDATE` a history row for today (same-day replace), `synthetic = 0`.
3. Rollover backfill: let `M_last` = the month of the account's latest prior history row (or account creation month if none). For every calendar month strictly between `M_last` and today's month, insert a synthetic row dated that month's last day carrying the **pre-update** contributions/balance (the values that stood during the empty months).
4. No explicit max writes — maxima are derived (below).

### Derivation (`calc.rs`)

- Per account: `rate = (balance − contributions)/contributions` (empty at 0); `gain = balance − contributions`. Same formulas applied to each history row.
- Per-account last month: latest row with `recorded_on` in the previous calendar month.
- Per-account max: `max_rate = MAX(seed_max_rate, every row's rate, current rate)`; `max_gain` likewise, independently.
- Portfolio figures use an **as-of merge** over the union of all history dates plus today: at each date, each account contributes its latest row ≤ that date (accounts with no such row are absent); portfolio rate/gain computed from the sums. `max_rate`/`max_gain` = the timeline maxima, floored by the `app_meta` portfolio seeds. Last month = the portfolio as-of the last day of the previous month. This handles accounts missing a month (carry-forward) and gives a true portfolio max rather than summing per-account peaks that never co-occurred.

Alternative considered: storing and eagerly raising `max_*` on every update. Rejected — deleting a mistyped row couldn't un-pollute the stored max. Pure derivation keeps the spec's "deleting a row recomputes without it" guarantee trivially.

### Import seeding (`xlsx.rs`, `import.rs`)

- Parse the account table by locating the header row containing 總供款額/帳戶結存; read columns A (label), B (trustee), C (contributions), D (balance), F (last-month rate), G (max rate), H (plan), I (contract), J (member) until the blank separator row. Fund-details and remark rows are ignored.
- Per account, seed one synthetic history row dated the last day of the previous calendar month: `balance = contributions × (1 + last_month_rate)`. With exactly two accounts the per-account rates plus the portfolio last-month rate+gain solve for the actual month-end contributions (`mpf_last_month_contributions`); otherwise `contributions = current contributions` and the gain is approximate (noted in the spec's tolerance).
- `seed_max_rate` = cached G value; `seed_max_gain` = G × current contributions (approximation). Portfolio seeds `mpf.seed_max_rate`/`mpf.seed_max_gain` = cached G2/H2.
- Idempotency key: account `label` (accounts are few and labels distinct).

### API

- `GET /api/mpf` → `{ accounts: [{ …fields, rate, gain, last_month: {rate, gain} | null, max: {rate, gain} }], totals: { buy, now, rate, gain, last_month, max }, note, history: [{ id, account_id, recorded_on, contributions, balance, synthetic }] }`
- `POST /api/mpf/accounts`, `PATCH /api/mpf/accounts/:id` (update flow above), `DELETE /api/mpf/accounts/:id` (refused when history exists)
- `PATCH /api/mpf/note` → `{ note }`
- `DELETE /api/mpf/history/:id`

### Frontend

New `MPF` nav group, single `總覽` tab → `MpfView.vue`: totals header, account table (inline-edit contributions/balance like the deposits/stocks patterns), metadata fields, note textarea, and a compact history list per account with delete affordance. Market toggle hidden per the nav delta.

## Risks / Trade-offs

- **Overview staleness**: `Overview!B6` keeps reading the frozen workbook sheet until Overview is migrated → the page carries a reminder hint; parity only compares MPF internals, not Overview.
- **Seeded figures are approximate** for gains (rates are exact) → parity compares buy/now/rate strictly and last-month/max gains with tolerance; after one real month of updates everything is exact.
- **Daily granularity**: `UNIQUE(account_id, recorded_on)` means intra-day peaks beyond the last edit of a day aren't kept — acceptable at 1–2 updates/month.
- **Month semantics**: "last month" means the latest row dated in the previous calendar month, not a statement-date boundary. If the owner backdates intent (updates Oct 2 meaning September), the row lands in October — same caveat the sheet had; the rollover backfill still covers fully-empty months.
- **Small drift risk**: synthetic rows use pre-update values; if the owner edits metadata mid-gap nothing changes — correct by construction.

## Migration Plan

1. Migration `0009_mpf.sql` creates the three tables.
2. `import_xlsx` parses + seeds; `check_parity` reports the MPF section.
3. API + `MpfView.vue` land together behind the new nav group.
4. Rollback: drop the new tables; the workbook is untouched throughout.
