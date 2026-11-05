# Horizontal backend layers and the two-tier write seam

The backend is horizontally layered — `routes/` (thin HTTP adapters, no SQL) → `service/` (multi-store and domain-rule composition) → `store/` (all SQL, returns `StoreError`), with `calc/` pure (no sqlx/axum) — chosen over vertical feature slices so each layer has a grep-checkable rule. Writes use a **two-tier seam**: single-domain atomic operations (cascades, batch writes) live in `store/` as pool-taking entry points next to `*_tx` variants; multi-store workflows and domain rules (merge precedence, dismissal clearing, apply/undo, history-on-change) live in `service/` behind `ServiceError`. Routes never call `pool.begin()`. Domain validation travels with the workflow; shape validation (parse, ranges, "at least one field") stays in routes.

### Considered options
- "Everything through service" — rejected: a `service::bonds::remove` that only calls the store is a shallow pass-through, the exact thing the refactor existed to remove.
- Repository traits / DI / ports-adapters — rejected: non-goals; tests use `db::connect_memory()`.
- Vertical feature slices — rejected: rules can't be grep-enforced across slices.

Source: `openspec/changes/archive/2026-10-08-layered-backend-architecture`, `2026-11-05-service-seam-writes`.
