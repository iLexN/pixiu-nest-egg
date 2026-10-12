## Why

The backend has no layer between HTTP and SQLite. `backend/src/routes/` has become the de-facto service and data-access layer for the whole crate, including the CLI tools, so every new capability adds more cross-module coupling and makes schema changes and refactors harder. The pure `calc.rs` core is the one healthy boundary, but at ~6.9k lines with 142 tests in one block it is becoming hard to navigate. Fixing this now, before more workbook sections (回報率 projections, 開心Pool, 香港年金, FIRE) are migrated, keeps each future capability cheap to add.

Observed problems (as of 2026-10-06):

- **CLI tools depend on HTTP modules.** `parity.rs` calls 12 `routes::` functions, including the axum handler `overview::overview`, plus `summary::build`, `year_review::build`, `months::live_totals_input`, `bonds::load_all_bonds`, `aia::facts_of`, and others. `import.rs` calls 7, such as `aia::insert_policy`, `bonds::insert_bond`, `dividends::insert_dividend` and `months::load_assets`.
- **Routes call routes.** Composition chains such as `overview → year_review::build → months::load_stat_rows_live → yearly::build → summary::build → stocks::load_stocks` live inside HTTP modules.
- **SQL for one table is scattered.** For example, `stocks` is queried in 9 files and `trades` in 9. Row mappers and column lists (`row_to_*`, `*_COLUMNS`, `*_SELECT`) sit in `routes/mod.rs`.
- **The data layer speaks HTTP.** Loading and mapping functions return `ApiError`, which carries HTTP status codes. `import.rs`/`parity.rs` convert it back to `anyhow`.
- **One giant calc module.** `calc.rs` mixes validation and maths for every capability.

Design choice recorded here: a full DDD / Clean Architecture setup (repository traits, ports and adapters, entity/DTO split) was considered and **rejected** as overkill. This is a single-user, loopback-only app with one storage backend (SQLite) that will not be swapped, and its domain is mostly spreadsheet-formula calculations rather than complex invariants. Vertical feature slices were also considered and rejected in favour of **horizontal layers**, because the most complex code (overview, year-in-review, month stat, parity) composes many features and needs a natural home above them. See `design.md`.

## What Changes

- Split `backend/src/calc.rs` into `backend/src/calc/<feature>.rs` modules (stocks/trades, deposits, dividends, mpf, bonds, aia, month stat, year review, overview, suggestions, forecast). Each module keeps its own tests. The functions stay pure.
- Introduce `backend/src/store/<feature>.rs`: the only place that issues SQL. It owns row mapping, column lists and a storage error type (`StoreError`) that carries no HTTP semantics.
- Introduce `backend/src/service/<feature>.rs` for compositions that combine store and calc: today's `summary::build`, `yearly::build`, `year_review::build`, `months::live_totals_input`, the overview assembly, and the import insert helpers.
- Reduce `backend/src/routes/<feature>.rs` to thin HTTP adapters: extract input, call service/store/calc, map errors to `ApiError`, return JSON. The `#[utoipa::path]` annotations and `api_router` registration are unchanged.
- `import.rs` and `parity.rs` depend on `service`/`store` and never on `routes`.
- No API, response-shape, database-schema, validation, calculation or frontend change. HTTP status codes for every error path stay the same.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a behaviour-preserving refactor of the backend's internal structure. No requirement in `openspec/specs/` changes, so `skip_specs: true` is set in `.openspec.yaml`, following the precedent of `row-action-menus`, `collapsible-add-forms` and `inline-stock-row-editing`.

## Impact

- **Code:**
  - `backend/src/calc.rs` becomes the `calc/` directory.
  - New `backend/src/store/` and `backend/src/service/`.
  - Every `backend/src/routes/*.rs` changes, plus `routes/mod.rs` (helpers move out).
  - `backend/src/import.rs` and `backend/src/parity.rs` (imports retargeted).
  - `backend/src/error.rs` (`From<StoreError> for ApiError`).
  - Possibly `backend/src/mpf.rs`, `market_history.rs` and `prices.rs` (they move into store/service).
- **Docs:**
  - Module-path references in `docs/overview.md`, `docs/stocks.md` and `docs/DATA_FLOW.md`.
  - The `Layout` section of `AGENTS.md`, which describes where handlers and calculations live.
- **Unaffected:** HTTP API and OpenAPI document (`routes::tests::openapi_documents_every_api_operation` must pass unchanged), migrations and schema, the frontend, the workbook import/parity CLI interfaces, and `data/wealth.db` contents.
- **Dependencies:** none added.
