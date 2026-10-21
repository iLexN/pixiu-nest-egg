# Spec Delta

## MODIFIED Requirements

### Requirement: Shared running averages

The block SHALL carry:

- `saved_avg`/`interest_avg`/`total_change_avg`: the `'Month Stat'` running averages (`L8`/`N8`/`C8`, monthly) feeding the `saving_year` AVERAGE and the formula-goal paces — echoed absent while no qualifying month exists, so the card can expand `存/年` and label each goal's pace without a second fetch.

#### Scenario: Averages echoed

- **WHEN** qualifying months exist
- **THEN** `saved_avg`/`interest_avg`/`total_change_avg` are echoed so the card can expand `存/年` and label each goal's pace

### Requirement: 目標 card

The 總覽 page SHALL show a `目標` card: every goal row carries target, a `基數` cell naming the figure `remain` subtracts, remain, a `步伐` cell naming the pace `years` divides by, years, and the ETA date — the `基數`/`步伐` cells each also showing their live value. The shared current invest value, saving per year / per month, and passive return per year / per month SHALL also show in the table footer.

#### Scenario: Card renders the block

- **WHEN** the block reports both milestones with targets, a shared `current_invest`/`saving_*`/`passive_return_*`, `years`/`eta` on each, and both formula goals with target/remain/years/eta
- **THEN** the card shows all four rows with those figures and the `N4` caption naming its year

#### Scenario: Rows name their base and pace

- **WHEN** the block reports all four goals with live values
- **THEN** the 1st/2nd rows label `基數` 現值 and `步伐` 存/年, the 3rd row labels 流動資產 and `(歷年存+息)×12`, and the Final row labels 總數 and `歷年總變化×12` — each cell showing that term's live value

#### Scenario: Footnote expands the formula targets

- **WHEN** `living_budget` is `14900`, `goal3_inflation` `1.07`, `goal3_years` `30`, `salary` `52700`, `final_replacement` `0.7`, and `n4_rate` `0.03851965006`
- **THEN** the footnote shows `14,900.00`, `1.07`, `30`, `52,700.00`, `70%`, and `3.852%` in place

#### Scenario: Footnote expands 存/年

- **WHEN** `averages.saved` is `30013.27`, `invest_targets.avg_invested` `326841.22`, `money_master.avg_per_month` `30400.14`, `saved_avg` `26362.84`, and `interest_avg` `4480.90`
- **THEN** the footnote shows `360,159.24`, `326,841.22`, `364,801.68`, `316,354.08`, and `370,124.88` in place

#### Scenario: Reached goal

- **WHEN** the 1st milestone's `remain` is `0` or negative
- **THEN** its years and date cells show `已達成`

#### Scenario: Nothing derivable

- **WHEN** every goals input is absent
- **THEN** the card still renders with `—` for each figure
