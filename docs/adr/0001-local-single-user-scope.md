# Local single-user scope: loopback-only, no auth, manual prices, no market data

The app is a single-user local tool replacing `財富分析報告.xlsx`: it binds to loopback only, has no authentication or multi-user support, takes stock prices only via manual entry / bulk `import_prices`, and deliberately has no market-data integration — not even optional. It never writes back to the workbook or to Google Sheets. The workbook is retired: `data/wealth.db` is the only source of truth, the import/parity tooling was deleted outright (no feature flag), and there is no path to rebuild the DB from the spreadsheet — the user backs up `data/wealth.db` instead.

### Considered options
- Market-data fetch (e.g. GOOGLEFINANCE-style) — rejected: out of scope; the sheet's cached values were the migration baseline and parity, not live quotes.
- Keep the importer "just in case" — rejected: no remaining consumer; git history preserves it, and the workbook is stale for every migrated section anyway.
- Rebuild DB from workbook on demand — rejected: impossible without losing data (workbook is stale); `data/` backups are the answer.

Source: `openspec/changes/archive/2026-09-15-add-stock-trade-tracker` (goals/non-goals), `2026-10-14-retire-workbook-import`.
