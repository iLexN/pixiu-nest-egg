# Design

## Decisions (from grilling)

- **D1 — Full precedent-faithful move.** Envelope + service-built components go to `service/`; calc-produced projections go to the `calc` file that builds them; stored/write types stay in `models/`. This is exactly the shape `months-read-models-in-service` left (`MonthYearSummary` etc. live in `calc/months.rs`, `MonthSummaryResponse` in `service/months.rs`, stored types in `models/months.rs`). _Rejected_: envelopes-only move — projections would still live away from their builders, leaving the half-adoption in a new shape.
- **D2 — `models/overview.rs` → `models/ibkr.rs`.** After the move it holds only the three IBKR write types (`IbkrPatch`, `IbkrTransfer`, `NewIbkrTransfer`); renaming puts it beside `service/ibkr.rs`/`store/ibkr.rs`. All imports are flat `crate::models::{…}` re-exports, so the rename touches only `models/mod.rs`. _Rejected_: keeping the name — "overview" would then mean "IBKR write types".
- **D3 — `InvestTargets` follows its producer.** `InvestTargets`, `InvestTargetRow`, `InvestTargetTerms` move to `calc/year_review.rs` next to `invest_targets()`, even though only `service::overview` consumes them. Producer-adjacency is the rule this change enforces; if `invest_targets()` is ever re-homed, its types ride along then. _Rejected_: `calc/overview.rs` (consumer-adjacency) — a second placement rule invites drift.
- **D4 — The placement rule gets written down.** GLOSSARY's **Page read model** term and the ADR-0005 amendment both say the read model lives in `service/` but are silent on projections; this change adds the three-way rule so the next read model doesn't re-litigate it.

## Type-by-type map

| Type | Producer | Home after |
|---|---|---|
| `OverviewResponse`, `IbkrBlock`, `OverviewAssetRow`, `SemiLiquid`, `TwelveMonthAverages` | `service::overview` (literal construction) | `service/overview.rs` |
| `ForecastResponse` | `service::forecast` | `service/forecast.rs` |
| `LiquidityTiers`, `MoneyMaster`, `FormulaGoal`, `SavingYearTerms`, `GoalMilestone`, `OverviewGoals` | `calc::overview` | `calc/overview.rs` |
| `ForecastPlacement`, `ForecastTrade`, `ForecastIbkrTransfer`, `ForecastCell`, `ForecastMonth` | `calc::forecast::forecast_grid` | `calc/forecast.rs` |
| `InvestTargets`, `InvestTargetRow`, `InvestTargetTerms` | `calc::year_review::invest_targets` | `calc/year_review.rs` |
| `IbkrPatch`, `IbkrTransfer`, `NewIbkrTransfer` | stored/write | `models/ibkr.rs` (renamed) |
| `ForecastItemKind`, `ForecastItem`, `NewForecastItem`, `ForecastItemPatch` | stored/write | `models/forecast.rs` (unchanged) |

`service::overview::update_ibkr` keeps taking `&IbkrPatch` from `models` and returning the moved `IbkrBlock` — service↔models import is already the norm.

## Mechanics

- Struct definitions move verbatim — names, fields, `#[derive]`s, doc comments. Only `use` lines and the `crate::models::` path prefixes inside calc function signatures change.
- `routes/overview.rs` gains `pub use crate::service::overview::{IbkrBlock, OverviewResponse}` (replacing those names in its `models` import); `routes/forecast.rs` gains `pub use crate::service::forecast::ForecastResponse`. This matches `routes/summary.rs`'s `pub use crate::service::summary::{…}` pattern and keeps `utoipa`'s schema registration unchanged.
- `service/forecast.rs` currently writes `crate::models::ForecastCell` in a helper — path updates to the local calc import.

## Risks / Trade-offs

- **[Schema-name drift in OpenAPI]** → struct names are unchanged and `utoipa::ToSchema` names the schema after the type, not the module; the `pnpm gen:api` + `api-types.ts` byte-diff is the drift check (same safeguard the first migration used).
- **[A projection turns out to be imported somewhere unexpected]** → `cargo build` catches every stale import; `backend/tests/api.rs` exercises JSON only and imports none of the moved types.
- **`calc/` files grow** (forecast.rs +~110 lines, overview.rs +~170) — accepted: size next to producer beats distance from it; the type is only a declaration.
