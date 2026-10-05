# Spec Delta

## MODIFIED Requirements

### Requirement: 投資目標 block

`GET /api/overview` SHALL return an `invest_targets` block mirroring the sheet's `J22:N27`: `avg_invested` (the sheet's `J22`) and `rows`, one per year covered by the year-review row set, ordered ascending. Each row SHALL carry `year`, `invested` (the year-review row's `invested`, absent while none), the year's effective `raise`, `interest` (the year-review row's 利息回報), `pool_spend` (the year-review row's 開心Pool支出), `target`, `remain`, and `growth` per the formula requirement below. `interest`/`pool_spend` SHALL be `0` when the year reports none — the same count-absent-as-0 semantics the target formula applies. `avg_invested` SHALL be the mean of `invested` over the rows for the three most recent completed years (years before the current year), skipping rows whose `invested` is absent, and SHALL be absent while no completed year reports `invested`.

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

### Requirement: 投資目標 card

The 總覽 page SHALL show an 投資目標 card with the `avg_invested` figure (近3年平均) as its header stat and a per-year table of 年份, `invested`, `target`, `remain`, and `growth` rendered as a percentage — `—` while a figure is absent. A muted footnote SHALL expand the current year's `target` into its formula terms — `上年invested`, `上年利息回報`, `上年Pool支出×0.7`, `月薪增幅×50%*12`, `今年利息回報`, `今年Pool支出×0.7` — each followed by its computed contribution in parentheses, `—` while an input is absent; an absent `raise` counts as `0`. The footnote SHALL be absent while no current-year row exists.

#### Scenario: Card renders the block

- **WHEN** the block returns four year rows and an `avg_invested`
- **THEN** the card shows the average and one table row per year, rendering absent figures as `—`

#### Scenario: Footnote expands the current-year target

- **WHEN** the current year is 2026, its row reports `raise` `1890`, `interest` `62753.15`, `pool_spend` `9593`, and the 2025 row reports `invested` `428635.41`, `interest` `51900.65`, `pool_spend` `38729.5`
- **THEN** the footnote shows each term's contribution: `428,635.41`, `51,900.65`, `27,110.65`, `11,340.00`, `62,753.15`, `6,715.10`

#### Scenario: Footnote hidden without a current-year row

- **WHEN** the block lists no row for the current year
- **THEN** the card shows no formula footnote
