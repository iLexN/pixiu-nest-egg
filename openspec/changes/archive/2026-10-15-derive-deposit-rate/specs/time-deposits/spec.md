# Spec Delta

## MODIFIED Requirements

### Requirement: Derived deposit fields
The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until the user confirms receipt; `end_month`/`end_year` taken from `end_date`; and, when no rate is stored, a derived annual `rate = interest ÷ principal × 365 ÷ days` with `days = end_date − start_date` in calendar days, reported only when principal is positive, interest and start_date are present, and end_date is after start_date, and flagged as derived. A stored rate is always reported as entered and never flagged as derived.

#### Scenario: Future end date is active
- **WHEN** a deposit's end_date is later than today
- **THEN** its status is active and it appears in the Upcoming section

#### Scenario: End date reached
- **WHEN** a deposit's end_date is today or in the past and it is not 收訖
- **THEN** it stays in the Upcoming section marked 已到期未收 until the user confirms receipt — the end date alone no longer ends it

#### Scenario: Total from partial fields
- **WHEN** a deposit has interest 539.25 and no principal
- **THEN** its derived total is 539.25

#### Scenario: Blank rate is derived from interest and dates
- **WHEN** a deposit has principal 140000, interest 1279, start_date `2026-10-13`, end_date `2027-02-02`, and no stored rate
- **THEN** its reported rate is 1279 ÷ 140000 × 365 ÷ 112 (about 0.02977), it is flagged as derived, and the stored rate stays empty

#### Scenario: Missing inputs leave the rate empty
- **WHEN** a deposit has no stored rate and lacks a start_date, has no interest, has no positive principal, or has an end_date not after its start_date
- **THEN** its reported rate is empty and not flagged as derived

#### Scenario: Entered rate wins
- **WHEN** a deposit stores rate 0.028162004662004664 alongside principal, interest, and both dates
- **THEN** its reported rate is the stored 0.028162004662004664 and it is not flagged as derived

#### Scenario: Derived rate follows an interest correction
- **WHEN** a deposit with a derived rate is 收訖'd with its interest corrected to a different amount
- **THEN** the next read reports a rate derived from the corrected interest

### Requirement: Display formatting
Deposit display SHALL follow the app's conventions: money to 2 decimal places and rates shown as percentages; stored values are never rounded. A derived rate SHALL be displayed like an entered one and identified as derived by a hover hint; editing a deposit whose rate is derived SHALL leave the rate entry empty, showing the derived value only as a hint, so saving does not store it.

#### Scenario: Rate display
- **WHEN** a deposit stores rate `0.028162004662004664`
- **THEN** the view displays it as a percentage such as `2.82%` while the stored value is unchanged

#### Scenario: Derived rate hint
- **WHEN** a deposit's rate is derived
- **THEN** the 利率 column shows it as a percentage with a hover hint saying it was calculated from the interest and dates

#### Scenario: Saving an edit keeps the rate derived
- **WHEN** the user edits a deposit whose rate is derived and saves without typing a rate
- **THEN** the stored rate stays empty and the rate continues to be derived
