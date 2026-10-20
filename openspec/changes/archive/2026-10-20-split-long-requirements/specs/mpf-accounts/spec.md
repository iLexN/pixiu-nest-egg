# Spec Delta

## MODIFIED Requirements

### Requirement: Automatic last-month and max figures

For each account the system SHALL derive, on read: the last-month rate and net gain from the latest history row in the previous calendar month (empty when no such row exists); and the all-time max rate and max net gain as the maxima over the account's seeded high-water marks, every history row, and the current values.

#### Scenario: Last month from history

- **WHEN** an account's latest history row in the previous month has contributions 120000 and balance 150000, viewed today
- **THEN** the account's last-month figures show rate `0.25` and net gain `30000`

#### Scenario: Max raised on update

- **WHEN** an account's reported max rate is `0.47` and an update makes its current rate `0.50`
- **THEN** the reported max rate becomes `0.50` with no manual step

#### Scenario: Independent maxima

- **WHEN** an account's highest-ever rate occurred in March and its highest-ever net gain occurred in August
- **THEN** the account reports the March rate and the August gain as its max figures

## ADDED Requirements

### Requirement: MPF max tracking rules

The max rate and max net gain SHALL be tracked independently — they MAY come from different moments. When an update produces a new high, the reported max SHALL rise automatically without any manual copy step, and deleting a history row SHALL remove that row's contribution to the maxima.

#### Scenario: Max rises and falls with rows

- **WHEN** a history row that produced the max net gain is deleted
- **THEN** the reported max falls back to the next highest row's figure

## MODIFIED Requirements

### Requirement: Derived account and portfolio figures

For each account the system SHALL derive on read: `rate = (balance − contributions) ÷ contributions` (empty when contributions is zero) and `net gain = balance − contributions`. Money and rates SHALL be stored and computed as `REAL`/`f64` matching the spreadsheet, with formatting applied only at display.

#### Scenario: Portfolio totals

- **WHEN** two accounts hold contributions 125171.43 and 459835.51 and balances 160494.56 and 697499.61
- **THEN** the section shows buy `585006.94`, now `857994.17`, rate ≈ `0.4666`, and net gain `272987.23`

#### Scenario: Empty contribution rate

- **WHEN** an account has zero contributions
- **THEN** its rate is empty rather than an error

## ADDED Requirements

### Requirement: MPF portfolio totals

For the section as a whole it SHALL derive: `buy = Σ contributions`, `now = Σ balance`, portfolio rate `(now − buy) ÷ buy`, portfolio net gain `now − buy`, and last-month and max figures aggregated across accounts.

#### Scenario: Portfolio totals across accounts

- **WHEN** two accounts hold contributions 125171.43 and 459835.51 and balances 160494.56 and 697499.61
- **THEN** the section shows buy `585006.94`, now `857994.17`, rate ≈ `0.4666`, and net gain `272987.23`

