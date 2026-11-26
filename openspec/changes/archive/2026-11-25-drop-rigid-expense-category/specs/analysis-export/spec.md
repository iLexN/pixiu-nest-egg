# Spec Delta

## MODIFIED Requirements

### Requirement: Rigid expenses per month

Each month's `rigid_expenses` SHALL be `{item, amount_hkd}` entries: one per stored plan item of kind Tax, Bill, or Other — `item` is the item's note (`null` when absent), `amount_hkd` its signed amount — plus a synthesized `{item: "差餉", amount_hkd: -<bill amount>}` in each Jan/Apr/Jul/Oct month whose bill came from the recurring-bill default rather than a stored Bill item.

#### Scenario: Stored expense items

- **WHEN** a month has a Tax item noted "AIA 醫療" of −40452.24 and a Bill item with no note of −2223.00
- **THEN** the month's `rigid_expenses` contains `{item:"AIA 醫療", amount_hkd:-40452.24}` and `{item:null, amount_hkd:-2223.00}`

#### Scenario: Quarterly bill is synthesized

- **WHEN** the window covers a January and the configured bill amount is 2158
- **THEN** January's `rigid_expenses` also contains `{item:"差餉", amount_hkd:-2158.00}` even though no stored item exists for it

#### Scenario: Stored bill replaces the synthesized default

- **WHEN** a Jan/Apr/Jul/Oct month has a stored Bill plan item
- **THEN** the month's `rigid_expenses` lists the stored item and no `差餉` entry — the stored item replaces the quarterly default, not stacks on it

#### Scenario: Deposit placements are not rigid expenses

- **WHEN** a month has an `sc_deposit` plan item of −170000
- **THEN** it does not appear in `rigid_expenses`
