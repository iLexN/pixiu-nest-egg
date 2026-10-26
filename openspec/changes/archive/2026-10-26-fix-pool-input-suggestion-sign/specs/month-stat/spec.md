# Spec Delta

## MODIFIED Requirements

### Requirement: Remaining suggested candidates

The candidates SHALL also include each HK dividend dated in the month (an `adjustment` of + `received_amount`, or + `estimated_amount` while pending; US dividends stay inside IBKR and are never suggested), each bond coupon dated in the month (an `adjustment` of + `received_amount`, or + the expected figure while pending), each AIA premium payment (an `extra_spend` of its HKD amount via the stored rate, absent with no rate), and the month's `pool_input` (an `adjustment` of + `pool_input`, suggested only while it is positive). A dividend or coupon with no known amount — a 待定 coupon or an estimate-less dividend — SHALL NOT be suggested.

#### Scenario: Received dividend suggested

- **WHEN** an HK dividend is marked received in the current month
- **THEN** a candidate `adjustment` of + `received_amount` appears

#### Scenario: Pending coupon suggested at its expected amount

- **WHEN** a bond coupon pays in the current month with a fixed rate but is not yet 收訖
- **THEN** a candidate `adjustment` of + its expected amount appears

#### Scenario: Amount-less receipt not suggested

- **WHEN** a 待定 coupon or an HK dividend without an estimate pays in the current month
- **THEN** no candidate is produced for it; it still previews in the interest breakdown with no figure

#### Scenario: Accept pending then 收訖

- **WHEN** the user accepts a pending coupon's candidate and later marks the coupon 收訖
- **THEN** the stored `coupon:<id>` item is updated to the `received_amount`, the 收訖 credits the cash account, and no second item is written

#### Scenario: Pool input suggested as positive adjustment

- **WHEN** a month row has `pool_input` `454.95`
- **THEN** a candidate `adjustment` labeled `Irene + 開心 Pool` with `auto_key` `pool-input` and amount `+454.95` appears
