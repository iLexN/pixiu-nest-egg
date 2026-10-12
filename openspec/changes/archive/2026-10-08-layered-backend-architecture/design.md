## Context

See proposal.md (Why) for the motivation. Current dependency shape:

```
  import_xlsx bin      check_parity bin
        |                    |
        v                    v
    import.rs            parity.rs ---- calls overview::overview (an axum handler),
        |  \                 |          summary::build, year_review::build, ...
        |   +-------+--------+
        v           v
  +-----------------------------------------------+
  | routes/*  = HTTP + SQL + row mapping + compose |
  |  overview -> year_review -> months -> yearly  |
  |      \            \           |         |     |
  |       +-> summary <-----------+---------+     |
  |  trades -> stocks   dividends -> trades       |
  +----------------------+------------------------+
                         |  (errors: ApiError, HTTP-shaped)
                         v
     calc.rs (pure, 6.9k lines) <-- models.rs (serde + utoipa types)
     mpf.rs / market_history.rs / prices.rs : SQL helpers outside routes
```

Constraints that shape the approach:

- Behaviour must be byte-for-byte identical: same JSON, same HTTP status per error path, same OpenAPI document, same side effects. Note that `GET /api/summary` writes `market_history` through `summary::build`, while `summary::market_value_only` deliberately does not.
- AGENTS.md rules still apply:
  - Handlers keep `#[utoipa::path]` and are registered via `utoipa_axum::routes!`.
  - Money stays `f64` and is not rounded.
  - The backend remains the source of truth for calculations.
- Tests are inline `#[cfg(test)]` modules next to the code they cover, and must move with the code.

## Goals / Non-Goals

**Goals:**

- A one-directional dependency graph: `routes -> service -> store -> (sqlx)` and `service -> calc`. `import`/`parity` sit beside `routes` as alternative entry points.
- Each layer has a rule that a grep can check (see Decisions 5).
- Do the work in small stages that each compile, pass the test suite and pass parity, so the refactor can stop safely between stages.

**Non-Goals:**

- No repository traits, ports/adapters, dependency-injection containers or mock stores. Tests keep using `db::connect_memory()`.
- No separation of domain entities from API DTOs. `models.rs` stays shared by all layers.
- No renaming of public API paths, JSON fields or DB tables and columns.
- No new crates or workspace split.
- No performance work. Query shapes move as they are, even where they look improvable; changing them would risk changing behaviour.

## Decisions

### 1. Horizontal layers over vertical feature slices

```
  backend/src/
    routes/<feature>.rs   HTTP adapters only
    service/<feature>.rs  multi-step / cross-feature composition
    store/<feature>.rs    all SQL, row mapping, column lists
    calc/<feature>.rs     pure validation + maths
    models.rs  error.rs  db.rs  xlsx.rs  import.rs  parity.rs  prices.rs
```

- **Why:** overview, year-in-review, month stat and parity each combine five or more features. With layers, those compositions have an obvious home (`service/`). With slices they would need a "composite" slice that reaches into the internals of every other slice.
- **Alternative:** vertical slices (`stocks/{routes,store,calc}.rs`). These map 1:1 to OpenSpec capabilities and keep each feature in one folder. They were rejected because the cross-feature compositions are where the complexity sits.
- **Mitigation for discoverability:** every layer uses the same feature file names, so "everything about deposits" is `*/deposits.rs`.

### 2. Concrete store functions, not repository traits

- Store modules expose plain `pub async fn load_all(pool: &SqlitePool) -> Result<Vec<Deposit>, StoreError>` style functions.
- **Why:** there is exactly one storage backend and tests already use an in-memory SQLite. A trait would add generic or `dyn` plumbing and async-trait friction for an implementation that never gets swapped.
- **Alternative:** `trait DepositRepo`, rejected as abstraction without a second implementation. If a second backend ever appears, the store boundary added here is exactly where a trait could be introduced later.

