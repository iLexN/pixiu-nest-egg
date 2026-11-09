# Spec Delta

## ADDED Requirements

### Requirement: 投資目標 formula terms

Each `invest_targets.rows[]` entry SHALL carry `terms`: `prior_invested` (prior year's `invested`), `prior_interest` and `prior_pool` (prior year's `interest` and `pool_spend × 0.7`), `raise_term` (`raise × 0.5 × 12`, absent `raise` as `0`), `interest`, and `pool` (`pool_spend × 0.7`). The `prior_*` terms SHALL be absent while their input is. Whenever `target` is present it SHALL equal `prior_invested − prior_interest − prior_pool + raise_term + interest + pool`.

#### Scenario: Terms expand the current-year target

- **WHEN** 2025 reports `invested` `428635.41`, `interest` `51900.65`, `pool_spend` `38729.5`, and 2026 reports effective `raise` `1890`, `interest` `62753.15`, `pool_spend` `9593`
- **THEN** the 2026 row's `terms` are `prior_invested` `428635.41`, `prior_interest` `51900.65`, `prior_pool` ≈ `27110.65`, `raise_term` `11340`, `interest` `62753.15`, `pool` ≈ `6715.1`, and they sum to its `target` ≈ `430432.36`

#### Scenario: First tracked year has no prior terms

- **WHEN** 2023 is the earliest row and reports no `raise`
- **THEN** its `terms.prior_invested`, `terms.prior_interest` and `terms.prior_pool` are absent, `terms.raise_term` is `0`, and `target` is absent

### Requirement: Saving-year terms

The `goals` block SHALL carry `saving_terms`, the five yearly terms `saving_year` averages: `saved_year` (`averages.saved × 12`), `avg_invested` (`invest_targets.avg_invested`), `mm_year` (`money_master.avg_per_month × 12`), `running_saved_year` (`saved_avg × 12`), and `running_combined_year` (`(saved_avg + interest_avg) × 12`). Each SHALL be absent while its inputs are, and `saving_year` SHALL be the mean of the present terms.

#### Scenario: Terms match the saving year

- **WHEN** `averages.saved` is `30013.27`, `avg_invested` is `326841.2233`, the money-master monthly is `30400.14056`, `saved_avg` is `26362.84273`, and `interest_avg` is `4480.899118`
- **THEN** `saving_terms` reports ≈ `360159.24`, `326841.22`, `364801.69`, `316354.11`, `370124.90`, and `saving_year` ≈ `347656.233` is their mean

#### Scenario: Absent money-master average

- **WHEN** `money_master.avg_per_month` is absent
- **THEN** `saving_terms.mm_year` is absent and `saving_year` averages the remaining present terms

### Requirement: Formula goal pace

Each formula goal SHALL carry `pace`, the yearly figure its `years` divides by: `third.pace` = `saving_terms.running_combined_year` and `final_goal.pace` = `total_change_avg × 12`, absent while their inputs are. `years` SHALL be `remain ÷ pace` under the existing reached and zero-divisor rules.

#### Scenario: Final pace from the running total change

- **WHEN** running `total_change_avg` is `59918.15472`
- **THEN** `final_goal.pace` ≈ `719017.86`

### Requirement: Goal pace cells

Each formula goal's 步伐 cell SHALL show that goal's `pace` from the response; the page SHALL NOT recompute the pace or any footnote term.

#### Scenario: Pace cells read the response

- **WHEN** `goals.third.pace` is `370124.90` and `goals.final_goal.pace` is `719017.86`
- **THEN** the 3rd row's 步伐 cell shows `370,124.90` and the Final row's shows `719,017.86`

## MODIFIED Requirements

### Requirement: Target formula footnote

A muted footnote SHALL expand the current year's `target` into its formula terms — `上年invested`, `上年利息回報`, `上年Pool支出×0.7`, `月薪增幅×50%*12`, `今年利息回報`, `今年Pool支出×0.7` — each followed by its contribution in parentheses, read from the current-year row's `terms` and never recomputed by the page, `—` while a term is absent. The footnote SHALL be absent while no current-year row exists.

#### Scenario: Footnote lists the terms

- **WHEN** a current-year row exists
- **THEN** the footnote shows each formula term with its contribution from the row's `terms`

### Requirement: Goal card formula footnotes

A muted footnote SHALL expand the two formula targets with live values — `3rd = 生活預算 (…) × 通脹 (…) × 12 × 年期 (…)` and `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` — each input's current figure in parentheses, `—` while an input is absent. The footnote SHALL likewise expand `存/年` as `AVERAGE` over its five yearly terms — `近12月存×12`, `近3年invested`, `MM月均×12`, `歷年存×12`, `(歷年存+息)×12` — each with its `goals.saving_terms` value in parentheses, `—` while absent.

#### Scenario: Footnote shows live inputs

- **WHEN** the 目標 card renders with a stored `final_replacement`
- **THEN** the footnote shows `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` with the live figures in parentheses
