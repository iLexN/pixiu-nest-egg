# Spec Delta

## MODIFIED Requirements

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance), the asset table (label, HKD amount, share — `—` while an amount is absent), the 半流動資金 block (已定期, each cash row, 活期, and the 半流動資金 total — rendered red while below 25% × 流動資產 and green while above — with its `A13` share and `C14` difference on a second footer row), and the IBKR block (stored inputs plus the derived figures). Manual `asset`/`cash` row amounts SHALL be editable inline via the existing manual-assets endpoints; the view SHALL reload after every mutation.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

### Requirement: 總覽 averages card

The 總覽 page SHALL show a "過去 12 個月平均" card listing 總數增加, 支出, 生活支出, 生活預算, 存, and 利息 — `—` while a figure is absent. 生活預算 SHALL render green while `living_budget_low` (the budget stays under the floor) and red while it exceeds the floor.

#### Scenario: Budget below floor

- **WHEN** `living_budget_low` is true
- **THEN** the 生活預算 figure renders in the positive/green style used elsewhere

#### Scenario: Budget above floor

- **WHEN** `living_budget` exceeds `living_budget_floor`
- **THEN** the 生活預算 figure renders in the negative/red style used elsewhere
