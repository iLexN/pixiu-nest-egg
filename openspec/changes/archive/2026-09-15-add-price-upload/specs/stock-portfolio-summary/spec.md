## ADDED Requirements

### Requirement: Bulk 現價 update from a price file
The system SHALL accept a user-supplied JSON price file of the form `{"stocks": [{"symbol": "0388.HK", "price": 395.4}, ...]}` and update 現價 and its last-updated timestamp for every entry that resolves to a stored stock, applying all matched updates in one transaction. The same update SHALL be available from the web UI (file picker) and from a CLI command that reads a local file path. The system SHALL NOT call any external market-data service; the file is user-supplied input equivalent to manual 現價 entry.

Symbols ending in `.HK` SHALL resolve to HK stocks by `ticker` (e.g. `0388.HK` → ticker `0388`). All other symbols SHALL resolve to US stocks by `code` or `ticker`, treating `-` and `.` as equivalent (e.g. `BRK-B` resolves to `BRK.B`).

The system SHALL report, after each upload: which stocks were updated, which symbols did not match any stock, which entries were invalid (missing symbol or non-positive price), and which stored stocks received no price in the file. Unmatched or invalid entries SHALL NOT prevent the matched entries from being applied.

#### Scenario: Uploading a price file
- **WHEN** the user uploads a file containing `{"stocks": [{"symbol": "0388.HK", "price": 395.4}, {"symbol": "VOO", "price": 699.3}]}`
- **THEN** the stock with ticker `0388` in market HK and stock `VOO` in market US each get their 現價 and last-updated timestamp set, and the summary uses the new prices on the next load

#### Scenario: US share-class symbol
- **WHEN** the file contains `{"symbol": "BRK-B", "price": 514.95}` and a US stock `BRK.B` exists
- **THEN** `BRK.B` is updated to 514.95

#### Scenario: Unmatched symbol
- **WHEN** the file contains a symbol such as `1310.HK` that matches no stored stock
- **THEN** that symbol is listed as unmatched in the report and every other matched entry is still applied

#### Scenario: Invalid entry
- **WHEN** the file contains an entry with a missing symbol or a non-positive price
- **THEN** that entry is listed as invalid in the report and is not applied

#### Scenario: Stock absent from the file
- **WHEN** a stored stock has no entry in the uploaded file
- **THEN** its 現價 is left unchanged and it is listed in the report as not updated

#### Scenario: Malformed file
- **WHEN** the uploaded content is not valid JSON or lacks a `stocks` array
- **THEN** the request is rejected with a validation error and no prices are changed
