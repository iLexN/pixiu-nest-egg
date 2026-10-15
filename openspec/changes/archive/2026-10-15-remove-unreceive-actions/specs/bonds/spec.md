# Spec Delta

## REMOVED Requirements

### Requirement: Bond principal 收訖 lifecycle

**Reason**: The 取消收訖 action is removed — the owner never uses it; carrying an unreachable reversal endpoint and its spec surface is dead weight. The surviving behavior is restated as "Bond principal 收訖 flag" and "Bond principal 收訖".
**Migration**: Receipt becomes irreversible in-app. A mistaken 收訖 is corrected by editing the credited cash row and month item manually, or deleting and re-creating the bond.

## ADDED Requirements

### Requirement: Bond principal 收訖 flag

Each bond SHALL carry a `received_at` date (NULL while the principal return is unconfirmed), separate from the per-coupon receipt lifecycle. Create SHALL record `received_at = maturity_date` when the entered maturity is already past. The active/matured split and `債券!B1` active principal SHALL remain maturity-date based.

#### Scenario: Past maturity auto-marks on create

- **WHEN** a bond is created with `maturity_date` already past
- **THEN** it is stored received with `received_at = maturity_date`

### Requirement: Bond principal 收訖

`POST /api/bonds/:id/receive` SHALL mark the bond's principal received at a date (default today), optionally crediting a chosen cash `manual_assets` row by an amount defaulting to `principal`, and recording the maturity month's `bond-end:<id>` `adjustment` item; re-receiving SHALL return 409. Receipt SHALL NOT be reversible. The matured-bonds section SHALL flag an unreceived bond 本金未收 and offer 收訖; a received one shows its 收訖日.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash row `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash row changes

#### Scenario: bond-end item skipped when already stored or monthless

- **WHEN** the maturity month already stores the `bond-end:<id>` item or has no month row
- **THEN** the credit and flag apply but no item is recorded
