# Spec Delta

## MODIFIED Requirements

### Requirement: Automatic last-month and max figures

For each account the system SHALL derive, on read: the last-month rate and net gain from the latest history row dated on or before the last day of the previous calendar month (empty when no such row exists); and the all-time max rate and max net gain as the maxima over the account's seeded high-water marks, every history row, and the current values.

#### Scenario: Last month from history

- **WHEN** an account's latest history row in the previous month has contributions 120000 and balance 150000, viewed today
- **THEN** the account's last-month figures show rate `0.25` and net gain `30000`

#### Scenario: Max raised on update

- **WHEN** an account's reported max rate is `0.47` and an update makes its current rate `0.50`
- **THEN** the reported max rate becomes `0.50` with no manual step

#### Scenario: Independent maxima

- **WHEN** an account's highest-ever rate occurred in March and its highest-ever net gain occurred in August
- **THEN** the account reports the March rate and the August gain as its max figures

#### Scenario: Month without an update carries forward

- **WHEN** an account's latest history row is dated `2026-08-28` with contributions 120000 and balance 150000, and it was not updated in September, viewed on `2026-10-05`
- **THEN** the account's last-month figures show rate `0.25` and net gain `30000`

## ADDED Requirements

### Requirement: First-update baseline row

When a contributions/balance update is recorded for an account that has no history rows, the system SHALL first record a baseline history row. That row carries the account's values from before the update, is dated the account's creation day, and is marked synthetic, so the as-of record holds the values that stood until the update. The update then records its own row as usual.

#### Scenario: Baseline before the first update

- **WHEN** an account created on `2026-09-17` with contributions 120000 and balance 150000 and no history is updated on `2026-10-05` to balance 160000
- **THEN** history holds a synthetic row dated `2026-09-17` carrying 120000 / 150000 and a row dated `2026-10-05` carrying the new values, and the account's last-month figures in October show rate `0.25`

#### Scenario: No baseline once history exists

- **WHEN** an account that already has history rows is updated
- **THEN** only the update's own row is recorded

## REMOVED Requirements

### Requirement: Month-rollover backfill

**Reason**: The last-month figures, and the portfolio's existing as-of merge, now read the latest row on or before the previous month-end. Synthetic month-end rows are therefore redundant. The one case they protected (an account's values from before its first update) is covered by the first-update baseline row.

**Migration**: None. Existing synthetic rows stay in `mpf_history` and remain valid inputs; no new month-end rows are created.