### 3. `StoreError` below the HTTP line

```rust
pub enum StoreError {
    Database(sqlx::Error),
    NotFound(String),  // e.g. "stock 42 not found"
    Corrupt(String),   // e.g. "stored trade type X is not valid"
}
impl From<StoreError> for ApiError { /* Database->Database, NotFound->NotFound, Corrupt->Conflict */ }
```

- **Why:** the data layer must not carry HTTP semantics, and `import`/`parity` can `?` it straight into `anyhow`.
- **Status codes are preserved:** today the "stored X is not valid" errors are `ApiError::Conflict` (409), so `Corrupt` maps to `Conflict`. Messages are carried through unchanged.
- **Validation stays where it is:** `Vec<FieldError>` stays in `calc` and becomes `ApiError::Validation` in routes, as it does today.
- Service functions return `Result<_, StoreError>`, or an `anyhow`-free service error if a service needs to raise validation errors. Settle this per function, preferring `StoreError` plus caller-side validation.

### 4. What counts as "service"

- A function belongs in `service/` when it loads from more than one store module, or runs store → calc → store (as with the side effect of `summary::build`).
- A single-table load-and-map stays in `store/`. A pure transform stays in `calc/`.
- Initial service inventory, taken from current cross-module calls:
  - `summary::{build, market_value_only}`
  - `yearly::{build, hk_sold_pl}`
  - `year_review::build`
  - `months::{live_totals_input, load_stat_rows_live, load_interest_events, goal_constants, semi_liquid_target}`
  - overview assembly (today inside the `overview::overview` handler)
  - `overview::ibkr_block`
  - `record_receipt_item`
  - `trades::resolve_stock`
  - the import insert helpers (`aia::insert_policy`, `bonds::insert_bond`/`insert_coupon`, `dividends::insert_dividend`)
- **`parity.rs` must stop calling the handler `overview::overview`.** The handler body moves to `service::overview::build`, and both the handler and parity call that.

### 5. Layer rules, checkable by grep

| Rule | Check |
|---|---|
| routes issue no SQL | no `sqlx::query` / `query_scalar` / `query_as` in `src/routes/` (outside `#[cfg(test)]` seeding helpers) |
| nothing depends on routes except `main.rs`/`lib.rs` | no `crate::routes` or `routes::` in `src/{service,store,calc,import,parity,prices}.rs`/dirs |
| `ApiError` stays at the HTTP edge | `ApiError` appears only in `src/routes/`, `src/error.rs`, and `src/prices.rs` (upload validation shared by the route and the `import_prices` bin, unchanged from before this change) |
| calc stays pure | no `sqlx`, `axum` or `SqlitePool` in `src/calc/` |
| store does not compose | `src/store/` does not import `crate::service`. It may import calc types it maps rows into (`*Facts`, `Validated*`) and the pure derived-field helpers that `row_to_*` mappers already used (e.g. `dividend_amount`, `bond_active`, `coupon_status`, `deposit_total`, `mpf_figures`) |

As built: `hk_sold_pl` is a single aggregate query, so it lives in `store::yearly`, not `service::yearly`.

Test-only SQL seeding inside route `#[cfg(test)]` modules is allowed. Those tests exercise handlers end to end against `connect_memory()`.

### 6. Existing standalone modules

- `mpf.rs` (history recording + `meta_*` helpers) → `store/mpf.rs` + `store/meta.rs`.
- `market_history.rs` → `store/market_history.rs`.
- `prices.rs` stays a top-level shared entry module (used by the route and by the `import_prices` bin), retargeted to `store::stocks`.
- `xlsx.rs` is workbook reading with no DB access and stays as it is.

### 7. Splitting `calc.rs`

