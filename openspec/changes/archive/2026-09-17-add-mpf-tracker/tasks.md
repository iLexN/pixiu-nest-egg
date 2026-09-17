# Tasks

## 1. Schema and models

- [x] 1.1 Add `backend/migrations/0009_mpf.sql` creating `mpf_accounts`, `mpf_history` (`UNIQUE(account_id, recorded_on)`, `synthetic` flag), and `app_meta`; verify `cargo test` migration tests pass and `sqlite3 data/wealth.db ".schema mpf_accounts"` shows the tables after running the backend once
- [x] 1.2 Add `MpfAccount`, `MpfHistoryRow`, and request/response types to `backend/src/models.rs`; verify `cargo build` compiles

## 2. Update flow and derivation

- [x] 2.1 In `backend/src/calc.rs` (or a new `mpf` helper module), implement the update transaction: account update + same-day `INSERT ... ON CONFLICT DO UPDATE` history upsert + rollover backfill of synthetic month-end rows for fully elapsed empty months using pre-update values; verify unit tests cover: same-day replace, metadata-only edit writes no row, Aug→Oct update backfills September
- [x] 2.2 Implement derived figures in `calc.rs`: per-account rate/gain, last-month (latest row in previous calendar month), independent maxima over `seed_max_*` + history + current, and portfolio as-of merge (union of history dates, latest row ≤ date per account) for totals/last-month/max floored by `app_meta` seeds; verify unit tests cover zero-contribution rate, independent maxima from different dates, and missing-month carry-forward

## 3. API

- [x] 3.1 Add `backend/src/routes/mpf.rs` with `GET /api/mpf`, `POST /api/mpf/accounts`, `PATCH /api/mpf/accounts/:id`, `DELETE /api/mpf/accounts/:id` (refused when history exists), `PATCH /api/mpf/note`, `DELETE /api/mpf/history/:id`; wire in `routes/mod.rs`/`main.rs`; verify with curl that PATCH returns updated derived figures and validation rejects negative balance/missing label
- [x] 3.2 Verify delete-history recomputes last-month/max without the row (integration test or curl check against `connect_memory` DB)

## 4. Workbook import and parity

- [x] 4.1 Extend `backend/src/xlsx.rs` to parse the `MPF` sheet account table (locate the 總供款額/帳戶結存 header row; read A/B/C/D/F/G/H/I/J; stop at the blank separator; ignore fund-details and remark rows); verify parse output against `財富分析報告.xlsx` yields the two accounts
- [x] 4.2 Extend `backend/src/import.rs` to upsert MPF accounts by label (idempotent), seed one synthetic last-month history row (`balance = contributions × (1 + last_month_rate)`), set `seed_max_*`, and store `mpf.seed_max_rate`/`mpf.seed_max_gain` in `app_meta`; verify a second `import_xlsx` run reports all skipped and adds no rows
- [x] 4.3 Extend `backend/src/parity.rs` with an MPF section comparing per-account contributions/balance/rate and portfolio buy/now/rate/gain strictly, and seeded last-month/max gains with tolerance as informational; verify `check_parity` reports matches against the workbook's cached figures

## 5. Frontend

- [x] 5.1 Add `MPF` nav group with a single `總覽` sub-tab in `App.vue` (market toggle hidden while active); verify nav shows 股票, 定期, MPF in order and the toggle hides on MPF
- [x] 5.2 Add `api.ts` types/endpoints and `frontend/src/views/MpfView.vue`: totals header (buy/now/rate/gain + last-month and max rate and net gain), per-account table with inline contributions/balance editing, metadata fields, note textarea, and per-account history list with delete; verify `pnpm exec vue-tsc --noEmit` and a manual UI pass: edit a balance, see rate/last-month/max recompute
- [x] 5.3 Show the workbook reminder hint (Overview/Month Stat unmigrated; `Overview!B6` reads the frozen sheet) on the MPF page

## 6. Docs and verification

- [x] 6.1 Update `docs/DATA_FLOW.md` with the MPF data flow and `AGENTS.md` roadmap to mark MPF migrated; verify docs match implemented behavior
- [x] 6.2 Run full verification: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`, plus a real `import_xlsx` + `check_parity` against `財富分析報告.xlsx`
