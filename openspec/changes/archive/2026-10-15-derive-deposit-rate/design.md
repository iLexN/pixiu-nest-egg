# Design

## Context

`deposits.rate` is a nullable fraction. `row_to_deposit` (store) builds every `Deposit` the API returns, and it already computes `total`, `status` and the end month/year on read. The rate is displayed only in the `DepositTable` 利率 column. No total, forecast or suggestion reads it.

Checks against the live data: two recent bank-quoted rates (140000 / 1279 over 112 days, 55000 / 435 over 97 days) match `interest ÷ principal × 365 ÷ (end − start)` exactly. Older workbook-derived rates used one day fewer.

## Goals / Non-Goals

**Goals:**
- Report a rate for blank-rate deposits that have enough inputs, always consistent with the current interest.

**Non-Goals:**
- Backfilling or rewriting stored rates.
- Modelling stepped-rate or compound interest. A blank-rate stepped deposit gets its blended effective rate.
- Family deposits, which have no rate field.

## Decisions

- **Derive on read rather than storing at save.** The 收訖 interest correction and later edits would leave a stored derived rate stale. Deriving on read matches how `total`/`status` already work. Alternative considered: deriving on save in `validate_deposit`, rejected because of staleness and because it can't tell an entered rate from a derived one.
- **Day count `end − start`, actual/365.** This matches the bank-quoted rows. The workbook's `−1` variant only appears in rates it derived itself.
- **Pure function in `calc/deposits.rs`:** `derive_rate(principal: Option<f64>, interest: Option<f64>, start_date: Option<&str>, end_date: &str) -> Option<f64>`. It returns `None` unless principal > 0, interest is present (0 allowed → 0.0), both dates parse, and days > 0. `calc/` stays free of sqlx.
- **API shape: `Deposit.rate` = stored ?? derived, plus `rate_derived: bool`.** Existing consumers keep reading `rate`. The flag drives the tooltip and the edit form. Alternative considered: a separate `effective_rate` field, rejected because every display would then have to pick between the two.
- **The edit form never resubmits a derived rate.** It loads `''` into the rate field when `rate_derived` and shows the derived percent as the placeholder, so a PATCH sends `rate: null` as before.

## Risks / Trade-offs

- [Derived and old workbook-derived rates differ slightly (days vs days−1).] → Accepted. Stored values stay as they are.
- [A stepped-rate deposit left blank shows a blended rate, not the headline rate.] → It is the correct effective rate. The user can type the headline rate to override it.
