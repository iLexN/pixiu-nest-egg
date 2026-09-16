## ADDED Requirements

### Requirement: Dividend-adjusted net position
For each stock the system SHALL report 累計派息 as the sum of its **received** dividend amounts only; pending estimates SHALL NOT count toward it. From this the system SHALL derive 累計派息% as `累計派息 ÷ 總買入成本`, 淨投入總本金 as `總買入成本 − 累計派息`, 淨攤薄單價 as `淨投入總本金 ÷ 股數 held`, and 實質動態總回報% as `(當前總市值 − 淨投入總本金) ÷ 淨投入總本金`. 累計派息% SHALL be empty when 總買入成本 is 0, 淨攤薄單價 SHALL be empty when holdings are 0, and 實質動態總回報% SHALL be empty when the stock has no 現價/市值 or when 淨投入總本金 is 0. These figures apply to both HK and US markets and are derived on each read, never stored.

#### Scenario: Net position after received dividends
- **WHEN** `中國銀行` has 總買入成本 208746.64, 股數 held 68000, 現價 5.91, and received dividends totalling 54023.87
- **THEN** the summary reports 累計派息 54023.87, 累計派息% 0.258801, 淨投入總本金 154722.77, 淨攤薄單價 2.275335, and 實質動態總回報% 1.597420 (rounded for display)

#### Scenario: Pending estimate excluded
- **WHEN** a stock has a pending dividend with only an estimated amount
- **THEN** that estimate does not change 累計派息, 淨投入總本金, 淨攤薄單價 or 實質動態總回報%

#### Scenario: No dividends recorded
- **WHEN** a stock has no dividend records
- **THEN** 累計派息 is 0 and 淨投入總本金 equals 總買入成本

#### Scenario: No holdings
- **WHEN** a stock's holdings are 0
- **THEN** 淨攤薄單價 is reported as empty rather than a division error

#### Scenario: No current price
- **WHEN** a stock has no 現價
- **THEN** 實質動態總回報% is reported as empty while the other four figures are still reported

#### Scenario: US market
- **WHEN** the user views the US summary
- **THEN** US stocks show the same five figures computed from their own trades and received dividends

### Requirement: Dividend-adjusted market totals
For each market the system SHALL report totals for 累計派息 and 淨投入總本金 across all stocks, an aggregate 累計派息% as `Σ 累計派息 ÷ Σ 總買入成本` (empty when total buy cost is 0), and an aggregate 實質動態總回報% as `(total market value − 淨投入總本金 of priced stocks) ÷ 淨投入總本金 of priced stocks`, reported as empty when no stock is priced or that denominator is 0.

#### Scenario: Market totals include dividends
- **WHEN** the HK summary is viewed
- **THEN** the totals show the summed 累計派息, the aggregate 累計派息%, the summed 淨投入總本金 across all HK stocks and the aggregate 實質動態總回報% over the priced subset

#### Scenario: Aggregate return excludes unpriced stocks
- **WHEN** a stock has no 現價
- **THEN** its 淨投入總本金 does not enter the denominator of the aggregate 實質動態總回報%, matching how `buy_cost_priced` already excludes it
