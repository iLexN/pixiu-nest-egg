# Spec Delta

## MODIFIED Requirements

### Requirement: 總覽 預測 grid card

The 總覽 page SHALL show a 預測 card rendering the grid with months as columns and the sheet's lines as rows — `ref check`, `半流動`, month-end dates, `活期`, `定期+SC`, `start`, `salary`, `支出`, `定期 finish`, `定期 HS`, `SC高息馬拉松`, `利息`, `Tax/基金/醫療保險`, `股票`, `繳費`, `TBC - 定期 end`, `TBC` — `—` while a figure is absent. Plan cells SHALL offer inline add/edit/delete per item and a `轉為定期` action on `hs_deposit`/`sc_deposit` items that opens the deposit form prefilled. Each of the month's `deposit_placements` SHALL appear as a read-only line (label, `−principal`, `start_date`, marked `已定期`) in the `定期 HS` row when its bank is `HS` and in the `SC高息馬拉松` row otherwise; the cell's figure SHALL be Σ its plan amounts plus Σ `−principal` of its placements, and placements SHALL offer no edit, delete, or convert action. `ref_check` SHALL render in the negative/red style while below `0`. A `活期` cell SHALL render in the negative/red style while its `cash` figure is below `20000` — a cell at exactly `20000`, or `—` while `cash` is absent, renders normally. The view SHALL reload after every mutation.

#### Scenario: Grid renders

- **WHEN** the forecast returns seven months
- **THEN** the card shows the sheet's row labels down the side and seven month columns

#### Scenario: Convert from the grid

- **WHEN** the user clicks `轉為定期` on an `sc_deposit` item of `-180000`
- **THEN** the deposit form opens with `principal` `180000` and bank `SC`, and saving it calls the convert endpoint

#### Scenario: Placement shown in its bank row

- **WHEN** `2026-09` has no plans and one placement `9077` of `55000` with bank `SC`
- **THEN** the `SC高息馬拉松` cell for `2026-09` shows `-55000`, its editor lists `9077` as a read-only `已定期` line, and the `定期 HS` cell stays `—`

#### Scenario: 活期 below the floor renders red

- **WHEN** a month's `cash` is `15000`, the next column's `cash` is `20000`, and a third column's `cash` is absent
- **THEN** the first `活期` cell renders in the negative/red style while the `20000` and `—` cells render normally
