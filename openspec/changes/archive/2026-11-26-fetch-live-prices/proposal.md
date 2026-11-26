# Proposal

## Why

Updating 現價 currently means hand-editing `current-price.json` and importing it — friction that was deliberate during the workbook migration (manual entry kept app figures directly comparable against the Google Sheet). The migration is complete and the workbook is retired, so that constraint is no longer needed: prices can be fetched from Yahoo Finance and applied in one click.

## What Changes

- New `POST /api/stocks/prices/fetch` endpoint that fetches the latest price for every stock in a given market from Yahoo Finance and applies it through the existing bulk price-update pipeline.
- HK stocks query Yahoo as `{ticker}.HK`; US stocks query as `code` with `.` converted to `-` (e.g. `BRK.B` → `BRK-B`).
- On a US fetch, the USD→HKD rate is also refreshed from `USDHKD=X` and stored unrounded.
- New `更新現價` action button in the 股票 group's second-level nav, shown after the 管理 sub-tab, acting on the currently selected 港股/美股 market with a full-screen blocking overlay while the fetch runs.
- **BREAKING (spec-level)**: the `stock-portfolio-summary` requirement "No external market-data service" is amended — the app now calls Yahoo Finance for price data. The migration-era manual-verification rationale is retired.

## Capabilities

### New Capabilities

### Modified Capabilities

- `stock-portfolio-summary`: prices may now also come from a Yahoo Finance fetch triggered from the UI; add the fetch endpoint, DB-driven symbol mapping, rounding, and failure reporting.
- `app-navigation`: the 股票 second-level row gains a `更新現價` action button after the 管理 sub-tab (an action, not a sub-tab).
- `fx-rates`: the stored USD rate gains a fetch-driven writer alongside manual entry; the unrounded-storage rule is unchanged.

## Impact

- `backend/src/prices.rs` — fetch orchestration, Yahoo symbol mapping, response parsing (extends the existing shared price module; `ApiError` layer rule).
- `backend/src/models/trades.rs` — `PriceReport` gains `failed` and `fx` fields (additive).
- `backend/src/routes/stocks.rs` + `routes/mod.rs` — new documented route; API operation list test updated.
- `backend/Cargo.toml` — adds `reqwest` (rustls) and tokio `time` feature; first outbound HTTP call in the app.
- `frontend/src/App.vue` — subnav button, blocking overlay, result message, view remount.
- `frontend/src/api.ts` + generated `api-types.ts` — new client call and report fields.
- `docs/stocks.md` — flow documentation.
