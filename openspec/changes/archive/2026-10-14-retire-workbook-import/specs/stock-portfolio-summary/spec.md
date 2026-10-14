# Spec Delta

## MODIFIED Requirements

### Requirement: Last-month and max unrealized figures
For each market the system SHALL derive, on read: the 上月 figures from the latest history row in the previous calendar month (empty when no such row exists), and the 最高 figures as the maxima over the market's stored seed marks (per market in `app_meta`, absent when none were ever stored), every history row, plus the market's current values. Each figure pair carries 未實現報酬率 as `(market_value − buy_cost_priced) ÷ buy_cost_priced`, reported empty when `buy_cost_priced` is 0, and 未實現金額 as `market_value − buy_cost_priced`. The max 未實現報酬率 and max 未實現金額 SHALL be tracked independently — they may come from different moments. When a new record sets a new high, the reported 最高 SHALL rise automatically without any manual copy step.

#### Scenario: Last month from history
- **WHEN** HK's latest history row in August 2026 carries `buy_cost_priced` 500000 and `market_value` 600000, viewed in September 2026
- **THEN** the HK summary's 上月 shows 未實現報酬率 `0.2` and 未實現金額 `100000`

#### Scenario: No previous-month rows
- **WHEN** a market has history rows only in the current calendar month
- **THEN** its 上月 figures are empty

#### Scenario: Max rises automatically
- **WHEN** a market's reported 最高 未實現報酬率 is `0.47` and a new record makes the current 未實現報酬率 `0.50`
- **THEN** the reported 最高 未實現報酬率 becomes `0.50` with no manual step

#### Scenario: Independent maxima
- **WHEN** a market's highest 未實現報酬率 occurred in March and its highest 未實現金額 occurred in August
- **THEN** 最高 reports the March rate and the August amount

#### Scenario: Current counts toward max
- **WHEN** the current 未實現金額 exceeds every recorded history row
- **THEN** 最高 未實現金額 reflects the current value even before a history row is written for it

#### Scenario: Seed mark floors the max
- **WHEN** HK's stored seed marks are `0.3641` / `362869.79` and no history row or current value exceeds them
- **THEN** 最高 reports `0.3641` and `362869.79`, and a new record above either mark raises that figure automatically

## ADDED Requirements

### Requirement: Stock display order
For each market the system SHALL persist a user-controlled stock display order. A newly created stock SHALL be placed after the market's existing stocks. The summary SHALL let the user drag a stock row to a new position and persist that order without changing any trade or calculation data.

#### Scenario: New stock appends
- **WHEN** the user creates a stock in a market that already lists stocks
- **THEN** it appears last in that market's order until the user moves it

#### Scenario: Reordering a stock
- **WHEN** the user drags a stock above or below another stock in 持倉總覽
- **THEN** the new order is saved, survives a reload, and applies only to that market

#### Scenario: Incomplete reorder request
- **WHEN** a reorder request omits, duplicates, or includes an unknown stock id for the selected market
- **THEN** the system rejects the request and leaves the existing order unchanged

## REMOVED Requirements

### Requirement: Manual stock display order
**Reason**: Its initial-order-from-workbook rule goes with the workbook import; the remaining behavior is restated as "Stock display order".
**Migration**: Existing stored order is unchanged; see "Stock display order".
