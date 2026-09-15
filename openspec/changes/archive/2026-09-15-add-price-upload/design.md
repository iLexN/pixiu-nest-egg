## Context

See proposal.md for motivation. 現價 is `stocks.manual_price` (+ `price_updated_at`), currently set one stock at a time via `PATCH /api/stocks/:id`. HK stocks are keyed by Chinese-name `code` with a numeric `ticker`; US stocks use the ticker as `code`. The price file uses Yahoo-style symbols (`0388.HK`, `BRK-B`). Both a UI upload and a CLI command must share one update path.

## Goals / Non-Goals

**Goals:**
- One shared Rust function that parses the file shape, resolves symbols, applies updates transactionally, and returns a report — used by both the HTTP route and the CLI bin.
- Resolution rules that cover the real file: `.HK` suffix → HK `ticker`; everything else → US `code`/`ticker` with `-`≡`.` equivalence.
- Partial success with a report instead of all-or-nothing failure.

**Non-Goals:**
- Fetching prices from any external API (file remains user-supplied).
- Matching HK stocks by Chinese-name `code` — the file only carries tickers.
- Multipart file upload on the API; creating stocks for unknown symbols; price history.

## Decisions

- **Shared `prices` module in the backend lib.** `apply(pool, entries) -> report` resolves each `{symbol, price}` to a stock, then runs one transaction of `UPDATE stocks SET manual_price = ?, price_updated_at = ? WHERE id = ?`. Route and `import_prices` bin are thin wrappers. Alternative — looping `PATCH /api/stocks/:id` semantics — rejected: it would need per-row full-stock merging and N transactions for no benefit.
- **Symbol resolution is market-directed, not global.** `.HK` is stripped and looked up only against HK `ticker`; other symbols look up US `code` then `ticker`, with `-` normalized to `.`. Alternative — searching all stocks by code/ticker without a market hint — rejected: it makes `BRK-B`→`BRK.B` work by accident but also lets a bare `0388` silently hit the wrong market and complicates the report.
- **JSON body, not multipart.** The UI reads the picked file with `FileReader`, parses it, and POSTs `{"stocks": [...]}` to `POST /api/stocks/prices`. Keeps the API JSON-only like every existing endpoint; no new axum feature. Malformed JSON fails in the browser or with a 400 from the same parser the CLI uses.
- **Report shape mirrors the spec:** `{updated: [{market, code, symbol, price}], unmatched: [symbols], invalid: [entries], not_updated: [codes]}`. `not_updated` is informational (e.g. `ＦＧ恆生紅利` has no file entry) so the user can spot prices that silently stayed stale.
- **Validation:** `price` must be a positive finite number; missing/blank `symbol` or bad `price` lands in `invalid`. A missing/non-array `stocks` field is a 400 validation error. Duplicate symbols: last entry wins, both recorded in `updated`.

## Risks / Trade-offs

- A symbol could match a `ticker` that isn't unique across markets → resolution is scoped per market by the `.HK` rule, and HK `ticker` is unique in practice (imported from the workbook); a duplicate resolves to the first row and is reported.
- User edits `manual_price` in the UI between upload and reload → last write wins; the upload is one transaction and the UI reloads the summary afterward.
- File format drifts (extra columns, nested fields) → serde ignores unknown fields; only `symbol`/`price` are read.
