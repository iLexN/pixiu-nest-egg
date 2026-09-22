# overview Specification

## Purpose

Replaces the `Overview` sheet's asset-allocation block (`A3:C18`) and the 美股 sheet's IBKR account header (`A1:B5`): an all-asset dashboard aggregating every migrated module into the sheet's asset table, 半流動資金 block, and headline 總數/流動資產, plus the manual IBKR account figures that feed the IBKR row.

## Requirements

### Requirement: Overview aggregation endpoint

`GET /api/overview` SHALL return the sheet's `A3:C18` block plus its headline figures, all derived on read: `total_assets` (總數, `B1`), `liquid_assets` (流動資產, `H1`), `liquid_ratio` (`J1` = `liquid_assets` ÷ (salary × 100), absent while no salary is stored), `assets` (the `A3:B9` rows), `assets_sum` (`B10`), `semi_liquid` (the `A14:C18` block plus the `A13` ratio), `ibkr` (the 美股 `A1:B5` block with derived figures), `salary`, and `rate` (the stored `aia.usd_hkd_rate`, absent while unset). Figures needing the USD→HKD rate SHALL be absent while it is unset, consistent with the other modules.

#### Scenario: Aggregation returned

- **WHEN** the user opens the 總覽 page with all modules populated and the rate stored
- **THEN** the response carries the headline totals, every asset row with its share, the 半流動資金 block, and the IBKR block

#### Scenario: No rate stored

- **WHEN** no `aia.usd_hkd_rate` is stored
- **THEN** the 基金 and IBKR amounts, and the rate-dependent totals terms, are absent rather than zero

### Requirement: Asset allocation table

The `assets` list SHALL contain, in the sheet's row order: 港股 (HK market value), 債券 (active bond principal), 基金 (AIA non-excluded value × rate), MPF (current total balance), every manual `asset` row in `sort_order` (e.g. Irene, HS人壽), and IBKR (the computed IBKR total). `assets_sum` SHALL equal the sum of the present row amounts (the sheet's `B10`), and each row SHALL carry `share` = amount ÷ `assets_sum`, absent while the amount is absent or the sum is zero.

#### Scenario: Shares of the total

- **WHEN** the asset rows are 港股 1323946, 債券 50000, 基金 700262.87, MPF 836275.87, HS人壽 69440.47, IBKR 134242.39
- **THEN** `assets_sum` is approximately 3114167.6 and 港股's `share` is approximately 0.4251

### Requirement: 半流動資金 block

`semi_liquid` SHALL carry `deposits` (已定期 = Σ principal over active deposits — principal only, matching `定期!B1`), `cash_rows` (every manual `cash` row, e.g. HS, 渣打), `cash_sum` (活期 = Σ cash rows), `total` (半流動資金 = `deposits` + `cash_sum`), `vs_quarter_liquid` (the sheet's `C14` = `total` − 25% × `liquid_assets`), and `share` (the sheet's `A13` = `total` ÷ (港股 + 債券 + `total` + IBKR), absent while any term is missing or the divisor is zero).

#### Scenario: Block derived

- **WHEN** active deposits hold principal 445000 and cash rows HS 30538.78 + 渣打 1364.89
- **THEN** `cash_sum` is 31903.67 and `total` is 476903.67

### Requirement: Headline totals

`total_assets` SHALL equal `assets_sum` + `semi_liquid.total` and `liquid_assets` SHALL equal 港股 + `semi_liquid.total` + 債券 + IBKR − the current 開心Pool balance — the same figures Month Stat's live totals produce for the current month.

#### Scenario: Totals match the sheet

- **WHEN** the block shows Sum 3114167.6, 半流動 476903.67, and the pool balance is 22172.4
- **THEN** `total_assets` is approximately 3591071.27 and `liquid_assets` is approximately 1962919.66

### Requirement: IBKR account figures

The system SHALL store four manual IBKR figures in `app_meta` — `ibkr.transferred_hkd` (美股 `B1`: cumulative bank→IBKR transfers in HKD), `ibkr.now_value` (`B2`: the account total as displayed in the IBKR app, which uses a different FX rate than `aia.usd_hkd_rate`), `ibkr.hkd_cash` (`B4`), and `ibkr.usd_cash` (`B5`) — exposed through `GET /api/ibkr` and `PATCH /api/ibkr`. Each field SHALL accept a finite non-negative number or `null` to clear. The response SHALL derive `computed_total_hkd` (美股 `B7` = (US market value + `usd_cash`) × rate + `hkd_cash`), `net` = `now_value` − `transferred_hkd`, `net_pct` = `net` ÷ `transferred_hkd`, and `vs_now_value` = `computed_total_hkd` − `now_value`, each absent while its inputs are missing.

#### Scenario: Computed total

- **WHEN** US stocks are worth 15813.73 USD, `usd_cash` is 1200.25, `hkd_cash` is 765.42, and the rate is 7.845135
- **THEN** `computed_total_hkd` is approximately 134242.39

#### Scenario: Net vs transferred

- **WHEN** `transferred_hkd` is 131000 and `now_value` is 134232.01
- **THEN** `net` is 3232.01 and `net_pct` is approximately 0.0247

#### Scenario: Clearing a field

- **WHEN** the user patches `usd_cash` to `null`
- **THEN** the stored value is cleared and `computed_total_hkd` drops the USD cash term

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with a headline strip (總數, 流動資產, and J1 as a percentage), the asset table (label, HKD amount, share — `—` while an amount is absent), the 半流動資金 block (已定期, each cash row, 活期, 半流動資金, its `C14` difference and `A13` share), and the IBKR block (stored inputs plus the derived figures). Manual `asset`/`cash` row amounts SHALL be editable inline via the existing manual-assets endpoints; the view SHALL reload after every mutation.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

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
