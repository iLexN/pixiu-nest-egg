# Month Stat & Overview — workbook analysis

Analysis of the last two unmigrated core sheets in `財富分析報告.xlsx`, captured 2026-09-21 so the workflow does not need re-deriving. Reflects both the sheet formulas and the user's confirmed usage.

> **Implemented in app (add-month-stat, 2026-09):** the `Month Stat` sheet is now migrated — month rows, items, suggestions, live totals, pool chain, salary, and the manual Overview cells (B8/B16/B17; B7 Irene was empty at import) live in the backend. See `docs/DATA_FLOW.md` → "Month Stat (月結)".
>
> **Implemented in app (add-overview, 2026-09):** the `Overview` A3:C18 block is now migrated — the asset table, its Sum and shares, A13, the 半流動資金 block (deposits at **principal only**), the B1/H1/J1 headline, and the 美股 IBKR account cells (`ibkr.*` meta, `now_value` stays manual since its implied FX differs from `aia.usd_hkd_rate`). Live 總數/流動資產 now count deposit principal only and include IBKR cash, matching the workbook. See `docs/DATA_FLOW.md` → "Load the 總覽 view".
>
> **Also implemented (add-overview-averages, 2026-09):** the `F3:G10` averages block (+`H6` 生活預算 with its 預測 threshold `流動資產 × 0.0001 × 30 + 9000` — green below it, red above) — trailing averages over the 12 months before the current one. Overview's remaining blocks (J–M strategy, targets, 預測 forecast grid) remain spreadsheet-side.

## Scope decision (confirmed with user)

