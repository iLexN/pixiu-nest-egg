# Proposal

## Why

On a deposit's maturity day the app stops counting it in 已定期, but the money doesn't reach 活期 until the user presses 收訖 → 存入活期. Until then the principal isn't counted anywhere. On 2026-10-13, SC #18/#19 (145,000) matured unreceived, and the 2026-10 Month Stat row's 流動資產 變動 dropped about 149k. Family 活躍本金 dipped the same way (媽媽 SC-9179, 100,000). Per-row status is already receipt-based; only the aggregates still use the sheet's end-date rule.

## What Changes

- A deposit counts as active in every total and rollup while it is not 收訖 (`received_at` unset), whatever its `end_date`. 收訖 removes it. This applies to:
  - own deposits: 定期 active principal, month rollup, bank rollup, Overview 已定期 / 半流動資金 / 總數 / 流動資產, and Month Stat live totals and 變動
  - family deposits: per-holder 活躍本金
- Counting stays at **principal only**, inside 已定期. There is no new line and no interest.
- A deposit 收訖'd before its `end_date` stops counting on receipt.
- The parity check keeps the workbook's end-date rule, because the sheet has no receipt concept. It keeps measuring import parity, not the app's live rule.
- Unchanged:
  - year/history tables (they group by end year)
  - the Month Stat / forecast `定期 + SC` locked chain and `定期 finish` (flow-based on dates)
  - receive/unreceive behavior
  - create/import auto-receive of past end dates (own deposits only)

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `time-deposits`: "Upcoming deposit view" — totals and rollups become receipt-based instead of end_date-based.
- `family-deposits`: "Per-holder summary and history" — active principal counts unreceived deposits instead of future end dates.

## Impact

- Backend:
  - `backend/src/calc/deposits.rs`: `DepositFacts`, `deposit_active`, `active_totals`, `active_month_rollup`, `bank_rollup`, and their tests
  - `backend/src/store/deposits.rs`: the fact loaders read `received_at`
  - callers in `routes/deposits.rs`, `routes/family.rs`, `service/months.rs`, `parity.rs`, plus the test fixture in `calc/year_review.rs`
- API: no shape changes. Values change only while a deposit is matured and unreceived (`/api/deposits/summary`, `/api/family/deposits/summary`, `/api/overview`, `/api/months*` live totals).
- Docs: `docs/deposits.md` (list/totals split, family 活躍本金), `docs/overview.md` (live 已定期 wording).
- No migration; `received_at` already exists on both tables.
