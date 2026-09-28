# Spec Delta

## ADDED Requirements

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
