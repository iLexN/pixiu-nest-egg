# Spec Delta

## REMOVED Requirements

### Requirement: Deposit 收訖 lifecycle

**Reason**: The 取消收訖 action is removed — the owner never uses it; carrying an unreachable reversal endpoint and its spec surface is dead weight. The surviving behavior is restated as "Deposit 收訖 flag" and "Deposit 收訖".
**Migration**: Receipt becomes irreversible in-app. A mistaken 收訖 is corrected by editing the credited cash row and month item manually, or deleting and re-creating the deposit.

## ADDED Requirements

### Requirement: Deposit 收訖 flag

Each deposit SHALL carry a `received_at` date (NULL while unreceived). Create SHALL record `received_at = end_date` when the entered `end_date` is already past.

#### Scenario: Past end_date auto-marks on create

- **WHEN** a deposit is created with `end_date` already past
- **THEN** it is stored received with `received_at = end_date`

### Requirement: Deposit 收訖

The system SHALL expose `POST /api/deposits/:id/receive` — marking the deposit received at a date (default today), optionally correcting `interest` to the amount received, optionally crediting a chosen cash `manual_assets` row by exactly `principal + interest` (not user-supplied), and recording the month's `dep-end:<id>` `adjustment` item for that amount; re-receiving SHALL return 409. Receipt SHALL NOT be reversible — no unreceive endpoint or action.

#### Scenario: One-click maturity

- **WHEN** the user 收訖's a deposit (principal `110000`, interest `993`, ending `2026-10-12`) choosing cash row `渣打`
- **THEN** the deposit is stored received, `渣打` gains `110993`, the `2026-10` month gains a `dep-end` adjustment item of `110993`, and the deposit leaves 未到期定期

#### Scenario: Corrected interest flows into the credit

- **WHEN** the user 收訖's a deposit (principal `110000`, stored interest `993`) entering 實收利息 `1000` and choosing cash row `渣打`
- **THEN** the stored interest becomes `1000`, `渣打` gains `111000`, and the `dep-end` item is `111000`

#### Scenario: Receipt without a bank-in

- **WHEN** the user 收訖's a deposit with `不存入` selected
- **THEN** only `received_at` and the dep-end item are recorded — no cash row changes

#### Scenario: dep-end item skipped when already stored or monthless

- **WHEN** the deposit's end month already stores its `dep-end:<id>` item or has no month row
- **THEN** the credit and flag apply but no item is recorded
