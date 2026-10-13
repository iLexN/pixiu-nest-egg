# Spec Delta

## MODIFIED Requirements

### Requirement: Per-holder summary and history

The system SHALL present a summary grouped by holder — every holder that has deposits or a note — each with its note, its unreceived deposits, and its active principal (Σ principal over the holder's deposits that are not 收訖, whatever their `end_date` — a matured-but-unreceived deposit keeps counting until receipt is confirmed). The page SHALL show one compact block per holder (name, active principal, note) and a **single** 未到期定期 table across all holders — every unreceived deposit ordered by end date (flagging 已到期未收 where past due) with a 持有人 column plus label, bank, principal, interest, total, start_date, end_date, and note; there SHALL NOT be a separate upcoming table per holder. No per-month rollup table is shown per holder. The page SHALL also offer a year filter, defaulting to the current year, listing deposits ending in that year.

#### Scenario: Active principal per holder

- **WHEN** 媽媽 has one unreceived deposit of 100000 ending in the future and five received ones
- **THEN** 媽媽's active principal is 100000 and only the unreceived deposit is in her upcoming list

#### Scenario: Matured deposit counts until 收訖

- **WHEN** 媽媽's deposit SC-9179 of 100000 ended `2026-10-13`, today is `2026-10-13`, and it is not 收訖
- **THEN** 媽媽's active principal still includes 100000 and the deposit is flagged 已到期未收; after 收訖 her active principal drops by 100000

#### Scenario: History by year

- **WHEN** the user selects year 2026
- **THEN** the family deposits ending in 2026 are listed with their holder, interest, and total
