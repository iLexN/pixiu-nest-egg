# year-in-review Specification

## Purpose

Replaces the workbook's `YearInReview` sheet: a per-year cross-domain review block combining the month ledger's yearly aggregates, the investment summary (interest, sold P/L, invested, pool), and per-asset-class returns (債券/港股/定期) with blended rates, income, savings rate, and year-over-year deltas — derived from stored data except for the figures the sheet always entered by hand.

## Requirements

### Requirement: Year review rows

`GET /api/year-review` SHALL return one row per calendar year for every year that has at least one stored month row or a stored year-review record, ordered ascending. Each row SHALL carry the sheet's three groups: the ledger aggregate figures, the investment summary figures, and the per-asset-class figures defined by the requirements below, plus the year-over-year delta fields. Figures SHALL be derived on read from stored months, trades, dividends, deposits, coupons, snapshots, and the stored year-review record — never recomputed and stored.

#### Scenario: Rows cover data years

- **WHEN** stored month rows span 2024–2026 and the workbook seeded 2024–2026
- **THEN** the response returns rows for 2024, 2025, and 2026

### Requirement: Ledger aggregate figures

For each year row the system SHALL derive, from the same month-row data as the Month Stat yearly block: `asset_gain` (總數+) = Σ `total_change`; `asset_gain_avg` (平均總數+) = `asset_gain ÷ 12`; `spend` (支出) = Σ `month_spend`; `spend_avg` (平均支出) = `spend ÷ 12`; `living_avg` (生活平均支出) = mean `living_spend`; `pool_income` (開心 Pool 收入) = `interest × pool rate for the year`; `pool_spend` (開心 Pool 支出) = Σ `entertainment` items; `pool_balance` (開心 Pool 結餘) = the chained pool balance through that year; and `pool_input` (Irene + 開心 Pool) = Σ `pool_input`. The ÷12 averages SHALL divide by 12 even for a partial current year, matching the sheet's `=C/12` cells (not the Month Stat block's AVERAGE-over-present-months). The row SHALL carry YoY deltas for `asset_gain`, `spend`, `living_avg`, and `pool_income`, each `(value − prior year's) ÷ prior year's`, absent when the prior-year value is missing or zero.

#### Scenario: Ledger figures match the sheet

- **WHEN** 2026's stored months derive Σ `total_change` `686862.81`, Σ `month_spend` `239477.08`, mean `living_spend` `14334.79`, Σ `entertainment` `8593`, Σ `interest` `62033.15`, and the 2026 pool rate is `0.337`
- **THEN** the row reports `asset_gain` `686862.81`, `asset_gain_avg` `57238.57`, `spend` `239477.08`, `spend_avg` `19956.42`, `living_avg` `14334.79`, `pool_spend` `8593`, and `pool_income` `20905.17`

#### Scenario: Pool balance chains

- **WHEN** the 2025 row's pool balance is `5717.57` and 2026 adds pool income `20905.17`, pool spend `8593`, and pool input `4142.66`
- **THEN** the 2026 row reports `pool_balance` `22172.4`

### Requirement: Investment summary figures

For each year row the system SHALL derive: `interest` (利息回報) = Σ the year's derived month `interest`; `interest_avg` (平均回報) = `interest ÷ 12`; `sold_pl` (投資P/L) = the year's stored HK `sold_pl`, reported absent when none is stored; `net_investment` (投資純利) = `interest + sold_pl`, absent without `sold_pl`; `us_sold_pl` (美股P/L) = the year's stored US `sold_pl`, reported absent when none is stored and never feeding `net_investment`; `transferred` (IBKR 轉入) = Σ the year's `ibkr_transfers`, absent while none; `invested` = the HK yearly row's `invested` (Σ BUY total − Σ SELL total of the year) + `transferred` + the year's `invested_adjustment`, absent while none of the three exists; `invested_pct` = `invested ÷ (income + interest)`, absent when `income` is missing; and `irene_pool` (Irene + 開心 Pool) = Σ `pool_input`. The row SHALL carry YoY deltas for `interest_avg` and `invested`.

#### Scenario: 投資純利 needs stored sold P/L

