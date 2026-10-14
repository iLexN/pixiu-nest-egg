# Spec Delta

## MODIFIED Requirements

### Requirement: Bond principal 收訖 lifecycle

Each bond SHALL carry a `received_at` date (NULL while the principal return is unconfirmed) alongside the per-coupon receipt lifecycle, which stays separate. The system SHALL expose `POST /api/bonds/:id/receive` — marking the bond's principal received at a given date (defaulting to today), optionally crediting a chosen cash `manual_assets` row by an amount defaulting to the bond's `principal`, and recording the maturity month's `bond-end:<id>` `adjustment` item (skipped when already stored or the month has no row); receiving an already-received bond SHALL return 409. The system SHALL expose `POST /api/bonds/:id/unreceive` — reversing the stored cash credit, deleting the `bond-end` item, and clearing the flag; unreceiving an unreceived bond SHALL return 409. Create SHALL record `received_at = maturity_date` when the entered maturity is already past. The matured-bonds section SHALL flag an unreceived bond 本金未收 and offer the 收訖 action; a received one shows its 收訖日 and a 取消收訖 action. The active/matured split and `債券!B1` active principal remain maturity-date based.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash row `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a bond that had credited `HS` `100000` and created a `bond-end` item
- **THEN** `HS` loses `100000`, the item is deleted, and the bond returns to 本金未收
