## Why

Time deposits (定期) are still tracked entirely in `財富分析報告.xlsx` across two sheets: `定期Info` holds the master list (`表_定期List`) and `定期` shows the active deposits with totals, month and bank rollups. Starting or ending a deposit means copying numbers across `Month Stat`, `回報率`, and `Overview` by hand — the owner described this as too much workload and wants reminder hints in the app until those sections are also migrated. Moving the deposit registry into the app removes the cross-sheet copying for this section and continues the section-by-section migration.

This change covers only the 定期 section. 派息, Month Stat/Overview, and MPF/債券/AIA remain in the workbook; 定期 is being done before 派息 at the owner's choice, out of the roadmap's listed order.

## What Changes

- New `deposits` table storing one row per 定期 entry: `label` (the sheet's `id` column, e.g. `SC-9632`), `principal` (`input`), `rate`, `interest` (利息), `end_date`, `note1`, `note2`, `sort_order`. `label`, `principal`, `rate`, and `interest` are nullable — the source list contains interest-only rows (no id/principal) and a label-only row.
- Derived on read, never stored: `total = principal + interest` (blanks count as 0), `status` (`End` when `end_date` is today or past — same rule as the sheet's `TODAY()` formula), and `end_month`/`end_year`.
- Deposit CRUD API: `GET/POST /api/deposits`, `PATCH/DELETE /api/deposits/:id`, plus `GET /api/deposits/summary` returning the aggregate views.
- New `定期` tab in the frontend (market nav hidden) with two sections:
  - **Upcoming**: active deposits (end date in the future) with the sheet's figures — total principal, per-month maturity rollup (Σ total / Σ 利息 / Σ principal), and per-bank rollup by label prefix (`SC-*`, `HS-*` generalized to "prefix before first `-`").
  - **History**: ended deposits filtered by end year (defaults to the current year), with that year's per-month aggregation — the equivalent of the `定期Info` year tables (表_2027定期 / 表_2028定期), generated for every year present in the data.
- The "定期 start step" / "定期 end step" checklists shown as a static reminder block in the deposits view, since the workbook steps they describe are still manual.
- `import_xlsx` extended to read `定期Info`'s `表_定期List` (cached formula values) and import deposits idempotently, preserving workbook row order via `sort_order`.
- `check_parity` extended with a deposits section comparing computed aggregates (active principal total, month rollups, bank rollups, year tables) against the workbook's cached values.
- Not in this change: the `定期Info` A1:B5 scratch rate calc, deposit currency (the sheet tracks none), start dates (the sheet has none), deposit reordering, and the other unmigrated sections. The workbook remains source of truth for non-migrated sections.

## Capabilities

### New Capabilities
- `time-deposits`: recording, listing, editing and deleting 定期 deposit records (including label-less interest-only rows), the derived status/total/month fields, and the Upcoming and History aggregate views with the reminder checklists.

### Modified Capabilities
- `spreadsheet-trade-import`: the import command also imports the `定期Info` deposit list idempotently, and the parity command also compares deposit aggregates against the workbook's cached 定期/定期Info figures.

## Impact

- Backend: new migration `0003_deposits.sql`; new `routes/deposits.rs`; additions to `models.rs`, `calc.rs`, `xlsx.rs`, `import.rs`, `parity.rs`, and both binaries.
- Frontend: new `定期` tab and `DepositsView.vue` + `DepositForm.vue`/`DepositTable.vue`; `api.ts` types and endpoints; `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` gains the deposit data flow; `AGENTS.md` roadmap marks 定期 migrated.
- No new dependencies; `財富分析報告.xlsx` stays read-only and its formulas keep working.
- Status depends on the current date, so cached workbook aggregates are point-in-time: a deposit that matured since the sheet last recalculated will surface as an expected parity difference.
