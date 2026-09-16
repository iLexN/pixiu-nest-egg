## ADDED Requirements

### Requirement: Hiding a stock
The system SHALL let the user mark a stock as hidden (inactive) and unhide it from the stock registry, regardless of whether it has trades or 派息 records. A hidden stock SHALL remain in the registry list — visually marked — so it can be unhidden, SHALL keep all of its trades, dividends and prices, and SHALL remain selectable when recording trades or dividends. Hiding SHALL NOT bypass the rule that a stock with trades or dividend records cannot be deleted.

#### Scenario: Hiding a fully-sold stock
- **WHEN** the user hides `香港寬頻`, which has trades and 派息 records but zero holdings
- **THEN** the stock is marked inactive and disappears from 持倉總覽 while staying in the registry, visually marked as hidden

#### Scenario: Unhiding a stock
- **WHEN** the user unhides a previously hidden stock
- **THEN** it becomes active again and reappears in 持倉總覽 in its saved order position

#### Scenario: Hidden stock keeps its history
- **WHEN** a stock is hidden
- **THEN** its trades, dividends and prices are unchanged, and it can still be selected in the trade and dividend forms
