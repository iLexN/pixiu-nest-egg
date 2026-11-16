# Tasks

## 1. Extract the 預測 grid

- [x] 1.1 Create `components/ForecastGrid.vue` with the script block (KIND_INFO/PLAN_ROWS through `onConverted`), the `<div class="card forecast">` template, and the forecast styles; wire `forecast` prop + `saved`/`failed` emits; `syncDrafts` on prop watch.
- [x] 1.2 Slim `OverviewView.vue`: delete moved script/template/styles, render `<ForecastGrid v-if="forecast" :forecast="forecast" @saved="…" @failed="…" />`, drop now-unused imports.

## 2. Verification

- [x] 2.1 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — clean.
- [x] 2.2 Restart the backend and spot-check the 預測 card on http://127.0.0.1:8787/ (select a cell, edit a plan item, 轉為定期 opens the form).

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
