# Design

## Context

`calc/deposits.rs::deposit_active(end_date, today)` (`end_date > today`) filters `active_totals`, `active_month_rollup`, and `bank_rollup`. `DepositFacts` carries only `bank`, `principal`, `interest`, and `end_date`. The callers that build facts are:

- `routes/deposits.rs::summary` — from `load_all`, which already has `received_at`
- `service/months.rs::live_totals_input` — via `store::deposits::active_facts`, whose SELECT omits `received_at`
- `routes/family.rs::summary` — from family rows, which have `received_at`
- `parity.rs::check_deposits` — from `load_all`

`store::deposits::year_facts` and the `calc/year_review.rs` tests also construct `DepositFacts`, but they don't filter by activity.

Data check (2026-10-13): the only past-dated unreceived rows are own #18/#19 and family #2, all maturing today. Own create/import auto-receive past end dates.

## Goals / Non-Goals

**Goals:**
- One activity rule, `!received`, for every aggregate except parity.
- No API shape change and no migration.

**Non-Goals:**
- Forecast/Month Stat `locked` anchor, `定期 finish`, `deposit_placements`, and the 利息 auto components. These stay date-based.
- Auto-receiving family deposits, or changing receive/unreceive.
- Showing 已到期未收 principal as a separate 半流動資金 line.

## Decisions

- **`DepositFacts` gains `received: bool`; `deposit_active(&DepositFacts) -> bool` returns `!received`.** `active_totals`, `active_month_rollup`, and `bank_rollup` drop their `today` parameter, since activity no longer depends on the date. The caller decides what "received" means.
  - Alternative considered: keep `today` and compute `end_date > today || !received`. Rejected: it double-checks the same thing and keeps an unused date dependency, and an early 收訖 must stop counting, which the OR form would get wrong.
- **Parity builds facts with `received = end_date <= today`.** That reproduces the sheet's rule exactly, so parity keeps measuring import correctness. The live rule is covered by unit tests.
  - Alternative considered: use stored `received_at` in parity and treat diffs as informational. Rejected: it adds noise on every maturity day.
- **`store::deposits::active_facts` and `year_facts` select `received_at`** and map it to `received_at.is_some()`. `routes/deposits.rs::summary` and `routes/family.rs::summary` map `deposit.received_at.is_some()`.
- **Existing workbook calc tests keep their expected numbers.** The `workbook_deposits()` fixture marks rows with `end_date <= 2026-09-15` as received, so 445,000 / 365,000 / 80,000 and the month buckets still hold.

## Risks / Trade-offs

- [The user edits 活期 by hand without 收訖, so the deposit is counted twice] → Documented: the workflow is 收訖 → 存入活期. The deposit stays visible as 已到期未收 in the list as a prompt.
- [A family deposit created with a past end date counts until it is 收訖'd manually] → Accepted. It is visible in the family 未到期定期 table with the 已到期未收 flag.
- [Live 已定期 (receipt-based) and the forecast `locked` anchor (date-based) can differ during the limbo window] → Accepted. They answer different questions, and the overview block keeps using the live active total as the spec already states.

## Migration Plan

No schema change. Deploy by restarting the backend. Rollback is a code revert.
