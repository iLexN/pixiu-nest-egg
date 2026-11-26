# Tasks

## 1. Backend fetch

- [x] 1.1 Add `reqwest` (`default-features = false`, `features = ["json", "rustls-tls"]`, pinned `0.12.x` ≥7 days old) and tokio `time` feature to `backend/Cargo.toml`; verify `cargo build` succeeds
- [x] 1.2 In `backend/src/prices.rs` add `yahoo_symbol` (HK `{ticker}.HK`, US `code` `.`→`-`) and `parse_chart_price` (`regularMarketPrice` → `previousClose`) with inline unit tests; verify `cargo test prices` passes
- [x] 1.3 Add `fetch_market` to `prices.rs`: sequential Yahoo requests (~200ms `tokio::time::sleep`, ~10s timeout, browser `User-Agent`), round HK 3dp / US 2dp, collect per-symbol failures, call `apply`, merge `failed`/`fx` into the report; verify `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass
- [x] 1.4 On US fetch also fetch `USDHKD=X` and store it raw via `store::fx::set`; failure reported in `failed`, stored rate unchanged

## 2. Endpoint + models

- [x] 2.1 Extend `PriceReport` in `backend/src/models/trades.rs` with `failed: Vec<InvalidPriceEntry>` and `fx: Option<f64>` (additive)
- [x] 2.2 Add `fetch_prices` route (`POST /api/stocks/prices/fetch`, body `{"market": "HK"|"US"}`) with `#[utoipa::path]` in `routes/stocks.rs`, register via its own `routes!` call in `routes/mod.rs`, and add `("/api/stocks/prices/fetch", "post")` to the `openapi_documents_every_api_operation` expected list; verify `cargo test` passes
- [x] 2.3 Run `cd frontend && pnpm gen:api` and confirm `api-types.ts` picks up the new report fields

## 3. Frontend

- [x] 3.1 Add `fetchPrices(market)` to `frontend/src/api.ts` calling the new endpoint; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 3.2 In `frontend/src/App.vue` render the `更新現價` button after the 管理 tab in the 股票 nav row, a `抓取中…` blocking overlay, the inline result message, and `<main :key>` remount on success; verify `pnpm build` passes

## 4. Docs + verification

- [x] 4.1 Update `docs/stocks.md` (and `docs/DATA_FLOW.md` index if it lists endpoints) for the fetch flow
- [x] 4.2 Restart the backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, detached `nohup cargo run -p wealth-backend --bin wealth-backend`), click 更新現價 on 港股 and 美股, verify `manual_price`/`price_updated_at` and `fx_rates` updated in `data/wealth.db`

## Workflow follow-up

- Archive the change (`openspec archive`), syncing delta specs into `openspec/specs/` automatically.
- Verify the archived result.
