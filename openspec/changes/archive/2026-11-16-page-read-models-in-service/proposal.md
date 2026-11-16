# Proposal

## Why

The migration went one sheet at a time, and each page's read model was written straight into its route handler, copying the previous page. So the routes hold domain composition — load rows, map them to calc facts, apply "active = not yet 收訖", sort, build `history_years` — which ADR 0005 says routes must not do, and the same snippets are now copied 3–5 times. `service::summary`, `service::year_review` and `service::yearly` already follow the target shape (the service owns the response type; the route re-exports it and makes one call).

## What Changes

- **Page read models move to `service/`**: `service::deposits::summary`, `service::bonds::summary`, `service::family::summary` (new read-side-only module), `service::dividends::summary` (takes `market`), `service::aia::summary`, `service::mpf::overview`. Each returns `Result<_, StoreError>`; the response structs move with identical names/fields/docs/serde attrs and are `pub use`-re-exported from the route modules, so OpenAPI schemas and `api-types.ts` don't change.
- **Dedupe the copied snippets**: stored-date re-parse moves into store `facts_of` helpers returning `StoreError::Corrupt` (`stored end_date {x} is not valid` / `stored pay_date {x} is not valid` / `stored date {x} is not valid` — same messages, Corrupt already maps to 409); `calc::history_years` (distinct years + current year, sorted); `routes::parse_status`, `routes::parse_received_at`, `routes::finite_or` for the query/body parsing every receive/list handler re-implements.
- **Family receive guard deepens**: the "already received → 409" check moves from `routes::family::receive` into `store::family::mark_received` (`StoreError::Conflict`, same message) — a single-domain atomic op per ADR 0005, matching deposits/bonds where the domain path owns the check.
- **Tests move with the code**: the route unit tests that only exercise a summary move into the service modules and call `service::x::summary(&pool)`; new `calc::history_years` unit test and a store test for family `mark_received` conflict. `backend/tests/api.rs` is untouched — it is the HTTP-level byte-identity guard.

One intentional edge-case change: a dividend whose stored `pay_date` is malformed now 409s the whole summary (the `end_date`/`pay_date` Corrupt convention) instead of silently returning an empty `history_years` — only reachable with corrupted DB data.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no requirement changes; `skip_specs: true` is set. The dividends `history_years` edge above is not covered by any requirement in `openspec/specs/stock-dividends`.

## Impact

- `backend/src/service/{deposits,bonds,dividends,aia,mpf}.rs`, new `backend/src/service/family.rs`, `backend/src/service/mod.rs`
- `backend/src/routes/{deposits,bonds,family,dividends,aia,mpf,mod}.rs`
- `backend/src/store/{deposits,family,dividends}.rs`, `backend/src/calc/mod.rs`
- `docs/{deposits,investments,DATA_FLOW}.md`, `docs/adr/0005-layered-backend-seams.md`, `GLOSSARY.md`
