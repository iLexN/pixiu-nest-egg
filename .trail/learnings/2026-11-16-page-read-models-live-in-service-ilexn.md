---
title: Page read models live in service; routes re-export and delegate
author: ILexN
date: 2026-11-16
branch: main
id: l-a43f6ed8
scope: repo
status: active
src: agent
touches:
  - GLOSSARY.md
  - backend/src/calc/mod.rs
  - backend/src/routes/aia.rs
  - backend/src/routes/bonds.rs
  - backend/src/routes/deposits.rs
  - backend/src/routes/dividends.rs
  - backend/src/routes/family.rs
  - backend/src/routes/mod.rs
  - backend/src/routes/mpf.rs
  - backend/src/service/aia.rs
---

## Decided
- Every `GET …/summary`-style page read model is one `service::<domain>` call returning `Result<_, StoreError>` — `service::{deposits,bonds,family,dividends,aia}::summary`, `service::mpf::overview`. The response struct lives in the service module and the route `pub use`-re-exports it (same names/fields/serde attrs), so OpenAPI schema names and the generated `api-types.ts` stay byte-identical. Putting read models in routes made each page copy the previous one's load→facts→sort→years composition.
- Stored-date parsing belongs in store `facts_of(&Model)` helpers returning `StoreError::Corrupt` ("stored <col> {x} is not valid"), which routes map to 409 — a stored date that fails to parse is data corruption, not a request error, so routes never re-parse stored dates.
- Three pages built the same "distinct years + current year, sorted" filter list and the same `?status=`/`received_at`/`interest` input parsing by copy-paste; they now share `calc::history_years` and `routes::{parse_status,parse_received_at,finite_or}` so a fix lands once.
- A domain's "already received → 409" guard belongs to the domain path (`store::family::mark_received` → StoreError::Conflict), not the route — same rule as deposits/bonds.

## Watch out
- Two deliberate edge changes (corrupt-DB-only): a malformed stored `pay_date` 409s the dividend summary rather than yielding an empty `history_years`; a family receive on an already-received deposit with a malformed `received_at` answers 400 (parse precedes the store guard) rather than 409.

In the code: `backend/src/service/{deposits,bonds,family,dividends,aia,mpf}.rs`, `backend/src/routes/mod.rs` (parse helpers), `backend/src/store/mod.rs` (stored_date), `backend/src/store/{deposits,family,dividends}.rs` (facts_of, mark_received guard), `backend/src/calc/mod.rs` (history_years); openspec/changes/archive/2026-11-16-page-read-models-in-service, docs/adr/0005 amendment.
