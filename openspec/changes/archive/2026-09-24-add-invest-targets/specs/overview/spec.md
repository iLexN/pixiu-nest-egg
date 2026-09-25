# Spec Delta

## ADDED Requirements

### Requirement: 投資目標 block

`GET /api/overview` SHALL return an `invest_targets` block mirroring the sheet's `J22:N27`: `avg_invested` (the sheet's `J22`) and `rows`, one per year covered by the year-review row set, ordered ascending. Each row SHALL carry `year`, `invested` (the year-review row's `invested`, absent while none), the year's effective `raise`, `target`, `remain`, and `growth` per the formula requirement below. `avg_invested` SHALL be the mean of `invested` over the rows for the three most recent completed years (years before the current year), skipping rows whose `invested` is absent, and SHALL be absent while no completed year reports `invested`.

#### Scenario: Rows cover the year-review years

- **WHEN** `GET /api/year-review` returns rows for 2023–2026 and the current year is 2026
- **THEN** the block lists exactly those four years ascending

#### Scenario: Average over the last three completed years

- **WHEN** the current year is 2026 and `invested` is `206523.15` (2023), `345365.11` (2024), and `428635.41` (2025)
- **THEN** `avg_invested` is approximately `326841.22`

#### Scenario: Average skips absent years

- **WHEN** only two of the three most recent completed years report `invested`
- **THEN** `avg_invested` is the mean over those two

### Requirement: 投資目標 formula

For each row the system SHALL derive `target` = `invested(Y-1) − interest(Y-1) − pool_spend(Y-1)×0.7 + raise(Y)×0.5×12 + interest(Y) + pool_spend(Y)×0.7`, reading `invested`/`interest`/`pool_spend`/`raise` off the stated years' year-review figures; `target` SHALL be absent while the prior year's `invested` is absent, and absent `interest`/`pool_spend`/`raise` inputs SHALL count as `0`. `remain` SHALL be `target − invested` on the current year's row only, absent on completed years and while either input is missing. `growth` SHALL be `(invested(Y) − invested(Y-1)) ÷ invested(Y-1)` for completed years and `(target(Y) − invested(Y-1)) ÷ invested(Y-1)` for the current year, absent while either input is missing or the divisor is zero.

#### Scenario: Current-year target derived

- **WHEN** 2025 reports `invested` `428635.41`, `interest` `51900.65`, `pool_spend` `38729.5`, and 2026 reports effective `raise` `1890`, `interest` `62753.15`, `pool_spend` `9593`, and `invested` `182935.39`
- **THEN** the 2026 row reports `target` ≈ `430432.36`, `remain` ≈ `247496.97`, and `growth` ≈ `0.0042`

#### Scenario: Completed-year growth

- **WHEN** 2024 and 2025 report `invested` `345365.11` and `428635.41`
- **THEN** the 2025 row reports `growth` ≈ `0.2411` and `remain` is absent

#### Scenario: First tracked year

- **WHEN** 2023 is the earliest row
- **THEN** its `target`, `remain`, and `growth` are absent while `invested` still reports

### Requirement: 投資目標 card

The 總覽 page SHALL show an 投資目標 card with the `avg_invested` figure (近3年平均) as its header stat and a per-year table of 年份, `invested`, `target`, `remain`, and `growth` rendered as a percentage — `—` while a figure is absent.

#### Scenario: Card renders the block

- **WHEN** the block returns four year rows and an `avg_invested`
- **THEN** the card shows the average and one table row per year, rendering absent figures as `—`
