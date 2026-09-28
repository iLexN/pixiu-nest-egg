# Proposal

## Why

The `Overview` sheet's `A20:H36` 預測 grid is the last actively-maintained block still living in the workbook. Every month the user re-anchors `B22`/`B24`/`B25`, block-copies `C28:H36` one column left, and rewrites `H28:H35` — pure spreadsheet bookkeeping that exists only because columns are fixed. Everything the ritual retypes is already in the database: the `445000` literal is Σ active deposit principal, `B25` is the current month's `start_cash`, and the finish/interest columns are aggregations of deposits, dividends, and coupons the app already tracks.

## What Changes

- New forecast API returning a rolling 7-month projection (current month → +6), rendered as a card on the 總覽 view mirroring the sheet's grid: per month, `start + salary − living_budget + 定期 finish + 利息 ± plans + returns` gives projected 活期; a locked 定期+SC chain and 半流動 give the `ref check` margin against 25% × 流動資產.
- New `forecast_items` table: per-month planned cash lines (`hs_deposit`, `sc_deposit`, `tax`, `stock`, `other`). Multiple items per month/kind allowed — the sheet's `=7000+6000` cell becomes two items. Deposit plans carry an optional return-month override (defaults: HS +3 months, SC +4 months) driving the auto `TBC - 定期 end` line.
- A `轉為定期` convert action on each deposit plan item: opens the deposit form prefilled (amount → principal, bank), and saving removes the plan and its derived return.
- 差餉 `繳費` as an auto line every Jan/Apr/Jul/Oct using a stored setting amount (default 2158), overridable per month once the real bill arrives.
- First-month `start` anchors on the current month's `start_cash` (月初出糧後) — no new manual input; if the current month has no `start_cash` yet it falls back to the live 活期 sum.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: add requirements for the `預測` forecast block — the `GET` response block, the derived lines (finish/interest/returns/差餉/chains), the stored plan items and their CRUD, the convert-to-deposit action, and the 總覽 view rendering. This continues the established pattern of migrating each `Overview` sheet block as requirements on this capability.

## Impact

- **Backend**: new `backend/migrations/0026_forecast_items.sql`; new `routes/forecast.rs` (or an extension of `routes/overview.rs`); `calc.rs` gains the per-month projection; `api.rs` test pins gain the new endpoints; deposits create flow gains a plan-consume hook.
- **Frontend**: new forecast card/grid on `OverviewView.vue`; deposit add form accepts prefill from a plan; settings card gains the 差餉 amount.
- **Docs**: `docs/overview.md` gains the forecast flow; `docs/database.md` gains the table; `docs/MONTH_STAT_OVERVIEW.md` notes the block migrated.
- **Not migrated / out of scope**: the sheet's `估值` history, `J10:S16` targets, and `P19:V36` long-term projection remain spreadsheet-side. The forecast window is forward-looking and diverges from the workbook immediately — no parity pinning.
