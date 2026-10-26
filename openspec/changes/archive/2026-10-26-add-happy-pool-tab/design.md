# Design

## Context

`GET /api/months/summary` already returns these per-year figures: `pool_income` (Σ interest × that year's rate), `pool_input_sum`, `entertainment_sum`, and `pool_balance`, which chains year to year as previous + income + input − 娛樂. It also returns the live `pool_balance`. These are exactly the figures the HappyPool table needs. The only figures the API does not return yet are the all-years totals and the Irene payable amount.

## Goals / Non-Goals

**Goals:**
- Compute the cross-year totals in the backend, because the backend owns all financial calculations.
- Reuse the existing pool chain without changing it.

**Non-Goals:**
- No editing on the HappyPool page; inputs are still entered in 月結.
- No new storage and no migration.

## Decisions

- **Extend `/api/months/summary` instead of adding an endpoint.** The pool chain is already computed there. A new endpoint would recompute it, and it would also force a change to the OpenAPI path pin. The alternative was a dedicated `GET /api/months/pool`; it was rejected as a duplicate load.
- **Pure `calc::months::pool_totals(&[MonthYearSummary]) -> PoolTotals`.** It sums the summaries that `month_year_summaries` already produces, so the totals always agree with the table rows. `PoolTotals` lives in `calc/months.rs` next to `MonthYearSummary` and derives `Serialize` + `ToSchema`.
- **Hiding empty years is a frontend display rule.** The API keeps returning every stored year, and the 月結 view relies on that.
- **Frontend view** `HappyPoolView.vue` loads `api.monthSummary()`. Its card strip reuses the `.total-card` markup and styling from `OverviewView.vue`, and money values are formatted with `fmtMoney`.

## Risks / Trade-offs

- [iLex input is derived from interest × rate, not entered by hand] → Changing a year's pool rate reprices both that year and the totals. This is the intended behavior, consistent with the existing chain.
