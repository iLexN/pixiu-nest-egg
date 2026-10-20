# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit registry

The system SHALL maintain a list of deposit records, each carrying an optional `label` (the sheet's `id` column, e.g. `SC-9632`), an optional `bank` code (e.g. `SC` for 渣打, `HS` for 恒生), optional `principal` (the sheet's `input`), optional `rate` (annual rate as a fraction, e.g. `0.03`), and optional `interest` (利息).

#### Scenario: Recording a deposit

- **WHEN** the user records a deposit with label `SC-9632`, principal 110000, rate 0.028, interest 993, and end_date `2026-10-12`
- **THEN** the deposit is stored and appears in the deposit list

#### Scenario: Interest-only row

- **WHEN** the user records a deposit with no label and no principal, interest 539.25, and end_date `2026-05-14`
- **THEN** the deposit is stored, matching the interest-only entries the workbook list contains

#### Scenario: Start date is optional

- **WHEN** the user records a deposit with no `start_date`
- **THEN** the deposit is stored with an empty start date

#### Scenario: Editing a deposit

- **WHEN** the user changes a deposit's end_date or interest
- **THEN** the stored record is updated and every aggregate view reflects the change on next read

#### Scenario: Deleting a deposit

- **WHEN** the user deletes a deposit
- **THEN** it no longer appears in the list or in any aggregate

## ADDED Requirements

### Requirement: Deposit date and note fields

Each deposit record SHALL also carry an optional `start_date` (`YYYY-MM-DD` — when the principal left the bank account), a required `end_date` (`YYYY-MM-DD`), and two optional free-text notes (`note1`, `note2`).

#### Scenario: Dates and notes stored

- **WHEN** the user records a deposit with `start_date`, `end_date`, `note1` and `note2`
- **THEN** the record stores all four fields

## ADDED Requirements

### Requirement: Deposit record operations

The system SHALL support creating, listing, editing, and deleting deposit records, and SHALL preserve each record's position via a stable `sort_order`, new records appending after existing ones.

#### Scenario: Ordering preserved

- **WHEN** a deposit is created after existing ones
- **THEN** it appends after them via its `sort_order`

## MODIFIED Requirements

### Requirement: Derived deposit fields

The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until the user confirms receipt; `end_month`/`end_year` taken from `end_date`.

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

## ADDED Requirements

### Requirement: Derived annual rate

The system SHALL compute on read, never store, when no rate is stored, a derived annual `rate = interest ÷ principal × 365 ÷ days` with `days = end_date − start_date` in calendar days, reported only when principal is positive, interest and start_date are present, and end_date is after start_date, and flagged as derived. A stored rate is always reported as entered and never flagged as derived.

#### Scenario: Rate derived from dates

- **WHEN** a deposit has principal `140000`, interest `1279`, `start_date` `2026-10-13` and `end_date` `2027-02-02`, and no stored rate
- **THEN** its reported `rate` is `1279 ÷ 140000 × 365 ÷ 112` (about `0.02977`), flagged as derived

## MODIFIED Requirements

### Requirement: Deposit 收訖

The system SHALL expose `POST /api/deposits/:id/receive`. It marks the deposit received at a date (default today) and MAY correct `interest` to the amount actually received.

It credits exactly `principal + interest` (not user-supplied) under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true) and an optional `account_id` override; the 收訖 form preselects the account matching the deposit's bank when one exists.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash account `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Corrected interest flows into the credit

- **WHEN** the user 收訖's a deposit (principal `110000`, stored interest `993`) entering 實收利息 `1000` and choosing cash account `渣打`
- **THEN** the stored interest becomes `1000`, `渣打` gains `111000`, and the `dep-end` item is `111000`

#### Scenario: Default account when none chosen

- **WHEN** the user 收訖's a deposit with no `account_id` and bank-in on, and HS is the HKD default
- **THEN** HS gains `principal + interest`

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected (`bank_in` false)
- **THEN** only `received_at` and the dep-end item are recorded — no cash account changes

#### Scenario: dep-end item skipped when already stored or monthless

- **WHEN** the deposit's end month already stores its `dep-end:<id>` item or has no month row
- **THEN** the credit and flag apply but no item is recorded

## ADDED Requirements

### Requirement: Deposit receipt month item and finality

The receive endpoint also records the month's `dep-end:<id>` `adjustment` item for that amount. Re-receiving SHALL return 409. Receipt SHALL NOT be reversible: there is no unreceive endpoint or action.

#### Scenario: Receive is final

- **WHEN** a deposit has been received
- **THEN** a second receive call returns 409 and no unreceive action exists

