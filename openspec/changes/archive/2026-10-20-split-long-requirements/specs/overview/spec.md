# Spec Delta

## MODIFIED Requirements

### Requirement: Overview aggregation endpoint

`GET /api/overview` SHALL return the sheet's `A3:C18` block plus its headline figures, all derived on read: `total_assets` (總數, `B1`), `liquid_assets` (流動資產, `H1`), `liquid_ratio` (`J1` = `liquid_assets` ÷ (salary × 100), absent while no salary is stored), `assets` (the `A3:B9` rows), `assets_sum` (`B10`), `semi_liquid` (the `A14:C18` block plus the `A13` ratio), `ibkr` (the 美股 `A1:B5` block with derived figures), `salary`, and `rate` (the stored USD fx rate, absent while unset).

#### Scenario: Aggregation returned

- **WHEN** the user opens the 總覽 page with all modules populated and the rate stored
- **THEN** the response carries the headline totals, every asset row with its share, the 半流動資金 block, and the IBKR block

#### Scenario: No rate stored

- **WHEN** no USD fx rate is stored
- **THEN** the 基金 and IBKR amounts, and the rate-dependent totals terms, are absent rather than zero

## ADDED Requirements

### Requirement: Rate-dependent overview figures

Figures needing the USD→HKD rate SHALL be absent while it is unset, consistent with the other modules.

#### Scenario: No rate blanks USD figures

- **WHEN** no USD fx rate is stored
- **THEN** overview figures needing USD→HKD are absent

## MODIFIED Requirements

### Requirement: 半流動資金 block

`semi_liquid` SHALL carry `deposits` (已定期 = Σ principal over active deposits — principal only, matching `定期!B1`), `cash_rows` (every `bank` cash account in `sort_order`, e.g. HS, 渣打 — broker accounts are excluded), `cash_sum` (活期 = Σ cash rows), and `total` (半流動資金 = `deposits` + `cash_sum`).

#### Scenario: Block derived

- **WHEN** active deposits hold principal 445000 and bank accounts HS 30538.78 + 渣打 1364.89
- **THEN** `cash_sum` is 31903.67 and `total` is 476903.67

#### Scenario: Broker cash stays out of 活期

- **WHEN** the IBKR HKD broker account holds `765.42`
- **THEN** it is not listed in `cash_rows` and does not change `cash_sum`

#### Scenario: Configured ratio applied

- **WHEN** `strategy.semi_liquid_target` is `0.3`, `total` is `476903.67`, and `liquid_assets` is `1962919.66`
- **THEN** `vs_quarter_liquid` is approximately `-111972.23`

## ADDED Requirements

### Requirement: Semi-liquid comparison figures

The block SHALL also carry `vs_quarter_liquid` (the sheet's `C14` = `total` − the configured `strategy.semi_liquid_target` ratio × `liquid_assets`; the sheet's `25%` literal promoted to the setting, `0.25` while unset), and `share` (the sheet's `A13` = `total` ÷ (港股 + 債券 + `total` + IBKR), absent while any term is missing or the divisor is zero).

#### Scenario: Share of semi-liquid

- **WHEN** 港股, 債券, `total` and IBKR figures are all present
- **THEN** `share` is `total` divided by their sum

## MODIFIED Requirements

### Requirement: IBKR account figures

The system SHALL store the IBKR app's displayed account total as the setting `ibkr.reported_value` (`B2`; the app uses a different FX rate than the stored USD fx rate). The cash positions SHALL be the amounts of the `broker` cash accounts: `hkd_cash` (`B4`) from the HKD broker account, `usd_cash` (`B5`) from the USD broker account.

#### Scenario: Computed total

- **WHEN** US stocks are worth 15813.73 USD, `usd_cash` is 1200.25, `hkd_cash` is 765.42, and the rate is 7.845135
- **THEN** `computed_total_hkd` is approximately 134242.39

#### Scenario: Net vs transferred

- **WHEN** `transferred_hkd` is 131000 and `now_value` is 134232.01
- **THEN** `net` is 3232.01 and `net_pct` is approximately 0.0247

#### Scenario: Clearing a field

- **WHEN** the user patches `usd_cash` to `null`
- **THEN** the USD broker account's amount becomes `0` and `computed_total_hkd` no longer includes any USD cash

#### Scenario: Cash edit is the account balance

- **WHEN** the user patches `usd_cash` to `1200.25`
- **THEN** `GET /api/cash-accounts` reports the USD broker account at `1200.25`

#### Scenario: Recording a transfer

- **WHEN** the user patches `transfer_hkd` `50000` with no `transfer_date`
- **THEN** a row dated today is appended to `ibkr_transfers` and `transferred_hkd` grows by `50000`

#### Scenario: Undoing a transfer

- **WHEN** the user patches `transfer_hkd` `-10000`
- **THEN** a negative row is appended and `transferred_hkd` drops by `10000`

## ADDED Requirements

### Requirement: IBKR transfers log

The system SHALL keep a dated `ibkr_transfers` log whose sum is `transferred_hkd` (美股 `B1`: cumulative bank→IBKR transfers in HKD, absent while the log is empty); each year's sum is the year's IBKR 轉入.

#### Scenario: Yearly transfer sum

- **WHEN** the log holds transfers dated in a year
- **THEN** that year's IBKR 轉入 is their sum

## ADDED Requirements

### Requirement: IBKR patch semantics

The IBKR figures are exposed through `GET /api/ibkr` and `PATCH /api/ibkr`, as `now_value`, `hkd_cash` and `usd_cash`. `now_value` SHALL accept a finite non-negative number, or `null` to clear. `hkd_cash`/`usd_cash` SHALL accept a finite non-negative number, which is written to the broker account, or `null`, which sets that account to `0`.

#### Scenario: Patch writes the broker account

- **WHEN** `PATCH /api/ibkr` carries `usd_cash` `1234.56`
- **THEN** the USD broker cash account's amount becomes `1234.56`

## ADDED Requirements

### Requirement: IBKR transfer entry

`transfer_hkd` SHALL append a log row for any finite delta (a negative value records a withdrawal or undoes a mistyped entry) dated `transfer_date`, defaulting to today. One PATCH SHALL apply all its fields in a single transaction.

#### Scenario: Negative delta undoes an entry

- **WHEN** `transfer_hkd` carries a negative delta
- **THEN** a log row is appended recording it

## ADDED Requirements

### Requirement: IBKR derived figures

The response SHALL derive:

- `computed_total_hkd` (美股 `B7` = (US market value + `usd_cash`) × rate + `hkd_cash`)
- `net` = `now_value` − `transferred_hkd`
- `net_pct` = `net` ÷ `transferred_hkd`
- `vs_now_value` = `computed_total_hkd` − `now_value`

Each is absent while its inputs are missing.

#### Scenario: Net versus transferred

- **WHEN** `now_value` exceeds `transferred_hkd`
- **THEN** `net` and `net_pct` report the gain

## MODIFIED Requirements

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with:

- a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance)
- the asset table (label, HKD amount, share — `—` while an amount is absent)
- the 半流動資金 block: 已定期, each bank cash row, 活期, and the 半流動資金 total.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

