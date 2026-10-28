# Design

## Context

`GET /api/forecast` (`routes/forecast.rs`) assembles `calc::ForecastInput` and calls `calc::forecast_months`, which today reads deposits, interest events, and `forecast_items` — never trades. Real deposit placements already follow the pattern this change needs: they flow into `cash` in `calc`, and the route emits them as read-only `deposit_placements` lines that `OverviewView.vue` folds into the matching plan cell (`cellPlacements` / cell total). `store::trades::hk_suggestion_trades(pool, first, next)` already loads HK-only trades in a date range as `SuggestionTrade { id, date, kind, total, code }` for the 月結 suggestions.

## Goals / Non-Goals

**Goals:**
- HK trades inside the window reduce/increase 活期 in their month and show read-only in the 股票 cell.

**Non-Goals:**
- No link between a trade and a stock plan item (no convert, no auto-suppression) — the user adjusts plans by hand.
- No handling of trades dated before the month's 月初 was entered; same documented caveat as deposit placements.
- No change to `locked` (定期+SC) — stock purchases are not lockups.

## Decisions

- **Reuse `hk_suggestion_trades`** with `[first_month, first_month + 7 months)` instead of a new store query. It already applies the HK-only rule and parses dates/types; the route calls it directly (store call, no SQL in routes). Alternative — a new forecast-specific query — duplicates the HK filter.
- **Calc input**: add `trades: &'a [SuggestionTrade]` to `ForecastInput`; `ForecastMonthDerived` gains `trade_flows: f64` = Σ (`−total` for BUY, `+total` for SELL) of trades in the month, `+ 0.0` to avoid `-0.0`. `cash` adds `trade_flows`. A small `pub fn trade_amount(&SuggestionTrade) -> f64` in calc gives the signed figure so the route's lines and the calc sum can't disagree.
- **API shape**: new `ForecastTrade { trade_id: i64, code: String, trade_type: TradeType, trade_date: String, amount: f64 }` and `ForecastMonth.trade_lines: Vec<ForecastTrade>`. `amount` is pre-signed so the frontend only sums. Using the existing `TradeType` serialization (`BUY`/`SELL`) keeps it consistent with trades endpoints.
- **Frontend**: the 股票 cell total becomes Σ stock items + Σ `trade_lines.amount`; the editor renders trade lines read-only with a `已交易` marker, mirroring the `已定期` placement rows. Only the `stock` kind carries trade lines.

## Risks / Trade-offs

- [A plan and its real trade both count until the user edits the plan] → intentional (user choice); the cell editor shows both lines side by side so the gap is obvious.
- [A trade dated in the current month before 月初 was recorded double-counts] → same caveat as deposit placements; documented in `docs/overview.md`.
