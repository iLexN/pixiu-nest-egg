# Design

## Context

`api_router()` in `backend/src/routes/mod.rs` registers ~55 axum routes (14 domain modules) under `/api`. Handlers already use typed extractors and responses (`Json<T>`, `Query<T>`, `Path<…>`), and ~100 serde types exist in `models.rs` plus route-local request/query structs. Errors funnel through `ApiError` (`error.rs`), which serializes `{error, message, fields?}` as 400/404/409/500. CORS is already permissive and the listener is loopback by default.

## Goals / Non-Goals

**Goals:**

- One declaration produces both the axum router and the OpenAPI doc — impossible for the doc to miss an endpoint.
- Scalar playground at `/scalar`; spec JSON at `/api-docs/openapi.json`.
- Request/response schemas derived from the real Rust types.

**Non-Goals:**

- No auth, versioning, or public-hosting concerns (loopback personal app).
- No frontend changes beyond a docs pointer in `docs/DATA_FLOW.md`/`AGENTS.md`.
- No behavioral changes to any endpoint.

## Decisions

### utoipa-axum `OpenApiRouter` instead of plain utoipa

Convert `api_router` to build an `OpenApiRouter` where each `.route(path, get(h).post(h2))` becomes `.routes(routes!(h, h2))` and each handler carries `#[utoipa::path(method, path, tag, params, request_body, responses)]`. `split_for_parts()` yields `(Router, OpenApi)`.

- **Why**: the route table itself generates the doc. With plain utoipa you maintain `api_router` *and* a `paths(...)` list in `ApiDoc` — two declarations of the same 55 endpoints that will drift.
- **Alternative** (`aide`): generates docs from the router too, but requires `schemars` derives on every type plus rewriting handler registration style more invasively; utoipa is the more common choice and its `ToSchema` derives are additive one-word edits.
- **Path prefix**: declare full `/api/...` paths in the `#[utoipa::path]` macros and merge the resulting `Router` at the app root instead of `.nest("/api", …)` — nested routers record unprefixed paths in the doc.

### utoipa-scalar for the playground

`Scalar::with_url("/scalar", api)` merges onto the app and serves an embedded UI (no runtime CDN calls) that loads the spec URL. A tiny handler serves `api.to_json()` at `GET /api-docs/openapi.json`.

- **Why Scalar over Swagger UI**: chosen by the user; `utoipa-scalar` is first-party alongside utoipa-axum.

### Dependency pinning: the 5.x line, not 6.x

`utoipa = "5.5"`, `utoipa-axum = "0.2"`, `utoipa-scalar = "0.3"`. The whole 6.0 line (`utoipa` 6.0.0, `utoipa-axum` 0.3.0, `utoipa-scalar` 0.4.0) was published 2026-09-22 — three days ago, violating the repo's ≥7-day dependency-age rule. The 5.x/0.2/0.3 releases (Jan 2025 – May 2026) all declare `axum ^0.8` / `utoipa ^5` compatibility.

### Annotation conventions

- `#[derive(utoipa::ToSchema)]` added to every serialized type in `models.rs` and to route-local request structs; `#[derive(utoipa::IntoParams)]` on query structs (`ListQuery`-style).
- One tag per domain module: `stocks`, `trades`, `summary`, `deposits`, `family`, `dividends`, `bonds`, `mpf`, `aia`, `months`, `overview`, `year-review`.
- Error responses documented via a shared schema matching `ErrorBody` (`error`/`message`/`fields`) — make it `pub` + `ToSchema` — listed as 400/404/409 on operations that produce them.
- Handlers returning `StatusCode`-only or `(StatusCode, Json<T>)` bodies document the JSON part; the macro does not type-check `responses` against the handler return, so per-endpoint review keeps status codes honest.

## Risks / Trade-offs

- **`responses` declarations can drift from handler return types** (utoipa checks path params against the signature but not the declared response bodies) → derive schemas from the real types; add a smoke test asserting the doc parses and contains all expected paths.
- **A future `.route()` call on the `OpenApiRouter` silently skips documentation** → leave a comment at `api_router` and a note in `AGENTS.md`: routes go through `routes!`, never `.route()`.
- **Exotic field types** (`serde_json::Value`, nested maps like per-year pool rates) may not implement `ToSchema` → resolve per-field with `#[schema(value_type = …)]` or a local mirror struct during implementation.
- **~100 derive additions + ~55 path macros is a large diff** → mechanical, compile-enforced; `cargo clippy --all-targets -- -D warnings` and the existing test suite gate regressions.

## Migration Plan

Purely additive: new deps, derives, annotations, two new routes. Rollback = revert the change; the database and all existing endpoints are untouched. No data migration.