#### Scenario: Edit IBKR on 總覽

- **WHEN** the user sets `usd_cash` to `1200.25` on the 總覽 IBKR card and saves
- **THEN** the view reloads showing the updated IBKR block, asset-table IBKR row, and dependent totals

## ADDED Requirements

### Requirement: 半流動資金 display rules

The block SHALL follow: The total renders red while below the configured `strategy.semi_liquid_target` share of 流動資產 and green while above. A second footer row shows its `A13` share and `C14` difference, labelled with the configured percent.

#### Scenario: Total below target shows red

- **WHEN** the 半流動資金 total is below the configured share of 流動資產
- **THEN** it renders red

## ADDED Requirements

### Requirement: Overview IBKR block and inline editing

- the IBKR block (stored inputs plus the derived figures)

Manual `asset` row amounts SHALL be editable inline via the manual-assets endpoints, and bank cash row amounts via the cash-accounts endpoints. The view SHALL reload after every mutation.

#### Scenario: Cash row edit hits cash-accounts API

- **WHEN** the user edits a bank cash row amount inline
- **THEN** the change saves through the cash-accounts endpoints and the view reloads

## ADDED Requirements

### Requirement: Overview IBKR editable card

The IBKR block SHALL be the same editable card as the 美股 summary: editable `transferred_hkd` delta + date, `now_value`, `hkd_cash`, and `usd_cash` inputs saving through `PATCH /api/ibkr`, beside the derived figures. The view SHALL reload after a save so the asset-table IBKR row and every dependent figure refresh.

#### Scenario: IBKR card save refreshes dependents

- **WHEN** the user saves the IBKR card
- **THEN** the view reloads so the asset-table IBKR row and every dependent figure refresh

## MODIFIED Requirements

### Requirement: 過去 12 個月平均 block

`GET /api/overview` SHALL return an `averages` block mirroring the sheet's `F3:G10` (+`H6`): trailing averages of the per-month 總數增加 (Changed), 支出 (month_spend), 生活支出 (living_spend), 存 (saved), and 利息 (interest) over the 12 most recent month rows whose `month` precedes the current month; the live 開心Pool balance; and `living_budget` = `ROUNDUP(living_spend_avg × 1.05, −2)` (up to the nearest 100).

#### Scenario: Trailing window

- **WHEN** the current month is `2026-09` and months `2025-09` through `2026-08` are stored
- **THEN** each average covers exactly those twelve months, skipping per-column months with no value

#### Scenario: Current month excluded

- **WHEN** the current month's row exists but its spend columns are not yet final
- **THEN** no figure in the block includes the current month

#### Scenario: Budget derived

- **WHEN** the 生活支出 average is `14119.67`
- **THEN** `living_budget` is `14900`, and `living_budget_low` is true only while `liquid_assets × 0.003 + 9000` exceeds `14900`

## ADDED Requirements

### Requirement: Averages skip semantics and low flag

Each average SHALL skip months where that column has no value (AVERAGE semantics), and SHALL be absent while the window holds no values for it. `living_budget_low` SHALL be true when `living_budget < liquid_assets × 0.0001 × 30 + 9000`, and the response SHALL carry that floor value for display.

#### Scenario: Empty window gives absent average

- **WHEN** the window holds no values for a column
- **THEN** its average is absent

## MODIFIED Requirements

### Requirement: 投資目標 block

