# Design

## Context

`invest_targets` (`backend/src/calc.rs`) already reads every formula input off the `YearReviewRow`s it iterates, but the response row (`backend/src/models.rs::InvestTargetRow`) only exposes the derived figures. `OverviewView.vue` fetches just `GET /api/overview` + `GET /api/forecast`, so the footnote's inputs must ride on the existing block.

## Goals / Non-Goals

- Goal: every formula input of a year's `target` is derivable from the `rows` array itself — the footnote needs no extra fetch and works for any year, not just the current one.
- Non-goal: changing the formula, the table, or where inputs are edited (the 年結 page already covers `raise`/`invested 調整` overrides).

## Decisions

- **Two flat `f64` fields (`interest`, `pool_spend`) on each row** rather than a nested `target_terms` struct or a second fetch.
  - vs. nested terms struct: the six terms are three fields off two adjacent rows (`prior.invested`/`prior.interest`/`prior.pool_spend`, `row.raise`/`row.interest`/`row.pool_spend`) — a struct would duplicate data the row set already carries once these two fields exist, and would bake the footnote's layout into the API.
  - vs. fetching `GET /api/year-review` in `OverviewView`: an extra request and a second data shape to correlate by year, for values already in scope during the overview derivation.
  - Flat fields also match the row's existing style (`raise` is already a formula input exposed flat, not a sheet column) and keep the fields reusable for future UI.
- **Non-optional `f64`, `0` when the year reports none** — identical to `YearReviewInvestment.interest`/`YearReviewLedger.pool_spend` and to the formula's count-absent-as-0 rule; no new absent semantics to document.
- **Footnote composes the current-year row only**, located via `new Date().getFullYear()`; hidden while no such row exists. The six contributions map to: `prior.invested`, `prior.interest`, `prior.pool_spend×0.7`, `(row.raise ?? 0)×6`, `row.interest`, `row.pool_spend×0.7`, each `fmtMoney`-formatted, `—` while the prior row is missing.

## Risks / Trade-offs

- [Six parenthesized money values make a long muted line] → `.muted` wraps naturally; the 目標 card's footnote already wraps to two lines.
- [`今年/上年` wording is relative to the calendar year] → the note only renders for the calendar-year row, so the labels are always accurate.

## Migration Plan

Additive response fields only — no migration, no client breakage.
