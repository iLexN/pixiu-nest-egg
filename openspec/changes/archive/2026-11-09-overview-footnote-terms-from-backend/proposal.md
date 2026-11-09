# Proposal

## Why

The 總覽 footnotes redo financial math in the browser. `OverviewView.vue` multiplies `pool_spend × 0.7` and `raise × 6` to expand the 投資目標 target. For 存/年 it computes `saved × 12`, `mm × 12`, `saved_avg × 12`, and `(saved_avg + interest_avg) × 12`. It also builds both formula goals' pace (`combined × 12`, `total_change_avg × 12`). Each of these coefficients already lives in `calc::invest_targets` or `calc::overview_goals`, so the formula exists twice. If the backend formula changes, the footnote silently shows terms that no longer add up to the figure beside it. AGENTS.md makes the backend the source of truth for every financial calculation; the footnotes are the last place on 總覽 that breaks it.

## What Changes

- Each `invest_targets.rows[]` entry gains `terms`, the six contributions the target formula sums, computed by the same code that computes `target`.
- `goals` gains `saving_terms`: the five yearly 存/年 terms that `saving_year` averages.
- `goals.third` and `goals.final_goal` gain `pace`: the yearly figure their `years` divides by.
- `OverviewView.vue` renders those fields and deletes `targetNotePrior` and the arithmetic in `targetNoteTerms`, `savingYearTerms`, and `formulaGoalRows`.
- Additive API change: existing fields are unchanged, so no figure moves.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `overview`: the 投資目標 rows and the goals block expose their formula terms and paces; the footnote requirements render those instead of recomputing them.

## Impact

- Backend: `backend/src/models/overview.rs` (new `InvestTargetTerms`, `SavingYearTerms`, `FormulaGoal.pace`), `backend/src/calc/year_review.rs` (`invest_targets`), `backend/src/calc/overview.rs` (`overview_goals`, `formula_goal`).
- Frontend: `frontend/src/views/OverviewView.vue`; `api-types.ts` regenerated via `pnpm gen:api`, plus alias lines in `api.ts` if the view names the new types.
- API: `GET /api/overview` gains fields (additive). No schema migration.
