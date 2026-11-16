# Design

## Context

Six page read models — `GET /api/deposits/summary`, `/api/bonds/summary`, `/api/family/deposits/summary`, `/api/dividends/summary`, `/api/aia/summary`, `/api/mpf` — are composed inside their route handlers. Each one loads rows, maps them to calc facts, applies the activity rule, sorts, and builds `history_years`; the same snippets are copied 3–5 times across `routes/`. ADR 0005 routes → service → store says routes are thin HTTP adapters: they parse input, call, and map errors.

## Goals / Non-Goals

**Goals:**
- One rule: a page read model is one service call returning `Result<_, StoreError>`.
- The service owns the response type; the route `pub use`-re-exports it (the `routes/summary.rs` + `service/summary.rs` pattern), keeping OpenAPI schema names and `api-types.ts` byte-identical.
- Kill the copy-pasted parsing: one `facts_of` per domain in store, one `calc::history_years`, one `parse_status`/`parse_received_at`/`finite_or` in routes.
- Byte-identical HTTP: same JSON, same status codes, same error messages.

**Non-goals:**
- Family/deposit unfork, overview blocks, workbook-reference cleanup, frontend sums.
- Trivial echo responses (`HolderNoteResponse`, `NoteResponse`) and `routes/months.rs` (already backed by MonthBook).
- Pass-through service fns for family writes — the family module is read-side only; writes stay store-direct per the two-tier seam.

## Decisions

- **Target interface** (one call per page):
  - `service::deposits::summary(pool) -> Result<DepositSummaryResponse, StoreError>`
  - `service::bonds::summary(pool) -> Result<BondSummaryResponse, StoreError>` — moves `BondTotals`, `BondWithCoupons`, `UpcomingCoupon` too
  - `service::family::summary(pool) -> Result<FamilyDepositSummary, StoreError>` — moves `FamilyHolderSummary`
  - `service::dividends::summary(pool, market: Option<Market>) -> Result<DividendSummaryResponse, StoreError>`
  - `service::aia::summary(pool) -> Result<AiaSummaryResponse, StoreError>` — moves `PolicyWithEvents`
  - `service::mpf::overview(pool) -> Result<MpfResponse, StoreError>`
- **`today()` stays internal**, consistent with `service::summary::build` — the services call `store::today()` themselves.
- **Stored-date parse in store `facts_of`.** `store::deposits` already has `rows_to_facts`; add `facts_of(&Deposit)` beside it, plus `facts_of(&FamilyDeposit)` in `store::family` and `facts_of(&Dividend)` in `store::dividends` — the `store::aia::facts_of` / `store::mpf::facts_of` shape. Each parses once and reports `StoreError::Corrupt` in the same message the route produced (`stored end_date {x} is not valid`, `stored pay_date {x} is not valid`); Corrupt already maps to 409. AIA's summary `parse_day` moves into `service::aia` returning `StoreError::Corrupt` (`stored date {x} is not valid`) and is shared with the event write path.
- **`calc::history_years(years, today) -> Vec<i32>`** — distinct input years plus today's year, sorted. Deposits/family feed it `end_year`s; dividends feed it parsed `pay_date` years. The dividends version previously swallowed a malformed `pay_date` into an empty list (`collect::<Option<Vec>>().unwrap_or_default()`); with `pay_date` parsed in store a bad date is now a 409, matching the other pages — the one intentional edge-case change.
- **Shared route parsers in `routes/mod.rs`**: `parse_status(raw, active, done)` (error text `status must be {active} or {done}`, field `status`), `parse_received_at(Option<&str>)` (defaults to today; `received_at must be YYYY-MM-DD`), `finite_or(field, Option<f64>, default)` (`{field} must be a number`) for `interest`/`credit_amount`. Used by the deposits/bonds/family/dividends list and receive handlers.
- **Family's already-received check lives in `store::family::mark_received`** returning `StoreError::Conflict("family deposit {id} is already received")` — the single-domain atomic op owns the guard, like deposits/bonds where the domain path checks. The route no longer pre-checks, so an already-received deposit submitted with a malformed `received_at` now answers 400 (parse first) rather than 409 — each error's own message is unchanged.
- **Tests move with the code.** `matured_unreceived_deposit_counts_until_received` → `service::deposits`; `matured_unreceived_bond_counts_until_received` → `service::bonds`; `matured_unreceived_deposit_counts_in_the_holders_active_principal` → `service::family`. Assertions unchanged; they call `service::x::summary(&pool)` and the service receive paths. New: `calc::history_years` unit test and a `store::family` test that `mark_received` on a received row returns `Conflict`. `routes/bonds.rs::early_received_bond_is_matured` stays — it exercises `store::bonds`, not the summary.
- **`backend/tests/api.rs` is the byte-identity guard** — untouched. `routes::tests::openapi_documents_every_api_operation` pins the path+method list unchanged.

## Risks / Trade-offs

- [Schema-name drift in OpenAPI if a moved struct changes name or utoipa picks up the module path] → Struct names are unchanged and `utoipa::ToSchema` names the schema after the type; the `pnpm gen:api` + `api-types.ts` diff check catches any drift.
- [Dividends malformed-`pay_date` behavior changes from empty `history_years` to 409] → Only reachable with corrupted DB data; consistent with the deposit/bond pages' Corrupt → 409 convention; called out in the proposal.
- [Family receive error precedence for the both-invalid case (already received + malformed `received_at`): 409 → 400] → Accepted: the parse must happen before the store call; each individual error is unchanged.

## Follow-up simplification (2026-11-16)

Two remaining copy-paste idioms got shared helpers in the same pass's spirit: `store::stored_date(column, raw)` now centralises the stored `YYYY-MM-DD` → `StoreError::Corrupt` parse, applied at every site in `store/{deposits,family,dividends}.rs` (each site keeps its existing column word, e.g. `date` vs `end_date` vs `pay_date`) and behind `service::aia::parse_day`; `routes::parse_order(Option<&str>)` centralises the `?order=desc` flag across the five list handlers. Scope stayed limited to the files this change touched — the other ~15 store copies of the date-parse idiom (bonds, trades, aia store, ibkr, mpf, service/forecast) are left for a separate follow-up.
