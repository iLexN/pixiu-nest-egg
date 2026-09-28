# Design

## Context

The `Overview` sheet's `A20:H36` 預測 grid projects 活期/locked cash over a fixed 7-column window. Verified against the workbook and live API: `B24`'s `445000` literal equals Σ active deposit principal (445000 today), `B25` equals `month_stats.start_cash` for the current month (23746.72 for 2026-09), rows 28/31 are SUMIF aggregations of the deposit ledger plus `港股Trade` dividends, and row 35's `=-(B30+C29)` encodes HS plans returning +3 months and SC plans +4 months (real SC tenors are all ~4 months). The sheet's `C24` references `C32` where siblings use `C35` — a harmless typo on an empty cell; the app implements the consistent rule.

Every input already exists: `month_stats` (start_cash, salary), the overview derived figures (living_budget, liquid_assets, semi_liquid.deposits), `deposits` (end_date, principal, interest, bank), HK dividends, bond coupons. Only the *plans* are new data.

## Goals / Non-Goals

**Goals:**

- Rolling 7-month projection where the only stored data is user intent: plan items, bill overrides, the bill-amount setting.
- Zero monthly ritual — the window, anchors, and chains all derive on read.
- Explicit plan→real lifecycle: one convert action instead of hand-editing cells.

**Non-Goals:**

- No import seeding of the sheet's current plan cells — the window is forward-looking and diverges immediately; the user re-enters ~10 numbers once.
- No parity pinning against the workbook for this block (same reason).
- No auto-matching of deposits to plans by amount/bank — reconciliation is explicit per decision below.

## Decisions

**Own endpoint and route module, not folded into `/api/overview`.** `GET /api/forecast` plus `POST /api/forecast/:ym/items`, `PATCH/DELETE /api/forecast-items/:id`, `POST /api/forecast-items/:id/convert`, served from `routes/forecast.rs` registered via `utoipa_axum::routes!`. The block has its own mutation surface and the overview response stays as-is. The settings key `forecast.bill_amount` rides on the existing `PATCH /api/months/settings` (it already hosts `overview.salary` and pool rates); default `2158` applies when the key is unset — no seeding needed.

**One `forecast_items` table, derived lines never stored.** `month` (TEXT `YYYY-MM-01`), `kind` (`hs_deposit`/`sc_deposit`/`interest`/`tax`/`stock`/`bill`/`other`), `amount` (signed REAL), `note`, `return_month` (nullable TEXT, only meaningful on the two deposit kinds — rejected otherwise). Multiple rows per (month, kind) replace the sheet's `=7000+6000` idiom. Plans are deliberately *not* `month_items` — those are actuals feeding `month_spend`; mixing plans in would corrupt the ledger.

**Returns derived at read, never stored.** `deposit_return(m) = −Σ hs_deposit amounts whose return month is m − Σ sc_deposit amounts whose return month is m`, where return month = `return_month ?? month + 3 (hs) / + 4 (sc)` — over items whose own month is inside the window. A return landing outside the window isn't shown, and a plan whose placement month has slid out of the window fires nothing — matching the sheet, where the shifted-out cell stops feeding row 35. That also closes the stale-plan footgun: if the user created the real deposit without clicking 轉為定期, the leftover plan stops affecting the projection once its month leaves the window rather than double-counting forever. When a plan converts, its return vanishes because it was never persisted.

**Locked chain base = Σ principal of deposits not yet ended** (`semi_liquid.deposits` semantics — same as `定期!B1`, which counts deposits until `end_date` passes, including not-yet-started ones). Alternative considered — filtering to `start_date <= m` — rejected: it would diverge from the sheet's own locked total, and the future-start quirk (locked counts it, cash-out unmodeled) exists in the sheet too.

**Convert is a backend endpoint, not frontend orchestration.** `POST /api/forecast-items/:id/convert` takes the deposit payload and runs deposit-insert + item-delete in one transaction. Alternative — frontend calls `POST /api/deposits` then `DELETE /api/forecast-items/:id` — is non-atomic and leaves a stale plan on partial failure. The endpoint also lets `bank` default from `kind` (`hs_deposit`→HS, `sc_deposit`→SC).

**Bill is a replacement override.** `bill(m)` = Σ `bill` items for `m` when any exist, else `−bill_amount` on quarter months. Editing the real 差餉 number into a `bill` item replaces the default rather than stacking on it — matches how the user corrects the cell today.

**Unknown receipt amounts skip rather than guess.** `interest` sums deposit `interest` (may be NULL → skip), HK dividends with a pay date in `m` (received or expected amount), and coupons due in `m`. US dividends stay inside IBKR, consistent with the month-stat rule. Components with no known amount are omitted from the sum rather than estimated.

**Grid mirrors the sheet on the 總覽 page.** Months as columns (7), the sheet's line labels as rows; plan rows render their items summed with an inline item editor and per-item 轉為定期. `ref_check` red when negative. The deposit add form gains optional initial values so convert can prefill.

## Risks / Trade-offs

- **Plan/deposit double-counting if the user creates a deposit without converting** → mitigated by the convert affordance sitting on the item itself; docs will note "create via 轉為定期".
- **Future-start deposits inflate `locked` without a matching cash-out** → inherited sheet quirk; documented, not modeled.
- **`start` fallback (live 活期 vs snapshot) can differ** → the live sum is only a payday-gap fallback; once `start_cash` is entered the column re-anchors.
- **Window edge**: plans placed >7 months out are stored but invisible → acceptable; the sheet's horizon was equally fixed.

## Migration Plan

Additive only: new migration `0026_forecast_items.sql`, new routes, new card. No backfill, no change to existing tables or behavior. Rollback = revert; the table is droppable with no dependents.
