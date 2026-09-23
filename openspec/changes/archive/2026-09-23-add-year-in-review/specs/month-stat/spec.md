# Spec Delta

## MODIFIED Requirements

### Requirement: Yearly aggregates and running averages

`GET /api/months/summary` SHALL return, per year present, the sheet's yearly block — 總數+ (Σ `total_change`), 平均總數, 支出 (Σ `month_spend`), 平均支出, 生活平均支出, 娛樂支出 (Σ `entertainment` items), 利息回報 (Σ derived `interest`), 平均回報, 投資純利 (Σ derived `interest` + the year's stored HK `sold_pl`, absent while none is stored), 開心Pool結餘 and Irene + 開心 Pool (Σ `pool_input`) — plus the all-time running averages of `total_change`, `liquid_change`, `saved` and `interest` (the sheet's row-8 cells).

#### Scenario: Yearly rollup

- **WHEN** 2026 has nine stored months with spends totalling `239477.08` and derived interest totalling `62033.15`
- **THEN** the year's aggregate reports 支出 `239477.08` and 利息回報 `62033.15`

#### Scenario: 投資純利 adds stored sold P/L

- **WHEN** 2026 derives interest `62033.15` and the HK `sold_pl` snapshot stores `0`
- **THEN** the year's aggregate reports 投資純利 `62033.15`; with no stored `sold_pl` it is absent
