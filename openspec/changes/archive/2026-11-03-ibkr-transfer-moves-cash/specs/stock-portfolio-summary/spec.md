# Spec Delta

## MODIFIED Requirements

### Requirement: IBKR account card on the 美股 summary

The 美股 持倉總覽 page SHALL show an IBKR account card (the 美股 sheet's `A1:B5` block) with editable inputs for `now_value` (IBKR app display value), `hkd_cash`, and `usd_cash`, saving through `PATCH /api/ibkr`; the 總覽 card's 「轉入」 form; and the read-only `transferred_hkd` (累計轉入 HKD) beside the derived `computed_total_hkd`, `net`, `net_pct`, and `vs_now_value` figures. The card SHALL NOT appear on the 港股 summary.

#### Scenario: Edit IBKR cash

- **WHEN** the user sets `usd_cash` to `1200.25` on the 美股 持倉總覽 and saves
- **THEN** the card reloads showing the updated computed total and the 總覽 page's IBKR row reflects it

#### Scenario: HK summary unaffected

- **WHEN** the user opens the 港股 持倉總覽
- **THEN** no IBKR card is shown

#### Scenario: 轉入 from the 美股 summary

- **WHEN** the user records a `50000` 轉入 on the 美股 持倉總覽 card
- **THEN** the card refetches the IBKR block showing 累計轉入 and HKD 現金 each `50000` higher
