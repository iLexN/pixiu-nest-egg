# Spec Delta

## MODIFIED Requirements

### Requirement: Goals block

`GET /api/overview` SHALL return a `goals` block mirroring the sheet's `J10:M20` milestone tracker and `N13:P16` formula targets. Every figure derives on read and is absent while its inputs are missing or its divisor is zero; the block never reports a figure the sheet would show as an error or a blank.

Shared figures:

- `n4_rate`/`n4_year` (`Overview!N4`): the `income_value_rate` of the latest completed year's YearInReview row — the year before today's — both absent while that row or its rate is missing.
- `current_invest` (`K13`): 港股 market value + 債券 active principal + the IBKR computed HKD total; absent while the IBKR total is absent (no USD→HKD rate).
- `saving_year` (`L12`): the mean of the present terms among `averages.saved × 12`, `invest_targets.avg_invested`, `money_master.avg_per_month × 12`, running `saved_avg × 12`, and `(running saved_avg + interest_avg) × 12`, absent terms skipped — matching the sheet's `AVERAGE`; absent while no term is present. `saving_month` (`M12`) = `saving_year ÷ 12`.
- `passive_return_year` (`L13`) = `current_invest × n4_rate`; `passive_return_month` (`M13`) = ÷ 12.
- `goal3_inflation`/`goal3_years`/`final_replacement`: the effective `overview.goal3_*`/`overview.final_replacement` constants feeding `third.target`/`final_goal.target`, reporting the `1.07`/`30`/`0.7` defaults while unset — echoed so the card can expand the formulas without a second fetch.

Each milestone row (`milestones[0]`/`milestones[1]` = the sheet's 1st/2nd 目標):

- `target` (`K12`/`K18`): the stored `overview.goal1_amount`/`overview.goal2_amount`; absent while the setting is unset, which blanks the whole milestone's figures.
- `remain` (`K14`/`K20`) = `target − current_invest`.
- `years` (`L14`/`L20`) = `remain ÷ saving_year`.
- `eta` (`J11`/`J17`) = today + `round(365 × years)` days, `YYYY-MM-DD`.

Each formula goal (`third` = `O14:O16`, `final_goal` = `P14:P16`):

- `third.target` (`O14`) = `averages.living_budget × overview.goal3_inflation × 12 × overview.goal3_years`; `third.remain` (`O15`) = `target − liquid_assets`; `third.years` (`O16`) = `remain ÷ ((running saved_avg + interest_avg) × 12)`; `third.eta` = today + `round(365 × years)` days, `YYYY-MM-DD`.
- `final_goal.target` (`P14`) = `salary × overview.final_replacement × 12 ÷ n4_rate`; `final_goal.remain` (`P15`) = `target − total_assets`; `final_goal.years` (`P16`) = `remain ÷ (running total_change_avg × 12)`; `final_goal.eta` = today + `round(365 × years)` days, `YYYY-MM-DD`.

Unlike the sheet, a goal whose `remain ≤ 0` is already reached: its `years` and `eta` SHALL be absent rather than negative or a past date.

#### Scenario: Sheet reproduction

- **WHEN** `current_invest` is `1495563.431`, the saving terms are `30013.27` (monthly saved), `326841.2233` (avg invested), `30400.14056` (money-master monthly), `26362.84273` (running saved) and `4480.899118` (running interest), `n4_rate` `0.03851965006`, the milestone targets `1550000`/`2500000`, budget `14900` with `1.07`/`30`, liquid assets `1951776.034`, salary `52700` with `0.7`, total assets `3579321.076`, and running total change `59918.15472`
- **THEN** `saving_year` ≈ `347656.233`, `saving_month` ≈ `28971.35275`, `passive_return_year` ≈ `57608.58002`, the 1st milestone reports `remain` ≈ `54436.56857` and `years` ≈ `0.1565815982`, the 2nd `remain` ≈ `1004436.569` and `years` ≈ `2.889166002`, `third.target` ≈ `5739480`, `third.remain` ≈ `3787703.966`, `third.years` ≈ `10.23358316`, `final_goal.target` ≈ `11492316.24`, `final_goal.remain` ≈ `7912995.166`, and `final_goal.years` ≈ `11.00528324`

#### Scenario: Constants echoed

- **WHEN** `overview.goal3_inflation` and `overview.goal3_years` are unset and `overview.final_replacement` is `0.6`
- **THEN** the block reports `goal3_inflation` `1.07`, `goal3_years` `30`, and `final_replacement` `0.6`

#### Scenario: Milestone ETA

- **WHEN** a milestone reports `years` `0.4383561644`
- **THEN** `eta` is today + `round(365 × 0.4383561644)` days, `YYYY-MM-DD`

#### Scenario: Missing inputs

- **WHEN** the IBKR rate is unset so `current_invest` is absent
- **THEN** both milestones report absent `remain`/`years`/`eta`, `passive_return_*` is absent, and the formula goals still derive from their own inputs

#### Scenario: Goal reached

- **WHEN** a milestone's `target` is below `current_invest` (or any goal's `remain ≤ 0`)
- **THEN** `remain` still reports the negative figure while `years` and `eta` are absent, marking the goal reached

#### Scenario: Cleared milestone amount

- **WHEN** `overview.goal1_amount` is unset
- **THEN** the 1st milestone reports absent `target`/`remain`/`years`/`eta` while the 2nd milestone and formula goals are unaffected

### Requirement: 目標 card

The 總覽 page SHALL show a `目標` card mirroring the sheet's layout: for the 1st/2nd milestones a row carrying target, remain, years, and the ETA date; for the 3rd and Final targets a row carrying target, remain, years, and the ETA date. The shared current invest value, saving per year / per month, and passive return per year / per month SHALL show once in the table footer rather than repeating on each milestone row. The card SHALL caption the `N4` rate with the year it belongs to. A goal whose `remain ≤ 0` SHALL show `已達成` instead of years/eta, and every absent figure renders as `—`. A muted footnote SHALL expand the two formula targets with live values — `3rd = 生活預算 (…) × 通脹 (…) × 12 × 年期 (…)` and `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` — each input's current figure in parentheses, `—` while an input is absent.

#### Scenario: Card renders the block

- **WHEN** the block reports both milestones with targets, a shared `current_invest`/`saving_*`/`passive_return_*`, `years`/`eta` on each, and both formula goals with target/remain/years/eta
- **THEN** the card shows all four rows with those figures and the `N4` caption naming its year

#### Scenario: Footnote expands the formula targets

- **WHEN** `living_budget` is `14900`, `goal3_inflation` `1.07`, `goal3_years` `30`, `salary` `52700`, `final_replacement` `0.7`, and `n4_rate` `0.03851965006`
- **THEN** the footnote shows `14,900.00`, `1.07`, `30`, `52,700.00`, `70%`, and `3.852%` in place

#### Scenario: Reached goal

- **WHEN** the 1st milestone's `remain` is `0` or negative
- **THEN** its years and date cells show `已達成`

#### Scenario: Nothing derivable

- **WHEN** every goals input is absent
- **THEN** the card still renders with `—` for each figure
