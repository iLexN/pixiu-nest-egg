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

The system SHALL store three manual IBKR figures in `app_meta` — `ibkr.now_value` (`B2`: the account total as displayed in the IBKR app, which uses a different FX rate than `aia.usd_hkd_rate`), `ibkr.hkd_cash` (`B4`), and `ibkr.usd_cash` (`B5`) — and SHALL keep a dated `ibkr_transfers` log whose sum is `transferred_hkd` (美股 `B1`: cumulative bank→IBKR transfers in HKD, absent while the log is empty); each year's sum is the year's IBKR 轉入. These are exposed through `GET /api/ibkr` and `PATCH /api/ibkr`: each account field SHALL accept a finite non-negative number or `null` to clear, while `transfer_hkd` SHALL append a log row for any finite delta (a negative value records a withdrawal or undoes a mistyped entry) dated `transfer_date`, defaulting to today. The response SHALL derive `computed_total_hkd` (美股 `B7` = (US market value + `usd_cash`) × rate + `hkd_cash`), `net` = `now_value` − `transferred_hkd`, `net_pct` = `net` ÷ `transferred_hkd`, and `vs_now_value` = `computed_total_hkd` − `now_value`, each absent while its inputs are missing.

#### Scenario: Computed total

- **WHEN** US stocks are worth 15813.73 USD, `usd_cash` is 1200.25, `hkd_cash` is 765.42, and the rate is 7.845135
- **THEN** `computed_total_hkd` is approximately 134242.39

#### Scenario: Net vs transferred

- **WHEN** `transferred_hkd` is 131000 and `now_value` is 134232.01
- **THEN** `net` is 3232.01 and `net_pct` is approximately 0.0247

#### Scenario: Clearing a field

- **WHEN** the user patches `usd_cash` to `null`
- **THEN** the stored value is cleared and `computed_total_hkd` drops the USD cash term

#### Scenario: Recording a transfer

- **WHEN** the user patches `transfer_hkd` `50000` with no `transfer_date`
- **THEN** a row dated today is appended to `ibkr_transfers` and `transferred_hkd` grows by `50000`

#### Scenario: Undoing a transfer

- **WHEN** the user patches `transfer_hkd` `-10000`
- **THEN** a negative row is appended and `transferred_hkd` drops by `10000`

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance), the asset table (label, HKD amount, share — `—` while an amount is absent), the 半流動資金 block (已定期, each cash row, 活期, and the 半流動資金 total — rendered red while below 25% × 流動資產 and green while above — with its `A13` share and `C14` difference on a second footer row), and the IBKR block (stored inputs plus the derived figures). Manual `asset`/`cash` row amounts SHALL be editable inline via the existing manual-assets endpoints; the view SHALL reload after every mutation.

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

The 總覽 page SHALL show a "過去 12 個月平均" card listing 總數增加, 支出, 生活支出, 生活預算, 存, and 利息 — `—` while a figure is absent. 生活預算 SHALL render green while `living_budget_low` (the budget stays under the floor) and red while it exceeds the floor.

#### Scenario: Budget below floor

- **WHEN** `living_budget_low` is true
- **THEN** the 生活預算 figure renders in the positive/green style used elsewhere

#### Scenario: Budget above floor

- **WHEN** `living_budget` exceeds `living_budget_floor`
- **THEN** the 生活預算 figure renders in the negative/red style used elsewhere

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

### Requirement: 策略 liquidity tiers block

`GET /api/overview` SHALL return a `liquidity_tiers` block mirroring the sheet's `J3:K7`, all derived on read: `cannot_use` (`K5` = the stored salary × 6 — the sheet's `N6` 6-month salary), `can_use` (`K4` = `semi_liquid.total` − `cannot_use`, negative allowed), `short_term` (`K6` = 港股 + 債券 + IBKR + Σ manual `asset` rows with `liquidity` `short`), and `long_term` (`K7` = 基金 + MPF + Σ manual `asset` rows with `liquidity` `long`). `cannot_use` and `can_use` SHALL be absent while no salary is stored. `short_term` and `long_term` SHALL be absent while `aia.usd_hkd_rate` is unset (their IBKR and 基金 terms need it). While every figure is present, `can_use + cannot_use + short_term + long_term` SHALL equal `total_assets`.

#### Scenario: Tiers derived

- **WHEN** the salary is `52700`, 半流動資金 is `477023.67`, 港股 + 債券 + IBKR total `1504503.865`, and 基金 + MPF + the long-term row HS人壽 total `1605802.914`
- **THEN** `cannot_use` is `316200`, `can_use` is approximately `160823.67`, `short_term` is approximately `1504503.87`, and `long_term` is approximately `1605802.91`

#### Scenario: Manual row placed by liquidity

- **WHEN** a manual `asset` row Irene `20000` has `liquidity` `short` and HS人壽 `69440.47` has `liquidity` `long`
- **THEN** `short_term` includes `20000` and `long_term` includes `69440.47`

#### Scenario: No salary stored

- **WHEN** `overview.salary` is unset
- **THEN** `cannot_use` and `can_use` are absent while `short_term` and `long_term` still report

#### Scenario: No rate stored

- **WHEN** `aia.usd_hkd_rate` is unset
- **THEN** `short_term` and `long_term` are absent rather than partial sums

#### Scenario: Buffer exceeds 半流動資金

- **WHEN** 半流動資金 is `300000` and the salary is `52700`
- **THEN** `can_use` is `-16200`

### Requirement: 總覽 策略 card

The 總覽 page SHALL show a 策略 card listing the four tiers in sheet order — 可動用 (`can_use`), 不可動用（6個月薪金） (`cannot_use`), 短期可取回 (`short_term`), 長期可取回 (`long_term`) — as HKD amounts, `—` while a figure is absent. 可動用 SHALL render in the negative/red style used elsewhere while it is below zero.

#### Scenario: Card renders the block

- **WHEN** the block returns all four figures
- **THEN** the card shows four labelled rows with the amounts formatted as money

#### Scenario: Negative 可動用

- **WHEN** `can_use` is `-16200`
- **THEN** the 可動用 figure renders in the negative/red style

#### Scenario: Absent figures

- **WHEN** no salary is stored
- **THEN** 可動用 and 不可動用 show `—` while the other two rows show amounts
