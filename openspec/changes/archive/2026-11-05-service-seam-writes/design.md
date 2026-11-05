# Design

## Context

See proposal.md — Why. Inventory of the ten route-owned write sites:

| Handler | What the route owns | Kind |
|---|---|---|
| `year_review::update` | tx over two stores, `SqliteRow` decode, merge rules, delete-if-empty, snapshot cleanup | multi-store |
| `aia::create_event` | tx: read policy, snapshot prev, insert, apply payment | domain workflow |
| `aia::remove_event` | tx: undo the event's derived effects | domain workflow |
| `mpf::update` | tx: `update_account` + `record_history` iff values changed | domain rule |
| `months::create_item` | tx: `clear_dismissal` + `insert_item` | domain rule |
| `months::upsert` | field > recapture > stored precedence; `null`→NULL; live snapshot | domain rule |
| `months::update_item` | entertainment-flag rules across category transitions | domain rule |
| `aia::remove`, `bonds::remove`, `stocks::reorder` | tx wrapping one store call | single-store |

Precedent: `store::cash_accounts` already owns `pool.begin()` internally; `service::deposits`/`receipts` show the deep-fn shape (tx compose, `ServiceError` out, inline `connect_memory()` tests).

## Goals / Non-Goals

**Goals:**
- `pool.begin()` appears in zero route handlers.
- Each moved workflow is callable in-process — one way in, no bypass.
- Domain rules (merge precedence, dismissal clearing, history-on-change, apply/undo effects, delete-if-empty) live next to their inverses in service.
- Zero externally observable change.

**Non-goals:**
- Moving single-store cascades through service (would create shallow pass-throughs).
- Changing store fn signatures beyond the three new pool-taking variants and `year_review`'s typed record.
- Touching the already-finished seams (deposits, trades, ibkr, dividends, bonds receive).

## Decisions

- **Two-tier seam, not "everything through service".** A `service::bonds::remove` that only calls the store would be a shallow pass-through — the thing this refactor exists to remove. Single-domain atomic ops (the three cascades) become pool-taking store fns matching `cash_accounts`; everything with a domain rule or a second store goes to service.
- **Domain validation moves with the workflow** into `ServiceError::Validation` — the entertainment flag's outcome depends on the resolved category, and `months::upsert`'s finite checks stay route-side as shape validation. Shape checks (`parse_ym`, year range, "at least one field required") stay in routes.
- **`store::year_review` returns a typed record** (income, invested_adjustment, salary_raise, bond/deposit principal+interest — all `Option<f64>`) instead of a raw `SqliteRow` the route decodes per column. Required by the layer rule anyway; the move just exposes it.
- **`service::months::upsert` re-presents via `MonthBook`** (same module, no extra dependency) — replaces the current route-side book-open + `present_one`.
- **Interface is the test surface.** New service fns get `connect_memory()` tests for their rules; `tests/api.rs` stays as the HTTP seam coverage.

## Risks / Trade-offs

- [Larger blast radius — ten call sites across six route files] → Each is mechanical; the api.rs suite pins observable behavior end-to-end.
- [Validation split between route (shape) and service (domain) blurs the line] → The split is by content, not location: if the check needs the merge result, it's domain.
