# Tasks

## 1. Move calc-produced projections next to their producers

- [x] 1.1 `calc/forecast.rs`: move `ForecastPlacement`, `ForecastTrade`, `ForecastIbkrTransfer`, `ForecastCell`, `ForecastMonth` in verbatim from `models/forecast.rs`; drop them from the file's `use crate::models::{…}`
- [x] 1.2 `calc/overview.rs`: move `LiquidityTiers`, `MoneyMaster`, `FormulaGoal`, `SavingYearTerms`, `GoalMilestone`, `OverviewGoals` in verbatim from `models/overview.rs`; update the `crate::models::` path prefixes in signatures
- [x] 1.3 `calc/year_review.rs`: move `InvestTargets`, `InvestTargetRow`, `InvestTargetTerms` in verbatim from `models/overview.rs` next to `invest_targets()`; update signature paths (`crate::models::InvestTargets` → local)

## 2. Move the envelopes + service-built components into service

- [x] 2.1 `service/overview.rs`: move `OverviewResponse`, `IbkrBlock`, `OverviewAssetRow`, `SemiLiquid`, `TwelveMonthAverages` in; update its `use crate::models::{…}` to keep only stored/write types; `routes/overview.rs` gains `pub use crate::service::overview::{IbkrBlock, OverviewResponse}` and drops them from its models import (keeps `IbkrPatch`, `IbkrTransfer`, `NewIbkrTransfer`)
- [x] 2.2 `service/forecast.rs`: move `ForecastResponse` in; `routes/forecast.rs` gains `pub use crate::service::forecast::ForecastResponse` and drops it from its models import; fix the `crate::models::ForecastCell` helper path
- [x] 2.3 `cargo build` green after the sweep — fix any import stragglers

## 3. `models/` cleanup + vocabulary

- [x] 3.1 Rename `models/overview.rs` → `models/ibkr.rs` (now holding only `IbkrPatch`, `IbkrTransfer`, `NewIbkrTransfer`); update `models/mod.rs` and the file's header comment
- [x] 3.2 GLOSSARY **Page read model** term + ADR-0005 amendment: record the three-way placement rule — envelope in `service/`, calc-produced projections next to their `calc` producer, stored/write types in `models/`

## 4. Verify

- [x] 4.1 `cargo build` + `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` — all green
- [x] 4.2 `pnpm gen:api` — `api-types.ts` regenerates byte-identical (schema names unchanged); restart the backend per AGENTS.md and spot-check 總覽 (incl. an IBKR save) and 預測 on the live app
