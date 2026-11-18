# Proposal

## Why

The ADR-0005 amendment ("page read models live in `service/`") landed for eight domains, but `models/overview.rs` and `models/forecast.rs` still hold the 總覽 and 預測 read models — the half-adopted pattern ADR-0006 warns about. Tracing one field of `OverviewResponse` today bounces models → service → calc → models, because the envelope, its service-built components, and its calc-produced projections are all parked in `models/` while their builders live elsewhere. The months migration already settled where each kind belongs (envelope in `service/`, projections next to their `calc` producer, stored/write types in `models/`); these two pages just never got the move.

## What Changes

- **Envelopes + service-built components → `service/`** — `OverviewResponse`, `IbkrBlock`, `OverviewAssetRow`, `SemiLiquid`, `TwelveMonthAverages` move into `service/overview.rs`; `ForecastResponse` moves into `service/forecast.rs`. Routes switch to `pub use crate::service::<domain>::{…}` re-exports, matching `routes/summary.rs`.
- **Calc-produced projections → their producer files** — `LiquidityTiers`, `MoneyMaster`, `FormulaGoal`, `SavingYearTerms`, `GoalMilestone`, `OverviewGoals` move to `calc/overview.rs`; `ForecastPlacement`, `ForecastTrade`, `ForecastIbkrTransfer`, `ForecastCell`, `ForecastMonth` move to `calc/forecast.rs`; `InvestTargets`, `InvestTargetRow`, `InvestTargetTerms` move to `calc/year_review.rs` next to `invest_targets()` (their producer — they are consumed by the overview read model but born in the year-review calc file).
- **Stored/write types stay in `models/`** — `models/forecast.rs` keeps `ForecastItemKind`, `ForecastItem`, `NewForecastItem`, `ForecastItemPatch`; `models/overview.rs` keeps `IbkrPatch`, `IbkrTransfer`, `NewIbkrTransfer` and is renamed `models/ibkr.rs` (one line in `models/mod.rs` — all imports go through flat `crate::models::{…}` re-exports).
- **Vocabulary records the placement rule** — GLOSSARY's **Page read model** term and the ADR-0005 amendment gain the three-way split: envelope in `service/`, calc-produced projections next to their producer, stored/write types in `models/`.
- **Byte-identical API surface** — struct names are unchanged, `utoipa::ToSchema` names schemas after the type, and routes re-export the moved types — so `pnpm gen:api` must produce a byte-identical `api-types.ts` and the frontend is untouched.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no requirement changes; the OpenAPI schema and every response body are unchanged; `skip_specs: true` is set.
