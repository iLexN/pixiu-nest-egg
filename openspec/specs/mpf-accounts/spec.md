# MPF Accounts Specification

## Purpose

Tracks MPF (強積金) accounts — contributions, current balance, and return — replacing the workbook `MPF` sheet so the owner only ever edits contributions and balance, while the app automatically derives the last-month and all-time-max figures the sheet makes them copy by hand.

## Requirements

### Requirement: MPF account registry

The system SHALL maintain a list of MPF account records, each carrying a `label` (the sheet's account name, e.g. `new type`, `強積金個人帳戶`), an optional `trustee` (e.g. `宏利`), `contributions` (總供款額), `balance` (帳戶結存), optional metadata `plan_name` and `member_no`, and a stable `sort_order` initially taken from the workbook row order. The system SHALL support creating, listing, and editing accounts. An account that has history records SHALL NOT be deletable.

#### Scenario: Listing accounts

- **WHEN** the user opens the MPF page
- **THEN** every account is shown with its label, trustee, contributions, balance, and metadata, ordered by `sort_order`

#### Scenario: Editing an account

- **WHEN** the user changes an account's contributions, balance, or metadata
- **THEN** the stored record is updated and every derived figure reflects the change on next read

#### Scenario: Deleting an account with history

- **WHEN** the user tries to delete an account that has at least one history record
- **THEN** the system refuses and keeps the account

### Requirement: Account field validation

The system SHALL validate MPF account input before storing it: `contributions` and `balance` MUST NOT be negative when present, and an account MUST have a non-empty `label`.

#### Scenario: Negative balance

- **WHEN** the user submits an account update with balance `-100`
- **THEN** the system rejects it with a validation error naming the balance field, and stores nothing

#### Scenario: Missing label

- **WHEN** the user creates an account with an empty label
- **THEN** the system rejects it with a validation error naming the label field

### Requirement: Balance updates record history

Every update to an account's `contributions` or `balance` SHALL record a history row carrying the account, the update date, and the new contributions and balance. Updating the same account again on the same calendar day SHALL replace that day's history row rather than append a second one, so same-day corrections do not pollute the record. Updates that touch only metadata SHALL NOT create history rows.

#### Scenario: Update appends a history row

- **WHEN** the user sets an account's balance to 160494.56 on `2026-09-17`
- **THEN** a history row exists for that account dated `2026-09-17` carrying the new balance and contributions

#### Scenario: Same-day correction replaces

- **WHEN** the user updates an account's balance twice on the same day
- **THEN** exactly one history row exists for that day, carrying the second update's values

#### Scenario: Metadata-only edit

- **WHEN** the user edits only an account's `member_no`
- **THEN** no new history row is created

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

### Requirement: First-update baseline row

When a contributions/balance update is recorded for an account that has no history rows, the system SHALL first record a baseline history row. That row carries the account's values from before the update, is dated the account's creation day, and is marked synthetic, so the as-of record holds the values that stood until the update. The update then records its own row as usual.

#### Scenario: Baseline before the first update

- **WHEN** an account created on `2026-09-17` with contributions 120000 and balance 150000 and no history is updated on `2026-10-05` to balance 160000
- **THEN** history holds a synthetic row dated `2026-09-17` carrying 120000 / 150000 and a row dated `2026-10-05` carrying the new values, and the account's last-month figures in October show rate `0.25`

#### Scenario: No baseline once history exists

- **WHEN** an account that already has history rows is updated
- **THEN** only the update's own row is recorded

### Requirement: MPF max tracking rules

The max rate and max net gain SHALL be tracked independently — they MAY come from different moments. When an update produces a new high, the reported max SHALL rise automatically without any manual copy step, and deleting a history row SHALL remove that row's contribution to the maxima.

#### Scenario: Max rises and falls with rows

- **WHEN** a history row that produced the max net gain is deleted
- **THEN** the reported max falls back to the next highest row's figure

### Requirement: Derived account and portfolio figures

For each account the system SHALL derive on read: `rate = (balance − contributions) ÷ contributions` (empty when contributions is zero) and `net gain = balance − contributions`. Money and rates SHALL be stored and computed as `REAL`/`f64` matching the spreadsheet, with formatting applied only at display.

#### Scenario: Portfolio totals

- **WHEN** two accounts hold contributions 125171.43 and 459835.51 and balances 160494.56 and 697499.61
- **THEN** the section shows buy `585006.94`, now `857994.17`, rate ≈ `0.4666`, and net gain `272987.23`

#### Scenario: Empty contribution rate

- **WHEN** an account has zero contributions
- **THEN** its rate is empty rather than an error

### Requirement: MPF portfolio totals

For the section as a whole it SHALL derive: `buy = Σ contributions`, `now = Σ balance`, portfolio rate `(now − buy) ÷ buy`, portfolio net gain `now − buy`, and last-month and max figures aggregated across accounts.

#### Scenario: Portfolio totals across accounts

- **WHEN** two accounts hold contributions 125171.43 and 459835.51 and balances 160494.56 and 697499.61
- **THEN** the section shows buy `585006.94`, now `857994.17`, rate ≈ `0.4666`, and net gain `272987.23`

### Requirement: MPF overview page

The system SHALL present an MPF page showing: the portfolio totals header (buy, now, rate, net gain, last-month and max rate and net gain); the per-account table with the same derived columns plus trustee and metadata; a free-text note for the section; and the recent history rows per account including a delete action for correcting stale mistakes. Contributions and balance SHALL be editable in place from this page.

#### Scenario: Editing a balance from the page

- **WHEN** the user edits an account's 帳戶結存 on the MPF page and saves
- **THEN** the page shows the recomputed rate, last-month, and max figures after reload

#### Scenario: Section note

- **WHEN** the user saves a note on the MPF page
- **THEN** the note is persisted and shown on subsequent loads

#### Scenario: Deleting a stale history row

- **WHEN** the user deletes a history row that recorded a mistyped balance
- **THEN** derived last-month and max figures recompute without that row on next read

### Requirement: Workbook reminder hint

The MPF page SHALL show a reminder that `Overview`, `Month Stat`, and the other unmigrated sections still live in `財富分析報告.xlsx`, including that `Overview!B6` reads the now-frozen workbook `MPF` sheet.

#### Scenario: Hint visible

- **WHEN** the user opens the MPF page
- **THEN** the reminder text is displayed alongside the account data
