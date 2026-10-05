# Tasks

## 1. Frontend

- [x] 1.1 `frontend/src/views/MonthStatView.vue`: `addMonth` sends `api.patchMonth(ym, { total_assets: null, liquid_assets: null })` so the created row stores NULL totals and derives live. Verify: `cd frontend && pnpm exec vue-tsc --noEmit` clean.

## 2. Docs

- [x] 2.1 `docs/overview.md`: payday-entry wording — the 新增月份 form now creates rows live (NULL totals); a bare create patch that omits the totals still snapshots. Verify: the paragraph no longer claims create always snapshots.

## 3. Verification

- [x] 3.1 `cd frontend && pnpm build` succeeds; restart the backend (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend`) and confirm `GET /api/months` answers.
- [x] 3.2 Manual: click 新增月份 → the new row shows italic 即時 totals without a 改為即時 click, and 重新擷取 still freezes it.
