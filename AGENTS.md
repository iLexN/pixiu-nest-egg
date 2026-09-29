# Wealth report stock tracker

Local Rust + SQLite + Vue app for migrating the stock-trade sections of `財富分析報告.xlsx` out of the spreadsheet. The workbook remains the source of truth for sections that have not been migrated yet.

## Documentation map

- `openspec/specs/` — normative behavior: what the app SHALL do, per capability. Authoritative for "is this correct?" questions; update it through an OpenSpec change, not by editing specs directly.
- `docs/DATA_FLOW.md` — mechanics index for human readers: UI action → API → table → calculation. The detail lives in topic files: `docs/stocks.md`, `docs/deposits.md`, `docs/investments.md`, `docs/overview.md`, `docs/import-parity.md`, `docs/database.md`, plus `docs/MONTH_STAT_OVERVIEW.md` (workbook analysis).
- `openspec/changes/archive/` — archived designs, delta specs, and checklists per migration step.
- `http://127.0.0.1:8787/scalar` (running backend) — live OpenAPI playground; the fastest way to inspect the API surface.
- This file — AI work rules only. Do not add behavior descriptions or flow narratives here; they belong in specs or docs.

## Change workflow

- Behavior changes (new capability, changed rule, new endpoint/table) go through OpenSpec: `openspec propose` creates `openspec/changes/<name>/` with proposal, design, delta specs, and tasks; `openspec apply` implements the tasks; `openspec archive` merges the delta specs into `openspec/specs/`. The `.devin/skills/openspec-*` skills wrap these steps.
- Bug fixes and refactors that keep specified behavior unchanged skip OpenSpec, but still update `docs/` if wiring changed.

## Layout

- `backend/` — axum HTTP API, SQLite persistence via sqlx, workbook importer, parity checker. Route handlers live in `backend/src/routes/<feature>.rs`; calculations in `calc.rs`; workbook reading in `xlsx.rs`/`import.rs`.
- `backend/migrations/` — numbered sqlx migrations, applied automatically on startup.
- `frontend/` — Vue 3 + TypeScript + Vite UI. Pages in `src/views/`, shared widgets in `src/components/`, API client in `src/api.ts`, display formatting in `src/format.ts`.
- `data/wealth.db` — default local SQLite database with real personal financial data. `data/` and `財富分析報告.xlsx` are gitignored; a fresh clone must be given the workbook manually before import/parity can run.

## Run locally

Backend, serving the built frontend and API on `http://127.0.0.1:8787`:

```sh
cd frontend && pnpm install && pnpm build
cd ..
cargo run -p wealth-backend --bin wealth-backend
```

Development mode with Vite on `http://127.0.0.1:5173` and `/api` proxied to the backend:

```sh
cargo run -p wealth-backend --bin wealth-backend
cd frontend && pnpm dev
```

Environment variables:

- `WEALTH_DB=/path/to/wealth.db` — override the database path.
- `WEALTH_ADDR=127.0.0.1:8787` — override the backend bind address; keep it loopback-only.

Never run experiments, seeded servers, or throwaway imports against `data/wealth.db`. Use a scratch database, e.g. `WEALTH_DB=/tmp/wealth-test.db cargo run ...`, and a spare port via `WEALTH_ADDR` if the real backend is already running.

## Import and parity check

```sh
cargo run -p wealth-backend --bin import_xlsx -- "財富分析報告.xlsx"      # workbook → database, idempotent
cargo run -p wealth-backend --bin import_prices -- current-price.json     # bulk 現價 update
cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx"     # database summary vs workbook cached values
```

All honor `WEALTH_DB` and open the workbook read-only. Formats, ticker mapping, seeding heuristics, and what a re-import may and may not overwrite are specified in `docs/import-parity.md` and `openspec/specs/spreadsheet-trade-import/`.

## Verification

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cd frontend && pnpm build && pnpm exec vue-tsc --noEmit
```

Backend tests are inline `#[cfg(test)]` modules next to the code they cover. The frontend has no unit tests; `vue-tsc` and the build are the checks.

## Working rules

- When asking the user questions, ask one focused question at a time and wait for the answer — never batch a list of questions.
- When a task changes backend or frontend-built code, restart the running backend afterwards so http://127.0.0.1:8787/ reflects it: kill the `wealth-backend` process (`lsof -ti :8787 | xargs kill`), then `cargo run -p wealth-backend --bin wealth-backend` in the background, and confirm the API answers. The user verifies against the live `data/wealth.db` — this restart is expected, not an "experiment" on the live DB.
- The backend is the source of truth for validation, persistence, and all financial calculations. The frontend collects input, renders API responses, and formats values for display; temporary previews are never authoritative.
- Handlers carry `#[utoipa::path]` annotations and are registered in `api_router` via `utoipa_axum::routes!` — never `.route()`, which compiles but skips documentation. `routes::tests::openapi_documents_every_api_operation` pins the exact path+method list; update it when adding or removing endpoints.
- Schema changes are a new `backend/migrations/NNNN_name.sql`. Never edit an existing migration — every local database has already applied it. Update `docs/database.md` in the same change.
- Money and prices are stored and calculated as SQLite `REAL`/`f64`, matching the spreadsheet. Do not round stored values. Dates are `TEXT` in `YYYY-MM-DD`. Display formatting only: money 2 decimal places, prices/averages up to 4, percentages 3.
- Frontend form inputs declared `type="number"` (or bound with `v-model.number`) store numbers, not strings — coerce with `Number(...)`/`String(...)` instead of calling `.trim()` on them.
- Spreadsheet-compatible behavior that is intentional — do not "fix" it:
  - HK trade input derives `fee = buy total − 股數 × 單價`; US input derives `buy total = 股數 × 單價 + fee`.
  - `加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數` divides by shares **bought**, not shares held; a SELL never reduces 總買入成本.
  - Stocks with no 現價, zero holdings, or no derived market value are excluded from market-value totals; the summary lists them in `totals.excluded_codes`.
- Import and parity never write to the workbook. Import writes settings and seeded marks once and never overwrites them on re-import.
- When behavior or wiring changes, update the specs and/or `docs/` — not this file.

## Migration status

Migrated capabilities are exactly the directories in `openspec/specs/`. Not yet migrated and still maintained by hand in the workbook: the `YYYY回報率` projection sheets, the 開心Pool independent ledger, 香港年金, FIRE, ref1.
