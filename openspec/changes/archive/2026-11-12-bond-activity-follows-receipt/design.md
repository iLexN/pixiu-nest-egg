# Design

## Context

See proposal.md (Why). Today `calc::bond_active(maturity_date, today)` (`maturity_date > today`) drives `store::bonds::row_to_bond`'s `BondStatus`, and `store::bonds::list` repeats the same predicate in SQL for the `status=` filter. Every active-principal consumer goes through `service::bonds::active_principal_of`, which filters on `BondStatus::Active`: the summary route, `service::wealth` (live 總數/流動資產), and `service::overview` (債券 row, 半流動 share, `current_invest`, 短期可取回). Deposits made the same move earlier; that pattern (`!received`, flag 已到期未收 in the UI) is the precedent.

## Goals / Non-Goals

**Goals:**
- One activity predicate for bonds, `received_at IS NULL`, used by the status derivation and the list filter.
- Consumers of active principal pick up the change without being edited.

**Non-Goals:**
- No change to the year-review bond rule (`min(first coupon year, maturity year) ≤ year ≤ maturity year`): that records which bonds were held in a year, not whether a bond is currently on the books.
- No new API fields, and the `BondStatus` enum is not renamed.
- No migration.

## Decisions

- **Activity = `received_at IS NULL`, date ignored.** Same as ADR 0008 for deposits: an early 收訖 must stop the bond counting, and `maturity > today || !received` gets that wrong. Rejected: keep date-based activity and only add the bond to totals while it is overdue. That would mean two rules for one concept.
- **Keep `ACTIVE`/`MATURED` enum values.** `MATURED` now means "principal 收訖'd". Renaming to `RECEIVED` would churn the API types, the frontend, and the `status=matured` query value for no behavior gain. The model doc comments are updated so the meaning is explicit.
- **`bond_active` moves to the receipt predicate in `calc/`** (e.g. `bond_active(received_at: Option<&str>) -> bool`), so the rule stays pure and unit-tested in one place. `row_to_bond` drops the `today` dependency if nothing else needs it. The SQL filter in `list` becomes `b.received_at IS NULL` / `IS NOT NULL`, and the `today` bind goes away.
- **The 已到期未收 flag is derived in the frontend from `maturity_date <= summary.date`.** It is a display marker, the same as for deposits, so no backend field is added. Rejected: a third `OVERDUE` status, which would leak a display distinction into every status consumer.
- **The 收訖 action moves into the active section.** The existing `startBondReceive` flow and form are reused unchanged. Only the place where the button renders changes.

## Risks / Trade-offs

- [Bonds that matured after migration 0019 and were never 收訖'd will reappear in active totals on deploy] → This is the intended correction. The user confirms each one with 收訖 (or deletes it). Check the live `/api/bonds/summary` after restart.
- [A client that reads `MATURED` as "past maturity date" is now wrong] → The frontend is the only client and is updated in the same change. The docs state the new meaning.
- [Upcoming coupons now include coupons of overdue bonds] → Correct: they are still unpaid. A bond's final coupon usually pays on its maturity date, so it stays visible until 收訖.

## Migration Plan

Code-only. Deploy by restarting the backend. Rollback is reverting the commit; no data was changed.