- **WHEN** 2026 derives interest `62033.15` and the stored HK `sold_pl` is `0`
- **THEN** the row reports `sold_pl` `0` and `net_investment` `62033.15`

#### Scenario: 美股P/L stays out of 投資純利

- **WHEN** 2026 derives interest `62033.15`, the stored HK `sold_pl` is `0`, and the stored US `sold_pl` is `500`
- **THEN** the row reports `us_sold_pl` `500` and `net_investment` `62033.15`

#### Scenario: invested adds 轉入 and the manual adjustment

- **WHEN** the 2026 HK yearly row invests `161935.39`, the year's `ibkr_transfers` sum to `131000`, and the year stores `invested_adjustment` `-110000`
- **THEN** the row reports `transferred` `131000` and `invested` `182935.39`

### Requirement: 月薪增幅 derivation

Each year row SHALL carry `raise` (月薪增幅 — the monthly salary increment feeding the Overview 投資目標 formula): the year's stored `raise` override when set, otherwise derived live as `max(0, lastSalary(Y) − lastSalary(Y-1))`, where `lastSalary(y)` is the `salary` of the latest stored month row of year `y` that carries one. The derived figure SHALL be absent while either year has no stored month salary; the Overview target formula counts an absent `raise` as `0`.

#### Scenario: Derived from stored salaries

- **WHEN** the latest 2026 month salary is `52700`, the latest 2025 month salary is `50810`, and no override is stored
- **THEN** the 2026 row reports `raise` `1890`

#### Scenario: Override wins

- **WHEN** the 2026 record stores `raise` `2500`
- **THEN** the 2026 row reports `raise` `2500` regardless of the salary history

#### Scenario: Flat salary derives zero

- **WHEN** the latest stored salary is unchanged between two consecutive years
- **THEN** the later year's derived `raise` is `0`

#### Scenario: No salary history

- **WHEN** the prior year has no stored month salary
- **THEN** the year's derived `raise` is absent and the Overview target counts it as `0`

### Requirement: Per-asset-class returns

Each year row SHALL carry a per-asset-class block with `bonds`, `stocks` (港股), `us_stocks` (美股), and `deposits` entries plus blended rates, income, and savings figures:

