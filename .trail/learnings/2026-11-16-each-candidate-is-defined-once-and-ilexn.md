---
title: Each Candidate is defined once and write paths rebuild it from stored facts; cash movement becomes its own module next
author: ILexN
date: 2026-11-16
branch: main
id: l-8be30681
scope: repo
status: active
src: agent
touches:
  - GLOSSARY.md
---

## Decided
- Candidate catalogue (openspec change candidate-catalogue) and Cash movement (follow-up change cash-movement-module) are two separate modules. Workflows call both inside their own transaction. A single "money event" module was rejected: the event × cash × item combinations vary too much (US dividend: cash, no item; HK trade: item, no cash; 家人: neither).
- Write paths build their Candidate from the same event shape the suggestion uses, loaded by id inside the transaction after the write, never from the request body. That makes suggestion and recorded item agree by construction.
- No service::candidates pass-through: calc builds the Candidate, store::months records or retires it, and service composes the two (ADR-0005's deletion test).
- Retiring a Candidate always removes the key's item in any month plus its dismissals.

## Tried and ruled out
- Keeping per-writer key/month/label code: it had already drifted three ways (dep-start label null vs 定期; coupon/div un-receive deleting from the new pay_date month; bond-end recording principal instead of credit_amount despite the bonds spec).

## Watch out
- US trades record only settled_amount, with no account id, and reverse against "the USD broker". That is safe only because broker cash_accounts rows can't be deleted. Keep that guard when building the cash movement module.
- With bank_in=false a bond stores no credited_amount, so bond-end falls back to principal on both the suggestion and write side.

In the code: openspec/changes/candidate-catalogue/{proposal,design,tasks}.md; calc/suggestions.rs build_suggestions; store/months.rs record_receipt_item (to be replaced); store/cash_accounts.rs delete (broker guard).
