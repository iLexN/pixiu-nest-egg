# Spec Delta

## MODIFIED Requirements

### Requirement: Dividend registry
The system SHALL maintain a list of dividend records, each tied to a registered stock and carrying a required `pay_date` (`YYYY-MM-DD`, the sheet's K column), an optional `per_share` (announced 每股派息), `shares_held` and `buy_cost` snapshots, an `estimated_amount` (預期派息), an optional `received_amount` (實收派息, the net amount actually credited) and `received_price`, an optional `withholding_tax` (Tax withheld from the gross 派息), and an optional free-text `note`. The system SHALL support creating, listing, editing, and deleting dividend records.

#### Scenario: Recording an expected dividend
- **WHEN** the user records a dividend for 中國銀行 with pay_date `2026-09-30` and per_share `0.25`
- **THEN** the record is stored and appears in the dividend list as pending

#### Scenario: Editing a dividend
- **WHEN** the user changes a dividend's pay_date or per_share
- **THEN** the stored record is updated and list/summary views reflect the change on next read

#### Scenario: Deleting a dividend
- **WHEN** the user deletes a dividend record
- **THEN** it no longer appears in any list or aggregate

#### Scenario: Existing records carry no tax
- **WHEN** dividend records created before withholding tax was tracked, or imported from the workbook, are read
- **THEN** their withholding_tax is empty and their received_amount is unchanged

### Requirement: Dividend validation
The system SHALL validate dividend input before storing it: `pay_date` MUST be a calendar date in `YYYY-MM-DD` form; `per_share`, `estimated_amount`, `received_amount`, `received_price`, `withholding_tax`, and a supplied `gross_amount` MUST be finite and non-negative when present; and a new record MUST carry at least one of `per_share` or `estimated_amount`. The referenced stock MUST exist. `withholding_tax` MUST NOT exceed the supplied `gross_amount`, and MUST NOT be present on a record that has no `received_amount` after the update. A single update MUST NOT supply both `gross_amount` and `received_amount`.

#### Scenario: Malformed pay date
- **WHEN** the user submits a dividend with pay_date `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Nothing to estimate
- **WHEN** the user submits a dividend with neither per_share nor estimated_amount
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Unknown stock
- **WHEN** the user submits a dividend for a stock code that is not registered in the given market
- **THEN** the system rejects it with a validation error naming the code

#### Scenario: Tax larger than gross
- **WHEN** the user 收訖 a dividend with gross_amount `10` and withholding_tax `12`
- **THEN** the system rejects it with a validation error naming withholding_tax, and stores nothing

#### Scenario: Gross and net together
- **WHEN** an update supplies both gross_amount and received_amount
- **THEN** the system rejects it with a validation error naming gross_amount, and stores nothing

#### Scenario: Tax on a pending dividend
- **WHEN** an update sets withholding_tax on a dividend that stays without a received_amount
- **THEN** the system rejects it with a validation error naming withholding_tax, and stores nothing

### Requirement: Derived dividend fields
The system SHALL compute on read, never store: `status` (`RECEIVED` once `received_amount` is present, otherwise `PENDING`); the effective `amount` (`received_amount` when present, otherwise `estimated_amount`); `yield_on_cost` = amount ÷ buy_cost when buy_cost is positive (the sheet's `rate` column); `yield_on_price` = amount ÷ (received_price × shares_held) when received_price and shares_held are present (the sheet's second rate column); `variance` = received_amount − estimated_amount when both are present; and `gross_amount` = received_amount + withholding_tax when both are present, otherwise empty. When only `per_share` is supplied, `estimated_amount` SHALL be derived as `per_share × shares_held`; when no `per_share` is supplied, it SHALL be implied as the effective amount (received preferred) ÷ `shares_held` whenever shares_held is positive.

#### Scenario: Pending yield on cost
- **WHEN** a pending dividend has estimated_amount 17000 and buy_cost 208746.64
- **THEN** its yield_on_cost is 17000 ÷ 208746.64 ≈ 0.0814 and its status is PENDING

#### Scenario: Received yield on price
- **WHEN** a dividend is received with amount 16500, received_price 5.41, and shares_held 68000
- **THEN** its yield_on_price is 16500 ÷ (5.41 × 68000) ≈ 0.0449 and yield_on_cost uses the received amount

#### Scenario: Missing denominators
- **WHEN** a dividend has buy_cost 0 or no received_price
- **THEN** the corresponding yield is empty rather than zero

#### Scenario: Per-share only input
- **WHEN** the user records a dividend with per_share 0.23 and the snapshot holds 10000 shares
- **THEN** estimated_amount is stored as 2300

#### Scenario: Amount-only input
- **WHEN** a dividend carries an amount but no per_share and the snapshot holds 4000 shares
- **THEN** per_share is implied as amount ÷ 4000 and stored on the record

#### Scenario: Gross derived from net and tax
- **WHEN** a received dividend has received_amount 8.93 and withholding_tax 3.83
- **THEN** its gross_amount is 12.76; a received dividend with no withholding_tax has an empty gross_amount

### Requirement: Receiving a dividend
The system SHALL let the user mark a pending dividend received by supplying either `received_amount` (the net amount), or `gross_amount` with an optional `withholding_tax`, plus optional `received_price`. When `gross_amount` is supplied, the system SHALL set `received_amount` = `gross_amount` − tax, where tax is the request's `withholding_tax` when supplied, otherwise the stored `withholding_tax`, otherwise 0; the `withholding_tax` from the request SHALL be stored and `gross_amount` itself SHALL NOT be stored. The same gross + tax input SHALL be accepted when editing an already-received dividend, recomputing its `received_amount`. When `received_amount` transitions from NULL to a value and the update supplies no `received_price` — the field absent or explicitly null — the record SHALL store the stock's current `manual_price` as `received_price`, or NULL when the stock has no `manual_price`. An explicitly supplied `received_price` SHALL be stored as given; clearing `received_price` on an already-received record (no receipt transition) SHALL keep it NULL. Recording receipt SHALL NOT modify the stored `shares_held`/`buy_cost` snapshots. Setting `received_amount` where it was previously NULL SHALL — atomically with the update — bank the net `received_amount` (HK dividends credit the `HS` cash manual-asset row; US dividends credit the `ibkr.usd_cash` meta value, already USD — unless the request sets `bank_in` false; the credited amount is stored for exact reversal) and, for HK dividends, record the `div:<id>` `adjustment` item for the pay_date month (skipped when already stored or the month has no row; US dividends never touch 活期 so they record none). Clearing `received_amount` SHALL reverse the stored credit, delete the item, and clear `withholding_tax`.

#### Scenario: Money arrives
- **WHEN** the user records received_amount 16432.10 and received_price 5.41 on a pending dividend
- **THEN** the record's status becomes RECEIVED, both yields are computed, and the earlier estimate remains stored for comparison

#### Scenario: US receipt from gross and tax
- **WHEN** a pending US dividend is 收訖 with gross_amount `12.76` and withholding_tax `3.83`
- **THEN** the record stores received_amount `8.93` and withholding_tax `3.83`, reads back gross_amount `12.76`, and `ibkr.usd_cash` gains `8.93`

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
- **WHEN** a pending HK dividend paying `2026-09-25` is 收訖 with `received_amount` `909.83`
- **THEN** the `HS` cash row gains `909.83`, the `2026-09` month gains a `div:<id>` adjustment item of `909.83`, and September's derived 利息 includes `909.83` on the next read

#### Scenario: US receipt banks into IBKR USD cash
- **WHEN** a pending US dividend is 收訖 with `received_amount` `200`
- **THEN** `ibkr.usd_cash` gains `200` (USD) and no month item is created

#### Scenario: Clearing the receipt reverses
- **WHEN** the user clears `received_amount` on a dividend that had credited `HS` `909.83`
- **THEN** `HS` loses `909.83` and the `div:<id>` item is deleted

#### Scenario: Clearing the receipt clears the tax
- **WHEN** the user clears `received_amount` on a US dividend that stored withholding_tax `3.83` and credited `ibkr.usd_cash` `8.93`
- **THEN** `ibkr.usd_cash` loses `8.93` and withholding_tax becomes empty