- `bonds`: `principal` = Σ principal of bonds held in the year — a bond counts when `min(first coupon pay year, maturity year) ≤ year ≤ maturity year` — and `interest` = Σ `received_amount` of coupons with `pay_date` in the year; a seeded override replaces either value when present. `rate` = `interest ÷ principal`.
- `stocks` (labeled 港股 — HK only, matching the sheet; US figures enter through `transferred`/`invested_adjustment`): `cost` = the year's 年末總成本 (snapshot or live cumulative Σ BUY), `dividends` = Σ `received_amount` of HK dividends with `pay_date` in the year, `rate` = `dividends ÷ cost`, `now_value` = the year's 總市值 (snapshot or live), `value_rate` = `dividends ÷ now_value`.
- `us_stocks` (labeled 美股 — USD figures, not HKD-converted): `cost`, `dividends`, `rate`, `now_value`, `value_rate` derived exactly as `stocks` but from US trades, dividends, and snapshots.
- `deposits`: `principal`/`interest` = Σ over 定期 rows whose `end_date` falls in the year and is not after today (the sheet's `End` filter); a seeded override replaces either value when present.
- Blended `rates`: `income_cost_rate` = `(債券 interest + 港股 dividends + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 cost + 美股 cost × usd_hkd_rate)`; `total_value_rate` = `(債券 + 港股 + 定期 interest + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 now_value + 美股 now_value × usd_hkd_rate)`; `income_value_rate` = `(債券 interest + 港股 dividends + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 now_value + 美股 now_value × usd_hkd_rate)` — each absent when its denominator is missing or zero. The stored `aia.usd_hkd_rate` applies to every year; a blend is also absent when the year carries any `us_stocks` figure while no rate is set.
- `income` (收入) = the stored manual figure, `income_avg` = `income ÷ 12`, and its YoY delta; `saved` (存) = `income − spend`, `saved_avg` = `saved ÷ 12`, `saved_pct` = `saved ÷ income` — each absent while `income` is missing or zero.

#### Scenario: Asset rows derive for the live year

- **WHEN** in 2026 the HK stocks' year-end snapshot is absent so live cost is `996600.21`, received HK dividends total `49330.19`, live 總市值 is `1323946`, received bond coupons total `1000` over the one held bond `50000`, and deposits ending by today hold principal `573266.77` and interest `6199.51`
- **THEN** the 2026 row reports stocks `cost` `996600.21`, `dividends` `49330.19`, `now_value` `1323946`; bonds `principal` `50000`, `interest` `1000`; deposits `principal` `573266.77`, `interest` `6199.51`

#### Scenario: US figures blend at the stored rate

- **WHEN** in 2026 the US yearly row reports `dividends` `200` USD and `now_value` `5000` USD, the stored `aia.usd_hkd_rate` is `7.8`, the HK figures are `dividends` `49330.19` and `now_value` `1323946`, and no bonds or deposits contribute
- **THEN** `income_value_rate` = `(49330.19 + 200 × 7.8) ÷ (1323946 + 5000 × 7.8)` ≈ `0.03734`

#### Scenario: Blends absent without an FX rate

- **WHEN** a year carries any US `cost`, `dividends`, or `now_value` figure and `aia.usd_hkd_rate` is unset
- **THEN** `income_cost_rate`, `total_value_rate`, and `income_value_rate` are absent

#### Scenario: Seeded override wins

- **WHEN** the 2025 record stores `bond_principal` `160000` and `bond_interest` `7486`
- **THEN** the 2025 row reports those values regardless of what coupons derive

#### Scenario: Income drives 存%

- **WHEN** the 2026 record stores `income` `737020` and the row's `spend` is `239477.08`
- **THEN** the row reports `saved` `497542.92`, `saved_avg` `41461.91`, and `saved_pct` ≈ `0.6751`

### Requirement: Stored manual figures and overrides

The system SHALL persist one record per year carrying `income` (收入, manual), `invested_adjustment` (manual add-on to the year's `invested`, on top of its 轉入), `raise` (manual override for the year's 月薪增幅; NULL derives live from month salaries), and the nullable overrides `bond_principal`, `bond_interest`, `deposit_principal`, `deposit_interest`. `PATCH /api/year-review/:year` SHALL upsert the record — absent fields unchanged, a value sets it, `null` clears an override back to live derivation — and SHALL also accept `sold_pl`, stored as the HK `year_snapshots` `sold_pl` for that year. Values SHALL be finite numbers; `income`, principals, and interests SHALL be non-negative; `raise` may be negative to deliberately lower the target.

#### Scenario: Clearing an override re-derives

- **WHEN** the 2025 record has `bond_principal` `160000` and the user patches the year with `bond_principal: null`
- **THEN** the override is cleared and the row reports the coupon-derived principal

#### Scenario: Editing income reprices derived cells

- **WHEN** the user patches 2026 with `income` `740000`
- **THEN** `income`, `income_avg`, `invested_pct`, `saved`, `saved_avg`, and `saved_pct` all reflect the new figure on the next read

#### Scenario: Setting raise reprices the target

- **WHEN** the user patches 2026 with `raise` `2500`
- **THEN** the 2026 row reports `raise` `2500` and the Overview 投資目標 `target`/`remain`/`growth` reflect it on the next read

### Requirement: Year review page

The frontend SHALL show a 總覽 → 年結 view listing one block per year reproducing the sheet's three groups — ledger aggregates, investment summary, and per-asset-class returns (the asset rows labeled 債券/港股/美股/定期, 港股 figures being HK-only and 美股 figures USD) — with their YoY columns, the derived `IBKR 轉入` figure, the effective `raise` per year, inline editing for `income`, `invested_adjustment`, `raise`, `sold_pl`, and the four overrides (clearing restores derived figures), and a reload after every mutation.

#### Scenario: Edit a manual figure

- **WHEN** the user edits the 2026 收入 cell to `740000`
- **THEN** the view patches the year and reloads with the new derived cells

#### Scenario: Clearing raise restores the derived figure

- **WHEN** the 2026 record stores a `raise` override and the user clears it
- **THEN** the view patches `raise: null` and reloads showing the salary-derived `raise`
