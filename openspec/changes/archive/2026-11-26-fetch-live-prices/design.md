# Design

## Context

`POST /api/stocks/prices` already accepts a `{"stocks":[{"symbol","price"}]}` body and applies it through `prices::apply` (resolve → transactional `set_price` + `price_updated_at` → `record_market` → `PriceReport`). `prices.rs` is the crate-root module the layer rules single out as allowed to return `ApiError` outside `routes/`. The `fx_rates` table stores one USD→HKD rate, unrounded. The backend today makes no outbound network calls and has no HTTP client dependency.

The spec's "No external market-data service" requirement was a migration-era constraint: while the workbook was being verified, manual entry kept app figures directly comparable against the Google Sheet. That verification is done, so the constraint is retired rather than worked around.

## Goals / Non-Goals

**Goals:**
- One click in the UI replaces the hand-edited `current-price.json` round-trip for the selected market.
- Reuse the existing resolve/apply/report pipeline so fetched and file-imported prices behave identically once applied.
- Per-symbol failures are reported, never fatal to the rest of the batch.
- Fetch calls Yahoo's public chart endpoint (`query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range=1d`) with a browser `User-Agent` (Yahoo 429s without one), sequential pacing between requests, and a `chart.result[0].meta.regularMarketPrice` → `previousClose` fallback.

**Non-Goals:**
- No scheduled/automatic refresh — fetch is user-triggered only.
- No changes to `current-price.json`, `POST /api/stocks/prices`, or the 匯入現價 JSON button; the manual path stays intact (it may be retired later as a separate change).
- No new stocks table columns, no symbol-mapping table — mapping is derived.
- No FX fetch on the HK click.

## Decisions

- **Fetch lives in `prices.rs`, not a new module or `service/`.** The layer rules permit `ApiError` only in `routes/`, `error.rs`, and `prices.rs`; the fetch orchestration (load stocks → HTTP → build entries → `apply`) returns `ApiError` and is the same shared entry-point shape as `apply`. Pure helpers (`yahoo_symbol`, `parse_chart_price`, rounding) are unit-testable without network.
- **Symbols are derived from the `stocks` table, not a hardcoded list.** HK: `{ticker}.HK` (stocks with no `ticker` are reported, not fetched). US: `code` with `.` → `-`, the exact inverse of `prices::resolve`'s `-` → `.` normalization, so fetched results re-resolve to the stored stock through the same path as file entries. Adding/removing a stock needs no config change.
- **Endpoint: `POST /api/stocks/prices/fetch` with `{"market": "HK"|"US"}`.** Market-scoped so the HK click never touches US prices and vice versa; `Market` already deserializes `"HK"`/`"US"`. A fetch-all-markets variant was rejected — the UI is always market-scoped and the FX rule differs per market.
- **`PriceReport` gains `failed` and `fx` (additive).** Fetch failures (HTTP error, timeout, missing price) don't fit `invalid` (malformed input) or `unmatched` (resolved to no stock), so a separate `failed: Vec<InvalidPriceEntry>` keeps semantics clean; `fx: Option<f64>` reports the applied USD rate on US fetches.
- **Rounding at the boundary: HK 3dp, US 2dp, FX raw.** User-chosen display-grade precision for stock prices; FX stays unrounded per the fx-rates spec ("Rates SHALL NOT be rounded"). Rounding before `apply` keeps stored values consistent with what a hand-edited file would carry.
- **`reqwest` with rustls, sequential loop with ~200ms `tokio::time::sleep`, ~10s client timeout.** The pacing (150–300ms per request) keeps the batch from tripping Yahoo's rate limit; sequential keeps the code trivially simple at ~19 symbols max. No connection pooling/config needed beyond a `User-Agent`.
- **UI: subnav action button + blocking overlay, not an in-page control.** The button sits after 管理 in the 股票 second-level row (before 轉入記錄 for US) acting on `market`. The overlay blocks all input for the fetch's duration (several seconds of paced requests), then the report message shows inline and the view remounts via a key bump so fresh prices render.

## Risks / Trade-offs

- Yahoo rate-limits or changes the endpoint → per-symbol failures surface in `failed`; worst case the report shows all-failed and nothing is applied (transactional apply only writes resolved entries). Manual JSON path remains as fallback.
- Mid-loop failure leaves a partial update → acceptable: applied prices are still valid, and `not_updated`/`failed` report the rest. Full rollback would discard good data.
- HK stock with no `ticker` can't map to a Yahoo symbol → reported as failed/unresolved; the user fixes the ticker in 管理.
- Blocking overlay freezes the whole app for ~3-5s → chosen deliberately so no stale-view actions race the apply; cheaper than making every view fetch-aware.
- First outbound network dependency → scoped to one endpoint, loopback-only server, no credentials involved.
