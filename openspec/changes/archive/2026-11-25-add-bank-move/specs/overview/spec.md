# Spec Delta

## MODIFIED Requirements

### Requirement: Overview IBKR block and inline editing

- the IBKR block (stored inputs plus the derived figures)

Manual `asset` row amounts SHALL be editable inline via the manual-assets endpoints, and bank cash row amounts via the cash-accounts endpoints. The 活期 row of the 半流動資金 block SHALL offer a 「調撥」 form — a source and a destination picked from the bank cash rows, and an amount — saving through `POST /api/cash-accounts/moves`; the per-row inline edit stays available beside it. The view SHALL reload after every mutation.

#### Scenario: Cash row edit hits cash-accounts API

- **WHEN** the user edits a bank cash row amount inline
- **THEN** the change saves through the cash-accounts endpoints and the view reloads

#### Scenario: 調撥 refreshes the cash rows

- **WHEN** the user moves `10000` from HS to 渣打 through the 「調撥」 form
- **THEN** the view reloads showing HS lower and 渣打 higher by `10000`, with 活期 and 半流動資金 unchanged

#### Scenario: 調撥 form fields

- **WHEN** the user opens the 「調撥」 form while exactly two bank cash rows exist
- **THEN** it offers a source and a destination listing only bank cash rows, preselects one row as the source and the other as the destination, and an amount input

#### Scenario: Rejected 調撥 shows the error

- **WHEN** the user submits a 調撥 larger than the source balance
- **THEN** the view shows the backend's error and both rows keep their amounts