`GET /api/overview` SHALL return an `invest_targets` block mirroring the sheet's `J22:N27`: `avg_invested` (the sheet's `J22`) and `rows`, one per year covered by the year-review row set, ordered ascending. Each row SHALL carry `year`, `invested` (the year-review row's `invested`, absent while none), the year's effective `raise`, `interest` (the year-review row's 利息回報), `pool_spend` (the year-review row's 開心Pool支出), `target`, `remain`, and `growth` per the formula requirement below.

#### Scenario: Rows cover the year-review years

- **WHEN** `GET /api/year-review` returns rows for 2023–2026 and the current year is 2026
- **THEN** the block lists exactly those four years ascending

#### Scenario: Rows carry the formula inputs

- **WHEN** a year reports `interest` `62753.15` and `pool_spend` `9593` on its year-review row
- **THEN** that year's `invest_targets` row carries `interest` `62753.15` and `pool_spend` `9593`

#### Scenario: Average over the last three completed years

- **WHEN** the current year is 2026 and `invested` is `206523.15` (2023), `345365.11` (2024), and `428635.41` (2025)
- **THEN** `avg_invested` is approximately `326841.22`

#### Scenario: Average skips absent years

- **WHEN** only two of the three most recent completed years report `invested`
- **THEN** `avg_invested` is the mean over those two

## ADDED Requirements

### Requirement: Invested average and absent-as-zero

`interest`/`pool_spend` SHALL be `0` when the year reports none — the same count-absent-as-0 semantics the target formula applies. `avg_invested` SHALL be the mean of `invested` over the rows for the three most recent completed years (years before the current year), skipping rows whose `invested` is absent, and SHALL be absent while no completed year reports `invested`.

#### Scenario: Average over completed years

- **WHEN** only one completed year reports `invested`
- **THEN** `avg_invested` is that figure

## MODIFIED Requirements

### Requirement: 投資目標 formula

For each row the system SHALL derive `target` = `invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7 + raise(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7`, reading `invested`/`interest`/`pool_spend`/`raise` off the stated years' year-review figures; `target` SHALL be absent while the prior year's `invested` is absent, and absent `interest`/`pool_spend`/`raise` inputs SHALL count as `0`.

#### Scenario: Current-year target derived

- **WHEN** 2025 reports `invested` `428635.41`, `interest` `51900.65`, `pool_spend` `38729.5`, and 2026 reports effective `raise` `1890`, `interest` `62753.15`, `pool_spend` `9593`, and `invested` `182935.39`
- **THEN** the 2026 row reports `target` ≈ `430432.36`, `remain` ≈ `247496.97`, and `growth` ≈ `0.0042`

#### Scenario: Completed-year growth

- **WHEN** 2024 and 2025 report `invested` `345365.11` and `428635.41`
- **THEN** the 2025 row reports `growth` ≈ `0.2411` and `remain` is absent

#### Scenario: First tracked year

- **WHEN** 2023 is the earliest row
- **THEN** its `target`, `remain`, and `growth` are absent while `invested` still reports

## ADDED Requirements

### Requirement: Remain and growth figures

`remain` SHALL be `target − invested` on the current year's row only, absent on completed years and while either input is missing. `growth` SHALL be `(invested(Y) − invested(Y-1)) ÷ invested(Y-1)` for completed years and `(target(Y) − invested(Y-1)) ÷ invested(Y-1)` for the current year, absent while either input is missing or the divisor is zero.

#### Scenario: Remain only on the current year

- **WHEN** a completed year's row is rendered
- **THEN** `remain` is absent

## MODIFIED Requirements

### Requirement: 投資目標 card

The 總覽 page SHALL show an 投資目標 card with the `avg_invested` figure (近3年平均) as its header stat and a per-year table of 年份, `invested`, `target`, `remain`, and `growth` rendered as a percentage — `—` while a figure is absent.

#### Scenario: Card renders the block

- **WHEN** the block returns four year rows and an `avg_invested`
- **THEN** the card shows the average and one table row per year, rendering absent figures as `—`

#### Scenario: Footnote expands the current-year target

- **WHEN** the current year is 2026, its row reports `raise` `1890`, `interest` `62753.15`, `pool_spend` `9593`, and the 2025 row reports `invested` `428635.41`, `interest` `51900.65`, `pool_spend` `38729.5`
- **THEN** the footnote shows each term's contribution: `428,635.41`, `51,900.65`, `27,110.65`, `11,340.00`, `62,753.15`, `6,715.10`

#### Scenario: Footnote hidden without a current-year row

- **WHEN** the block lists no row for the current year
- **THEN** the card shows no formula footnote

## ADDED Requirements

### Requirement: Target formula footnote

A muted footnote SHALL expand the current year's `target` into its formula terms — `上年invested`, `上年利息回報`, `上年Pool支出×0.7`, `月薪增幅×50%*12`, `今年利息回報`, `今年Pool支出×0.7` — each followed by its computed contribution in parentheses, `—` while an input is absent; an absent `raise` counts as `0`. The footnote SHALL be absent while no current-year row exists.

#### Scenario: Footnote lists the terms

- **WHEN** a current-year row exists
- **THEN** the footnote shows each formula term with its computed contribution

## MODIFIED Requirements

### Requirement: Goals block

`GET /api/overview` SHALL return a `goals` block mirroring the sheet's `J10:M20` milestone tracker and `N13:P16` formula targets. Every figure derives on read and is absent while its inputs are missing or its divisor is zero; the block never reports a figure the sheet would show as an error or a blank.

#### Scenario: Sheet reproduction

- **WHEN** `current_invest` is `1495563.431`, the saving terms are `30013.27` (monthly saved), `326841.2233` (avg invested), `30400.14056` (money-master monthly), `26362.84273` (running saved) and `4480.899118` (running interest), `n4_rate` `0.03851965006`, the milestone targets `1550000`/`2500000`, budget `14900` with `1.07`/`30`, liquid assets `1951776.034`, salary `52700` with `0.7`, total assets `3579321.076`, and running total change `59918.15472`
- **THEN** `saving_year` ≈ `347656.233`, `saving_month` ≈ `28971.35275`, `passive_return_year` ≈ `57608.58002`, the 1st milestone reports `remain` ≈ `54436.56857` and `years` ≈ `0.1565815982`, the 2nd `remain` ≈ `1004436.569` and `years` ≈ `2.889166002`, `third.target` ≈ `5739480`, `third.remain` ≈ `3787703.966`, `third.years` ≈ `10.23358316`, `final_goal.target` ≈ `11492316.24`, `final_goal.remain` ≈ `7912995.166`, and `final_goal.years` ≈ `11.00528324`

#### Scenario: Constants echoed

- **WHEN** `goals.goal3_inflation` and `goals.goal3_years` are unset and `goals.final_replacement` is `0.6`
- **THEN** the block reports `goal3_inflation` `1.07`, `goal3_years` `30`, and `final_replacement` `0.6`

#### Scenario: Running averages echoed

- **WHEN** the stored month rows produce running averages `saved_avg` `26362.84` and `interest_avg` `4480.90`
- **THEN** the block carries `saved_avg` `26362.84` and `interest_avg` `4480.90`

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

- **WHEN** `goals.goal1_amount` is unset
- **THEN** the 1st milestone reports absent `target`/`remain`/`years`/`eta` while the 2nd milestone and formula goals are unaffected

## ADDED Requirements

### Requirement: Shared rate and current invest figures

The block SHALL carry these shared figures:

- `n4_rate`/`n4_year` (`Overview!N4`): the `income_value_rate` of the latest completed year's YearInReview row — the year before today's — both absent while that row or its rate is missing.
- `current_invest` (`K13`): 港股 market value + 債券 active principal + the IBKR computed HKD total; absent while the IBKR total is absent (no USD→HKD rate).

#### Scenario: N4 absent without prior year

- **WHEN** the latest completed year's YearInReview row has no `income_value_rate`
- **THEN** `n4_rate` and `n4_year` are absent

## ADDED Requirements

### Requirement: Saving-year figure

The block SHALL carry:

- `saving_year` (`L12`): the mean of the present terms among `averages.saved × 12`, `invest_targets.avg_invested`, `money_master.avg_per_month × 12`, running `saved_avg × 12`, and `(running saved_avg + interest_avg) × 12`, absent terms skipped — matching the sheet's `AVERAGE`; absent while no term is present. `saving_month` (`M12`) = `saving_year ÷ 12`.

#### Scenario: Saving-year averages present terms

- **WHEN** two of the five `saving_year` terms are present
- **THEN** `saving_year` is the mean of those two

## ADDED Requirements

### Requirement: Passive return and goal constants

The block SHALL carry:

- `passive_return_year` (`L13`) = `current_invest × n4_rate`; `passive_return_month` (`M13`) = ÷ 12.
- `goal3_inflation`/`goal3_years`/`final_replacement`: the effective `goals.goal3_*`/`goals.final_replacement` constants feeding `third.target`/`final_goal.target`, reporting the `1.07`/`30`/`0.7` defaults while unset — echoed so the card can expand the formulas without a second fetch.

#### Scenario: Defaults while unset

- **WHEN** no goal constants are stored
- **THEN** the block reports the `1.07`/`30`/`0.7` defaults

## ADDED Requirements

### Requirement: Shared running averages

The block SHALL carry:

- `saved_avg`/`interest_avg`: the `'Month Stat'` running averages (`L8`/`N8`, monthly) feeding the `saving_year` AVERAGE and the formula-goal paces — echoed absent while no qualifying month exists, so the card can expand `存/年` without a second fetch.

#### Scenario: Averages echoed

- **WHEN** qualifying months exist
- **THEN** `saved_avg`/`interest_avg` are echoed so the card can expand `存/年`

## ADDED Requirements

### Requirement: Milestone figures

Each milestone row (`milestones[0]`/`milestones[1]` = the sheet's 1st/2nd 目標) SHALL carry:

- `target` (`K12`/`K18`): the stored `goals.goal1_amount`/`goals.goal2_amount`; absent while the setting is unset, which blanks the whole milestone's figures.
- `remain` (`K14`/`K20`) = `target − current_invest`.
- `years` (`L14`/`L20`) = `remain ÷ saving_year`.
- `eta` (`J11`/`J17`) = today + `round(365 × years)` days, `YYYY-MM-DD`.

#### Scenario: Milestone ETA from years

- **WHEN** `remain` is positive and `saving_year` is present
- **THEN** `eta` is today plus `round(365 × years)` days

## ADDED Requirements

### Requirement: Third formula goal

Each formula goal (`third` = `O14:O16`, `final_goal` = `P14:P16`) SHALL carry for `third`:

- `third.target` (`O14`) = `averages.living_budget × goals.goal3_inflation × 12 × goals.goal3_years`; `third.remain` (`O15`) = `target − liquid_assets`; `third.years` (`O16`) = `remain ÷ ((running saved_avg + interest_avg) × 12)`; `third.eta` = today + `round(365 × years)` days, `YYYY-MM-DD`.

#### Scenario: Third goal from the budget

- **WHEN** `averages.living_budget` is present
- **THEN** `third.target` is `living_budget × 1.07 × 12 × 30` (defaults while unset)

## ADDED Requirements

### Requirement: Final formula goal and reached goals

- `final_goal.target` (`P14`) = `salary × goals.final_replacement × 12 ÷ n4_rate`; `final_goal.remain` (`P15`) = `target − total_assets`; `final_goal.years` (`P16`) = `remain ÷ (running total_change_avg × 12)`; `final_goal.eta` = today + `round(365 × years)` days, `YYYY-MM-DD`.

Unlike the sheet, a goal whose `remain ≤ 0` is already reached: its `years` and `eta` SHALL be absent rather than negative or a past date.

#### Scenario: Reached goal shows no ETA

- **WHEN** a goal's `remain` is `0` or negative
- **THEN** its `years` and `eta` are absent

## MODIFIED Requirements

### Requirement: Goal settings

The settings SHALL carry the five goal inputs as `app_meta` keys:

- `goals.goal1_amount` and `goals.goal2_amount`: the milestone targets, the sheet's `K12`/`K18` literals
- `goals.goal3_inflation`: the sheet's `1.07`
- `goals.goal3_years`: the sheet's `25+5`
- `goals.final_replacement`: the sheet's `0.7`

#### Scenario: Constants round-trip through defaults

- **WHEN** no constant is stored
- **THEN** settings report `goal3_inflation` `1.07`, `goal3_years` `30`, `final_replacement` `0.7`; patching `goal3_inflation` `1.05` returns `1.05`, and patching `null` returns `1.07` again

#### Scenario: Validation

- **WHEN** the patch carries `goal1_amount` `-1` or `goal3_years` `0`
- **THEN** the request is rejected with a field error and nothing is written

#### Scenario: Clearing a milestone amount

- **WHEN** `goal1_amount` was `1550000` and the patch carries `null`
- **THEN** settings report `goal1_amount` `null` and the overview's 1st milestone is blank

## ADDED Requirements

### Requirement: Goal settings endpoints

- `GET /api/settings` SHALL return the two amounts as stored (`null` while unset) and the three constants as their effective values (the defaults while unset).
- `PATCH /api/settings` SHALL accept each field: the amounts must be finite and ≥ 0, the constants finite and > 0; invalid values are rejected with a field error. `null` clears an amount (blanking that milestone's figures) or resets a constant to its default; an absent field leaves the stored value untouched.

#### Scenario: Null clears a milestone amount

- **WHEN** `goals.goal1_amount` is patched to `null`
- **THEN** that milestone's figures blank

## MODIFIED Requirements

### Requirement: 目標 card

The 總覽 page SHALL show a `目標` card mirroring the sheet's layout: for the 1st/2nd milestones a row carrying target, remain, years, and the ETA date; for the 3rd and Final targets a row carrying target, remain, years, and the ETA date. The shared current invest value, saving per year / per month, and passive return per year / per month SHALL show once in the table footer rather than repeating on each milestone row.

#### Scenario: Card renders the block

- **WHEN** the block reports both milestones with targets, a shared `current_invest`/`saving_*`/`passive_return_*`, `years`/`eta` on each, and both formula goals with target/remain/years/eta
- **THEN** the card shows all four rows with those figures and the `N4` caption naming its year

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

## ADDED Requirements

### Requirement: Goal card reached and caption rules

The card SHALL caption the `N4` rate with the year it belongs to. A goal whose `remain ≤ 0` SHALL show `已達成` instead of years/eta, and every absent figure renders as `—`.

#### Scenario: Reached goal displays 已達成

- **WHEN** a goal's `remain` is `0` or negative
- **THEN** the row shows `已達成` instead of years/eta

## ADDED Requirements

### Requirement: Goal card formula footnotes

A muted footnote SHALL expand the two formula targets with live values — `3rd = 生活預算 (…) × 通脹 (…) × 12 × 年期 (…)` and `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` — each input's current figure in parentheses, `—` while an input is absent. The footnote SHALL likewise expand `存/年` as `AVERAGE` over its five yearly terms — `近12月存×12`, `近3年invested`, `MM月均×12`, `歷年存×12`, `(歷年存+息)×12` — each with its live contribution in parentheses, `—` while a term is absent.

#### Scenario: Footnote shows live inputs

- **WHEN** the 目標 card renders with a stored `final_replacement`
- **THEN** the footnote shows `Final = 薪金 (…) × 替代率 (…) × 12 ÷ N4 (…)` with the live figures in parentheses

## MODIFIED Requirements

### Requirement: 策略 liquidity tiers block

`GET /api/overview` SHALL return a `liquidity_tiers` block mirroring the sheet's `J3:K7`, all derived on read:

- `cannot_use` (`K5`) = the stored salary × 6, the sheet's `N6` 6-month salary
- `can_use` (`K4`) = `semi_liquid.total` − `cannot_use`, negative allowed
- `short_term` (`K6`) = 港股 + 債券 + IBKR + Σ manual `asset` rows with `liquidity` `short`
- `long_term` (`K7`) = 基金 + MPF + Σ manual `asset` rows with `liquidity` `long`

#### Scenario: Tiers derived

- **WHEN** the salary is `52700`, 半流動資金 is `477023.67`, 港股 + 債券 + IBKR total `1504503.865`, and 基金 + MPF + the long-term row HS人壽 total `1605802.914`
- **THEN** `cannot_use` is `316200`, `can_use` is approximately `160823.67`, `short_term` is approximately `1504503.87`, and `long_term` is approximately `1605802.91`

#### Scenario: Manual row placed by liquidity

- **WHEN** a manual `asset` row Irene `20000` has `liquidity` `short` and HS人壽 `69440.47` has `liquidity` `long`
- **THEN** `short_term` includes `20000` and `long_term` includes `69440.47`

#### Scenario: No salary stored

- **WHEN** `ledger.salary` is unset
- **THEN** `cannot_use` and `can_use` are absent while `short_term` and `long_term` still report

#### Scenario: No rate stored

- **WHEN** the USD fx rate is unset
- **THEN** `short_term` and `long_term` are absent rather than partial sums

#### Scenario: Buffer exceeds 半流動資金

- **WHEN** 半流動資金 is `300000` and the salary is `52700`
- **THEN** `can_use` is `-16200`

## ADDED Requirements

### Requirement: Liquidity tier absence rules

`cannot_use` and `can_use` SHALL be absent while no salary is stored. `short_term` and `long_term` SHALL be absent while the USD fx rate is unset, because their IBKR and 基金 terms need it. While every figure is present, `can_use + cannot_use + short_term + long_term` SHALL equal `total_assets`.

#### Scenario: Tiers sum to total

- **WHEN** every tier figure is present
- **THEN** `can_use + cannot_use + short_term + long_term` equals `total_assets`

## MODIFIED Requirements

### Requirement: Forecast projection block

`GET /api/forecast` SHALL return a `months` array mirroring the sheet's `預測` grid `A20:H36`: seven entries for the current calendar month through current +6, each carrying `month` (`YYYY-MM-01`) and the sheet's rows as derived fields — `start`, `salary`, `spend`, `deposit_finish`, `interest`, `bill`, `plan_items`, `deposit_placements`, `deposit_return`, `cash`, `locked`, `semi_liquid`, and `ref_check`.

#### Scenario: First column anchors on 月初

- **WHEN** the current month `2026-09` has `start_cash` `23746.72`, salary `52700`, `living_budget` `14900`, and no deposits, plans, bills or receipts in the month
- **THEN** month `2026-09` reports `start` `23746.72`, `salary` `52700`, `spend` `-14900`, and `cash` approximately `61546.72`

#### Scenario: Start falls back to live cash

- **WHEN** the current month has no stored `start_cash` and the `cash` manual rows total `32934.62`
- **THEN** the first month's `start` is `32934.62`

#### Scenario: Chained start

- **WHEN** month `2026-09` derives `cash` `61546.72`
- **THEN** month `2026-10` reports `start` `61546.72` regardless of `2026-10`'s own `start_cash`

#### Scenario: Deposit finish and interest

- **WHEN** deposits SC-9632 `110000` and SC-7006 `35000` end `2026-10-12` with interest `993` and `285`, and a `145`-HKD dividend pays in `2026-10`
- **THEN** `2026-10` reports `deposit_finish` `145000` and `interest` includes `993 + 285 + 1423` equivalents — `interest` totals `993 + 285 + 145` plus any coupons due that month

#### Scenario: Real deposit placed in the month

- **WHEN** deposit `9077` with `principal` `55000`, bank `SC`, `start_date` `2026-09-30` exists and the window starts at `2026-09`
- **THEN** `2026-09` lists it in `deposit_placements` and its `cash` is `55000` lower than it would be without the deposit

#### Scenario: Quarter month bill

- **WHEN** `forecast.bill_amount` is `2158` and the window covers `2027-01`
- **THEN** `2027-01` reports `bill` `-2158` and `2027-02` reports no `bill`

## ADDED Requirements

### Requirement: Forecast start, salary and spend rows

Per month `m`:

- `start` SHALL be the current month's stored `start_cash` (月初出糧後) when `m` is the first month — falling back to the live 活期 sum (Σ `bank` cash account balances) while no `start_cash` is stored — and the previous month's `cash` otherwise; absent while its source is absent.
- `salary` SHALL be the stored `ledger.salary`, absent while unset.
- `spend` SHALL be −`living_budget` from the overview averages block, absent while no budget exists.

#### Scenario: Next month starts from previous cash

- **WHEN** month `m` is not the first month
- **THEN** its `start` is the previous month's `cash`

## ADDED Requirements

### Requirement: Forecast deposit finish and bill rows

For each forecast month `m`:

- `deposit_finish` SHALL be Σ `principal` of deposits whose `end_date` falls in `m` (the sheet's `定期 finish` row sourced from `2026回報率`/`定期Info` SUMIFs), `0` when none.
- `bill` SHALL be − the stored `forecast.bill_amount` setting on Jan/Apr/Jul/Oct months (the sheet's 差餉 quarters reading `B39`), absent on other months; when one or more `bill` forecast items exist for `m`, `bill` SHALL be their Σ instead (an override, not an addition).

#### Scenario: Bill only on quarter months

- **WHEN** `m` is not Jan, Apr, Jul or Oct and no `bill` items exist
- **THEN** `bill` is absent

## ADDED Requirements

### Requirement: Forecast interest figure

For each forecast month `m`:

- `interest` SHALL be Σ `interest` of deposits ending in `m` + Σ HK dividend amounts with a pay/record date in `m` + Σ bond coupon amounts due or received in `m` + Σ `interest`-kind forecast items in `m` — counting known expected or received amounts and skipping components with no known amount, `0` when none.

#### Scenario: Interest sums the components

- **WHEN** a month has a deposit ending and a received coupon
- **THEN** `interest` sums their amounts

## ADDED Requirements

### Requirement: Forecast interest components

The month entry SHALL also carry `interest_components` listing the auto receipts (source `deposit`/`coupon`/`dividend`, label, amount, `received`) so the breakdown behind the figure is inspectable.

#### Scenario: Components inspectable

- **WHEN** a month has auto receipts behind `interest`
- **THEN** `interest_components` lists each receipt

## ADDED Requirements

### Requirement: Forecast plan and placement rows

For each forecast month `m`:

- `plan_items` SHALL list the month's stored forecast items.
- `deposit_placements` SHALL list the real deposits whose `start_date` falls in `m` — each with `deposit_id`, `label`, `bank`, `principal`, and `start_date` — empty when none. Deposits without a `start_date` are never placements.
- `deposit_return` SHALL be the sum described by the deposit-returns requirement, `0` when none.

#### Scenario: Placement requires a start date

- **WHEN** a deposit has no `start_date`
- **THEN** it never appears in `deposit_placements`

## ADDED Requirements

### Requirement: Forecast cash figure

For each forecast month `m`:

- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m` − Σ `principal` of `deposit_placements` in `m`, absent while `start` is absent; absent components count as `0`. (`interest` and `bill` items are already inside `interest`/`bill`, so they are not summed again.)

#### Scenario: Cash chains the rows

- **WHEN** `start` is present
- **THEN** `cash` derives as the stated sum with absent components counting as `0`

## ADDED Requirements

### Requirement: Forecast ref check

`ref_check` SHALL be `semi_liquid − the configured `strategy.semi_liquid_target` ratio × liquid_assets` using the live `liquid_assets` (the sheet's `B20:H20` against `$N$7` — `H1 ÷ 4`, the same threshold as `C14` written as a division), absent while either side is absent.

#### Scenario: Ref check absent without liquid assets

- **WHEN** `liquid_assets` is absent
- **THEN** `ref_check` is absent

## MODIFIED Requirements

### Requirement: Locked 定期+SC chain

Each forecast month SHALL derive `locked` (the sheet's `定期 + SC` row) from a month-start anchor: Σ `principal` of deposits held on the window's first day — `start_date` absent or before the first month's 1st, and `end_date` on or after that 1st.

#### Scenario: Locked evolves

- **WHEN** the anchor is `445000`, `2026-09` has no flows, and `2026-10` finishes `145000`, places an `sc_deposit` plan `-180000`, and returns `55000`
- **THEN** `2026-10.locked` is `445000 − 145000 + 180000 − 55000 = 425000`

#### Scenario: Anchor excludes deposits starting in the window

- **WHEN** the window starts `2026-09` and deposits are: `140000` started `2026-07-01` ending `2026-11-03`, `80000` with no `start_date` ending `2026-11-20`, `80000` started `2026-08-31` ending `2027-01-05`, and `55000` started `2026-09-30` ending `2027-01-05`
- **THEN** the anchor is `300000` and `2026-09.locked` is `355000`

#### Scenario: Matured earlier this month

- **WHEN** today is `2026-10-13`, the window starts `2026-10`, and deposits of `110000` and `35000` (started before October) ended `2026-10-12`
- **THEN** the anchor includes both, `2026-10.deposit_finish` is `145000`, and `2026-10.locked` is the anchor minus `145000` — they are not subtracted twice

#### Scenario: Margin check

- **WHEN** `2026-09` derives `semi_liquid` `520071.25` and live `liquid_assets` is `1964860.57`
- **THEN** `ref_check` is approximately `28856.10`

## ADDED Requirements

### Requirement: Locked chain and semi-liquid

Each month `m` (the first included, chaining from the anchor) SHALL derive `locked(m) = locked(m−1) − deposit_finish(m) − Σ hs_deposit/sc_deposit plan amounts(m) + Σ principal of deposit_placements(m) − deposit_return(m)` — plan amounts are stored negative, so subtracting them adds to `locked`. `semi_liquid` SHALL be `cash + locked`, absent while either side is absent.

#### Scenario: Chain subtracts finishes

- **WHEN** month `m` follows a month with `deposit_finish`
- **THEN** `locked(m)` is `locked(m−1)` minus that finish plus placements, minus `deposit_return`

## ADDED Requirements

### Requirement: Locked anchor edge cases

The anchor SHALL NOT use the live "end date not passed" total, so a deposit that matured earlier in the current month stays in the anchor and leaves through `deposit_finish`, and a deposit starting on or after the 1st enters through its placement month.

#### Scenario: Early-matured deposit stays in the anchor

- **WHEN** a deposit matured earlier in the current month
- **THEN** it stays in the anchor and leaves through `deposit_finish`

## ADDED Requirements

### Requirement: Live semi-liquid block unchanged

The overview `半流動資金` block SHALL keep using the live active-deposit total.

#### Scenario: Live block unaffected by the chain

- **WHEN** the overview renders the 半流動資金 block
- **THEN** it uses the live active-deposit total, not the forecast anchor

## MODIFIED Requirements

### Requirement: Forecast plan items

The system SHALL store forecast items — `id`, `month` (`YYYY-MM-01`), `kind`, signed `amount`, `note`, and optional `return_month` — exposed through `POST /api/forecast/:ym/items`, `PATCH /api/forecast-items/:id`, and `DELETE /api/forecast-items/:id`. `kind` SHALL be one of an enumerated set that includes `hs_deposit` and `sc_deposit` (planned lockups, stored negative — they reduce `cash`, add to `locked`, and schedule a `deposit_return`).

#### Scenario: Two placements in one cell's place

- **WHEN** items `{month: 2026-10, kind: hs_deposit, amount: -70000}` and `{month: 2026-10, kind: hs_deposit, amount: -60000}` exist
- **THEN** `2026-10` lists both in `plan_items`, `cash` drops by `130000`, and `locked` rises by `130000`

#### Scenario: return_month rejected on wrong kind

- **WHEN** an item is created with `kind` `tax` and `return_month` `2027-03`
- **THEN** the request fails with 400

## ADDED Requirements

### Requirement: Remaining forecast kinds

The `kind` enumeration continues with `interest` (manual interest additions, the sheet's `+1000`-style cell), `tax` (Tax/基金/醫療保險), `stock` (股票), `bill` (a `bill` override), and `other` (TBC). Multiple items SHALL be allowed per month and kind — the sheet's `=7000+6000` cell is two items.

#### Scenario: Multiple items per kind

- **WHEN** two `hs_deposit` items are stored for a month
- **THEN** both are kept, like the sheet's `=7000+6000` cell

## ADDED Requirements

### Requirement: Forecast item validation

`return_month` SHALL be rejected on kinds other than `hs_deposit`/`sc_deposit`, and `amount` SHALL be finite and non-zero.

#### Scenario: Return month on a tax item rejected

- **WHEN** a `tax` item is created with a `return_month`
- **THEN** it is rejected

## ADDED Requirements

### Requirement: Out-of-window plan items

Items dated outside the current window SHALL still be stored: items ahead of the window enter it as months slide forward; items whose month is before the window are stale — their placement and return effects apply only while the item's own month is inside the window (the sheet's shifted-out cells fire nothing either, and this also prevents a real deposit created without conversion from double-counting a stale plan's return).

#### Scenario: Stale item fires nothing

- **WHEN** an item's month is before the window
- **THEN** its placement and return effects apply only while its own month is inside the window

## MODIFIED Requirements

### Requirement: Convert plan to deposit

`POST /api/forecast-items/:id/convert` SHALL accept the deposit fields (`principal` prefilled with `−amount` by the frontend, `end_date` required, `rate`/`interest`/`start_date`/`label`/`note` optional, `bank` defaulting from the item kind), create the deposit row, and delete the forecast item in one transaction.

#### Scenario: Conversion replaces the plan

- **WHEN** `sc_deposit` item `-180000` in `2026-10` is converted with `start_date` `2026-10-12` and `end_date` `2027-02-10`
- **THEN** a deposit row exists with `principal` `180000` and `bank` `SC`, the forecast item is gone, `2026-10` lists the deposit in `deposit_placements` in place of the plan, and `2027-02` gains the deposit's real `deposit_finish`/`interest` instead of the plan's derived return

#### Scenario: Conversion keeps the grid totals

- **WHEN** the current month `2026-09` holds an `sc_deposit` plan `-55000` and it is converted with `start_date` `2026-09-30`, `end_date` `2027-01-05`, and `interest` `435`
- **THEN** every month's `cash` and `semi_liquid` are the same as before the conversion, except that from `2027-01` onward they are `435` higher, and every month's `locked` is unchanged

## ADDED Requirements

### Requirement: Conversion forecast effects

The item's scheduled `deposit_return` disappears with it since returns are derived. When the deposit's `start_date` falls in the plan's month, and its `end_date` falls in the plan's return month, the forecast's `cash`, `locked`, `semi_liquid`, and `ref_check` SHALL be unchanged by the conversion except for the deposit's own `interest` now counting in its end month.

#### Scenario: Conversion preserves the forecast

- **WHEN** a plan converts to a deposit with `start_date` in the plan's month and `end_date` in its return month
- **THEN** `cash`, `locked`, `semi_liquid` and `ref_check` are unchanged except for the deposit's `interest` in its end month

## MODIFIED Requirements

### Requirement: 總覽 預測 grid card

The 總覽 page SHALL show a 預測 card rendering the grid with months as columns and the sheet's lines as rows — `ref check`, `半流動`, month-end dates, `活期`, `定期+SC`, `start`, `salary`, `支出`, `定期 finish`, `定期 HS`, `SC高息馬拉松`, `利息`, `Tax/基金/醫療保險`, `股票`, `繳費`, `TBC - 定期 end`, `TBC` — `—` while a figure is absent.

#### Scenario: Grid renders

- **WHEN** the forecast returns seven months
- **THEN** the card shows the sheet's row labels down the side and seven month columns

#### Scenario: Convert from the grid

- **WHEN** the user clicks `轉為定期` on an `sc_deposit` item of `-180000`
- **THEN** the deposit form opens with `principal` `180000` and bank `SC`, and saving it calls the convert endpoint

#### Scenario: Placement shown in its bank row

- **WHEN** `2026-09` has no plans and one placement `9077` of `55000` with bank `SC`
- **THEN** the `SC高息馬拉松` cell for `2026-09` shows `-55000`, its editor lists `9077` as a read-only `已定期` line, and the `定期 HS` cell stays `—`

#### Scenario: 活期 below the floor renders red

- **WHEN** a month's `cash` is `15000`, the next column's `cash` is `20000`, and a third column's `cash` is absent
- **THEN** the first `活期` cell renders in the negative/red style while the `20000` and `—` cells render normally

## ADDED Requirements

### Requirement: Plan cell editing

Plan cells SHALL offer inline add/edit/delete per item and a `轉為定期` action on `hs_deposit`/`sc_deposit` items that opens the deposit form prefilled.

#### Scenario: Convert action on a plan cell

- **WHEN** an `hs_deposit` plan cell is shown
- **THEN** it offers a `轉為定期` action opening the deposit form prefilled

## ADDED Requirements

### Requirement: Deposit placement lines

Each of the month's `deposit_placements` SHALL appear as a read-only line (label, `−principal`, `start_date`, marked `已定期`) in the `定期 HS` row when its bank is `HS` and in the `SC高息馬拉松` row otherwise; the cell's figure SHALL be Σ its plan amounts plus Σ `−principal` of its placements, and placements SHALL offer no edit, delete, or convert action.

#### Scenario: Placement routed by bank

- **WHEN** a placement's bank is `HS`
- **THEN** its read-only line appears in the `定期 HS` row

## ADDED Requirements

### Requirement: Grid styling and reload

`ref_check` SHALL render in the negative/red style while below `0`. A `活期` cell SHALL render in the negative/red style while its `cash` figure is below `20000` — a cell at exactly `20000`, or `—` while `cash` is absent, renders normally. The view SHALL reload after every mutation.

#### Scenario: Low cash cell shows red

- **WHEN** a `活期` cell's `cash` figure is `19999`
- **THEN** it renders in the negative/red style

## MODIFIED Requirements

### Requirement: Money Master block

`GET /api/overview` SHALL return a `money_master` block mirroring the sheet's `J29:N35` challenge tracker: the stored inputs `start_date` (challenge start `YYYY-MM-DD`), `saved` (the bank app's saved figure), `target_months`, and `target_amount`; the effective `month_now` and `coming_save`; and the derived `months_left`, `avg_per_month` (`M31`), `yearly_rate` (`N31`), `time_progress` (`K33`), `saved_progress` (`L33`), `progress_gap` (`M33`), and `can_use` (`K35`).

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

## ADDED Requirements

### Requirement: Money Master month counters

- `month_now` SHALL be the stored `money_master.month_now` override when set, else `full months elapsed since start_date + 1` (the bank's 1-indexed month counter), absent while no `start_date` is stored and no override exists.
- `months_left` SHALL be `max(1, target_months − month_now + 1)`, absent while either input is missing.

#### Scenario: Override wins

- **WHEN** `money_master.month_now` is set
- **THEN** `month_now` is the override, not the elapsed count

## ADDED Requirements

### Requirement: Money Master coming save

- `coming_save` SHALL be the stored `money_master.coming_save` override when set, else `salary + (target_amount − saved) ÷ months_left` — negative while ahead of target, matching the bank app — absent while any of salary, `target_amount`, `saved`, or `months_left` is missing.

#### Scenario: Negative while ahead

- **WHEN** `saved` is ahead of `target_amount`
- **THEN** `coming_save` is negative, matching the bank app

## ADDED Requirements

### Requirement: Money Master rates and can-use

- `avg_per_month` SHALL be `saved ÷ month_now`, `yearly_rate` SHALL be `avg_per_month × 12`, each absent while its inputs are missing or the divisor is zero.
- `time_progress` SHALL be `month_now ÷ target_months`, `saved_progress` SHALL be `saved ÷ target_amount`, `progress_gap` SHALL be `saved_progress − time_progress`, each absent while its inputs are missing or the divisor is zero.

#### Scenario: Progress gap

- **WHEN** `month_now` and `target_months` and `saved` and `target_amount` are present
- **THEN** `progress_gap` is `saved_progress − time_progress`

## ADDED Requirements

### Requirement: Money Master can-use

- `can_use` SHALL be `salary − coming_save`, absent while either input is missing; it MAY exceed salary while `coming_save` is negative.

#### Scenario: Can-use may exceed salary

- **WHEN** `coming_save` is negative
- **THEN** `can_use` exceeds salary

