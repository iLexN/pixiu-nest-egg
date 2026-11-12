---
title: "Bonds stay active until principal 收訖, like deposits; 活期 means only the HS and 渣打 bank accounts"
author: ILexN
date: 2026-11-12
branch: main
id: l-6f49eb7d
scope: repo
status: active
src: agent
touches:
  - "backend/src/calc/bonds.rs"
  - "backend/src/store/bonds.rs"
  - "backend/src/models/bonds.rs"
  - "backend/src/routes/bonds.rs"
  - "backend/src/service/bonds.rs"
  - "frontend/src/views/BondsView.vue"
  - "GLOSSARY.md"
  - "docs/database.md"
  - "docs/investments.md"
  - "docs/adr/0010-cash-movement-semantics.md"
  - "openspec/specs/bonds/spec.md"
---

## Decided
- A bond counts as active (債券!B1, Overview 債券, live 總數/流動資產) until its principal is 收訖, whatever its maturity date — the same rule deposits use (ADR 0008). The old maturity-date rule existed only for workbook parity, and the workbook is retired.
- A matured-but-unreceived bond stays in the 債券 page's active section, flagged 已到期未收, with its 收訖 action there. The 已到期 history lists only received bonds.
- 活期 means only the bank-role cash accounts (HS, 渣打). IBKR HKD/USD broker cash is a cash account but not 活期.

## Tried and ruled out
- Renaming the API status MATURED to RECEIVED: churns API types and the status=matured query for no behavior gain. MATURED now means "principal 收訖'd".
- A third OVERDUE status: 已到期未收 is a display marker derived from maturity_date vs the summary date, the same as for deposits.

## Watch out
- The year-review bond rule (held when first coupon year ≤ year ≤ maturity year) is a historical record, not activity — leave it date-based.

In the code: openspec/changes/archive/2026-11-12-bond-activity-follows-receipt; calc::bond_active, store::bonds::row_to_bond / list.
