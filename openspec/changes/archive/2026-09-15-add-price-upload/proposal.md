## Why

現價 must currently be typed one stock at a time in 持倉總覽, even though the owner already exports all current prices into a `current-price.json` file (symbols such as `0388.HK`, `VOO`, `BRK-B`). Applying that file in one step removes ~15 manual edits per refresh.

## What Changes

- Accept a `current-price.json`-style upload (`{"stocks": [{"symbol": "0388.HK", "price": 395.4}, ...]}`) that bulk-updates `manual_price` and `price_updated_at` for every matched stock in one transaction.
- Symbol resolution: symbols ending in `.HK` match HK stocks by `ticker` (e.g. `0388.HK` → ticker `0388`); all other symbols match US stocks by `code`/`ticker`, with `-` normalized to `.` so `BRK-B` resolves to `BRK.B`.
- Partial application with a report: matched entries are applied; unmatched symbols, invalid entries, and stocks left without a price in the file are reported back instead of failing the whole upload.
- Two entry points sharing one backend code path: a `POST /api/stocks/prices` endpoint driven by a file-picker button in the Vue UI, and a `cargo run -p wealth-backend --bin import_prices -- current-price.json` CLI command honoring `WEALTH_DB`.
- Still no external market-data calls: the file is user-supplied input, equivalent to today's manual 現價 entry.

## Capabilities

### New Capabilities
<!-- None. -->

### Modified Capabilities
- `stock-portfolio-summary`: extends "Manually entered market data" — 現價 can additionally be set in bulk from an uploaded JSON price file, with per-entry match reporting.

## Impact

- Backend: new `prices` module with shared parse/resolve/apply logic; new `POST /api/stocks/prices` route; new `import_prices` bin. No schema migration — reuses `manual_price`/`price_updated_at`.
- Frontend: file-picker button in 持倉總覽 that reads the JSON file, posts it, shows the update report (updated count, unmatched/invalid/unpriced), and reloads the summary.
- Docs: `docs/DATA_FLOW.md` gains a flow section; `AGENTS.md` run/commands list gains the CLI entry.
- No changes to trades, calculations, or the workbook; `財富分析報告.xlsx` remains untouched.
