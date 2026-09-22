# Spec Delta

## ADDED Requirements

### Requirement: Bond principal 收訖 lifecycle

Each bond SHALL carry a `received_at` date (NULL while the principal return is unconfirmed) alongside the per-coupon receipt lifecycle, which stays separate. The system SHALL expose `POST /api/bonds/:id/receive` — marking the bond's principal received at a given date (defaulting to today), optionally crediting a chosen cash `manual_assets` row by an amount defaulting to the bond's `principal`, and recording the maturity month's `bond-end:<id>` `adjustment` item (skipped when already stored or the month has no row); receiving an already-received bond SHALL return 409. The system SHALL expose `POST /api/bonds/:id/unreceive` — reversing the stored cash credit, deleting the `bond-end` item, and clearing the flag; unreceiving an unreceived bond SHALL return 409. Import and create SHALL record `received_at = maturity_date` when the entered maturity is already past. The matured-bonds section SHALL flag an unreceived bond 本金未收 and offer the 收訖 action; a received one shows its 收訖日 and a 取消收訖 action. The active/matured split and `債券!B1` active principal remain maturity-date based.

#### Scenario: One-click principal return

- **WHEN** the user 收訖's a matured bond (principal `100000`, maturity `2026-10-25`) choosing cash row `HS`
- **THEN** the bond is stored received, `HS` gains `100000`, and the `2026-10` month gains a `bond-end` adjustment item of `100000`

#### Scenario: Principal receipt without a bank-in

- **WHEN** the user 收訖's a matured bond with `不存入` selected
- **THEN** only `received_at` and the `bond-end` item are recorded — no cash row changes

#### Scenario: Undo restores the pending state

- **WHEN** the user 取消收訖's a bond that had credited `HS` `100000` and created a `bond-end` item
- **THEN** `HS` loses `100000`, the item is deleted, and the bond returns to 本金未收

## MODIFIED Requirements

### Requirement: Coupon receipt lifecycle

A coupon's `status` SHALL be derived on read: `received` once `received_amount` is set; `待定` while `annual_rate`/`per_10k` are unset; otherwise `pending`. `variance` SHALL be derived as `received_amount − expected` when both exist. Entering the announced rate SHALL be a `PATCH` that sets `annual_rate`/`per_10k`; recording receipt SHALL be a `PATCH` that sets `received_amount`. Setting `received_amount` where it was previously NULL SHALL — atomically with the update — credit the amount to the `HS` cash manual-asset row (unless the request sets `bank_in` false or no such row exists; the credited amount is stored for exact reversal) and record the `coupon:<id>` `adjustment` item for the pay_date month (skipped when already stored or the month has no row). Clearing `received_amount` SHALL reverse the stored credit and delete the item. A coupon SHALL be editable and deletable regardless of status.

#### Scenario: Rate fixing moves coupon to pending

- **WHEN** the user patches a 待定 coupon with `annual_rate` `0.04` and `per_10k` `200.55`
- **THEN** the coupon's status becomes `pending` and its `expected` is `1002.75` for a `50000` principal

#### Scenario: Receipt recorded

- **WHEN** the user patches a pending coupon with `received_amount` `1002.75`
- **THEN** its status becomes `received` and `variance` is `0`

#### Scenario: Received amount differing from expected

- **WHEN** a coupon with `expected` `997.25` is marked received with `received_amount` `1000`
- **THEN** its `variance` reports `2.75`

#### Scenario: Receipt banks into HS and records the item

- **WHEN** a pending coupon paying `2026-10-23` is 收訖 with `received_amount` `1000`
- **THEN** the `HS` cash row gains `1000`, the `2026-10` month gains a `coupon:<id>` adjustment item of `1000`, and October's derived 利息 includes `1000` on the next read

#### Scenario: Clearing the receipt reverses

- **WHEN** the user clears `received_amount` on a coupon that had credited `HS` `1000`
- **THEN** `HS` loses `1000` and the `coupon:<id>` item is deleted
