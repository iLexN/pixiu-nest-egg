---
title: "Group utoipa_axum routes!() by path, not by handler"
author: ILexN
date: 2026-11-17
branch: main
id: l-2632b726
scope: repo
status: active
src: agent
touches:
  - GLOSSARY.md
  - backend/src/calc/suggestions.rs
  - backend/src/models/months.rs
  - backend/src/routes/mod.rs
  - backend/src/routes/months.rs
  - backend/src/store/months.rs
  - docs/database.md
  - docs/overview.md
  - frontend/src/api-types.ts
  - frontend/src/api.ts
---

## Decided
Keep one `utoipa_axum::routes!(…)` call per route path in `api_router` — each call may hold multiple handlers only when they share the path and differ by method (e.g. `routes!(months::show, months::upsert, months::remove)` all on `/api/months/{ym}`). Handlers on different paths each get their own `.routes(routes!(handler))` line.

## Tried and ruled out
Grouping `months::dismiss_item` + `months::undismiss_item` (`/items/dismiss` vs `/items/undismiss`, both POST) into one `routes!(…)` call compiles cleanly, then panics at router build: "Overlapping method route. Cannot add two method routes that both handle POST". Only `#[cfg(test)]` route tests (and startup) catch it — `cargo build` alone won't.

## Watch out
The panic message names the method, not the paths — looks like a duplicate handler but the real cause is mixing paths in one call.

In the code: `backend/src/routes/mod.rs` `api_router` (~line 90-110); `routes::tests::openapi_documents_every_api_operation` is the test that trips on it.