- Follow the existing section boundaries:
  - trades/stocks + rollups (start to ~460)
  - deposits (~460-775)
  - dividends (~776-1030)
  - yearly (~1030-1168)
  - MPF (~1169-1425)
  - bonds (~1427-1592)
  - AIA (~1595-1870)
  - month stat (~1871-2261)
  - year review (~2262-2837)
  - live totals / liquidity / money master / goals (~2838-3187), which becomes `calc/overview.rs`
  - suggestions/interest (~3188-3439)
  - forecast (~3440-3680)
- Shared helpers (`FieldError`, `approx_eq`, `has_note`, date helpers) go in `calc/mod.rs` or `calc/common.rs`.
- `calc/mod.rs` re-exports every public item (`pub use stocks::*;` …), so existing `crate::calc::X` imports keep compiling during stage 1. Callers can later switch to explicit paths, but that is not required.
- The 142 tests are distributed to the module whose functions they cover.

## Risks / Trade-offs

- **[Silent behaviour drift while moving code]** → Move code verbatim, one feature per step. Before starting, capture a `check_parity` baseline and API snapshots, and diff them after every stage (see Migration Plan).
- **[Side effect lost or duplicated]** `summary::build` writes `market_history` today; `market_value_only` must not. → Keep two distinct service functions with the same names and doc comments. The existing `routes::summary` tests (`build_records_todays_totals`, `unpriced_market_records_nothing`, …) move with them and must still pass.
- **[Error status changes]** → `From<StoreError> for ApiError` maps exactly as described in Decision 3. Run the existing route tests that assert 404/409.
- **[`months.rs` is tangled (1.8k lines, nine responsibilities)]** → Do it last, as its own stage, after every dependency it composes already has a store/service home.
- **[Big diff, hard review]** → One commit per feature per stage. Each commit compiles and passes `cargo test`.
- **[More files to navigate]** → Accepted trade-off. Mitigated by uniform naming (Decision 1) and the updated AGENTS.md Layout section.
- **[Re-exports in `calc/mod.rs` hide the new structure]** → Accepted for stage 1 to keep the diff mechanical. Optional cleanup afterwards.

## Migration Plan

1. **Baseline.** Before any code change:
   - Copy `data/wealth.db` to a scratch path.
   - Run `WEALTH_DB=<scratch> cargo run -p wealth-backend --bin check_parity -- "財富分析報告.xlsx" > /tmp/parity-before.txt`.
   - Start a scratch backend on a spare `WEALTH_ADDR`.
   - Save the JSON of every GET endpoint listed in the OpenAPI document, plus `/api-docs/openapi.json` itself.
   - Never run any of this against `data/wealth.db` directly.
2. **Stage 1:** split `calc.rs` (Decision 7).
3. **Stage 2:** introduce `StoreError` and `store/`, feature by feature: stocks+trades, dividends, deposits, family, bonds, aia, mpf+meta+market_history, yearly snapshots, forecast, then months last.
4. **Stage 3:** introduce `service/` and thin the routes (Decision 4). Retarget `import.rs`/`parity.rs`, then delete the helpers in `routes/mod.rs` that have moved.
5. **After every stage:**
   - `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
   - Re-run parity and the GET snapshots on a fresh scratch copy (the GET summary writes history, so always copy from the same baseline file). Diff them against the baseline; the only expected difference is today's date-stamped history row, and it must be identical.
6. **Finish:**
   - Run the layer-rule greps (Decision 5).
   - Update the `docs/` module paths and the AGENTS.md Layout section.
   - Restart the real backend per AGENTS.md.

**Rollback:** each stage is a set of self-contained commits with no schema or data change, so `git revert` of the stage's commits is a complete rollback.

## Open Questions

- Whether service functions that need to raise validation errors (for example, parts of `months::upsert`) should return a small `ServiceError { Store, Validation }` or keep validation in the route. Either preserves behaviour; decide when that function is moved.
- Whether to replace the `calc/mod.rs` glob re-exports with explicit paths after stage 1. This is cosmetic and can be deferred.