- **In scope:** `Month Stat` + `Overview` only.
- **Out of scope (stay manual):** `HappyPool` (its own ledger — the pool *figures* inside Month Stat/Overview still migrate), `Mum`, `Dad` (family members' trackers), `香港年金` (annuity comparison calculator), `FIRE` (notes/rules of thumb that read Overview cells), `ref1` (scratch calc).
- `YYYY回報率` sheets are still maintained but exist mainly so the user can hand-copy per-source income into `YearInReview`; the user wants the app to auto-generate that data instead.

## Why this workflow exists

Minimal-effort bookkeeping: the user records ~two numbers per month (bank balance at payday + adjustments) and spending/saving fall out by subtraction — no transaction logging. The outputs feed three things the user confirmed they actively use:

1. **Spending awareness** — 月支出 / 生活支出 / 存 per month vs budget.
2. **Cash planning** — how much is safe to lock into a new 定期 (liquidity tiers + the 預測 forecast grid).
3. **Long-term targets** — 1st/2nd/Final Target progress and FIRE-style projections.

Plus `YearInReview`, which aggregates the yearly figures.

## Month Stat sheet

A manual monthly ledger. One row per month since 2023-12 (rows 10+; ~row 44 is the live current month).

### Columns (row 9 headers)

| Col | Header | Manual or derived |
|---|---|---|
| A | month (1st) | manual date |
| B | 總數(月初出糧後) | current month: `=Overview!$B$1` live; history: paste-valued literal |
| C | Changed | derived `=B(n+1)−B(n)` |
| D | 流動資產 | current month: `=Overview!$H$1` live; history frozen |
| E | 流動資產 Changed | derived `=D(n+1)−D(n)` |
| F | 月初(出糧後) | **manual** — total 活期 bank balance (Overview B18 = HS+渣打) right after salary lands; includes the new salary |
| G | 調整 | **manual** sum-expression of bank in/out that must not count as 支出: 定期 start/end, HK stock buy/sell, 派息, 債券 coupons, Irene→開心Pool, IBKR transfers, bonus |
| H | 月尾(出糧前) - All DR | derived `=F(n+1) − <row n's own salary>` — the literal in H always equals that row's salary era (45500 → 47850 → 50810 → 52700); e.g. March 2025's H subtracts 47850 even though April's L already uses 50810. Exception: `H10` (2023-12) is `=78110.32-8000-45500`, the real bank balance typed before the 月初 convention existed — the app stores such hand-frozen cells as `end_cash_override` |
| I | 月支出 | derived `=F+G−H` — spending falls out of the balance difference |
| J | 生活支出 | derived `=I − extra spends` (TV, AIA premium, tax, doctor…) — the "extra" amounts are inline in the formula |
| K | (change vs same month last year, some rows) | `=(J−J')/J'` |
| L | 存 | derived `=salary − I (+ extras)` |
| N | 利息 | **manual** sum-expression of interest received that month |
| O | 娛樂支出 | **manual** — a number or ad-hoc sum (`=500 + 4700 + 75`); amounts that also appear in J's `=I − …` exclusion are typed twice. In the app this is itemized (`entertainment` items; the `exclude_from_living` flag replaces the second entry) |
| P | Irene + 開心 Pool | **manual** pool inputs |

### Top block

- Rows 2–4: per-year aggregates for 2024/2025/2026 — 總數+ `SUM(C)`, 平均總數 `AVERAGE(C)`, 支出 `SUM(I)`, 平均支出, 生活平均支出 `AVERAGE(J)`, 娛樂支出 `SUM(O)`, 利息回報 `SUM(N)`, 平均回報, 投資純利 (K), 開心Pool結餘 (M), Irene+開心Pool `SUM(P)`, Note (O).
- `M` (pool balance): `=M(prev year) + 利息回報 × Overview!N8 − 娛樂支出 + Irene+開心Pool`. **The rate is per-year**: 2024 = 53%, 2025 = 42.5%, 2026 = 33.7%; changing it at year-end retroactively reprices that whole year's pool income.
- Row 8: running averages `C8=AVERAGE(C10:C1008)`, `L8`, `N8` — consumed by Overview (e.g. `L8*12` in the saving/year estimate, `N8` nowhere — Overview reads `N8` as the pool rate instead; note the name collision between Month Stat row-8 averages and `Overview!N8`).

## Overview sheet

A ~90%-derived dashboard. Blocks:

- **Asset table A3:C10**: 港股 `='港股'!D2`, 債券 `='債券'!B1`, 基金 `=AIA!B2`, MPF `=MPF!B2`, Irene (B7 **manual**), HS人壽 (B8 **manual** 69440.47), IBKR `='美股'!B7`; Sum B10. C column = share of B10; D/E = ratios vs 流動資產.
- **半流動資金 B14** = 已定期 `='定期'!B1` (B15) + 活期 `=B16+B17` (B18). B16 HS, B17 渣打 are **manual** bank balances.
- **總數 B1** = B10+B14. **流動資產 H1** = B3+B14+B4+B9 − G10 (港股 + 半流動 + 債券 + IBKR − 開心Pool). J1 = H1 ÷ (salary×100).
- **Salary E1** = `54200−1500` — **manual**, updates only when salary changes.
- **Averages F/G** (trailing 12 months via `AVERAGE(OFFSET('Month Stat'!…31,0,0,12))`): 總數增加 G4←C, 支出 G5←I, 生活支出 G6←J → 生活支出 budget **H6 = ROUNDUP(G6×1.05, −2)**, 存 G7←L, 利息 G8←N, 開心Pool G10 `='Month Stat'!M4`.
- **Liquidity tiers J/K**: can-use K4 = B14−K5; cannot-use K5 = N6 (6-month salary); short-term K6 = B7+B4+B3+B9; long-term K7 = B5+B6+B8.
- **Rates M/N**: N3 GOOGLEFINANCE USD→HKD (already in app as `aia.usd_hkd_rate`, seeded at import), N4 avg return/year `=YearInReview!L13`, N5/N6 3/6-month salary, N7 = 25% of 流動資產, **N8 = 娛樂/pool rate (manual, per year)**.
- **估值 history Q4:R8** — manual checkpoints; user says ignorable now.
- **Targets J10:M18 / N13:S16**: 1st 目標 K12 = 1.55M, 2nd K18 = 2.5M; saving/year L12 = average of several sources incl. `'Month Stat'!L8*12`; 3rd target O14 = budget×1.07×12×30yr; Final Target P14 = salary×0.7×12÷N4, R14 = 10M; years-to-target and projected dates. Inputs essentially static.
- **預測 forecast grid A20:H36**: monthly cash projection Sep 2026→Mar 2027 — start balance + salary − budget + 定期 finish (`'2026回報率'!D44:D47` / `'定期Info'!D9:D11`) + 利息 (`'2026回報率'!C44:C47` + trade-sheet dividend sums) − tax/AIA/差餉 items − SC高息馬拉松 placements. This is the cash-planning tool.
- **Invested table J22:N34**: per-year invested (K23 literal, K24/K25 `YearInReview!F5/F14`, K26 `YearInReview!F23`), target vs now, months saved.
- **Long-term projection P19:V36**: 12-year compounding (total+利息, per-month, without-play-pool, HappyPool split at N8).
- **Notes B38/B39**: 差餉 quarters (Jan/Apr/Jul/Oct), 2158.

## Dependency graph (workbook)

```
trades/deposits/bonds/MPF/AIA sheets (all migrated)
        │                         ▲
        ▼                         │ live totals B1/H1
   Overview ◄──── 12-mo averages ── Month Stat
        │                              ▲
        └──── current row B44/D44 ─────┘
YearInReview ◄── Month Stat yearly rows + YYYY回報率 sheets + 港股 cells
YYYY回報率 ◄── manual per-source interest/income tracking
```

The circularity is only apparent: Month Stat's *current row* needs Overview's live totals; Overview's *averages* need Month Stat's *history*. In the app, the backend computes 總數/流動資產 directly from migrated modules + cash balances, dissolving the link — **so Month Stat migrates first**; Overview's averages/budget/pool blocks all consume Month Stat rows and cannot exist without them.

## Confirmed user inputs & wishes

- Monthly manual entry: F (活期 total after payday), G adjustments; J extras and L/N/O/P adjustments as needed.
- Overview manual cells actually touched: B16, B17, B8, B7, E1 (on salary change), N8 (once a year, applies to the whole year).
- 估值 checkpoints and targets: inputs ignorable (static).
- **Wanted automation**: 調整 items auto-derived from app events — 定期 start/end, HK buy/sell, 派息, 債券 coupons, Irene→pool — plus manual additions (IBKR transfer, yearly bonus). Only HK trades hit the bank directly; US side moves via the manual IBKR transfer item.
- 利息 N should prefill from deposit interest + coupon receipts in the month; bank 活期 interest stays manual.
- Salary: the typed 活期 balance already includes the just-paid salary.

## Migration notes (decided direction)

- **Phase 1 — Month Stat**: `month_stats` (month PK; start_cash, salary snapshot, end_cash_override, frozen-or-live total_assets/liquid_assets, interest, pool_input, note) + `month_items` (category: adjustment | extra_spend | income | entertainment; label, amount, exclude_from_living, auto_key) + meta for salary/cash balances/HS人壽/Irene/per-year pool rate. Import freezes history verbatim; sheet G/N formula text preserved as item notes. Auto-調整 suggests items from dated events; dismissals need stable exclusion keys.
- **Phase 2 — Overview**: `GET /api/overview` aggregates allocation/liquidity/averages→budget/targets/forecast/pool; dashboard + settings UI; yearly-review endpoint auto-generating the YearInReview/回報率 figures from deposits/coupons/dividends/trades already in the DB.
- Parity: pin Month Stat yearly cells (rows 2–4) and Overview aggregates against workbook cached values; loose tolerance where the sheet is stale.
- Watch out: `Month Stat`!N8 (running avg 利息) vs `Overview!N8` (pool rate) — same address, different sheets; G-column adjustments vs N-column interest must not double-count 定期-end interest.
