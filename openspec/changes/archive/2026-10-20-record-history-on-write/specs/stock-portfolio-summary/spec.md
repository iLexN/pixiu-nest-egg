# Spec Delta

## MODIFIED Requirements

### Requirement: Market totals history

After any successful write that may change a market's totals, the system SHALL record a history row for each affected market. Such writes are creating, updating or deleting a stock or a trade, and a bulk price update. The row is dated with the day of the write, carries that day's `buy_cost_priced` and `market_value`, and is recorded only while at least one stock in the market is priced. A same-day record SHALL replace that day's row.

#### Scenario: Summary records a history row
- **WHEN** the user updates the 現價 of an HK stock on `2026-09-18` and at least one HK stock is priced
- **THEN** a history row exists for market HK dated `2026-09-18` carrying that day's `buy_cost_priced` and `market_value`

#### Scenario: Same-day replace
- **WHEN** two writes change a market's totals on the same calendar day
- **THEN** exactly one history row exists for that market and day, carrying the latest values

#### Scenario: Month-rollover backfill
- **WHEN** a market's latest history row is dated in August 2026 and the next write that changes its totals happens on `2026-10-05`
- **THEN** no synthetic September row is created, `2026-10-05` gets the new values' row, and 上月 viewed in October still reports the August values through the as-of lookup

#### Scenario: Unpriced market records nothing
- **WHEN** no stock in a market has 現價
- **THEN** a write to that market records no history row

#### Scenario: Bulk price update captured for every touched market
- **WHEN** a price file updates 現價 for stocks in both HK and US
- **THEN** both markets have a history row for that day, even if no summary is viewed afterward

## ADDED Requirements

### Requirement: Reads never record history

Computing or viewing a market summary, the yearly summary, or the year review, and freezing a year-end snapshot, SHALL NOT write market history.

#### Scenario: Viewing the summary writes nothing

- **WHEN** the user opens the HK 持倉總覽 several times on a day with no writes
- **THEN** the market history for HK is unchanged

### Requirement: Unchanged totals record nothing

A write SHALL add or replace a history row only when the market's `buy_cost_priced` or `market_value` differs from that market's latest history row. A write that leaves both unchanged, such as editing a stock's note or a trade's note, SHALL record nothing.

#### Scenario: Note edit records nothing

- **WHEN** a market's latest history row matches its current totals and the user edits only a stock's note
- **THEN** no history row is added or changed

#### Scenario: Trade moved between markets

- **WHEN** a trade is edited so it moves from an HK stock to a US stock
- **THEN** each market whose totals changed gets a history row for that day

## MODIFIED Requirements

### Requirement: Last-month and max unrealized figures

For each market the system SHALL derive, on read: the 上月 figures from the latest history row dated on or before the last day of the previous calendar month (empty when no such row exists), and the 最高 figures as the maxima over the market's stored seed marks (per market in `app_meta`, absent when none were ever stored), every history row, plus the market's current values.

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

#### Scenario: Month without writes carries forward
- **WHEN** HK's latest history row is dated `2026-08-28` and no write happened in September, viewed on `2026-10-05`
- **THEN** 上月 reports the `2026-08-28` values

## REMOVED Requirements

### Requirement: Month-end history backfill

**Reason**: 上月 is now an as-of lookup (the latest row on or before the end of the previous month), which yields the same values that synthetic month-end rows carried, without fabricating rows on read.

**Migration**: None. Existing synthetic rows stay in `market_history` and remain valid inputs to 上月 and 最高; no new ones are created.
