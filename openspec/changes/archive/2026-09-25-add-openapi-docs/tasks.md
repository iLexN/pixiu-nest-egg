# Tasks

## 1. Dependencies and schema derives

- [x] 1.1 Add `utoipa = "5.5"`, `utoipa-axum = "0.2"`, `utoipa-scalar = "0.3"` to `backend/Cargo.toml` (5.x/0.2/0.3 line per design — the 6.x line is under 7 days old) and verify `cargo build` succeeds
- [x] 1.2 Add `utoipa::ToSchema` to the derives of every serialized struct/enum in `backend/src/models.rs` and make `ErrorBody` in `backend/src/error.rs` `pub` + `ToSchema`; verify `cargo check` passes
- [x] 1.3 Add `ToSchema`/`IntoParams` derives to route-local request and query structs across `backend/src/routes/*.rs` (use `#[schema(value_type = …)]` or a local mirror for any field type that lacks `ToSchema`); verify `cargo check` passes

## 2. Handler path annotations

- [x] 2.1 Annotate all handlers in `stocks.rs`, `trades.rs`, `summary.rs`, and `yearly.rs` with `#[utoipa::path]` (method, full `/api/...` path, tag, params, request_body, success + error responses); verify `cargo check` passes
- [x] 2.2 Annotate all handlers in `deposits.rs`, `family.rs`, `dividends.rs`, and `bonds.rs`; verify `cargo check` passes
- [x] 2.3 Annotate all handlers in `mpf.rs`, `aia.rs`, `months.rs`, `overview.rs`, and `year_review.rs`; verify `cargo check` passes

## 3. Router conversion and serving

- [x] 3.1 Convert `api_router` in `backend/src/routes/mod.rs` to an `OpenApiRouter` using `.routes(routes!(…))` for every handler (one tag per domain module), `split_for_parts()` to return `(Router, OpenApi)` with a comment that routes must use `routes!` not `.route()`; verify `cargo build` passes
- [x] 3.2 In `main.rs`, merge the converted router at the app root (paths already carry `/api`), serve `api.to_json()` at `GET /api-docs/openapi.json`, and merge `Scalar::with_url("/scalar", api)`; verify `cargo run -p wealth-backend --bin wealth-backend` starts and `curl localhost:8787/api-docs/openapi.json` returns OpenAPI JSON listing every `/api` operation
- [x] 3.3 Verify `curl localhost:8787/scalar` returns the playground HTML and the loaded page lists all tagged operations in a browser

## 4. Verification and docs

- [x] 4.1 Add a backend test asserting the generated `OpenApi` document contains every expected `/api` path+method; verify `cargo test` passes
- [x] 4.2 Update `docs/DATA_FLOW.md` and `AGENTS.md` with the `/scalar` and `/api-docs/openapi.json` endpoints and the `routes!`-not-`.route()` convention; verify the documented commands work as written
- [x] 4.3 Run the repo verification suite (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`) and fix any failures
