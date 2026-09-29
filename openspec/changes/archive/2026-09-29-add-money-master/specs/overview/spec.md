# Spec Delta

## ADDED Requirements

### Requirement: Money Master block

`GET /api/overview` SHALL return a `money_master` block mirroring the sheet's `J29:N35` challenge tracker: the stored inputs `start_date` (challenge start `YYYY-MM-DD`), `saved` (the bank app's saved figure), `target_months`, and `target_amount`; the effective `month_now` and `coming_save`; and the derived `months_left`, `avg_per_month` (`M31`), `yearly_rate` (`N31`), `time_progress` (`K33`), `saved_progress` (`L33`), `progress_gap` (`M33`), and `can_use` (`K35`).

- `month_now` SHALL be the stored `money_master.month_now` override when set, else `full months elapsed since start_date + 1` (the bank's 1-indexed month counter), absent while no `start_date` is stored and no override exists.
- `months_left` SHALL be `max(1, target_months − month_now + 1)`, absent while either input is missing.
- `coming_save` SHALL be the stored `money_master.coming_save` override when set, else `salary + (target_amount − saved) ÷ months_left` — negative while ahead of target, matching the bank app — absent while any of salary, `target_amount`, `saved`, or `months_left` is missing.
- `avg_per_month` SHALL be `saved ÷ month_now`, `yearly_rate` SHALL be `avg_per_month × 12`, each absent while its inputs are missing or the divisor is zero.
- `time_progress` SHALL be `month_now ÷ target_months`, `saved_progress` SHALL be `saved ÷ target_amount`, `progress_gap` SHALL be `saved_progress − time_progress`, each absent while its inputs are missing or the divisor is zero.
- `can_use` SHALL be `salary − coming_save`, absent while either input is missing; it MAY exceed salary while `coming_save` is negative.

#### Scenario: Live challenge derived

- **WHEN** `start_date` is `2023-10-27`, today is `2026-09-29`, `saved` is `1094405.06`, `target_months` `36`, `target_amount` `1000000`, salary `52700`, and no overrides exist
- **THEN** the block reports `month_now` `36`, `months_left` `1`, `coming_save` approximately `-41705.06`, `can_use` approximately `94405.06`, `avg_per_month` approximately `30400.14`, `saved_progress` approximately `1.094`, and `progress_gap` approximately `0.094`

#### Scenario: Override pins a figure

- **WHEN** `money_master.month_now` is `35` and `start_date` would derive `36`
- **THEN** the block reports `month_now` `35`, `months_left` `2`, and `coming_save`/`time_progress`/`avg_per_month` follow the override; clearing the override returns them to the derived values

#### Scenario: Inputs absent

- **WHEN** no `start_date`, `saved`, or salary is stored
- **THEN** `month_now`, `coming_save`, `can_use`, and every derived figure are absent rather than zero

#### Scenario: Challenge past its end

- **WHEN** the derived `month_now` exceeds `target_months`
- **THEN** `months_left` is `1`, `time_progress` exceeds `1`, and `coming_save` still derives from the full remaining gap

### Requirement: Money Master card

The 總覽 page SHALL show a `Money Master` card mirroring the sheet's layout: a `now` row and a `target` row carrying month · saved · avg, a progress row showing `time_progress`, `saved_progress`, and `progress_gap` as percentages, a `comming save per month` row, and a `can use` row — each figure rendered as `—` while absent. `can use` SHALL render in the negative/red style while below zero.

#### Scenario: Card renders the block

- **WHEN** the block reports `month_now` `36`, `saved` `1094405.06`, `avg_per_month` `30400.14`, `time_progress` `1.0`, `saved_progress` `1.094`, `progress_gap` `0.094`, `coming_save` `-41705.06`, and `can_use` `94405.06`
- **THEN** the card shows both rows, the progress percentages, and the two money figures

#### Scenario: Nothing stored

- **WHEN** no Money Master settings exist
- **THEN** the card shows `—` for every figure
