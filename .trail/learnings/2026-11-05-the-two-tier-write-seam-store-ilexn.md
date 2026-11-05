---
title: "The two-tier write seam: store owns single-domain txs, service owns domain workflows"
author: ILexN
date: 2026-11-05
branch: main
id: l-cda68aed
scope: repo
status: active
src: agent
touches:
  - backend/src/routes/aia.rs
  - backend/src/routes/bonds.rs
  - backend/src/routes/months.rs
  - backend/src/routes/mpf.rs
  - backend/src/routes/stocks.rs
  - backend/src/routes/year_review.rs
  - backend/src/service/mod.rs
  - backend/src/service/months.rs
  - backend/src/service/mpf.rs
  - backend/src/service/year_review.rs
---

## Decided
- Routes never open a transaction. Single-domain atomic ops (cascades like `delete_with_events`, batch writes like `reorder`) live in the store as pool-taking entry points next to `*_tx(conn)` variants — the `cash_accounts`/`next_sort_order` pattern. Multi-store and domain-rule workflows live in `service::*` behind `ServiceError`.
- Domain validation travels with the workflow (`ServiceError::Validation`); shape checks (parse, ranges, "at least one field") stay in routes.
- Implemented in change `service-seam-writes`: new `service::aia::{create_event,remove_event}`, `service::months::{upsert,create_item,update_item}`, `service::mpf::update`, `service::year_review::update` (which also replaced the route's `SqliteRow` decode with a typed `YearReviewRecord`).

## Tried and ruled out
- Routing cascades through service too (`service::bonds::remove` calling the store): a shallow pass-through — the depth the refactor exists to create, not to fake.

## Watch out
- `pool.begin()`, `sqlx::query*`, `try_get` must not appear in non-test `routes/` code; the 2026-11-05 learning about bypassed `service::deposits::create` is why.
- `merge_text`/`merge_num` PATCH helpers are duplicated privately in `service::{months,mpf}` — if a third service needs them, lift them out of `routes/patch.rs` into a shared home.
