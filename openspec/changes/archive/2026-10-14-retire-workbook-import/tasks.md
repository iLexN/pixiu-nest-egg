# Tasks

## 1. Remove the workbook code

- [x] 1.1 Delete `backend/src/xlsx.rs`, `backend/src/import.rs`, `backend/src/parity.rs`, `backend/src/bin/import_xlsx.rs`, `backend/src/bin/check_parity.rs`, and `backend/tests/import.rs`. Drop `pub mod xlsx; pub mod import; pub mod parity;` from `backend/src/lib.rs` and remove `calamine` from `backend/Cargo.toml` (`cargo remove -p wealth-backend calamine`). Verify that `cargo build -p wealth-backend --all-targets` succeeds.
- [x] 1.2 Remove helpers that are now dead. Run `cargo clippy --all-targets -- -D warnings` and delete every item it reports as unused. Then grep each `crate::store::*`/`crate::models::*`/`crate::calc::*` symbol the deleted files called, and delete any that has no remaining non-test caller. Keep symbols runtime still uses, e.g. `seed_max_percent_key`, `seed_max_amount_key`, `routes::today`, `store::today`. Verify that `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` all pass, with no import test failures left.

## 2. Docs

- [x] 2.1 Delete `docs/import-parity.md` and `docs/MONTH_STAT_OVERVIEW.md`. Update `docs/DATA_FLOW.md`: remove the import-parity and MONTH_STAT_OVERVIEW index rows, the "Workbook import"/"Parity check" rows, and the "Does the app modify the Excel workbook?" FAQ. Verify that no link in `docs/` points to a deleted file.
- [x] 2.2 Update `docs/database.md`, `docs/deposits.md`, and `docs/investments.md`:
  - describe columns as data, not import behavior: `bank` comes from the entered label/bank; `sort_order` means new entries append; seed marks are "stored all-time marks (from the retired workbook import)"; `synthetic` rows are backfilled
  - `docs/deposits.md` L59: drop the "still-unmigrated" wording about the 手動步驟提醒 hints
  - `docs/investments.md` ~L114: the rate is entered manually

  Verify that grepping these files for `import`, `parity`, and `workbook` shows only intentional provenance notes.
- [x] 2.3 Update `README.md`:
  - intro: no "imports the workbook / verify the two agree"
  - the 工具 row: OpenAPI docs and `import_prices` only
  - remove the "not migrated" paragraph (those sections were dropped)
  - quick start: build the frontend, run the backend, and an empty `data/wealth.db` is created by migrations
  - commands: drop `import_xlsx`/`check_parity`, keep `import_prices`

  Verify that the documented commands match the remaining binaries (`ls backend/src/bin`).
- [x] 2.4 Update `AGENTS.md`:
  - intro and Layout: no workbook-as-source-of-truth wording; remove `xlsx.rs`/`import.rs` from the layout
  - drop "a fresh clone must be given the workbook"
  - replace "Import and parity check" with a short `import_prices` note
  - drop the "Import and parity never write to the workbook" rule
  - keep the "Spreadsheet-compatible behavior that is intentional" rules (they're calculation rules)
  - rewrite "Migration status" as: migration complete, workbook retired; `YYYY回報率` projection sheets, 開心Pool ledger, 香港年金, FIRE, ref1 were dropped, not migrated

  Verify by reading the file end to end for stale references.

## 3. Integration

- [x] 3.1 Grep `backend/`, `frontend/src`, `docs/`, `README.md`, and `AGENTS.md` for `import_xlsx|check_parity|parity|xlsx|calamine`. Every remaining hit is intentional (e.g. `.gitignore`, provenance comments). Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; both pass.
- [x] 3.2 Restart the backend on the live DB (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend` in the background). Confirm that `/api/overview`, `/api/deposits/summary`, and the HK market summary answer, and that HK 最高 still reports the stored seed marks.

## Workflow follow-up

- Back up `data/wealth.db`; it's now the only copy of the data.
- Optionally keep one final `.xlsx` export outside the repo, then delete the local `財富分析報告.xlsx` (gitignored).
- Archive the change. Archive retires the `spreadsheet-trade-import` main spec (`retire_capabilities: true`).
