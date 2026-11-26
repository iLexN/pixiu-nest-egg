# Tasks

## 1. Service

- [x] 1.1 Drop `category` from `RigidExpense` in `backend/src/service/export.rs`; detect stored Bill items by scanning the month's Bill-kind cells (non-empty `plan_items`) instead of matching on collected expenses.
- [x] 1.2 Update the export tests: assert the `{item, amount_hkd}` shape, keep the kind-filter / synthesis / no-double-count coverage.

## 2. Generated types and docs

- [x] 2.1 Run `cd frontend && pnpm gen:api` so `api-types.ts` reflects the dropped field.
- [x] 2.2 Update `docs/overview.md` and `GLOSSARY.md` wording (entries no longer carry `category`).

## 3. Verification

- [x] 3.1 `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm exec vue-tsc --noEmit`; restart the backend and confirm `rigid_expenses` entries have no `category` on the live endpoint.
