# Spec Delta

## MODIFIED Requirements

### Requirement: Per-asset-class returns

Each year row SHALL carry a per-asset-class block with `bonds`, `stocks` (港股), `us_stocks` (美股), and `deposits` entries plus blended rates, income, and savings figures:

- `bonds`: `principal` = Σ principal of bonds held in the year — a bond counts when `min(first coupon pay year, maturity year) ≤ year ≤ maturity year` — and `interest` = Σ `received_amount` of coupons with `pay_date` in the year; a seeded override replaces either value when present. `rate` = `interest ÷ principal`.
- `stocks` (labeled 港股 — HK only, matching the sheet; US figures enter through `transferred`/`invested_adjustment`): `cost` = the year's 年末總成本 (snapshot or live cumulative Σ BUY), `dividends` = Σ `received_amount` of HK dividends with `pay_date` in the year, `rate` = `dividends ÷ cost`, `now_value` = the year's 總市值 (snapshot or live), `value_rate` = `dividends ÷ now_value`.
- `us_stocks` (labeled 美股 — USD figures, not HKD-converted): `cost`, `dividends`, `rate`, `now_value`, `value_rate` derived exactly as `stocks` but from US trades, dividends, and snapshots.
- `deposits`: `principal`/`interest` = Σ over 定期 rows whose `end_date` falls in the year and is not after today (the sheet's `End` filter); a seeded override replaces either value when present.
- Blended `rates`: `income_cost_rate` = `(債券 interest + 港股 dividends + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 cost + 美股 cost × usd_hkd_rate)`; `total_value_rate` = `(債券 + 港股 + 定期 interest + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 now_value + 美股 now_value × usd_hkd_rate)`; `income_value_rate` = `(債券 interest + 港股 dividends + 美股 dividends × usd_hkd_rate) ÷ (債券 principal + 港股 now_value + 美股 now_value × usd_hkd_rate)` — each absent when its denominator is missing or zero. `usd_hkd_rate` is the stored USD fx rate and applies to every year; a blend is also absent when the year carries any `us_stocks` figure while no USD fx rate is set.
- `income` (收入) = the stored manual figure, `income_avg` = `income ÷ 12`, and its YoY delta; `saved` (存) = `income − spend`, `saved_avg` = `saved ÷ 12`, `saved_pct` = `saved ÷ income` — each absent while `income` is missing or zero.

#### Scenario: Asset rows derive for the live year

- **WHEN** in 2026 the HK stocks' year-end snapshot is absent so live cost is `996600.21`, received HK dividends total `49330.19`, live 總市值 is `1323946`, received bond coupons total `1000` over the one held bond `50000`, and deposits ending by today hold principal `573266.77` and interest `6199.51`
- **THEN** the 2026 row reports stocks `cost` `996600.21`, `dividends` `49330.19`, `now_value` `1323946`; bonds `principal` `50000`, `interest` `1000`; deposits `principal` `573266.77`, `interest` `6199.51`

#### Scenario: US figures blend at the stored rate

- **WHEN** in 2026 the US yearly row reports `dividends` `200` USD and `now_value` `5000` USD, the stored USD fx rate is `7.8`, the HK figures are `dividends` `49330.19` and `now_value` `1323946`, and no bonds or deposits contribute
- **THEN** `income_value_rate` = `(49330.19 + 200 × 7.8) ÷ (1323946 + 5000 × 7.8)` ≈ `0.03734`

#### Scenario: Blends absent without an FX rate

- **WHEN** a year carries any US `cost`, `dividends`, or `now_value` figure and no USD fx rate is stored
- **THEN** `income_cost_rate`, `total_value_rate`, and `income_value_rate` are absent

#### Scenario: Seeded override wins

- **WHEN** the 2025 record stores `bond_principal` `160000` and `bond_interest` `7486`
- **THEN** the 2025 row reports those values regardless of what coupons derive

#### Scenario: Income drives 存%

- **WHEN** the 2026 record stores `income` `737020` and the row's `spend` is `239477.08`
- **THEN** the row reports `saved` `497542.92`, `saved_avg` `41461.91`, and `saved_pct` ≈ `0.6751`
