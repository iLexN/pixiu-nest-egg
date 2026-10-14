# Proposal

## Why

The migration out of `財富分析報告.xlsx` is finished. Every section the app tracks lives in `data/wealth.db`, and the remaining workbook-only sections (the `YYYY回報率` projection sheets, the 開心Pool independent ledger, 香港年金, FIRE, ref1) were dropped. The importer and parity checker no longer have a job:

- re-importing would bring back data older than the app's own
- parity compares against cached formula values frozen at the last export, so it breaks on every date-based rollover; two import tests have failed since 2026-10-13 for exactly that reason

Keeping them costs three large modules, two binaries, an integration test file, a dependency, and a 35-requirement spec.

## What Changes

- **BREAKING** (tooling only): remove the `import_xlsx` and `check_parity` binaries, the workbook reader, the importer, the parity checker, their integration tests, and the `calamine` dependency. No HTTP endpoint or UI changes; the running app never used this code.
- Retire the `spreadsheet-trade-import` capability entirely.
- Drop the import/parity clauses from the capabilities that mention them. The live rules stay: for example, create still records `received_at` for a past end date, and seed marks already stored still floor the 最高 figures.
- Keep the data the import once wrote (seed marks, synthetic history rows, year-review overrides, settings) and every existing migration. Runtime code reads those columns.
- Keep `import_prices` (JSON 現價 updates). It doesn't depend on the workbook.
- Update `README.md`, `AGENTS.md`, and `docs/`:
  - setup no longer imports a workbook; a fresh database starts empty via migrations
  - the migration is recorded as complete, with the five sections dropped
  - `docs/import-parity.md` and the workbook-analysis `docs/MONTH_STAT_OVERVIEW.md` are deleted (git history keeps them)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `spreadsheet-trade-import`: all 35 requirements removed — capability retired.
- `time-deposits`: "Deposit registry" (imported start_date / workbook sort order), "Upcoming deposit view" (import scenario and parity clause), "Deposit 收訖 lifecycle" (import auto-receive).
- `family-deposits`: "Isolation from the user's own figures" — drop the import/parity sentence.
- `bonds`: "Bond principal 收訖 lifecycle" — drop import auto-receive.
- `aia-policies`: "Manual USD→HKD rate" — drop seeding from the workbook.
- `overview`: "Goal settings" — drop "seeded at import".
- `stock-portfolio-summary`: "Manual stock display order" (initial workbook order) and "Last-month and max unrealized figures" (seed marks are stored, not imported).
- `stock-yearly-summary`: "Workbook seeding" removed.
- `year-in-review`: "Workbook seeding" and "Parity check" removed.

## Impact

- **Deleted code:** `backend/src/xlsx.rs`, `backend/src/import.rs`, `backend/src/parity.rs`, `backend/src/bin/import_xlsx.rs`, `backend/src/bin/check_parity.rs`, and `backend/tests/import.rs`, plus their `pub mod` lines in `backend/src/lib.rs` and `calamine` in `backend/Cargo.toml`. Any store helpers left unused once these are gone are removed too.
- **Tests:** the two failing import tests go away. `cargo test` should be fully green again.
- **Data:** no migration, no change to `data/wealth.db`. After this change, `data/wealth.db` is the only copy of the data, so the user should back it up. Deleting the local `財富分析報告.xlsx` (gitignored) is left to the user.
- **Docs:** `README.md`, `AGENTS.md`, `docs/DATA_FLOW.md`, `docs/database.md`, `docs/deposits.md`, `docs/investments.md`; delete `docs/import-parity.md` and `docs/MONTH_STAT_OVERVIEW.md`.
