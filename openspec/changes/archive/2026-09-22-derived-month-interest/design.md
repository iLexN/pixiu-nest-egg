# Design

## Context

`month_stats.interest` is a stored scalar imported from the workbook's N cell (see proposal.md — Why). The auto part is already computable: `load_events` (routes/months.rs) loads per-month deposits/HK dividends/coupons and `suggested_interest` (calc.rs) sums them — but only as a flat hint, and only into the detail response. Downstream, `MonthStatRow.interest` feeds the month row, `month_year_summaries` (利息回報/平均回報), and `pool_balances` (開心Pool income), so making `interest` derived at the `to_stat_rows` boundary propagates everywhere for free.

Data check against the live DB: 28 stored months have `interest − auto ≠ 0` — mostly positive leftovers (bank 活期 interest, promos), two negative (2026-04 −15.27; 2026-10 −1,278 where the sheet cell is blank but a deposit is already scheduled to end).

## Goals / Non-Goals

**Goals:**
- 利息 derived on read = auto events + `interest` items; receipt actions update it with no extra write.
- Per-event breakdown in the detail response and UI.
- Labeled manual extras via the existing item machinery (label + amount + note + edit/delete).
- Imported totals preserved exactly via residual items; blank sheet cells stay blank-sourced.

**Non-Goals:**
- US dividends — excluded today (`market='HK'`), stays excluded.
- Interest auto-*suggestions* (`int:` items) — auto events derive directly; no accept step.
- Changing which auto sources count (deposit interest + received coupons + received HK dividends — same set as `suggested_interest` today).

## Decisions

**Derived on read, not write-on-receipt.** The 收訖 triggers already exist as data: `received_amount` on dividends/coupons, `received_at` on deposits. Deriving `interest = auto(month) + Σ interest items` at read time means receipt actions update the figure with zero coupling — matches the app's "backend derives, frontend displays" convention. Alternative (PATCH handlers write into `month_stats.interest`) was rejected: it couples unrelated routes, breaks on edits/deletes, and can't retro-fix imported months.

**Deposit interest gates on 收訖, not `end_date`.** Deposit maturity is a real-world event the user confirms, exactly like a coupon or dividend: a deposit can reach `end_date` while its money is still settling. `deposits.received_at` (NULL = on the books) drives three things — the deposit stays in 未到期定期 until 收訖 (overdue rows show 已到期未收), its interest counts in the end_date month only once received (unreceived in-month deposits preview in `interest_auto` with `received: false`), and 收訖 is the one action that closes the loop. Totals and rollups (`定期!B1`, month/bank tables) keep the `end_date` rule — that's the sheet's own formula, so parity is untouched. Migration 0018 backfills `received_at = end_date` for ended deposits (their interest already counted); create/import apply the same rule for past-dated entries.

**收訖 collapses the 定期 end step.** `POST /api/deposits/:id/receive` in one transaction: sets `received_at` (defaults today) + optional interest correction, optionally credits principal+interest to a chosen cash `manual_assets` row (stored as `credited_asset_id`/`credited_amount` for exact undo), and inserts the `dep-end:<id>` adjustment item (auto_key dedupes a manually accepted suggestion). `POST /:id/unreceive` reverses all three. Rejected alternative: keep dep-end as a manual suggestion accept — the user asked for the one-action flow ("auto bank in ... so i don't need to manual update the amount").

**`interest` as a fifth item category, not a second scalar column.** Manual extras need a label ("HS promo rebate int") and sometimes a note — items already provide label/amount/note plus edit/delete UI and API. A bare `interest_other` column would carry no label. Rejected alternative: keep `interest` as the editable total and add items on top — two manual channels, divergent totals.

**Residual-item migration preserving totals.** Following `0016_itemize_entertainment.sql`: rebuild `month_items` with `'interest'` in the CHECK, insert one `interest` item per month where `stored ≠ 0 AND stored − auto ≠ 0` (label `其他利息`, note `匯入差額`), then `ALTER TABLE month_stats DROP COLUMN interest`. User decision: skip months whose stored value is 0 — 2026-10's blank cell must not spawn a −1,278 item that would cancel real interest when it arrives. Negative residuals (2026-04: −15.27) are kept: they preserve the sheet total and are deletable.

Residual SQL (pure-SQL migration; `month` is `YYYY-MM-01` text so `date(month,'+1 month')` bounds work):

```sql
auto = dep(end_date in month).interest + coupons(received, pay_date in month) + hk_dividends(received, pay_date in month)
INSERT interest item WHERE interest <> 0 AND interest - auto <> 0
```

**Breakdown as a new response field.** `MonthDetailResponse.suggested_interest: f64` → `interest_auto: Vec<{source, label, amount, received}>` (per deposit/coupon/dividend; `amount` is the received figure, else the expected/estimated one, NULL while 待定/no estimate). Unreceived events of all three kinds preview muted — only `received` ones count. Manual `interest` items are already in `items` — the UI merges the two lists. The 建議/使用 hint is removed: there is nothing to "apply" anymore since events compose the figure directly.

**Plumbing.** `to_stat_rows` gains the derived interest (auto events + item sums); `load_events` is generalized to load all-month events once for `list`/`summary`/parity (`load_all_events`), keeping the month-scoped version for `show` suggestions — or one unbounded loader reused everywhere; events are small. `MonthStatPatch.interest` is removed (serde ignores unknown keys). Import computes `residual = sheet N − auto` per month and inserts the item (months import last, so deposits/dividends/coupons are already loaded). Parity compares derived interest vs sheet N — identical totals post-migration, with blank-cell months like 2026-10 reporting the derived figure as an informational diff.

## Risks / Trade-offs

- [Derived interest shifts when a deposit `end_date` or dividend `received_amount`/`pay_date` is edited later] → Intended — it's a live derivation; historical totals recompute like every other derived column.
- [Negative residual items (2026-04) look odd in the breakdown] → Truthful (sheet counted less than events); user can edit/delete the line.
- [`interest_avg` divisor uses derived interest — 2026-10 contributes 0 until its deposits are 收訖, matching the sheet's blank cell] → Verified: parity's Oct interest/year-sum/avg/pool diffs all resolve to exact matches.
- [A deposit received late still lands its interest in the `end_date` month — a past month can gain value after the fact] → Intended: the money belongs to the maturity month, same convention as coupons' `pay_date`.
- [Residual SQL/imports must mirror auto-event semantics (HK-only dividends, received-only coupons/dividends; the import residual subtracts ALL in-month deposit components so a sheet cell typed ahead of 收訖 isn't double-counted)] → Same predicates; verify with check_parity after migrating.
