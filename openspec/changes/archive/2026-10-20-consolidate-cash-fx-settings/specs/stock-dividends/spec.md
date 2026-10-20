# Spec Delta

## MODIFIED Requirements

### Requirement: Receiving a dividend
The system SHALL let the user mark a pending dividend received by supplying either `received_amount` (the net amount), or `gross_amount` with an optional `withholding_tax`, plus optional `received_price`.

**Gross input.** When `gross_amount` is supplied, the system SHALL set `received_amount` = `gross_amount` − tax. The tax is the request's `withholding_tax` when supplied, otherwise the stored `withholding_tax`, otherwise 0. The request's `withholding_tax` SHALL be stored; `gross_amount` itself SHALL NOT be stored. The same gross + tax input SHALL be accepted when editing an already-received dividend, recomputing its `received_amount`.

**Price snapshot.** When `received_amount` transitions from NULL to a value and the update supplies no `received_price` (the field absent or explicitly null), the record SHALL store the stock's current `manual_price` as `received_price`, or NULL when the stock has no `manual_price`. An explicitly supplied `received_price` SHALL be stored as given. Clearing `received_price` on an already-received record (no receipt transition) SHALL keep it NULL.

**Snapshots untouched.** Recording receipt SHALL NOT modify the stored `shares_held`/`buy_cost` snapshots.

**Bank-in.** Setting `received_amount` where it was previously NULL SHALL, atomically with the update, credit the net `received_amount` under the cash-accounts receipt crediting rule. The receipt currency is `HKD` for HK dividends and `USD` for US dividends. The request MAY carry `bank_in` (default true) and an `account_id` override. For HK dividends, the same transaction SHALL record the `div:<id>` `adjustment` item for the pay_date month. The item is skipped when already stored or when the month has no row. US dividends never touch 活期, so they record none.

**Reversal.** Clearing `received_amount` SHALL reverse the recorded credit on the account it went to, delete the item, and clear `withholding_tax`.

#### Scenario: Money arrives
- **WHEN** the user records received_amount 16432.10 and received_price 5.41 on a pending dividend
- **THEN** the record's status becomes RECEIVED, both yields are computed, and the earlier estimate remains stored for comparison

#### Scenario: US receipt from gross and tax
- **WHEN** a pending US dividend is 收訖 with gross_amount `12.76` and withholding_tax `3.83`, and the IBKR USD broker account is the USD default
- **THEN** the record stores received_amount `8.93` and withholding_tax `3.83`, reads back gross_amount `12.76`, and the IBKR USD account gains `8.93`

#### Scenario: Gross without tax
- **WHEN** a pending dividend is 收訖 with gross_amount `200` and no withholding_tax
- **THEN** the record stores received_amount `200` and withholding_tax stays empty

#### Scenario: Blank price snapshots the current 現價
- **WHEN** the user 收訖 a pending dividend supplying received_amount only, and the stock's 現價 is 5.41
- **THEN** the record stores received_price 5.41 and yield_on_price is computed from it

#### Scenario: Blank price with no 現價 recorded
- **WHEN** the user 收訖 a pending dividend supplying received_amount only, and the stock has no 現價
- **THEN** the record stores received_price NULL and yield_on_price stays empty

#### Scenario: Clearing price on a received record
- **WHEN** the user clears received_price on a dividend that is already received
- **THEN** received_price stays NULL — the current 現價 is not re-applied

#### Scenario: HK receipt banks into HS and records the item
- **WHEN** HS is the HKD default and a pending HK dividend paying `2026-09-25` is 收訖 with `received_amount` `909.83`
- **THEN** HS gains `909.83`, the `2026-09` month gains a `div:<id>` adjustment item of `909.83`, and September's derived 利息 includes `909.83` on the next read

#### Scenario: HK receipt into a chosen account
- **WHEN** a pending HK dividend is 收訖 with `received_amount` `909.83` and `account_id` = 渣打
- **THEN** 渣打 gains `909.83`, HS is unchanged, and the `div:<id>` item is still recorded

#### Scenario: US receipt banks into IBKR USD cash
- **WHEN** a pending US dividend is 收訖 with `received_amount` `200`
- **THEN** the USD default (the IBKR USD account) gains `200` (USD) and no month item is created

#### Scenario: Clearing the receipt reverses
- **WHEN** the user clears `received_amount` on a dividend that had credited HS `909.83`
- **THEN** HS loses `909.83` and the `div:<id>` item is deleted

#### Scenario: Clearing the receipt clears the tax
- **WHEN** the user clears `received_amount` on a US dividend that stored withholding_tax `3.83` and credited the IBKR USD account `8.93`
- **THEN** the IBKR USD account loses `8.93` and withholding_tax becomes empty
