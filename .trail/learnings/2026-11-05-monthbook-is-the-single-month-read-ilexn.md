---
title: "MonthBook is the single month-read path; live totals moved to service::wealth"
author: ILexN
date: 2026-11-05
branch: main
id: l-7094cf36
scope: repo
status: active
src: agent
touches:
  - backend/src/routes/deposits.rs
  - backend/src/routes/forecast.rs
  - backend/src/routes/months.rs
  - backend/src/service/mod.rs
  - backend/src/service/months.rs
  - backend/src/service/overview.rs
  - backend/src/service/year_review.rs
---

## Decided
- All month-derived reads go through `service::months::MonthBook` (`open` computes live only when `wants_live`; `open_with_live` takes totals the caller already holds — overview, forecast). The pipeline's protocol (`wants_live`/`needs_live`/`to_stat_rows`/`live_tail`) is private; `load_stat_rows`, `load_stat_rows_live`, `live_if_needed`, `present_month` were deleted, not layered over.
- The 總資產/流動資產 aggregate (Overview!B1/H1) lives in `service::wealth::totals_input` / `live_totals`, not `service::months` — forecast and overview consume it but it isn't a months concept. Renamed from `live_totals_input`/`live_if_needed`.
- `service::months::stat_rows_no_live` is `pub(crate)` — the pool-balance chain inside `totals_input` must read stored figures only; calling `MonthBook::open` there would recurse (open → live_totals → totals_input → open).

## Tried and ruled out
- Keeping `load_stat_rows`/`load_stat_rows_live` public alongside the book: a second interface invites copying whichever variant is nearest — exactly how `load_stat_rows_live` ended up adopted by only year_review.
- Lazy accessor methods on the book: async-in-accessors friction for no savings, since the `wants_live` gate already skips the ~10-store live computation when nothing needs it.

## Watch out
- New month-derived reads: open the book; don't re-load months/items/events or rebuild the live gate in a route or service.
- Glossary term recorded in `GLOSSARY.md` ("Month book", "Live totals") — created this session, so keep adding terms there.
