# Spec Delta

## MODIFIED Requirements

### Requirement: Receiving a dividend
The system SHALL let the user mark a pending dividend received by supplying `received_amount`, with optional `received_price`. When `received_amount` transitions from NULL to a value and the update supplies no `received_price` — the field absent or explicitly null — the record SHALL store the stock's current `manual_price` as `received_price`, or NULL when the stock has no `manual_price`. An explicitly supplied `received_price` SHALL be stored as given; clearing `received_price` on an already-received record (no receipt transition) SHALL keep it NULL. Recording receipt SHALL NOT modify the stored `shares_held`/`buy_cost` snapshots. Setting `received_amount` where it was previously NULL SHALL — atomically with the update — bank the amount (HK dividends credit the `HS` cash manual-asset row; US dividends credit the `ibkr.usd_cash` meta value, already USD — unless the request sets `bank_in` false; the credited amount is stored for exact reversal) and, for HK dividends, record the `div:<id>` `adjustment` item for the pay_date month (skipped when already stored or the month has no row; US dividends never touch 活期 so they record none). Clearing `received_amount` SHALL reverse the stored credit and delete the item.

#### Scenario: Money arrives
- **WHEN** the user records received_amount 16432.10 and received_price 5.41 on a pending dividend
- **THEN** the record's status becomes RECEIVED, both yields are computed, and the earlier estimate remains stored for comparison

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
- **WHEN** a pending HK dividend paying `2026-09-25` is 收訖 with `received_amount` `909.83`
- **THEN** the `HS` cash row gains `909.83`, the `2026-09` month gains a `div:<id>` adjustment item of `909.83`, and September's derived 利息 includes `909.83` on the next read

#### Scenario: US receipt banks into IBKR USD cash
- **WHEN** a pending US dividend is 收訖 with `received_amount` `200`
- **THEN** `ibkr.usd_cash` gains `200` (USD) and no month item is created

#### Scenario: Clearing the receipt reverses
- **WHEN** the user clears `received_amount` on a dividend that had credited `HS` `909.83`
- **THEN** `HS` loses `909.83` and the `div:<id>` item is deleted
