# Tasks

## 1. Extract the 設定 panel

- [x] 1.1 Create `components/MonthSettingsPanel.vue` with the settings/accounts/assets script (`settingsOpen` through `removeAccount`), the `<div class="card">` template, and the needed styles; wire props (`settings`, `summary`, `accounts`, `assets`, `year`) + `saved`/`failed` emits.
- [x] 1.2 Slim `MonthStatView.vue`: delete moved script/template/styles, render `<MonthSettingsPanel :settings="settings" :summary="summary" :accounts="accounts" :assets="assets" :year="year" @saved="…" @failed="…" />`, drop now-unused imports.

## 2. Verification

- [x] 2.1 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — clean.
- [x] 2.2 Restart the backend and spot-check the 月結 page's 設定 card (toggle, edit an account/asset, edit settings).

## Workflow follow-up

- Archive the change (`openspec archive`) and sync delta specs into `openspec/specs/`.
