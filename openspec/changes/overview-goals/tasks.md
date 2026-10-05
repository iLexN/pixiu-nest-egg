# Tasks

## 1. Backend

- [x] 1.1 `backend/src/models.rs`: `GoalMilestone` (`target`, `remain`, `years`, `eta`), `FormulaGoal` (`target`, `remain`, `years`, `eta`), `OverviewGoals` (`n4_rate`, `n4_year`, `current_invest`, `saving_year`, `saving_month`, `passive_return_year`, `passive_return_month`, `milestones`, `third`, `final_goal`) with `ToSchema` and sheet-cell doc comments; `goals` on `OverviewResponse`; five fields on `MonthSettings`/`MonthSettingsPatch` (`Option<Option<f64>>` patch fields).
- [x] 1.2 `backend/src/calc.rs`: `OverviewGoalsInput` + pure `overview_goals`; inline tests reproducing the sheet's cached values (L12 347656.233, L13 57608.58002, K14 54436.56857/L14 0.1565815982, K20 1004436.569/L20 2.889166002, O14 5739480/O15/O16 10.23358316, P14 11492316.24/P15/P16 11.00528324) plus absent-input, zero-divisor, and reached-goal cases.
- [x] 1.3 `backend/src/routes/months.rs`: `GOAL1_AMOUNT_KEY`, `GOAL2_AMOUNT_KEY`, `GOAL3_INFLATION_KEY`, `GOAL3_YEARS_KEY`, `FINAL_REPLACEMENT_KEY` + defaults; settings GET/PATCH wiring and validation; round-trip test.
- [x] 1.4 `backend/src/routes/overview.rs`: gather inputs (asset rows, `averages`, `invest_targets`, `money_master`, `month_running_averages`, `year_review` N4 row, settings, `today`) and attach `goals`; route test.
- [x] 1.5 `backend/src/xlsx.rs`: `SheetGoals` on `OverviewCached`; `parse_overview` reads `K12`/`L12`/`M12`/`K13`/`L13`/`M13`/`K14`/`L14`, `K18`/`L18`/`M18`/`K19`/`L19`/`M19`/`K20`/`L20`, `O14`/`O15`/`O16`/`P14`/`P15`/`P16`; extend the inline test.
- [x] 1.6 `backend/src/import.rs`: seed `overview.goal1_amount`/`overview.goal2_amount` from `K12`/`K18` insert-if-absent.
- [x] 1.7 `backend/src/parity.rs`: `check_overview` compares `K12`/`K18` deterministically and `K13`, `L12`, `L13`, `K14`, `L14`, `K20`, `L20`, `O14`, `O15`, `O16`, `P14`, `P15`, `P16` informationally.

## 2. Frontend

- [x] 2.1 `frontend/src/api.ts`: `GoalMilestone`, `FormulaGoal`, `OverviewGoals` types; `goals` on `OverviewResponse`; the five fields on `MonthSettings`/`MonthSettingsPatch`.
- [x] 2.2 `frontend/src/views/OverviewView.vue`: `目標` card — milestone rows (target, remain, years, date), formula rows (target, remain, years, date), footer with the shared current value, saving/yr·/mo and return/yr·/mo, `N4`+year caption, `已達成` for reached goals, `—` for absent figures.
- [x] 2.3 `frontend/src/views/MonthStatView.vue`: settings card gains the two goal amounts and the three constants, following the `semi_liquid_target` edit/save pattern.

## 3. Docs

- [x] 3.1 `docs/overview.md`: new 目標 section.
- [x] 3.2 `docs/import-parity.md`: seeding + parity cells.
- [x] 3.3 `docs/database.md`: the five new `app_meta` keys.
- [x] 3.4 `docs/MONTH_STAT_OVERVIEW.md`: mark the targets block migrated.

## 4. Verify

- [x] 4.1 `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- [x] 4.2 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`.
- [x] 4.3 `import_xlsx` + `check_parity` against a scratch copy of the DB — `K12`/`K18` match; the derived goals cells are informational.
- [x] 4.4 Restart the live backend; `GET /api/overview` returns `goals` and the 總覽 `目標` card renders.
