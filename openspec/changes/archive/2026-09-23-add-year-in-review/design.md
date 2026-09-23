# Design

## Context

`YearInReview` holds one block per year (2024, 2025, 2026), each with three column groups:

- **A–D ledger aggregates** — 總數+/支出/生活平均支出/開心Pool trio, which for the live year are pure links into `Month Stat`'s yearly block (`B4`/`D4`/`F4`/`G4`), already computed by `month_year_summaries`/`pool_balances`.
- **E–G investment summary** — 利息回報 (`Month Stat` `H4`), 投資P/L (`港股` year block's hand-entered `sold P/L` cell), 投資純利, invested (`港股!C32 − 110000 + 美股!B1`), invested%, Irene+開心Pool (`N4`).
- **H–M asset returns** — 債券/股票/定期 principal+interest+rates, blended 回報率 row, 收入 (hand-entered), 存%.

What the app already provides: the full Month Stat yearly block (`month_year_summaries`), the pool chain (`pool_balances`), per-market-year cost/value/invested/dividends (`yearly.rs` + `year_snapshots`), deposit year rollups, and coupon/dividend receipt records. What it lacks: sold P/L (no SELL trades exist anywhere — the sheet always hand-entered it), income (hand-entered formula), the invested adjustment mix, and pre-app bond/deposit history (the sheets delete matured entries; only 2026+ deposits and the single active 2027 bond survive in the DB).

## Goals / Non-Goals

**Goals:**
- Reproduce every YearInReview cell for the three block years, deriving wherever the app has the underlying records.
- Store only what cannot be derived, seeded by import, clearable back to live derivation.
- Reuse the existing yearly/month-summary machinery rather than duplicating aggregation.

**Non-Goals:**
- The `YYYY回報率` sheets (per-year actuals + dividend projection scratchpad) stay unmigrated; YearInReview only needs the derivable equivalents of their `B7:C9` totals.
- Recording SELL trades / computing realized P/L — the sheet never did this.
- The sheet's `D24 = C15 × 1.1` quirk (a next-year pool-income target, not YoY) is dropped; YoY columns are uniform.
- The 2025-only blended-rate variant (`回報率` row J cell) is dropped; all years use the 2026 layout's three rates.

## Decisions

**Two storage sites, split by granularity.** `sold_pl` lives on `year_snapshots` (per market-year) because the sheet keeps it in each market sheet's year block and `stock-yearly-summary` reserved the column there — YearInReview reads HK's. The remaining per-year manual cells live on a new `year_review` table (`income`, `invested_adjustment`, `bond_principal`, `bond_interest`, `deposit_principal`, `deposit_interest`). Alternatives considered: `app_meta` keys per year (rejected — a table expresses the nullable-override shape and is easier to extend); putting sold_pl on `year_review` (rejected — it is per-market and belongs with the yearly table's other snapshot columns).

**Overrides follow the NULL-derives convention.** Like `year_snapshots` and `month_stats` totals, a NULL override column means "compute live". Import seeds overrides only for years `< current_year` — the same boundary `import_year_snapshots` uses — so the live year keeps moving with the data. Seeding the current year would freeze figures the sheet itself keeps live-linked.

**`invested` = HK net invested + `invested_adjustment`.** The sheet's formula is idiosyncratic (2026: `港股!C32 − 110000 + 美股!B1` — the 110000 silver-bond purchase plus US principal in HKD). The adjustment reproduces all three sheet years exactly (2024: +44991.49, 2025: 0, 2026: +21000) without needing a US→HKD conversion the app deliberately avoids. Alternative considered — storing `invested` wholesale: rejected because it would freeze a figure the sheet derives live each year.

**Averages are `sum ÷ 12`, not the Month Stat `AVERAGE`.** The sheet's 平均總數+/平均支出/平均回報 divide by 12 flat, while Month Stat row 4 averages over present months only — for the partial current year they differ (57238 vs 76318). YearInReview reproduces the ÷12 semantics; 生活平均支出 is already a monthly average and divides by present months on the sheet too.

**Bond "held in year" = `min(first coupon year, maturity year) ≤ y ≤ maturity year`.** Bonds carry no purchase date; the coupon schedule's first pay year approximates acquisition. The 2027 silver bond (coupons 2025–2027) counts in 2025/2026/2027 — matching the sheet. Deposits count when `end_date` is in the year and ≤ today — the sheet's `End` status check.

**`sold_pl` stays manual.** The DB contains zero SELL trades (the trade registry never recorded them) and the sheet hand-entered the figure; storing it per market-year is faithful and unblocks Month Stat's 投資純利, which is already parameterized on an `hk_sold_pl` map currently passed empty.

**Editing lands in one PATCH.** `PATCH /api/year-review/:year` accepts the six `year_review` fields plus `sold_pl` (routed to the HK snapshot); the existing `PATCH /api/summary/yearly/:market/:year` also grows a `sold_pl` field so the yearly table can edit it in place.

## Risks / Trade-offs

- **Current-year bond row under-reports** (50000/1000 derived vs the sheet's 160000/6503) because the matured 2026 silver bond was deleted from the 債券 sheet before import → user backfills the matured bond into the registry (improving history) or sets the overrides by hand; parity reports the gap as informational.
- **Seeded overrides can drift** if underlying records are later corrected → acceptable: the sheet froze the same cells, overrides are clearable, and past-year records rarely change.
- **Stock figures are HK-only**, matching the sheet — US enters solely via the manual invested adjustment → documented in the view and spec rather than silently mixed.

## Migration Plan

`0021_year_review.sql` creates `year_review` and adds `year_snapshots.sold_pl`. Then `import_xlsx` seeds the manual figures/overrides from the workbook — re-running is a no-op like existing seeds. Rollback: drop the table and the column; nothing else references them.
