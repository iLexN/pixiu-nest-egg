# Design

## Context

See proposal.md (Why). Current state:

- `calc::invest_targets` (`backend/src/calc/year_review.rs`) computes `target` inline from the prior row's `invested`/`interest`/`pool_spend` and the row's `raise`/`interest`/`pool_spend`, and keeps none of the intermediate terms.
- `calc::overview_goals` (`backend/src/calc/overview.rs`) builds the five `saving_year` terms into a throwaway `Vec`, computes `combined_running`, and passes the pace for each formula goal into `formula_goal(target, assets, pace, today)`. That function drops `pace` once it has computed `years`.
- In `OverviewView.vue`:
  - `targetNotePrior` and `targetNoteTerms` re-derive the six target terms (`× 0.7`, `× 6`).
  - `savingYearTerms` re-derives the five 存/年 terms (`× 12`).
  - `formulaGoalRows` re-derives both paces (`combined12`, `total_change_avg × 12`).

## Goals / Non-Goals

**Goals:**
- Every term a footnote shows is computed once, next to the formula that consumes it.
- The view only formats and labels.

**Non-Goals:**
- Changing any formula, constant, or existing response field.
- Making `0.7` or `0.5 × 12` configurable.
- The raw inputs the footnotes echo (`averages.saved`, `money_master.avg_per_month`, `saved_avg`, `interest_avg`, `living_budget`, salary, constants). They are already response fields with no math, and the view keeps reading them.

## Decisions

### D1. `InvestTargetTerms` on every row

```rust
pub struct InvestTargetTerms {
    pub prior_invested: Option<f64>,
    pub prior_interest: Option<f64>,   // absent while no prior-year row
    pub prior_pool: Option<f64>,       // prior pool_spend × 0.7; absent while no prior row
    pub raise_term: f64,               // raise × 0.5 × 12, absent raise as 0
    pub interest: f64,
    pub pool: f64,                     // pool_spend × 0.7
}
```

`InvestTargetRow.terms: InvestTargetTerms` (always present). `invest_targets` builds `terms` first and derives `target` from it:

```rust
prior_invested.map(|p| p - prior_interest.unwrap_or(0.0) - prior_pool.unwrap_or(0.0) + raise_term + interest + pool)
```

`target` therefore can't disagree with its terms. It keeps the existing operand order (invested − interest − pool + raise + interest + pool), so floating-point results are bit-identical.

The `prior_*` absence semantics match today's footnote: `prior?.interest ?? null`, and `prior === null ? null : prior.pool_spend * 0.7`.

*Alternative: put terms only on the current-year row (`Option<InvestTargetTerms>`).* Rejected: every row has a target, so the terms belong to every row, and the view already picks the current year.

### D2. `SavingYearTerms` on goals and `pace` on `FormulaGoal`

```rust
pub struct SavingYearTerms {
    pub saved_year: Option<f64>,
    pub avg_invested: Option<f64>,
    pub mm_year: Option<f64>,
    pub running_saved_year: Option<f64>,
    pub running_combined_year: Option<f64>,
}
```

`OverviewGoals.saving_terms: SavingYearTerms` and `FormulaGoal.pace: Option<f64>`. `overview_goals` builds `SavingYearTerms` first, and `saving_year` becomes the mean of its present fields, in the same field order as the current `Vec`, so the sum order is unchanged. `formula_goal` stores the `pace` it already receives.

### D3. View

- `targetNoteTerms` becomes the current-year row's `terms` (or `null` without a row); `targetNotePrior` is deleted.
- `savingYearTerms` reads `goals.saving_terms` for the yearly values and keeps reading the raw monthly inputs (`averages.saved`, `money_master.avg_per_month`, `goals.saved_avg`, `goals.interest_avg`) for the "(… x 12 = …)" left-hand figures.
- `formulaGoalRows` reads `goal.pace`.

After the change, `grep -nE "\* ?(0\.7|6|12)\b" OverviewView.vue` finds nothing in `<script>`. Labels in the template, such as `×0.7` text, stay.

## Risks / Trade-offs

- [Float drift in `target` or `saving_year` from the refactor] → Keep the operand order. Compare `/api/overview` before and after on the real DB: every pre-existing field must be byte-identical.
- [The footnote figure differs from the old frontend math in its last digit] → The backend value is now authoritative. The new spec scenarios pin the values to 2 dp, the displayed precision.
