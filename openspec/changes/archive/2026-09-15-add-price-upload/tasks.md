## 1. Shared price-import logic

- [x] 1.1 Add `PriceEntry { symbol, price }` and `PriceReport { updated, unmatched, invalid, not_updated }` models in `backend/src/models.rs` (Deserialize/Serialize), verify `cargo check` passes
- [x] 1.2 Create `backend/src/prices.rs` with `parse_entries(&str) -> Result<Vec<PriceEntry>>` (rejects malformed JSON / missing `stocks` array) and `apply(pool, entries) -> Result<PriceReport>`; symbol resolution: `.HK` → HK `ticker`, else US `code`/`ticker` with `-`≡`.` normalization; updates run in one transaction; verify with unit tests via `db::connect_memory()`

## 2. API endpoint

- [x] 2.1 Add `POST /api/stocks/prices` route + handler returning `200` with `PriceReport` (400 on malformed body), wire it in `routes/mod.rs`; verify with a curl POST against a running server
- [x] 2.2 Add `backend/tests/api.rs` cases covering the spec scenarios (HK ticker match, `BRK-B`→`BRK.B`, unmatched, invalid, not-updated, malformed body); verify `cargo test` passes

## 3. CLI command

- [x] 3.1 Create `backend/src/bin/import_prices.rs` (`cargo run -p wealth-backend --bin import_prices -- current-price.json`, honors `WEALTH_DB`, prints the report); verify by running it against `current-price.json` and checking `manual_price` values in `data/wealth.db`

## 4. Frontend upload

- [x] 4.1 Add `api.uploadPrices(payload)` to `frontend/src/api.ts`; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 4.2 Add a 匯入現價 file-picker button in `SummaryView.vue` that reads the JSON file, posts it, displays the report (updated count, unmatched, invalid, not-updated), and reloads the summary; verify in the browser that prices and 當前總市值 update after picking `current-price.json`

## 5. Docs and verification

- [x] 5.1 Add a "Bulk 現價 update" flow section to `docs/DATA_FLOW.md` and the CLI command to `AGENTS.md`
- [x] 5.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm build`, `pnpm exec vue-tsc --noEmit` — all clean
