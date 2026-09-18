# Spec Delta

## ADDED Requirements

### Requirement: Market totals history
When a market summary is computed and at least one stock is priced, the system SHALL record a history row for that market dated with the day of computation, carrying that day's `buy_cost_priced` and `market_value`. Computing the same market's summary again on the same calendar day SHALL replace that day's row rather than append a second one, so repeated builds do not pollute the record. When a summary is computed and one or more calendar months have fully elapsed with no history rows since the market's previous record, the system SHALL insert a synthetic month-end history row for each elapsed empty month, carrying the last-recorded values forward. No history row SHALL be recorded for a market while none of its stocks is priced.

#### Scenario: Summary records a history row
- **WHEN** the HK summary is computed on `2026-09-18` with at least one priced stock
- **THEN** a history row exists for market HK dated `2026-09-18` carrying that day's `buy_cost_priced` and `market_value`

#### Scenario: Same-day replace
- **WHEN** a market's summary is computed twice on the same calendar day and the totals changed between builds
- **THEN** exactly one history row exists for that market and day, carrying the latest values

#### Scenario: Month-rollover backfill
- **WHEN** a market's latest history row is dated in August 2026 and the next record happens on `2026-10-05`
- **THEN** a synthetic history row dated in September 2026 is created carrying the August values, and `2026-10-05` gets the current values' row

#### Scenario: Unpriced market records nothing
- **WHEN** no stock in a market has 現價
- **THEN** computing that market's summary records no history row

#### Scenario: Bulk price update captured for every touched market
- **WHEN** a price file updates 現價 for stocks in both HK and US
- **THEN** both markets have a history row for that day, even if only one market's summary is viewed afterward

### Requirement: Last-month and max unrealized figures
For each market the system SHALL derive, on read: the 上月 figures from the latest history row in the previous calendar month (empty when no such row exists), and the 最高 figures as the maxima over the market's imported seed marks (stored per market in `app_meta`), every history row, plus the market's current values. Each figure pair carries 未實現報酬率 as `(market_value − buy_cost_priced) ÷ buy_cost_priced`, reported empty when `buy_cost_priced` is 0, and 未實現金額 as `market_value − buy_cost_priced`. The max 未實現報酬率 and max 未實現金額 SHALL be tracked independently — they may come from different moments. When a new record sets a new high, the reported 最高 SHALL rise automatically without any manual copy step.

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
- **WHEN** HK's imported seed marks are `0.3641` / `362869.79` and no history row or current value exceeds them
- **THEN** 最高 reports `0.3641` and `362869.79`, and a new record above either mark raises that figure automatically

### Requirement: 上月/最高 on the summary
The market summary response SHALL include the derived 上月 and 最高 figures, each carrying `percent` (未實現報酬率) and `amount` (未實現金額). 持倉總覽 SHALL display them in the totals area beside 未實現報酬率, each showing the `percent / amount` pair: 上月 colored by comparison with the corresponding current figure, 最高 rendered plain. When no figures exist the cells SHALL render empty rather than zero.

#### Scenario: Figures returned and shown
- **WHEN** the user opens 持倉總覽 for HK and 上月/最高 figures exist
- **THEN** the summary response includes both figure pairs, and the totals area shows them next to 未實現報酬率

#### Scenario: Comparison coloring
- **WHEN** the current 未實現報酬率 is below the 上月 未實現報酬率
- **THEN** the displayed 上月 figure reflects the comparison, consistent with the sign coloring used elsewhere in the summary

#### Scenario: No figures yet
- **WHEN** a market has no history rows, no seed marks, and no priced stocks
- **THEN** the 上月/最高 cells render empty rather than `0` or `0%`

#### Scenario: Max without current values
- **WHEN** a market has seed marks or history rows but no priced stock today
- **THEN** 最高 still reports the maxima over the marks and history
