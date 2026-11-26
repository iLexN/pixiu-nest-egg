# Tasks

## 1. Service read model

- [x] 1.1 Widen `summarize_market` visibility in `backend/src/service/summary.rs` (`pub(crate)` or a thin public wrapper) so `service::export` can call it.
- [x] 1.2 Create `backend/src/service/export.rs` with `AnalysisExport` + section structs (`Serialize + ToSchema`) and `pub async fn build(pool: &SqlitePool) -> Result<AnalysisExport, StoreError>` that calls `overview::build`, `forecast::build`, and `summarize_market` for both markets, then reshapes per design.md's mapping table.
- [x] 1.3 Build `rigid_expenses` per month: `cells[].plan_items` of kinds Tax/Bill/Other as `{category, item, amount_hkd}`, plus a synthesized `{category:"Bill", item:"差餉", amount_hkd:-bill}` when `ForecastMonth.bill` is present.
- [x] 1.4 Register `pub mod export;` in `backend/src/service/mod.rs`.
- [x] 1.5 Add inline `#[cfg(test)]` tests: seeded pool → snapshot sections populated; Tax/Bill/Other items appear as rigid expenses with note/kind rules; quarterly bill synthesized in a Jan month; `sc_deposit` item excluded; inactive stock excluded; `dividend_yield_pct` null when no cost.

## 2. Endpoint

- [x] 2.1 Create `backend/src/routes/export.rs`: `#[utoipa::path]` `GET /api/export/analysis` handler returning the JSON body with `Content-Disposition: attachment; filename="wealth-export-<today>.json"`.
- [x] 2.2 Register the route in `api_router` via `utoipa_axum::routes!` and declare the module in `backend/src/routes/mod.rs`.
- [x] 2.3 Update `routes::tests::openapi_documents_every_api_operation` with the new path+method.

## 3. Frontend

- [x] 3.1 In `frontend/src/App.vue`, add `links: [{ label: 'Export', href: '/api/export/analysis' }]` to the `overview` nav group (renders as `tab-link` after `HappyPool`).

## 4. Docs

- [x] 4.1 Add the export endpoint to `docs/DATA_FLOW.md` (or the matching topic file `docs/overview.md`).

## 5. Verification

- [x] 5.1 `cargo test`
- [x] 5.2 `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
- [x] 5.3 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`
- [x] 5.4 Restart the backend (`lsof -ti :8787 | xargs kill`; detached `nohup cargo run -p wealth-backend --bin wealth-backend`) and confirm `curl 127.0.0.1:8787/api/export/analysis` returns the attachment with the expected shape; click the `Export` link and verify the download.

## Workflow follow-up

- Run `pnpm gen:api` if the generated types are needed later; not required for the plain download link.
- `trail check` before committing; `openspec archive add-analysis-export` (sync delta specs into `openspec/specs/`) once implemented and verified.
