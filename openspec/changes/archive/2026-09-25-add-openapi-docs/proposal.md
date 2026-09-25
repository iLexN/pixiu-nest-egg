# Proposal

## Why

The backend exposes ~55 REST endpoints under `/api`, but the only documentation is prose (`docs/DATA_FLOW.md`, OpenSpec specs). There is no machine-readable contract and no way to try endpoints interactively — checking a request shape means reading handler source or the Vue call sites. Generating an OpenAPI spec from the same code that serves the routes keeps the contract honest, and mounting Scalar gives a local "try it" UI.

## What Changes

- Add `utoipa`, `utoipa-scalar`, and `utoipa-axum` dependencies to `backend/`.
- Annotate every handler in `backend/src/routes/` with `#[utoipa::path]` covering all ~55 endpoints.
- Derive `utoipa::ToSchema` on all request/response structs and enums in `backend/src/models.rs` and route-local types.
- Serve the generated spec at `/api-docs/openapi.json` and a Scalar playground UI at `/scalar` (same origin, loopback-only like the rest of the app).
- Document request bodies, path/query params, success responses, and error responses (`ApiError` shape) for every endpoint.
- No endpoint behavior changes; purely additive documentation surface.

## Capabilities

### New Capabilities

- `api-docs`: generated OpenAPI specification of the whole `/api` surface plus an interactive Scalar UI served by the backend.

### Modified Capabilities

<!-- None — no existing spec-level behavior changes. -->

## Impact

- **Code**: `backend/Cargo.toml` (3 new crates), `backend/src/models.rs` (derives), all 14 files under `backend/src/routes/` (path annotations), `backend/src/main.rs` or `routes/mod.rs` (doc + UI mounting).
- **Dependencies**: `utoipa`, `utoipa-axum`, `utoipa-scalar` — versions pinned at least 7 days old per repo convention.
- **Docs**: `docs/DATA_FLOW.md` and `AGENTS.md` gain a pointer to `/scalar` and `/api-docs/openapi.json`.
- **Risk**: annotations are additive; path params are compile-checked against handler signatures, so spec/route drift is caught at build time.
