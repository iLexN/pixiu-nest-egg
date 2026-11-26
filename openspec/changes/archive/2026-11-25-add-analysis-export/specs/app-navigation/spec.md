# Spec Delta

## ADDED Requirements

### Requirement: Export link in the 總覽 sub-tab row

The 總覽 group's sub-tab row SHALL show an `Export` link after the `HappyPool` sub-tab. The link SHALL NOT be a selectable sub-tab and SHALL NOT become active; clicking it SHALL download the analysis export (`GET /api/export/analysis`) rather than navigate to a page.

#### Scenario: 總覽 group is active

- **WHEN** the 總覽 group is active
- **THEN** the sub-tab row shows 總覽, 月結, 年結, HappyPool followed by an `Export` link, and the Export link is never marked active

#### Scenario: Download the export

- **WHEN** the user clicks the `Export` link
- **THEN** the browser downloads `wealth-export-<as_of_date>.json` and the current page and active sub-tab do not change

#### Scenario: Export link only under 總覽

- **WHEN** any group other than 總覽 is active
- **THEN** no `Export` link is shown in its sub-tab row
