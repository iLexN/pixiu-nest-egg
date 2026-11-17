# Tasks

## 1. Extract the panel

- [x] 1.1 `MonthDetailPanel.vue`: prop `detail: MonthDetail`; emits `saved`/`failed`/`deleted`; month draft + figure save/recapture/goLive/delete; items CRUD; suggestion actions; `interest_auto` rows; `CATEGORY_*`/`INTEREST_SOURCE_LABELS`/`hasImportedTotal`; scoped styles for the detail card

## 2. Slim the view

- [x] 2.1 `MonthStatView.vue`: drop the detail-card script + template + styles; `onPanelSaved`/`onPanelFailed` reused; `onPanelDeleted` clears `selectedMonth`/`detail` then reloads

## 3. Verify

- [x] 3.1 `pnpm exec vue-tsc --noEmit && pnpm build`
- [x] 3.2 Restart backend (serves the fresh `dist/`); spot-check the 月結 page's detail card actions in-browser or via code review

## Workflow follow-up

- Archive with `openspec archive` (skip_specs — nothing to sync)
