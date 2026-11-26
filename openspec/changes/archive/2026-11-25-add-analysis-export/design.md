# Design

## Context

See proposal.md — Why. The snapshot is a read model, not stored data: every section already exists as a service read (`service::overview::build`, `service::forecast::build`, `service::summary::summarize_market`, `MonthBook::trailing_averages`). Layer rules apply: no SQL in `routes/`, `calc/` stays pure, the export is another `service` read model like the other page reads.

## Goals / Non-Goals

**Goals:**

- One serializable `AnalysisExport` struct composed in `service::export`; the route is a thin adapter that only adds the attachment headers.
- Field names follow the `a.json` sample's sections but use real model names/values.

**Non-Goals:**

- No new stored fields (`bucket`, display `name`), no schema migration, no auth, no CLI bin, no client-side `api.ts` wiring (a plain link downloads it).
- No UI to configure the export shape.

## Decisions

**Compose in a new `service::export` module.** `build(pool)` calls `overview::build`, `forecast::build`, and `summarize_market(pool, Market::Hk)` + `Market::Us`, then reshapes the results into `AnalysisExport` (`Serialize + ToSchema`). Alternative — a route that fans out to the three endpoints' handlers — rejected: handlers return `Json`/`ApiError` and would duplicate response plumbing; composing in service follows the "page read models live in service" rule. `summarize_market` is currently private; widen it to `pub(crate)` (or add a thin `pub async fn` wrapper in `service::summary`) — no behavior change.

**Field mapping** (verified against the models):

| Export field | Source |
|---|---|
| `meta.as_of_date` | `OverviewResponse.today` |
| `meta.currency` | `"HKD"` literal |
| `meta.usd_hkd_rate` | `OverviewResponse.rate` |
| `meta.target_liquid_assets_hkd` | `OverviewResponse.goals.third.target` |
| `liquid_assets_summary.hk_stocks_hkd` | `assets` row key `"hk"` |
| `…us_stocks_ibkr_hkd` | `ibkr.computed_total_hkd` (assets row `"ibkr"`) |
| `…bonds_hkd` | `assets` row `"bonds"` |
| `…semi_liquid_total_hkd` | `semi_liquid.total` |
| `…semi_liquid_breakdown.cash_current_hkd` | `semi_liquid.cash_sum` |
| `…semi_liquid_breakdown.time_deposits_hkd` | `semi_liquid.deposits` |
| `…deductions.happy_pool_hkd` | `averages.pool_balance` |
| `…total_liquid_assets_hkd` | `liquid_assets` |
| `cash_flow_momentum.monthly_salary_hkd` | `OverviewResponse.salary` |
| `…avg_monthly_saving_hkd` | `averages.saved` |
| `…avg_monthly_interest_hkd` | `averages.interest` |
| `…normal_monthly_living_expense_hkd` | `averages.living_spend` |
| `upcoming_*.month` | `ForecastMonth.month` |
| `…projected_cash_current_hkd` | `ForecastMonth.cash` |
| `…projected_deposits_and_sc_hkd` | `ForecastMonth.semi_liquid` |
| `…maturing_deposits_hkd` | `ForecastMonth.deposit_finish` |
| `…rigid_expenses[]` | `cells[].plan_items` kinds Tax/Bill/Other → `{category: kind, item: note (null ok), amount_hkd: amount}`; plus synthesized `{category:"Bill", item:"差餉", amount_hkd:-bill}` per `ForecastMonth.bill` (Jan/Apr/Jul/Oct) |
| `equity_holdings.*` | `summarize_market` rows where `stock.is_active`: `code`, `ticker`, `exchange`, `sector`, `shares_held`, `market_value`, `total_buy_cost`, `dividends_received`, `dividend_yield_pct = dividends_received ÷ total_buy_cost` (null when cost 0/absent). US rows also emit `market_value_usd` alongside `_hkd` (converted at `rate`; null when rate unset). |

**Rigid-expense filter = kinds Tax/Bill/Other + synthesized bill.** `sc_deposit`/`hs_deposit` are placements already represented by the month's projections; `Interest` and `Stock` are income/discretionary. The auto-derived quarterly `bill` (not a stored item) is injected as a `"差餉"` Bill line so the export doesn't drop it — matching both the 預測 grid and the `a.json` sample.

**Attachment via `Content-Disposition`.** The handler returns the JSON body with `Content-Disposition: attachment; filename="wealth-export-<today>.json"`. This makes the existing nav `links` mechanism (`target="_blank"` anchors) download instead of navigate — no template change needed beyond the `links` entry. `#[utoipa::path]` documents the JSON body type as usual.

**No rounding.** Values serialize as stored `f64` (repo rule: no rounding of stored money).

## Risks / Trade-offs

- [Composing three heavy reads per request] → Each is already a per-page cost the app pays constantly; a download click is rare. No caching needed.
- [`dividend_yield_pct` is cumulative, not annualized] → Documented in the spec; matches spreadsheet semantics (received ÷ cost). An AI reading it gets the same number the 派息 page implies.
- [`summarize_market` visibility widened] → Signature unchanged; only reachable from `service::export`.
- [Forecast `cash`/`semi_liquid` can be `null` (missing inputs)] → Serialize as `null`; spec's "momentum/history absent → null" rule covers it.
