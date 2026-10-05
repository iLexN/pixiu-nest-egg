# Proposal

## Why

The `Overview` sheet's goal blocks — `J10:M20` (the 1st/2nd 目標 milestones) and `N13:P16` (the 3rd target / Final Target) — are still maintained in the workbook. The milestone amounts are literals and the sheet derives saving/year, passive return, years-to-target, and ETA dates on top; every other input already exists in the app, so the blocks migrate as a derived `goals` block on `GET /api/overview` plus five settings.

## What Changes

- `GET /api/overview` gains a `goals` block mirroring `J10:M20` and `N13:P16`:
  - Shared figures: `n4_rate`/`n4_year` (`N4` = the latest completed year's `income_value_rate` from YearInReview), `current_invest` (`K13` = 港股 + 債券 + IBKR), `saving_year`/`saving_month` (`L12`/`M12` = average over the five saving sources, absent terms skipped), and `passive_return_year`/`passive_return_month` (`L13`/`M13` = `K13` × `N4`).
  - Two milestones (`K12` 1st, `K18` 2nd): `target`, `remain` (`K14`/`K20` = target − `current_invest`), `years` (`L14`/`L20` = remain ÷ `saving_year`), and `eta` (`J11`/`J17` = today + round(365 × years) days).
  - Two formula goals: `third` (`O14`/`O15`/`O16` = budget × inflation × 12 × years, vs 流動資產, paced by (saved + interest) × 12) and `final_goal` (`P14`/`P15`/`P16` = salary × replacement × 12 ÷ N4, vs 總數, paced by total-change × 12).
- Five `app_meta` settings: `overview.goal1_amount`/`overview.goal2_amount` (finite, ≥ 0; `null` clears so that milestone's figures go absent) and `overview.goal3_inflation` (1.07), `overview.goal3_years` (30), `overview.final_replacement` (0.7) (finite, > 0; `null` restores the default — the `semi_liquid_target` pattern). All editable through `PATCH /api/months/settings`; no new endpoints.
- Deliberate deviation from the sheet: when `remain ≤ 0` a goal's `years`/`eta` are absent and the UI shows `已達成`, rather than the sheet's negative years / past date.
- Import seeds `overview.goal1_amount`/`overview.goal2_amount` once from `Overview!K12`/`K18` and never overwrites them on re-import.
- Parity: `K12`/`K18` compare deterministically (seeded settings reproduce the literals); `K13`, `L12`, `L13`, `K14`, `L14`, `K20`, `L20`, and `O14`–`P16` compare as informational — they drift with live prices, the date, and the app's fresher `N4`.
- The 總覽 page gains a `目標` card mirroring the sheet layout; the 月結 settings card gains the two goal amounts plus the three constants.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: `GET /api/overview` additionally returns the `goals` block (shared figures, two milestones, two formula goals); the 總覽 page shows the `目標` card; `GET`/`PATCH /api/months/settings` carries the five `overview.*` goal settings with validation.
- `spreadsheet-trade-import`: import additionally seeds `overview.goal1_amount`/`overview.goal2_amount` from `Overview!K12`/`K18` once; the Overview parity report gains the goals-cell comparisons (deterministic for the seeded amounts, informational for the derived cells).

## Impact

- **Backend**: `backend/src/models.rs` (`OverviewGoals`/`GoalMilestone`/`FormulaGoal`, `goals` on `OverviewResponse`, five fields on `MonthSettings`/`MonthSettingsPatch`), `backend/src/routes/months.rs` (key constants, settings GET/PATCH), `backend/src/calc.rs` (pure `overview_goals`), `backend/src/routes/overview.rs` (input gathering + wiring), `backend/src/xlsx.rs` (`OverviewCached.goals` + `parse_overview`), `backend/src/import.rs` (seed-once), `backend/src/parity.rs` (new comparisons). No migration — `app_meta` is generic key-value; no new endpoints, so the OpenAPI pin test is untouched.
- **Frontend**: `frontend/src/api.ts` types, `frontend/src/views/OverviewView.vue` (目標 card), `frontend/src/views/MonthStatView.vue` (settings fields).
- **Docs**: `docs/overview.md` (目標 section), `docs/import-parity.md`, `docs/database.md` (new keys), `docs/MONTH_STAT_OVERVIEW.md` (marked migrated).
- **Reconfiguration**: the milestone amounts, inflation multiplier, horizon, and replacement ratio are settings edits; `N4` steps once each January when the latest completed year rolls over.
