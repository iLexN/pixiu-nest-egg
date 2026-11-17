---
title: "debit_bank's two field params are deliberate"
author: ILexN
date: 2026-11-17
branch: main
id: l-4089e529
scope: repo
status: active
src: agent
touches:
  - backend/src/service/bonds.rs
  - backend/src/service/cash_movement.rs
  - backend/src/service/deposits.rs
  - backend/src/service/dividends.rs
  - backend/src/service/ibkr.rs
  - backend/src/service/mod.rs
  - backend/src/service/trades.rs
---

## Decided
`service::receipts` was renamed to `service::cash_movement` — the glossary's "Cash movement" concept (credit, placement debit, 轉入, settlement) now lives in one module. Verbs: `credit`/`reverse`/`refund` (moved), `debit_bank`, `apply_to_broker`, `shift`.

## Watch out
`debit_bank` takes TWO error-field params (`field` + `amount_field`) on purpose: callers name different request fields for the role check vs the overdraft check — deposits passes `debit_account_id`/`debit_account_id`, ibkr passes `from_account_id`/`amount_hkd`. Collapsing to one param would silently change error payloads. Same story for the "must be an HKD bank account" text — it was standardized from ibkr's "source must be…" variant intentionally.
