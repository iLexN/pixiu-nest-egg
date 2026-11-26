# Spec Delta

## Purpose

Lets the user download their whole financial position as one JSON snapshot — shaped for pasting into an external AI for analysis — without exposing any new stored data or mutating anything.

## ADDED Requirements

### Requirement: Analysis export endpoint

`GET /api/export/analysis` SHALL return a JSON snapshot derived on read from the same figures the 總覽, 預測, and 股票 pages show. The response SHALL carry `Content-Type: application/json` and `Content-Disposition: attachment` with a filename of `wealth-export-<as_of_date>.json`. The endpoint SHALL take no parameters and SHALL write nothing.

#### Scenario: Downloading the snapshot

- **WHEN** `GET /api/export/analysis` is requested
- **THEN** the response is `application/json` with `Content-Disposition: attachment; filename="wealth-export-<as_of_date>.json"` and a body containing the `meta`, `liquid_assets_summary`, `cash_flow_momentum`, `upcoming_cash_flow_and_rigid_expenses`, and `equity_holdings` sections

#### Scenario: Snapshot is read-only and live

- **WHEN** a trade, deposit, or setting changes and the endpoint is requested again
- **THEN** the second response reflects the new data; no request to the endpoint ever persists or mutates state

### Requirement: Meta section

`meta` SHALL carry `as_of_date` (the overview's today, `YYYY-MM-DD`), `currency` (`"HKD"`), `usd_hkd_rate` (the stored USD→HKD rate, `null` while unset), and `target_liquid_assets_hkd` (the goal-3 生活保障 target, `null` while unset).

#### Scenario: Rate and target unset

- **WHEN** no USD→HKD rate and no goal-3 inputs are configured
- **THEN** `meta.usd_hkd_rate` and `meta.target_liquid_assets_hkd` are `null` and the response still succeeds

### Requirement: Liquid assets summary section

`liquid_assets_summary` SHALL break the overview's 流動資產 into `hk_stocks_hkd` (港股 market value), `us_stocks_ibkr_hkd` (the IBKR HKD total), `bonds_hkd` (active bond principal), `semi_liquid_total_hkd` with `semi_liquid_breakdown` (`cash_current_hkd` = 活期, `time_deposits_hkd` = 已定期), `deductions.happy_pool_hkd` (開心Pool balance), and `total_liquid_assets_hkd` equal to the overview's 流動資產.

#### Scenario: Total matches the overview

- **WHEN** the snapshot is generated
- **THEN** `liquid_assets_summary.total_liquid_assets_hkd` equals 港股 + 半流動資金 + 債券 + IBKR − 開心Pool, identical to the overview page's 流動資產

### Requirement: Cash-flow momentum section

`cash_flow_momentum` SHALL carry `monthly_salary_hkd` (the stored 薪金, `null` while unset), `avg_monthly_saving_hkd`, `avg_monthly_interest_hkd`, and `normal_monthly_living_expense_hkd` from the trailing twelve-month averages shown on the overview.

#### Scenario: No history yet

- **WHEN** fewer months are recorded than the averages need
- **THEN** the corresponding momentum fields are `null` rather than zero or omitted

### Requirement: Upcoming cash flow section

`upcoming_cash_flow_and_rigid_expenses` SHALL list every month of the 預測 window (the same seven columns the forecast page shows), each with `projected_cash_current_hkd` (活期), `projected_deposits_and_sc_hkd` (半流動資金), and `maturing_deposits_hkd` (deposits ending that month).

#### Scenario: Seven-month window

- **WHEN** the snapshot is generated in November
- **THEN** the section lists seven months starting at the current month, each with its projected 活期, 半流動資金, and maturing-deposit totals identical to the 預測 grid

### Requirement: Rigid expenses per month

Each month's `rigid_expenses` SHALL be `{category, item, amount_hkd}` entries: one per stored plan item of kind Tax, Bill, or Other — `category` is the kind in its serialized `snake_case` form, `item` its note (`null` when absent), `amount_hkd` its signed amount — plus a synthesized `{category: "bill", item: "差餉", amount_hkd: -<bill amount>}` in each Jan/Apr/Jul/Oct month.

#### Scenario: Stored expense items

- **WHEN** a month has a Tax item noted "AIA 醫療" of −40452.24 and a Bill item with no note of −2223.00
- **THEN** the month's `rigid_expenses` contains `{category:"tax", item:"AIA 醫療", amount_hkd:-40452.24}` and `{category:"bill", item:null, amount_hkd:-2223.00}`

#### Scenario: Quarterly bill is synthesized

- **WHEN** the window covers a January and the configured bill amount is 2158
- **THEN** January's `rigid_expenses` also contains `{category:"bill", item:"差餉", amount_hkd:-2158.00}` even though no stored item exists for it

#### Scenario: Deposit placements are not rigid expenses

- **WHEN** a month has an `sc_deposit` plan item of −170000
- **THEN** it does not appear in `rigid_expenses`

### Requirement: Equity holdings section

`equity_holdings` SHALL list every active stock grouped as `hk_stocks` and `us_stocks`, each row carrying `code`, `ticker`, `exchange`, `sector`, `shares_held`, `market_value`, `total_buy_cost`, `dividends_received`, and `dividend_yield_pct` = `dividends_received ÷ total_buy_cost` (`null` while cost is absent or zero). Monetary fields on US rows SHALL be emitted in both USD (`*_usd`) and HKD (`*_hkd`).

#### Scenario: Inactive stock excluded

- **WHEN** a stock is marked inactive
- **THEN** it does not appear in `equity_holdings`

#### Scenario: No cost basis

- **WHEN** a stock has dividends but no recorded buy cost
- **THEN** its `dividend_yield_pct` is `null`
