# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit registry

The system SHALL maintain a list of deposit records, each carrying an optional `label` (the sheet's `id` column, e.g. `SC-9632`), an optional `bank` code (e.g. `SC` for 渣打, `HS` for 恒生), optional `principal` (the sheet's `input`), optional `rate` (annual rate as a fraction, e.g. `0.03`), optional `interest` (利息), an optional `start_date` (`YYYY-MM-DD` — when the principal left the bank account), a required `end_date` (`YYYY-MM-DD`), and two optional free-text notes (`note1`, `note2`). The system SHALL support creating, listing, editing, and deleting deposit records, and SHALL preserve each record's position via a stable `sort_order`, new records appending after existing ones.

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

### Requirement: Deposit 收訖 lifecycle

Each deposit SHALL carry a `received_at` date (NULL while unreceived). The system SHALL expose `POST /api/deposits/:id/receive` — marking the deposit received at a given date (defaulting to today), optionally correcting the stored `interest` to the amount actually received, optionally crediting a chosen cash `manual_assets` row by an amount defaulting to `principal + interest`, and recording the month's `dep-end:<id>` `adjustment` item (skipped when already stored, when the month has no row, or after re-entry); receiving an already-received deposit SHALL return 409. The system SHALL expose `POST /api/deposits/:id/unreceive` — reversing the stored cash credit, deleting the `dep-end` item, and clearing the flag; unreceiving an unreceived deposit SHALL return 409. Create SHALL record `received_at = end_date` when the entered `end_date` is already past.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash row `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected
- **THEN** only `received_at` and the dep-end item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a deposit that had credited `渣打` `110993` and created a `dep-end` item
- **THEN** `渣打` loses `110993`, the item is deleted, and the deposit returns to 未到期定期

## ADDED Requirements

### Requirement: Unreceived deposit list
The system SHALL present the **unreceived** deposits ordered by end date — a matured-but-unreceived deposit is included and flagged 已到期未收 — each showing label, principal, rate, interest, total, end_date, and end_month, with a 收訖 action per row. A deposit leaves the list once 收訖, including one 收訖'd before its `end_date`.

#### Scenario: Overdue deposit stays listed
- **WHEN** a deposit ended `2026-10-13`, today is `2026-10-13`, and it is not 收訖
- **THEN** it appears in 未到期定期 with a 已到期未收 marker

#### Scenario: Early receipt leaves the list
- **WHEN** a deposit whose `end_date` is still in the future is 收訖'd
- **THEN** it no longer appears in 未到期定期

### Requirement: Receipt-based deposit totals
The system SHALL derive the deposit totals over exactly the deposits the unreceived list shows, whatever their `end_date`: the total active principal (Σ principal); a month rollup by end year and month with Σ total, Σ interest, and Σ principal; and a bank rollup by `bank` with the same sums, deposits without a bank in their own bucket. The active principal SHALL feed Overview 已定期 and the live 總數/流動資產 at principal only.

#### Scenario: Active principal total
- **WHEN** the unreceived deposits hold principal 145,000, 140,000, 80,000, and 80,000 and every other deposit is received
- **THEN** the total active principal is 445,000

#### Scenario: Matured deposit keeps counting
- **WHEN** deposits of 110,000 and 35,000 ended `2026-10-13`, today is `2026-10-13`, and neither is 收訖
- **THEN** the total active principal and Overview 已定期 still include 145,000, and live 流動資產 is unchanged by the maturity

#### Scenario: 收訖 removes the deposit from the totals
- **WHEN** the user 收訖's an overdue deposit of 110,000 with interest 993, choosing 存入活期 on the HS cash row
- **THEN** the total active principal drops by 110,000, the HS cash row rises by 110,993, and live 流動資產 rises by only the 993 interest

#### Scenario: Month rollup
- **WHEN** active deposits end in October 2026 and November 2026
- **THEN** the rollup lists those months with their Σ total, Σ interest, and Σ principal

#### Scenario: Bank rollup by bank
- **WHEN** active deposits carry banks `SC` (four deposits) and `HS` (one deposit)
- **THEN** the bank rollup reports an `SC` group (Σ principal 365,000) and an `HS` group (Σ principal 80,000)

## REMOVED Requirements

### Requirement: Upcoming deposit view
**Reason**: Its parity clause and import scenario go with the workbook import; the remaining behavior is restated as "Unreceived deposit list" and "Receipt-based deposit totals".
**Migration**: No behavior change for the app; see the two ADDED requirements.
