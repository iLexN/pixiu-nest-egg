# Tasks

## 1. Settings view

- [x] 1.1 Create `frontend/src/views/SettingsView.vue`: `usePage(load)`; `load()` fetches `api.getSettings()`, `api.listManualAssets()`, `api.listCashAccounts()` in parallel — verify the view compiles standalone
- [x] 1.2 Move the 活期帳戶 table + add/edit `InlineForm` and the 資產 table + add/edit `InlineForm` from `MonthSettingsPanel.vue` verbatim (including the bank-role HKD currency snap and the broker no-delete rule) — verify tables render with live data and CRUD round-trips in the browser
- [x] 1.3 Replace the single `editingSettings`/`saveSettings` with `editingSection` + per-section `startXxxEdit`/`saveXxx` for 一般, Money Master, 目標, and 日曆, each showing its muted summary line + 編輯 and PATCHing only its own fields — verify each section saves independently and other settings are unchanged (`PATCH` bodies carry only that section's keys)
- [x] 1.4 Add the 年份 input to the 一般 form (default `settings.pool_rate_year`); prefill the rate from `settings.pool_rate` only when the entered year equals `pool_rate_year`; save sends `pool_rate` + `pool_rate_year` from the input — verify saving a past year's rate writes `pool_rate_year` correctly and the field stays blank for non-current years (superseded by 4.3's rates table)
- [x] 1.5 Backend: add `pool_rates: Vec<PoolRate>` (`{year, rate}`) to `AppSettings` and populate it in `service::settings::get` from `store::months::pool_rates` — verify `GET /api/settings` returns every stored year's rate
- [x] 1.6 Run `cd frontend && pnpm gen:api && pnpm build` — verify `api-types.ts` gains `pool_rates` on the settings schema
- [x] 1.7 `SettingsView.vue`: remove the rate/year fields and their drafts from the 一般 section; add a 開心Pool section — a per-year rates table （年份/利率） with 編輯/刪除 row actions, a 新增 action, and the current year's effective rate shown — saving PATCHes `pool_rate` + `pool_rate_year`, deleting PATCHes `pool_rate: null` for that year — verify rows list, edit, add, and delete in the browser

## 2. Navigation and 月結 cleanup

- [x] 2.1 `frontend/src/App.vue`: add `'settings'` to the `Tab` and `Group` unions, append `{ id: 'settings', label: '設定', tabs: [{ id: 'settings', label: '設定' }] }` to `NAV`, and render `<SettingsView v-else-if="tab === 'settings'" />` — verify the 設定 group appears last, opens its 設定 tab, and the market toggle stays hidden
- [x] 2.2 `frontend/src/views/MonthStatView.vue`: remove the `MonthSettingsPanel` usage/import, the `settings`/`assets`/`accounts` refs and their fetches, and now-unused type imports — verify 月結 loads with no settings card and no extra requests
- [x] 2.3 Delete `frontend/src/components/MonthSettingsPanel.vue` — verify nothing references it (`rg MonthSettingsPanel` returns no hits)

## 3. Verification and docs

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and `cargo test` — verify clean (no API changes, so the OpenAPI pin test still passes)
- [x] 3.2 Update `docs/overview.md`, `docs/deposits.md`, `docs/DATA_FLOW.md` — repoint every "月結 設定"/"月結 settings card" editing-location reference to the 設定 tab
- [x] 3.3 Restart the backend (`lsof -ti :8787 | xargs kill`, `nohup cargo run -p wealth-backend --bin wealth-backend > /tmp/wealth-backend.log 2>&1 < /dev/null & disown`) — verify http://127.0.0.1:8787 shows the 設定 group and sections save independently

## Workflow follow-up

- Archive the change after the project's review requirements are satisfied (`openspec archive`), syncing delta specs into `openspec/specs/`.
- Verify the archived result.
