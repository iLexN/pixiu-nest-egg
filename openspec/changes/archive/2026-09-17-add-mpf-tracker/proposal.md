# Proposal

## Why

The MPF section is still tracked entirely in `財富分析報告.xlsx` on the `MPF` sheet. The monthly ritual is: update 總供款額 and 帳戶結存 once or twice a month, then hand-copy the current 回報率/收益 into the `last month` column at month-end and into the `max` column whenever a new high is noticed. Moving MPF into the app removes the hand-copying entirely — every balance update is recorded, and last-month / all-time-max figures are derived automatically.

This change covers only the MPF section. The fund-details table on the MPF sheet is deliberately not migrated (owner's choice). 債券, AIA, 香港年金, Month Stat and Overview remain in the workbook; MPF is being done before Month Stat/Overview at the owner's choice, out of the roadmap's listed order.

## What Changes

- New `mpf_accounts` table storing one row per MPF account (the sheet has two: `new type` and `強積金個人帳戶`): `label`, `trustee`, `contributions` (總供款額), `balance` (帳戶結存), `plan_name`, `member_no`, `sort_order`, plus seeded high-water marks `max_rate` and `max_gain`.
- New `mpf_history` table recording every balance/contribution update: `account_id`, `recorded_on` (date), `contributions`, `balance`, `synthetic` flag. `UNIQUE(account_id, recorded_on)` — editing again on the same day replaces that day's row instead of appending.
- Update flow (single transaction on each account update):
  1. Upsert today's history row with the new values.
  2. Month-rollover backfill: for every calendar month fully elapsed since the previous record that has no rows, insert a synthetic month-end row carrying the last-known values forward, so "last month" never shows a gap when the owner skips a month.
  3. Auto-raise `max_rate` / `max_gain` independently when the new values set a new high (independent maxima — the max rate and max net gain may come from different moments).
- Derived on read, never stored: per-account 回報率 `(balance − contributions) ÷ contributions`; last-month rate + net gain (latest history row in the previous calendar month); max rate + max net gain (MAX over seeded marks, all history, and current); portfolio buy / now / 回報率 / net gain / last-month / max as account aggregates.
- MPF API: `GET /api/mpf` (accounts + derived figures + note), `PATCH /api/mpf/accounts/:id` (contributions/balance/metadata update with the history side effects above), `PATCH /api/mpf/note` (single free-text note for the page, replacing the sheet's remark row), and history row deletion for correcting stale mistakes.
- New `MPF` top-level nav group with a single `總覽` page: totals header, per-account table with inline editing of 總供款額/帳戶結存, account metadata, the free-text note, and a recent-history list. Market toggle stays hidden.
- `import_xlsx` extended to read the `MPF` sheet's account table (rows 5–7) idempotently, seed one synthetic last-month history row per account from the cached last-month rate, and seed `max_rate`/`max_gain` from the cached max column.
- `check_parity` extended with an MPF section comparing buy/now/回報率 against the workbook's cached values. Seeded last-month/max net gains are approximations (the sheet stores rates only), so those comparisons use tolerance and report rather than fail.
- The workbook `MPF` sheet becomes frozen after import. `Overview!B6 = MPF!B2` will go stale until Overview is migrated — same phased-migration caveat as 定期; the MPF page shows a reminder hint that Overview/Month Stat still live in the workbook.
- Not in this change: the fund-details table, 債券/AIA/香港年金 sheets, Month Stat/Overview, per-fund allocation, and any rate history charting. The workbook remains source of truth for non-migrated sections.

## Capabilities

### New Capabilities
- `mpf-accounts`: recording MPF accounts and their balance/contribution updates, the automatic history/rollover/max tracking, the derived last-month and max figures, and the MPF overview page with its free-text note.

### Modified Capabilities
- `spreadsheet-trade-import`: the import command also imports the `MPF` sheet's accounts idempotently and seeds their history/maxima, and the parity command also compares MPF totals against the workbook's cached figures.
- `app-navigation`: the top-level nav gains a third `MPF` group containing a `總覽` sub-tab; the market toggle stays hidden while it is active.

## Impact

- Backend: new migration `0009_mpf.sql`; new `routes/mpf.rs`; additions to `models.rs`, `calc.rs`, `xlsx.rs`, `import.rs`, `parity.rs`, and both binaries.
- Frontend: new `MPF` nav group and `MpfView.vue`; `api.ts` types and endpoints; `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` gains the MPF data flow; `AGENTS.md` roadmap marks MPF migrated.
- No new dependencies; `財富分析報告.xlsx` stays read-only and its formulas keep working.
- `Overview!B6` keeps pointing at the now-frozen workbook MPF sheet; that staleness is expected until Month Stat/Overview is migrated.
