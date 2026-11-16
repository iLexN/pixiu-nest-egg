# Proposal

## Why

Three calc files write the same micro-idioms out by hand:

- `year_review_rows` repeats `match (n, d) { (Some, Some) if d > 0.0 => Some(n/d), _ => None }` five times (bond/stock/stock-value/US rates) and `x.map(|v| v / 12.0)` four times; it also reads `records.get(&year)` twice per year.
- `forecast_grid` collapses `-0.0` with `+ 0.0` at seven sites; only the first carries the comment explaining why.
- `month_derived_with_tail` writes `total_change` and `liquid_change` as twin `map_or_else` blocks that differ only in the field they diff.

## What Changes

- `calc/year_review.rs`: add `fn rate(numerator, denominator)` for the five `d > 0.0` ratios and `fn monthly(total)` for the `/12.0` averages; reuse `stored_record` for the second `records.get`. `invested_pct`/`saved_pct` stay inline — their `!= 0.0` guards differ.
- `calc/forecast.rs`: add `fn unsigned_zero(v)` carrying the `-0.0` comment; use at all seven sites.
- `calc/months.rs`: fold the twin diff blocks into one closure over the picked field.

No behavior change: `skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; `skip_specs: true` is set.

## Impact

- `backend/src/calc/{year_review,forecast,months}.rs` only.
