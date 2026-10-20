# Spec Delta

## MODIFIED Requirements

### Requirement: Overview aggregation endpoint

`GET /api/overview` SHALL return the sheet's `A3:C18` block plus its headline figures, all derived on read: `total_assets` (總數, `B1`), `liquid_assets` (流動資產, `H1`), `liquid_ratio` (`J1` = `liquid_assets` ÷ (salary × 100), absent while no salary is stored), `assets` (the `A3:B9` rows), `assets_sum` (`B10`), `semi_liquid` (the `A14:C18` block plus the `A13` ratio), `ibkr` (the 美股 `A1:B5` block with derived figures), `salary`, and `rate` (the stored USD fx rate, absent while unset). Figures needing the USD→HKD rate SHALL be absent while it is unset, consistent with the other modules.

#### Scenario: Aggregation returned

- **WHEN** the user opens the 總覽 page with all modules populated and the rate stored
- **THEN** the response carries the headline totals, every asset row with its share, the 半流動資金 block, and the IBKR block

#### Scenario: No rate stored

- **WHEN** no USD fx rate is stored
- **THEN** the 基金 and IBKR amounts, and the rate-dependent totals terms, are absent rather than zero

### Requirement: 半流動資金 block

`semi_liquid` SHALL carry `deposits` (已定期 = Σ principal over active deposits — principal only, matching `定期!B1`), `cash_rows` (every `bank` cash account in `sort_order`, e.g. HS, 渣打 — broker accounts are excluded), `cash_sum` (活期 = Σ cash rows), `total` (半流動資金 = `deposits` + `cash_sum`), `vs_quarter_liquid` (the sheet's `C14` = `total` − the configured `strategy.semi_liquid_target` ratio × `liquid_assets`; the sheet's `25%` literal promoted to the setting, `0.25` while unset), and `share` (the sheet's `A13` = `total` ÷ (港股 + 債券 + `total` + IBKR), absent while any term is missing or the divisor is zero).

#### Scenario: Block derived

- **WHEN** active deposits hold principal 445000 and bank accounts HS 30538.78 + 渣打 1364.89
- **THEN** `cash_sum` is 31903.67 and `total` is 476903.67

#### Scenario: Broker cash stays out of 活期

- **WHEN** the IBKR HKD broker account holds `765.42`
- **THEN** it is not listed in `cash_rows` and does not change `cash_sum`

#### Scenario: Configured ratio applied

- **WHEN** `strategy.semi_liquid_target` is `0.3`, `total` is `476903.67`, and `liquid_assets` is `1962919.66`
- **THEN** `vs_quarter_liquid` is approximately `-111972.23`

### Requirement: IBKR account figures

The system SHALL store the IBKR app's displayed account total as the setting `ibkr.reported_value` (`B2`; the app uses a different FX rate than the stored USD fx rate). The cash positions SHALL be the amounts of the `broker` cash accounts: `hkd_cash` (`B4`) from the HKD broker account, `usd_cash` (`B5`) from the USD broker account. The system SHALL keep a dated `ibkr_transfers` log whose sum is `transferred_hkd` (美股 `B1`: cumulative bank→IBKR transfers in HKD, absent while the log is empty); each year's sum is the year's IBKR 轉入.

These are exposed through `GET /api/ibkr` and `PATCH /api/ibkr`, as `now_value`, `hkd_cash` and `usd_cash`. `now_value` SHALL accept a finite non-negative number, or `null` to clear. `hkd_cash`/`usd_cash` SHALL accept a finite non-negative number, which is written to the broker account, or `null`, which sets that account to `0`. `transfer_hkd` SHALL append a log row for any finite delta (a negative value records a withdrawal or undoes a mistyped entry) dated `transfer_date`, defaulting to today. One PATCH SHALL apply all its fields in a single transaction.

The response SHALL derive:

- `computed_total_hkd` (美股 `B7` = (US market value + `usd_cash`) × rate + `hkd_cash`)
- `net` = `now_value` − `transferred_hkd`
- `net_pct` = `net` ÷ `transferred_hkd`
- `vs_now_value` = `computed_total_hkd` − `now_value`

Each is absent while its inputs are missing.

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

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with:

- a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance)
- the asset table (label, HKD amount, share — `—` while an amount is absent)
- the 半流動資金 block: 已定期, each bank cash row, 活期, and the 半流動資金 total. The total renders red while below the configured `strategy.semi_liquid_target` share of 流動資產 and green while above. A second footer row shows its `A13` share and `C14` difference, labelled with the configured percent.
- the IBKR block (stored inputs plus the derived figures)

Manual `asset` row amounts SHALL be editable inline via the manual-assets endpoints, and bank cash row amounts via the cash-accounts endpoints. The view SHALL reload after every mutation.

