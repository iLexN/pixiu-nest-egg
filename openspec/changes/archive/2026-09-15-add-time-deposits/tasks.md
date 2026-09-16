## 1. Schema and models

- [x] 1.1 Add `backend/migrations/0003_deposits.sql` creating `deposits(id, label TEXT, principal REAL, rate REAL, interest REAL, end_date TEXT NOT NULL, note1 TEXT, note2 TEXT, sort_order INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL)` and verify `cargo run -p wealth-backend --bin wealth-backend` migrates cleanly on a fresh and an existing `data/wealth.db`
- [x] 1.2 Add `Deposit`, `NewDeposit`, `DepositPatch` (with the `nullable` deserializer for optional fields), and summary response types to `backend/src/models.rs`; verify `cargo check` passes

## 2. Calculation layer

- [x] 2.1 Implement `validate_deposit` in `backend/src/calc.rs` (end_date must be a real `YYYY-MM-DD` date; present principal/rate/interest ≥ 0; rate < 1; at least one of label/principal/interest present) and verify new unit tests cover each rejection plus the interest-only and label-only cases
- [x] 2.2 Implement the pure rollup functions in `calc.rs` — `total` (COALESCE principal+interest), `status` (End iff end_date <= today), active month rollup by (year, month), bank rollup by label prefix, and per-year month tables — and verify unit tests reproduce the workbook figures (active principal 445,000; SC group 365,000; 2027 Jan: 利息 807 / total 80,807)

## 3. Deposit API

- [x] 3.1 Add `backend/src/routes/deposits.rs` with `list` (`?status=active|ended&year=YYYY`), `create`, `update`, `remove`, and `summary` handlers, and wire the routes in `backend/src/routes/mod.rs`; verify `cargo check` passes
- [x] 3.2 Add API tests in `backend/tests/api.rs` covering create/list/edit/delete, validation errors, the interest-only row, and the summary shape; verify `cargo test` passes

## 4. Workbook import and parity

- [x] 4.1 Extend `backend/src/xlsx.rs` to parse `定期Info`'s `表_定期List` by header names into `SheetDeposit` (cached values for formula cells, blanks → `None`, row order → `sort_order`) and to read the cached aggregate cells (`定期!B1`, month table, bank rows, both year tables); verify a unit test asserts 23 deposit rows and the cached totals
- [x] 4.2 Extend `backend/src/import.rs` to insert deposits idempotently on the `(label, end_date, principal, interest)` key with multiplicity counting, and extend `import_xlsx`'s report; verify a second import reports all rows skipped
- [x] 4.3 Extend `backend/src/parity.rs` and `check_parity` with the deposits section (active principal total, month rollup, bank rollups, year tables, tolerance-based); verify `cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"` reports no deposit diffs

## 5. Frontend

- [x] 5.1 Add deposit types (`Deposit`, `NewDeposit`, `DepositSummary`, filters) and endpoints to `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 5.2 Create `frontend/src/components/DepositForm.vue` (label, principal, rate, 利息, end date, note1, note2, live derived-total preview) and `frontend/src/components/DepositTable.vue` mirroring the trade components
- [x] 5.3 Create `frontend/src/views/DepositsView.vue` with the Upcoming section (active table, total principal, month rollup, bank rollup), the History section (year selector defaulting to the current year, per-year month table and rows), and the static 定期 start/end step reminder block
- [x] 5.4 Wire a `定期` tab into `frontend/src/App.vue`, hiding the HK/US market nav while it is active; verify `pnpm build` succeeds and the tab renders against the dev backend

## 6. Verification and docs

- [x] 6.1 Run the full gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — all clean
- [x] 6.2 Run `import_xlsx` then `check_parity` against `財富分析報告.xlsx` and manually compare the Upcoming section against the 定期 sheet (total 445,000; month buckets; SC/HS rollups) and the History view against the year tables
- [x] 6.3 Update `docs/DATA_FLOW.md` with the deposit data flow and `deposits` table, and mark 定期 done in `AGENTS.md`'s roadmap (noting it preceded 派息)
