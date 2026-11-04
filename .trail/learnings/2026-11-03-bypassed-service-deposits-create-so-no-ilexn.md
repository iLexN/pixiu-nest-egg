---
title: "轉為定期 bypassed service::deposits::create, so no dep-start item and no 活期 debit"
author: ILexN
date: 2026-11-03
branch: main
id: l-2bfe6f02
scope: repo
status: active
src: agent
touches:
  - backend/src/calc/forecast.rs
  - backend/src/calc/suggestions.rs
  - backend/src/models/forecast.rs
  - backend/src/models/overview.rs
  - backend/src/routes/forecast.rs
  - backend/src/routes/mod.rs
  - backend/src/routes/overview.rs
  - backend/src/service/mod.rs
  - backend/src/service/months.rs
---

## Decided
- 新增定期 (POST /api/deposits) and 轉為定期 (POST /api/forecast-items/{id}/convert) must share one placement path in service::deposits: insert, optional 活期 debit (debit_account_id), and the dep-start:<id> item. OpenSpec change: deposit-placement-debits-cash.
- The debit takes an explicit debit_account_id with no server default. The UI preselects from the bank (HS→HS, SC→渣打). Delete reverses the stored debit. PATCH never re-syncs it. Auto-received (past end_date) inserts never debit.

## Tried and ruled out
- Calling service create and then deleting the plan in a second transaction: a failure in between leaves both, and the forecast double counts.

## Watch out
- A month showing a dep-start 建議 for a new deposit means it was inserted through store::deposits::insert_deposit directly, not through the service. routes/forecast.rs convert was the offender.
- The forecast anchors on the current month's start_cash (月初). Only while that is unset does a 活期 debit double count with the placement.
