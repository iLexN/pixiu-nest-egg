# Spreadsheet-parity quirks reproduced verbatim

Several financially-odd behaviors are deliberate reproductions of the workbook — "fixing" them would silently change figures the owner has tracked for years:

- **HK trades** derive `fee = buy total − 股數 × 單價` (user enters the fee-inclusive total); **US trades** derive `buy total = 股數 × 單價 + fee`. The input direction differs per market because that's how the sheets were filled.
- **加權平均買入單價 = 總買入成本 ÷ Σ BUY 股數** divides by shares *bought*, not shares held — a SELL never reduces 總買入成本.
- **Yearly ÷12 averages** divide by 12 even for a partial year (the sheet's `=C/12`).
- **Active deposits count at principal only** — expected interest does not enter totals until 收訖.
- **Unpriced/zero-holding stocks** are excluded from market-value totals (`totals.excluded_codes`).

These are specified, tested behaviors — treat them as the contract, not bugs.

Source: `openspec/changes/archive/2026-09-15-add-stock-trade-tracker`, `2026-09-21-add-month-stat`, `2026-09-17-add-yearly-summary`; pinned in `openspec/specs/` + AGENTS.md "Working rules".
