# Spec Delta

## ADDED Requirements

### Requirement: 過去 12 個月平均 block

`GET /api/overview` SHALL return an `averages` block mirroring the sheet's `F3:G10` (+`H6`): trailing averages of the per-month 總數增加 (Changed), 支出 (month_spend), 生活支出 (living_spend), 存 (saved), and 利息 (interest) over the 12 most recent month rows whose `month` precedes the current month; the live 開心Pool balance; and `living_budget` = `ROUNDUP(living_spend_avg × 1.05, −2)` (up to the nearest 100). Each average SHALL skip months where that column has no value (AVERAGE semantics), and SHALL be absent while the window holds no values for it. `living_budget_low` SHALL be true when `living_budget < liquid_assets × 0.0001 × 30 + 9000`, and the response SHALL carry that floor value for display.

#### Scenario: Trailing window

- **WHEN** the current month is `2026-09` and months `2025-09` through `2026-08` are stored
- **THEN** each average covers exactly those twelve months, skipping per-column months with no value

#### Scenario: Current month excluded

- **WHEN** the current month's row exists but its spend columns are not yet final
- **THEN** no figure in the block includes the current month

#### Scenario: Budget derived

- **WHEN** the 生活支出 average is `14119.67`
- **THEN** `living_budget` is `14900`, and `living_budget_low` is true only while `liquid_assets × 0.003 + 9000` exceeds `14900`

### Requirement: 總覽 averages card

The 總覽 page SHALL show a "過去 12 個月平均" card listing 總數增加, 支出, 生活支出, 生活預算, 存, 利息, and the live 開心Pool balance — `—` while a figure is absent. 生活預算 SHALL render green while `living_budget_low` (the budget stays under the floor) and red while it exceeds the floor.

#### Scenario: Budget below floor

- **WHEN** `living_budget_low` is true
- **THEN** the 生活預算 figure renders in the positive/green style used elsewhere

#### Scenario: Budget above floor

- **WHEN** `living_budget` exceeds `living_budget_floor`
- **THEN** the 生活預算 figure renders in the negative/red style used elsewhere
