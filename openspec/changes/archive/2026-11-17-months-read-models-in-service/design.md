# Design

## Context

`service::months` already owns `MonthBook` and the write workflow
(`upsert`, `create_item`, `update_item`). Only the three GET read models
still compose in `routes::months`. The six-page precedent
(`page-read-models-in-service`, ADR 0005 amendment) sets the shape: the
service owns the response type and the route `pub use`-re-exports it.

## Goals / Non-Goals

**Goals:**
- `routes::months` contains zero domain composition — same thinness as the
  other six pages.
- The "month detail" concept answers in one place: `service::months::detail`.

**Non-Goals:**
- No response shape, field, or error-status changes.
- No changes to MonthBook's interface; `load_items`/`dismissed_keys`/
  `load_events` stay `pub(crate)` inside the store/service.

## Decisions

### D1: Free functions, not MonthBook methods

`service::months::{list, summary, detail}` — the detail read composes book +
items + events + dismissed keys + suggestions, wider than the book's
stat-row scope. Matches `service::{deposits,bonds,family,dividends,aia}::summary`
and `service::mpf::overview`.

### D2: `detail` takes `NaiveDate`, returns `StoreError::NotFound`

The route keeps `parse_ym` (400 on a malformed key — shape validation); an
unknown but well-formed month is `StoreError::NotFound` → 404 — the same
split as every other handler.

### D3: Year filter lives in service

`list(year: Option<i32>)` filters inside; the route extracts `ListQuery`
and passes the year through — same treatment `?market=`/`?status=` got.

## Risks / Trade-offs

- Response structs live in service but are re-exported — identical
  `ToSchema` names keep `api-types.ts` byte-identical except churn-free
  regeneration.
