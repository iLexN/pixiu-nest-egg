# Stock Dividends Specification

## Purpose

Tracks per-stock dividend events (派息) — an expected amount derived from a point-in-time holdings snapshot, later confirmed by the amount that actually arrives — replacing the J–O columns of the trade sheets so yields on cost and on market price are computed without hand-typed denominators.

## Requirements

### Requirement: Dividend registry
The system SHALL maintain a list of dividend records, each tied to a registered stock and carrying a required `pay_date` (`YYYY-MM-DD`, the sheet's K column), an optional `per_share` (announced 每股派息), `shares_held` and `buy_cost` snapshots, an `estimated_amount` (預期派息), an optional `received_amount` (實收派息) and `received_price`, and an optional free-text `note`. The system SHALL support creating, listing, editing, and deleting dividend records.

#### Scenario: Recording an expected dividend
- **WHEN** the user records a dividend for 中國銀行 with pay_date `2026-09-30` and per_share `0.25`
- **THEN** the record is stored and appears in the dividend list as pending

#### Scenario: Editing a dividend
- **WHEN** the user changes a dividend's pay_date or per_share
- **THEN** the stored record is updated and list/summary views reflect the change on next read

#### Scenario: Deleting a dividend
- **WHEN** the user deletes a dividend record
- **THEN** it no longer appears in any list or aggregate

### Requirement: Point-in-time snapshots
On creation the system SHALL derive `shares_held` (Σ BUY 股數 − Σ SELL 股數) and `buy_cost` (Σ BUY total, consistent with 總買入成本) from the stock's trades with `trade_date` on or before the `pay_date`, and store them on the record. Callers MAY supply explicit snapshot values to override the derivation. Stored snapshots SHALL NOT change when trades are later added, edited, or deleted; the system SHALL offer an explicit snapshot refresh that re-derives both values from trades on or before the record's (possibly edited) pay_date.

#### Scenario: Snapshot at creation
- **WHEN** a dividend is recorded for a stock whose trades up to the pay_date sum to 68000 shares held and 208746.64 buy cost
- **THEN** the record stores shares_held 68000 and buy_cost 208746.64

#### Scenario: Later buys do not rewrite history
- **WHEN** a new BUY trade for the same stock is recorded after the dividend exists
- **THEN** the dividend's stored shares_held and buy_cost are unchanged

#### Scenario: Snapshot refresh after a date edit
- **WHEN** the user edits a dividend's pay_date and requests a snapshot refresh
- **THEN** shares_held and buy_cost are re-derived from trades on or before the new pay_date

### Requirement: Dividend validation
The system SHALL validate dividend input before storing it: `pay_date` MUST be a calendar date in `YYYY-MM-DD` form; `per_share`, `estimated_amount`, `received_amount`, and `received_price` MUST be finite and non-negative when present; and a new record MUST carry at least one of `per_share` or `estimated_amount`. The referenced stock MUST exist.

#### Scenario: Malformed pay date
- **WHEN** the user submits a dividend with pay_date `2026/13/45`
- **THEN** the system rejects it with a validation error naming the date field, and stores nothing

#### Scenario: Nothing to estimate
- **WHEN** the user submits a dividend with neither per_share nor estimated_amount
- **THEN** the system rejects it with a validation error, and stores nothing

#### Scenario: Unknown stock
- **WHEN** the user submits a dividend for a stock code that is not registered in the given market
- **THEN** the system rejects it with a validation error naming the code

### Requirement: Derived dividend fields
The system SHALL compute on read, never store: `status` (`RECEIVED` once `received_amount` is present, otherwise `PENDING`); the effective `amount` (`received_amount` when present, otherwise `estimated_amount`); `yield_on_cost` = amount ÷ buy_cost when buy_cost is positive (the sheet's `rate` column); `yield_on_price` = amount ÷ (received_price × shares_held) when received_price and shares_held are present (the sheet's second rate column); and `variance` = received_amount − estimated_amount when both are present. When only `per_share` is supplied, `estimated_amount` SHALL be derived as `per_share × shares_held`; when no `per_share` is supplied, it SHALL be implied as the effective amount (received preferred) ÷ `shares_held` whenever shares_held is positive.

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

### Requirement: Receiving a dividend
The system SHALL let the user mark a pending dividend received by supplying `received_amount`, with optional `received_price`. Recording receipt SHALL NOT modify the stored `shares_held`/`buy_cost` snapshots.

#### Scenario: Money arrives
- **WHEN** the user records received_amount 16432.10 and received_price 5.41 on a pending dividend
- **THEN** the record's status becomes RECEIVED, both yields are computed, and the earlier estimate remains stored for comparison

### Requirement: Dividend list and filters
The system SHALL list dividends ordered by pay_date (ascending by default, descending on request), with optional filters by market, status (`pending` or `received`), and pay_date year.

#### Scenario: Market filter
- **WHEN** the user requests dividends for market US
- **THEN** only dividends of US-registered stocks are returned

#### Scenario: Pending filter
- **WHEN** the user requests dividends with status `pending`
- **THEN** only records with no received_amount are returned

#### Scenario: Year filter
- **WHEN** the user requests dividends for year 2026
- **THEN** only records whose pay_date falls in 2026 are returned

### Requirement: Dividend summary
The system SHALL provide a per-market dividend summary containing: the pending records; per-year totals of received amounts grouped by the year of `pay_date`; and a per-stock breakdown of received amounts within each year.

#### Scenario: Yearly totals
- **WHEN** a market has three received dividends in 2026 totalling 24000
- **THEN** the summary's 2026 bucket reports 24000 and lists each stock's contribution

#### Scenario: Pending section
- **WHEN** a market has two unreceived dividends
- **THEN** the summary's pending section lists them both

### Requirement: Stock deletion guard
The system SHALL refuse to delete a stock that has dividend records, reporting how many exist, until the dividends are removed first.

#### Scenario: Deleting a stock with dividends
- **WHEN** the user deletes a stock that still has 2 dividend records
- **THEN** the system rejects the deletion and names the count
