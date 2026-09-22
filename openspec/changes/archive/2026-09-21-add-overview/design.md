# Design

## Context

Every figure in `Overview!A3:C18` is derivable from already-migrated modules; `live_totals`/`live_totals_input` (built for Month Stat) already aggregates them. Two gaps remain: the IBKR account block on the 美股 sheet (`A1:B5`) is manual input with no storage yet, and `live_totals` counts active deposits at principal + expected interest while the sheet's `已定期` (`定期!B1 = sum(F:F)` on `input`) is principal only — deposit interest lands in 活期 at maturity rather than being accrued.

## Goals / Non-Goals

**Goals:**
- `GET /api/overview` reproducing `A3:C18` + headline `B1`/`H1`/`J1`, all derived on read.
- IBKR manual figures stored once and shared by the 美股 summary card, the 總覽 page, and live totals.
- Live totals matching the workbook exactly: principal-only deposits, IBKR cash included.

**Non-Goals:**
- Overview F/G averages, J/K liquidity tiers, M/N rates, targets, the 預測 forecast grid, YearInReview — later changes.
- Dated IBKR transfer records feeding Month Stat 調整 suggestions — `ibkr.transferred_hkd` stays a cumulative manual figure.
- The D/E ratio columns (`B3/(B3+B4+B9)`, `B3/H1`) and the D12:E15 upcoming-expense scratch cells.

## Decisions

- **IBKR fields in `app_meta`, not `manual_assets` or a table.** The four cells are a fixed single-row block, like `aia.usd_hkd_rate`/`overview.salary`; `manual_assets` rows would wrongly add IBKR USD cash to the HKD `cash_sum`, and a dedicated table buys nothing for one row. Keys: `ibkr.transferred_hkd`, `ibkr.now_value`, `ibkr.hkd_cash`, `ibkr.usd_cash`.
- **`GET /api/ibkr` + `PATCH /api/ibkr` as a small resource**, embedded in `GET /api/overview` too. The US summary card needs just the block, not the whole aggregation.
- **IBKR row uses the computed total (美股!B7 semantics), never `now_value` (B2).** B2 is a cross-check only — the sheet itself links `Overview!B9` to `B7`, not `B2`. The derived `vs_now_value` surfaces the FX-rate gap the user described.
- **`deposits_active_total` → principal only.** `live_totals` switches to `active_totals(..).principal` (field renamed `deposits_active_principal`), aligning every consumer (Month Stat live rows, the new endpoint) with `定期!B1`. Alternative — keep principal+interest for "true" asset value — was rejected: it double-counts interest that later arrives as income, and diverges from the sheet.
- **Manual rows generalize by `kind`, not by hardcoded labels.** The asset table renders every `kind='asset'` row (in `sort_order`) between MPF and IBKR, and every `kind='cash'` row inside 半流動資金 — so a user-added balance appears without code changes, matching the sheet's free-form rows.
- **Inline edit of manual amounts on 總覽 reuses `/api/manual-assets/:id`.** Add/delete stays in the Month Stat settings card.
- **Parity adds `check_overview`** comparing the cached block cells; manual/self-edited cells report informational outcomes like the existing Month Stat parity does for edited rows.

## Risks / Trade-offs

- [Month Stat live totals shift (~−2.7k deposits, +~10k IBKR cash)] → Intentional correction toward workbook semantics; called out in the month-stat delta and parity expectations.
- [Sheet cached values are stale vs recomputed figures (GOOGLEFINANCE prices, rate)] → Parity uses the existing informational-outcome convention for drift-prone cells.
- [`now_value` drifting out of date makes the net/net% figures stale] → Same as the sheet: it's a user-maintained checkpoint, displayed beside `computed_total_hkd` so the gap is visible.

## Migration Plan

Import seeds `ibkr.*` once (INSERT-OR-IGNORE semantics); existing databases get the keys on their next `import_xlsx` run. No schema migration needed — `app_meta` already exists.
