# Spec Delta

## MODIFIED Requirements

### Requirement: 總覽 page

The frontend SHALL show a 總覽 view with a headline strip (總數, 流動資產, J1 as a percentage, and the live 開心Pool balance), the asset table (label, HKD amount, share — `—` while an amount is absent), the 半流動資金 block (已定期, each cash row, 活期, and the 半流動資金 total — rendered red while below the configured `overview.semi_liquid_target` share of 流動資產 and green while above — with its `A13` share and `C14` difference on a second footer row whose label reflects the configured percent), and the IBKR block (stored inputs plus the derived figures). Manual `asset`/`cash` row amounts SHALL be editable inline via the existing manual-assets endpoints; the view SHALL reload after every mutation. The IBKR block SHALL be the same editable card as the 美股 summary — editable `transferred_hkd` delta + date, `now_value`, `hkd_cash`, and `usd_cash` inputs saving through `PATCH /api/ibkr` beside the derived figures — and the view SHALL reload after a save so the asset-table IBKR row and every dependent figure refresh.

#### Scenario: Edit a manual row inline

- **WHEN** the user edits the HS cash row to `31000` on 總覽 and saves
- **THEN** the view reloads showing 活期 and 半流動資金 updated by the same amount

#### Scenario: Edit IBKR on 總覽

- **WHEN** the user sets `usd_cash` to `1200.25` on the 總覽 IBKR card and saves
- **THEN** the view reloads showing the updated IBKR block, asset-table IBKR row, and dependent totals
