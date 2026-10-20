# Spec Delta

## MODIFIED Requirements

### Requirement: Deposit 收訖

The system SHALL expose `POST /api/deposits/:id/receive`. It marks the deposit received at a date (default today) and MAY correct `interest` to the amount actually received.

It credits exactly `principal + interest` (not user-supplied) under the cash-accounts receipt crediting rule, with receipt currency `HKD`. The request carries `bank_in` (default true) and an optional `account_id` override; the 收訖 form preselects the account matching the deposit's bank when one exists.

It also records the month's `dep-end:<id>` `adjustment` item for that amount. Re-receiving SHALL return 409. Receipt SHALL NOT be reversible: there is no unreceive endpoint or action.

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
