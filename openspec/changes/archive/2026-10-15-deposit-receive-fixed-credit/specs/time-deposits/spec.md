# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit 收訖 lifecycle

Each deposit SHALL carry a `received_at` date (NULL while unreceived). The system SHALL expose `POST /api/deposits/:id/receive` — marking the deposit received at a given date (defaulting to today), optionally correcting the stored `interest` to the amount actually received, optionally crediting a chosen cash `manual_assets` row by exactly `principal + interest` (the corrected interest when given; the amount is not user-supplied), and recording the month's `dep-end:<id>` `adjustment` item for the same amount (skipped when already stored, when the month has no row, or after re-entry); receiving an already-received deposit SHALL return 409. The system SHALL expose `POST /api/deposits/:id/unreceive` — reversing the stored cash credit, deleting the `dep-end` item, and clearing the flag; unreceiving an unreceived deposit SHALL return 409. Create SHALL record `received_at = end_date` when the entered `end_date` is already past.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash row `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Corrected interest flows into the credit

- **WHEN** the user 收訖's a deposit (principal `110000`, stored interest `993`) entering 實收利息 `1000` and choosing cash row `渣打`
- **THEN** the stored interest becomes `1000`, `渣打` gains `111000`, and the `dep-end` item is `111000`

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected
- **THEN** only `received_at` and the dep-end item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a deposit that had credited `渣打` `110993` and created a `dep-end` item
- **THEN** `渣打` loses `110993`, the item is deleted, and the deposit returns to 未到期定期
