# Tasks

## 1. Frontend

- [x] 1.1 In `frontend/src/views/DividendsView.vue`: initialize `year` to `new Date().getFullYear()`; in `load()`, after the summary arrives, fall back to `history_years.at(-1)` when the current year isn't in `history_years` (DepositHistoryView convention — keep current year when `history_years` is empty); change the market-switch watcher reset from `null` to the current year. Verify `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` passes.
- [x] 1.2 Update `docs/stocks.md` (~L321): note the 已收記錄 `year` parameter defaults to the current year with fallback to the latest year holding received records.

## 2. Verification

- [x] 2.1 Restart the backend (`lsof -ti :8787 | xargs kill`, then `nohup cargo run -p wealth-backend --bin wealth-backend > /tmp/wealth-backend.log 2>&1 < /dev/null & disown`); confirm the 港股 and 美股 已收記錄 cards open on the current year (or the latest year with records) and 全部年份 still works.

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied.
