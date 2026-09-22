# Proposal

## Why

The month-stat 利息 figure is a bare stored number imported from the workbook's N cell — the app can't show what composes it, so a value like 2026-08's 19,301.37 is unverifiable (it is actually deposit interest + received dividends + a one-off "HS promo rebate int" the user tracked by hand). The `suggested_interest` hint computes the auto part but shows no breakdown and never explains the remainder. The user wants to see the composition, enter labeled one-off extras, and have the figure update itself when dividends/coupons are marked 收訖 or a 定期 ends.

## What Changes

- 利息 becomes **derived on read**: Σ interest of **received** deposits ending in the month + Σ `received_amount` of bond coupons paid in the month + Σ `received_amount` of HK dividends paid in the month + Σ the month's `interest` items. Nothing auto-writes; receipt actions update the figure for free.
- Deposits gain a 收訖 lifecycle (`received_at`, `POST /api/deposits/:id/receive`/`unreceive`): a deposit stays in 未到期定期 until confirmed, its interest only counts once received (unreceived ones preview muted in the breakdown), and 收訖 optionally auto-credits principal+interest to a cash manual asset plus records the month's `dep-end` adjustment item — the sheet's 定期 end step in one click.
- New month-item category `interest` — labeled manual entries (e.g. "HS promo rebate int / 2,698") created/edited/deleted through the existing item endpoints and the 新增項目 form.
- `GET /api/months/:ym` returns `interest_auto`: a per-event breakdown list (source + label + amount + received) so the user can check where each amount came from; `suggested_interest` and the 建議/使用 hint are removed.
- **BREAKING**: `PATCH /api/months/:ym` no longer accepts `interest`; the edit form's 利息 input becomes a read-only derived total with its breakdown.
- **BREAKING**: `month_stats.interest` column is dropped by migration; each imported month's leftover (`stored − auto`, where `stored ≠ 0`) migrates into an `interest` item so every displayed total stays identical to the sheet. Months whose sheet cell was blank (e.g. 2026-10) get no item, so known upcoming interest simply appears on 收訖.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `month-stat`: 利息 changes from a stored editable field to a derived figure (auto events + `interest` items); month items gain a fifth category `interest`; the detail response exposes the auto-event breakdown; the month editor shows the breakdown instead of a free-form input.
- `time-deposits`: deposits gain an explicit 收訖 receipt lifecycle — the upcoming list is receipt-based (overdue rows flagged 已到期未收 while totals/rollups stay end_date-based like the sheet), and the receive action can bank-in principal+interest to a cash manual asset.

## Impact

- `backend/migrations/0017_itemize_interest.sql` — rebuild `month_items` with `'interest'` in the CHECK, insert residual items, drop `month_stats.interest` (same pattern as `0016_itemize_entertainment.sql`); `0018_deposit_received.sql` — `deposits.received_at` + credit tracking, backfilling ended deposits.
- `backend/src/models.rs`, `calc.rs`, `routes/months.rs`, `routes/deposits.rs`, `import.rs`, `parity.rs` — new category, derived-interest plumbing through `to_stat_rows` (feeds month rows, year summaries, the pool chain, and parity), breakdown response, residual-item import, receive/unreceive endpoints.
- `frontend/src/api.ts`, `views/MonthStatView.vue`, `views/DepositsView.vue`, `views/DepositHistoryView.vue`, `components/DepositTable.vue` — read-only 利息 + breakdown list (pending deposits muted), new item category, suggestion hint removed, 收訖 form + row actions.
- `docs/DATA_FLOW.md`, `AGENTS.md` — Month Stat and 定期 section wording.