The IBKR block SHALL be the same editable card as the 美股 summary: editable `transferred_hkd` delta + date, `now_value`, `hkd_cash`, and `usd_cash` inputs saving through `PATCH /api/ibkr`, beside the derived figures. The view SHALL reload after a save so the asset-table IBKR row and every dependent figure refresh.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

#### Scenario: Edit IBKR on 總覽

- **WHEN** the user sets `usd_cash` to `1200.25` on the 總覽 IBKR card and saves
- **THEN** the view reloads showing the updated IBKR block, asset-table IBKR row, and dependent totals

### Requirement: Goal settings

The settings SHALL carry the five goal inputs as `app_meta` keys:

- `goals.goal1_amount` and `goals.goal2_amount`: the milestone targets, the sheet's `K12`/`K18` literals
- `goals.goal3_inflation`: the sheet's `1.07`
- `goals.goal3_years`: the sheet's `25+5`
- `goals.final_replacement`: the sheet's `0.7`

- `GET /api/settings` SHALL return the two amounts as stored (`null` while unset) and the three constants as their effective values (the defaults while unset).
- `PATCH /api/settings` SHALL accept each field: the amounts must be finite and ≥ 0, the constants finite and > 0; invalid values are rejected with a field error. `null` clears an amount (blanking that milestone's figures) or resets a constant to its default; an absent field leaves the stored value untouched.

#### Scenario: Constants round-trip through defaults

- **WHEN** no constant is stored
- **THEN** settings report `goal3_inflation` `1.07`, `goal3_years` `30`, `final_replacement` `0.7`; patching `goal3_inflation` `1.05` returns `1.05`, and patching `null` returns `1.07` again

#### Scenario: Validation

- **WHEN** the patch carries `goal1_amount` `-1` or `goal3_years` `0`
- **THEN** the request is rejected with a field error and nothing is written

#### Scenario: Clearing a milestone amount

- **WHEN** `goal1_amount` was `1550000` and the patch carries `null`
- **THEN** settings report `goal1_amount` `null` and the overview's 1st milestone is blank

### Requirement: 策略 liquidity tiers block

`GET /api/overview` SHALL return a `liquidity_tiers` block mirroring the sheet's `J3:K7`, all derived on read:

- `cannot_use` (`K5`) = the stored salary × 6, the sheet's `N6` 6-month salary
- `can_use` (`K4`) = `semi_liquid.total` − `cannot_use`, negative allowed
- `short_term` (`K6`) = 港股 + 債券 + IBKR + Σ manual `asset` rows with `liquidity` `short`
- `long_term` (`K7`) = 基金 + MPF + Σ manual `asset` rows with `liquidity` `long`

`cannot_use` and `can_use` SHALL be absent while no salary is stored. `short_term` and `long_term` SHALL be absent while the USD fx rate is unset, because their IBKR and 基金 terms need it. While every figure is present, `can_use + cannot_use + short_term + long_term` SHALL equal `total_assets`.

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

### Requirement: Bill amount setting

`PATCH /api/settings` SHALL also accept `bill_amount` (finite, non-negative), stored as `forecast.bill_amount` in `app_meta`. The seeded default SHALL be `2158` (the sheet's `B39`).

#### Scenario: Updated amount flows through

- **WHEN** `bill_amount` is patched to `2260` and no `bill` items exist
- **THEN** the next quarter month in the window reports `bill` `-2260`

### Requirement: 半流動資金 target ratio setting

`PATCH /api/settings` SHALL also accept `semi_liquid_target`, the share of 流動資產 that 半流動資金 should cover. It is stored as `strategy.semi_liquid_target` in `app_meta`. The value SHALL be a finite fraction in `[0, 1)`; `null` clears the stored value. `GET /api/settings` SHALL return the effective value (`0.25` while unset, the sheet's `C14`/`N7` literal). `GET /api/overview` SHALL carry the effective ratio so the 總覽 footer can label the configured percent.

#### Scenario: Updated ratio flows through

- **WHEN** `semi_liquid_target` is patched to `0.3`
- **THEN** the overview's `vs_quarter_liquid` and every forecast month's `ref_check` compare against `30%` of `liquid_assets`, and the settings response reports `0.3`

#### Scenario: Reset restores the sheet literal

- **WHEN** `semi_liquid_target` is patched to `null`
- **THEN** the effective ratio is `0.25` and both figures compare against `25%` of `liquid_assets` again

#### Scenario: Out-of-range ratio rejected

- **WHEN** `semi_liquid_target` is patched to `1.5` or a negative value
- **THEN** the request fails with 400 and a field error on `semi_liquid_target`
