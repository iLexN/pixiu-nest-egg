# Spec Delta

## MODIFIED Requirements

### Requirement: Forecast plan and placement rows

For each forecast month `m`:

- `plan_items` SHALL list the month's stored forecast items.
- `deposit_placements` SHALL list the real deposits whose `start_date` falls in `m` — each with `deposit_id`, `label`, `bank`, `principal`, and `start_date` — empty when none. Deposits without a `start_date` are never placements.
- `trade_lines` SHALL list the real HK-market trades whose `trade_date` falls in `m` — each with `trade_id`, `code`, `trade_type`, `trade_date`, and a signed `amount` (`−total` for `BUY`, `+total` for `SELL`) — empty when none. US-market trades are never trade lines.
- `deposit_return` SHALL be the sum described by the deposit-returns requirement, `0` when none.

#### Scenario: Placement requires a start date

- **WHEN** a deposit has no `start_date`
- **THEN** it never appears in `deposit_placements`

#### Scenario: HK buy becomes a negative trade line

- **WHEN** an HK `BUY` of 匯豐 dated `2026-10-27` has `total` `54063.17` and `2026-10` is in the window
- **THEN** the `2026-10` column's `trade_lines` holds that trade with `amount` `-54063.17`

#### Scenario: HK sell becomes a positive trade line

- **WHEN** an HK `SELL` dated in a window month has `total` `12000`
- **THEN** that month's `trade_lines` holds it with `amount` `12000`

#### Scenario: US trades are excluded

- **WHEN** a US-market trade is dated in a window month
- **THEN** it appears in no `trade_lines` and does not affect `cash`

#### Scenario: Trades outside the window fire nothing

- **WHEN** an HK trade is dated before the window's first month
- **THEN** it appears in no column's `trade_lines` and does not affect any `cash`

### Requirement: Forecast cash figure

For each forecast month `m`:

- `cash` (活期) SHALL be `start + salary + spend + deposit_finish + interest + bill + deposit_return + Σ amounts of `tax`/`stock`/`other` items in `m` + Σ amounts of `hs_deposit`/`sc_deposit` items in `m` + Σ `amount` of `trade_lines` in `m` − Σ `principal` of `deposit_placements` in `m`, absent while `start` is absent; absent components count as `0`. (`interest` and `bill` items are already inside `interest`/`bill`, so they are not summed again.)

#### Scenario: Cash chains the rows

- **WHEN** `start` is present
- **THEN** `cash` derives as the stated sum with absent components counting as `0`

#### Scenario: Plan and real trade both count

- **WHEN** `2026-10` holds a `stock` item of `-50000` and a trade line of `-54063.17`
- **THEN** both reduce that month's `cash` (a combined `-104063.17`) until the user edits or deletes the plan item — stock plans are never suppressed automatically

## ADDED Requirements

### Requirement: Trade lines in the 股票 row

Each of the month's `trade_lines` SHALL appear as a read-only line (code, trade type, signed `amount`, `trade_date`, marked `已交易`) in the `股票` row. The `股票` cell's figure SHALL be Σ its `stock` plan amounts plus Σ its trade line amounts, `—` while it has neither, and trade lines SHALL offer no edit, delete, or copy action.

#### Scenario: Trade shown beside a plan

- **WHEN** `2026-10` has a `stock` plan of `-50000` and a trade line 匯豐 `BUY` of `-54063.17`
- **THEN** the `股票` cell for `2026-10` shows `-104063.17` and its editor lists the plan as editable and 匯豐 as a read-only `已交易` line

#### Scenario: Trade alone fills an empty cell

- **WHEN** a month has no `stock` plans and one trade line of `-20000`
- **THEN** the `股票` cell shows `-20000` instead of `—`
