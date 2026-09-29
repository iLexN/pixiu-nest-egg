# Tasks

## Backend

- [x] `backend/src/models.rs`: `MoneyMaster` struct (`start_date`, `month_now`, `months_left`, `saved`, `coming_save`, `target_months`, `target_amount`, `avg_per_month`, `yearly_rate`, `time_progress`, `saved_progress`, `progress_gap`, `can_use` — Option-wrapped per the absent-input rules); `money_master` field on `OverviewResponse`; `money_master_*` fields on `MonthSettings`/`MonthSettingsPatch`.
- [x] `backend/src/routes/months.rs`: `MM_*` key constants; `settings` GET returns the stored fields; `update_settings` validates (`start_date` parses `YYYY-MM-DD`, month counts positive integers, `target_amount` positive, `saved`/`coming_save` finite sign-free; `null` clears) and writes via `meta_put`.
- [x] `backend/src/routes/overview.rs`: read the keys + salary, resolve `month_now` (override else `full months since start_date + 1`), `months_left`, `coming_save` (override else `salary + (target − saved)/months_left`), and the remaining derived figures with absent-while-missing rules.
- [x] `backend/src/xlsx.rs`: `SheetMoneyMaster` on `OverviewCached`; parse K31/L31/M31/N31 (r30), K32/L32 (r31), K33/L33/M33 (r32), K34 (r33), K35 (r34); update the inline xlsx test.
- [x] `backend/src/import.rs`: seed `start_date` `2023-10-27`, `saved` `1094405.06`, `target_months` `36`, `target_amount` `1000000` once; overrides unset.
- [x] `backend/src/parity.rs`: informational `J29:N35` comparisons in `check_overview`.
- [x] Inline `#[cfg(test)]` tests: month-derivation boundaries, `months_left` floor, override precedence, absent inputs, negative `coming_save`.

## Frontend

- [x] `frontend/src/api.ts`: `MoneyMaster` type; `money_master` on `OverviewResponse`; settings fields on `MonthSettings`/`MonthSettingsPatch`.
- [x] `frontend/src/views/OverviewView.vue`: `Money Master` card in `.overview-grid` (now/target rows month·saved·avg, progress row with `signClass` gap, `comming save per month`, `can use`; `—` while absent).
- [x] `frontend/src/views/MonthStatView.vue`: settings card gains `start_date` (`type="date"`), `month_now`/`coming_save` override inputs (blank = auto), `saved`, `target_months`, `target_amount`.
- [x] `frontend/src/views/DepositsView.vue`: drop "money master" from the 試算表手動步驟 muted note; keep the checklist `<li>`.

## Docs

- [x] `docs/overview.md`: Money Master paragraph.
- [x] `docs/MONTH_STAT_OVERVIEW.md`: "Implemented in app (add-money-master)" header note.
- [x] `docs/DATA_FLOW.md`: extend the overview.md index line.

## Verify

- [x] `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
- [x] `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`
- [x] `check_parity` on a scratch DB after fresh `import_xlsx` — informational Money Master rows, no new failures
- [x] Restart backend; `GET /api/overview` shows month 36, coming −41,705.06, can use 94,405.06
