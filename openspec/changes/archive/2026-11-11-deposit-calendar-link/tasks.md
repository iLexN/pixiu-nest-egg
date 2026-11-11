# Tasks

## 1. Shared link builder

- [x] 1.1 Add `fmtBankName(code)` to `frontend/src/format.ts` (reusing `BANK_NAMES`; `''` for null, unknown code as itself) and verify `pnpm exec vue-tsc --noEmit` passes
- [x] 1.2 Add `googleCalendarUrl({ title, date, lines })` to `frontend/src/format.ts`: `action=TEMPLATE`, `text`, `dates=YYYYMMDD/YYYYMMDD` with the exclusive end computed by UTC date arithmetic, `details` from non-empty `label: value` pairs joined by `\n`, encoded with `URLSearchParams`. Verify with a throwaway `node`/`tsx` check that `2027-02-10` gives `20270210/20270211`, `2026-12-31` gives `20261231/20270101`, and empty-value pairs are dropped; delete the throwaway afterwards

## 2. Row menu item

- [x] 2.1 Add an optional `calendar` prop and `calendar` emit to `frontend/src/components/RowActions.vue`, rendering 加到日曆 after 複製 and before 編輯; verify `vue-tsc` passes and existing menus (trades 複製, deposits 收訖/編輯/刪除) are unchanged in the browser

## 3. 定期 page

- [x] 3.1 Add an optional `calendar` prop to `frontend/src/components/DepositTable.vue`; when set, pass it to `RowActions` and on `@calendar` open the 定期 link (title parts `[label, fmtBankName(bank)]` joined by ` - `, falling back to `定期到期`; lines 到期日, 編號, 銀行 via `fmtBank`, 本金, 利率 via `fmtPercent(rate)`, 利息, 合計, 開始日) with `window.open(url, '_blank', 'noopener')`. Verify `vue-tsc` passes
- [x] 3.2 Pass `calendar` only on the 未到期定期 `<DepositTable>` in `frontend/src/views/DepositsView.vue`; verify in the browser that an upcoming row's 加到日曆 opens Google Calendar with the title, all-day date, and details from the `time-deposits` spec scenarios, and that history rows (DepositHistoryView) have no 加到日曆
- [x] 3.3 Update `docs/deposits.md` to mention 加到日曆 in the 未到期 row's ⋯ menu (opens a pre-filled Google Calendar event; no API call); verify the doc reads correctly next to the existing 編輯/刪除 notes

## 4. 家人 page

- [x] 4.1 In `frontend/src/views/FamilyDepositsView.vue`, add `calendar` and `@calendar` to the 未到期定期 table's `RowActions` only, opening the 家人 link (title parts `[holder, label, fmtBankName(bank)]`; lines 持有人, 到期日, 編號, 銀行, 本金, 利息, 合計, 開始日). Verify in the browser that the `Irene - HS-0090 - 恒生` example matches the `family-deposits` spec scenarios and that the 定期記錄 rows have no 加到日曆
- [x] 4.2 Update the family-deposits section of `docs/deposits.md` with the same 加到日曆 note; verify the doc reads correctly

## 5. Integration checks

- [x] 5.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and verify both pass
- [x] 5.2 Run `openspec validate deposit-calendar-link --strict` and verify it passes
- [x] 5.3 Restart the backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, then the detached `nohup cargo run …`) and verify http://127.0.0.1:8787/ serves the new build with 加到日曆 on both pages

## 6. Private calendar setting (backend)

- [x] 6.1 Add `calendar.private_id` to `backend/src/store/settings.rs` (`Settings` load + `SettingsPatch` apply), `calendar_private_id: Option<String>` to `AppSettings` and `Option<Option<String>>` (nullable) to `AppSettingsPatch` in `backend/src/models/settings.rs`, and trim / blank-clears handling in `service::settings::update`. Add a round-trip test next to `update_settings_round_trips_the_semi_liquid_target` covering padded value → trimmed, `""` → null, and `null` → null; verify `cargo test` passes
- [x] 6.2 Add `calendar.private_id` to the `app_meta` key list in `docs/database.md`; verify the doc lists it with its meaning (Private Google calendar ID for 加到日曆)

## 7. Private calendar setting (frontend)

- [x] 7.1 Run `cd frontend && pnpm gen:api` and verify `api-types.ts` gains `calendar_private_id` on `AppSettings` / `AppSettingsPatch`
- [x] 7.2 In `frontend/src/views/MonthStatView.vue`, add a `Private 日曆 ID` text input to the 設定 form (prefilled in `startSettingsEdit`, saved trimmed with blank → `null` in `saveSettings`) and a summary line showing the ID or `—`; verify `vue-tsc` passes and saving round-trips via `GET /api/settings`
- [x] 7.3 Add an optional `src` to `googleCalendarUrl` in `frontend/src/format.ts` (omitted from the URL when null/empty); verify with a throwaway script that a set `src` appears as `src=…%40group.calendar.google.com` and an empty one is absent; delete the script afterwards
- [x] 7.4 `DepositsView.vue` loads `api.getSettings()` with its summary and passes `calendar_private_id` to the 未到期 `<DepositTable>` via a new `calendarSrc?: string | null` prop, which `DepositTable.openCalendar` forwards as `src`; verify `vue-tsc` passes and the throwaway script shows own deposits carry the Private `src`
- [x] 7.5 `FamilyDepositsView.vue` loads `api.getSettings()` in `load()` and `openCalendar` passes `src` = `null` when `deposit.holder === 'Irene'`, else `calendar_private_id`; verify with the throwaway script that `Irene` gets no `src` and `媽媽` gets the Private `src`
- [x] 7.6 Update `docs/deposits.md` 加到日曆 notes with the routing (Irene → default calendar, others and own 定期 → Private via `calendar_private_id`, editable in 月結 設定); verify the doc reads correctly

## 8. Integration checks (calendar routing)

- [x] 8.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; verify all pass
- [x] 8.2 Run `openspec validate deposit-calendar-link --strict` and verify it passes
- [x] 8.3 Restart the backend per AGENTS.md and verify `GET http://127.0.0.1:8787/api/settings` includes `calendar_private_id` and the served bundle contains `Private 日曆 ID`

## 9. Title prefix

- [x] 9.1 Prefix both titles with `定期: ` (`DepositTable.vue` and `FamilyDepositsView.vue` `openCalendar`; own-deposit fallback becomes `定期`); verify `pnpm build`, `vue-tsc`, `openspec validate --strict`, and restart the backend

## Workflow follow-up

- Archive the change with `openspec-archive-change` (syncing delta specs into `openspec/specs/`) after review.
- Commit with a one-line message suggested at archive time.
