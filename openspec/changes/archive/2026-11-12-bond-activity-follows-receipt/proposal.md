# Proposal

## Why

A 定期 stays active until its 收訖 (已到期未收 while overdue), but a 債券 drops out of active principal — and so out of `債券!B1`, Overview 債券, and the live 總數/流動資產 — on its `maturity_date`, even though bonds have had their own principal 收訖 (`received_at`) since migration 0019. Between maturity and the principal actually landing, the money vanishes from every total. The bond rule was kept date-based for workbook parity; the workbook is retired and deposits already moved to the receipt rule (`count-unreceived-deposits-as-active`, ADR 0008), so bonds should follow.

A domain-modeling pass over the DB structure also found doc drift that this change corrects alongside: the glossary's 收訖 entry omits bond principal, `docs/database.md` still describes deposit status as end-date based and omits `deposits.start_date`, `cash_accounts` is described as "活期" although 活期 means only the bank accounts (HS, 渣打), and ADR 0010 mentions only HK dividends crediting cash.

## What Changes

- A bond's derived `status` becomes receipt-based: `ACTIVE` while `received_at` is NULL, `MATURED` once 收訖 — whatever `maturity_date` is. An early 收訖 ends activity too.
- `GET /api/bonds?status=active|matured` filters on the same rule.
- `GET /api/bonds/summary`: a matured-but-unreceived bond is in `active`, counts in `totals.active_principal`, and its unreceived coupons stay in `upcoming_coupons`.
- Every consumer of active bond principal (Overview 債券 row, 半流動 share, `current_invest`, live 總數/流動資產, 短期可取回) follows automatically, so a bond keeps counting until 收訖.
- 債券 page: the 已到期未收 bond stays in the active section, flagged 已到期未收, with its 收訖 action; the 已到期 history section lists only received bonds with their 收訖日. The 本金未收 flag goes away.
- Unchanged: create-time auto-receipt of a past-maturity bond, receipt finality (409, no reversal), the coupon lifecycle, the year-review "held in year" bond rule.
- Docs/glossary corrections listed under Why (no behavior impact).

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `bonds`: bond status, the status filter, the summary split, the principal 收訖 flag requirement, and the matured display move from maturity-date to receipt based.

## Impact

- Backend: `backend/src/calc/bonds.rs` (`bond_active`), `backend/src/store/bonds.rs` (`row_to_bond`, `list` filter), `backend/src/models/bonds.rs` (doc comments), bond route/service tests.
- Frontend: `frontend/src/views/BondsView.vue` (move the 收訖 action and the flag into the active section).
- API: same shape and enum values; `MATURED` now means "principal 收訖'd". No migration — migration 0019 already backfilled `received_at` for bonds matured before it, so only currently unreceived matured bonds change status.
- Docs: `GLOSSARY.md`, `docs/database.md`, `docs/investments.md`, `docs/adr/0010-cash-movement-semantics.md`.
