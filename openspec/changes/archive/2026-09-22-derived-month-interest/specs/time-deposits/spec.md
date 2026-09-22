# Spec Delta

## ADDED Requirements

### Requirement: Deposit 收訖 lifecycle

Each deposit SHALL carry a `received_at` date (NULL while unreceived). The system SHALL expose `POST /api/deposits/:id/receive` — marking the deposit received at a given date (defaulting to today), optionally correcting the stored `interest` to the amount actually received, optionally crediting a chosen cash `manual_assets` row by an amount defaulting to `principal + interest`, and recording the month's `dep-end:<id>` `adjustment` item (skipped when already stored, when the month has no row, or after re-entry); receiving an already-received deposit SHALL return 409. The system SHALL expose `POST /api/deposits/:id/unreceive` — reversing the stored cash credit, deleting the `dep-end` item, and clearing the flag; unreceiving an unreceived deposit SHALL return 409. Import and create SHALL record `received_at = end_date` when the entered `end_date` is already past, matching the previously date-based behavior.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash row `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected
- **THEN** only `received_at` and the dep-end item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a deposit that had credited `渣打` `110993` and created a `dep-end` item
- **THEN** `渣打` loses `110993`, the item is deleted, and the deposit returns to 未到期定期

## MODIFIED Requirements

### Requirement: Derived deposit fields

The system SHALL compute, on read and never store: `total = COALESCE(principal, 0) + COALESCE(interest, 0)`; `status` = `End` once 收訖 (`received_at` set), otherwise active — a deposit stays active past its `end_date` until the user confirms receipt; and `end_month`/`end_year` taken from `end_date`.

#### Scenario: Future end date is active

- **WHEN** a deposit's end_date is later than today
- **THEN** its status is active and it appears in the Upcoming section

#### Scenario: End date reached

- **WHEN** a deposit's end_date is today or in the past and it is not 收訖
- **THEN** it stays in the Upcoming section marked 已到期未收 until the user confirms receipt — the end date alone no longer ends it

#### Scenario: Total from partial fields

- **WHEN** a deposit has interest 539.25 and no principal
- **THEN** its derived total is 539.25

### Requirement: Upcoming deposit view

The system SHALL present the **unreceived** deposits ordered by end date (a matured-but-unreceived deposit is included and flagged 已到期未收), each showing label, principal, rate, interest, total, end_date, and end_month, with a 收訖 action per row. The totals and rollups remain end_date-based to match the sheet's formulas: the total active principal (Σ principal of deposits whose `end_date` is in the future); a month rollup grouping those deposits by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup by `bank` with the same three sums. Deposits without a bank SHALL be grouped under their own bucket rather than matched to a bank.

#### Scenario: Active principal total

- **WHEN** the imported workbook data is live
- **THEN** the total active principal equals the sum of principal over deposits whose end date is in the future (445,000 for the current workbook)

#### Scenario: Overdue deposit stays listed

- **WHEN** a deposit ended `2026-10-12` is not yet 收訖
- **THEN** it still appears in 未到期定期 with a 已到期未收 marker while the totals no longer count it

#### Scenario: Month rollup

- **WHEN** active deposits end in October 2026 and November 2026
- **THEN** the rollup lists those months with their Σ total, Σ interest, and Σ principal

#### Scenario: Bank rollup by bank

- **WHEN** active deposits carry banks `SC` (four deposits) and `HS` (one deposit)
- **THEN** the bank rollup reports an `SC` group (Σ principal 365,000) and an `HS` group (Σ principal 80,000)
