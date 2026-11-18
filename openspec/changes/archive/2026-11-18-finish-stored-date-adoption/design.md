# Design

## Decisions (from grilling)

- **D1 — Scope boundary: error paths only, not leniency.** The 9 sites that already map a failed stored-column parse to `StoreError::Corrupt` swap to `stored_date` verbatim, plus `market_history`'s `sqlx::Error::Decode` map — which is the same "stored parse failure" concept pre-dating the helper, so unifying it puts corrupt rows on the documented 409 contract instead of a decode error. _Rejected_: full strictness sweep (also converting `mpf`'s `.unwrap_or(created_on)` fallback and `overview`'s `.ok() → None`). Those are deliberate defaults — a missing/corrupt optional value renders a default by design; making them strict is a behavior change that deserves its own change with specs.
- **D2 — Each call passes the stored column's name.** `stored_date(column, raw)` already takes the column; call sites pass `maturity_date`, `pay_date`, `trade_date`, `event_date`, `transfer_date`, `recorded_on`. The sites that today emit the generic `"stored date {x}"` get more specific messages — matching what adopters already produce (`"stored end_date {x} is not valid"`).
- **D3 — No optional-column handling needed.** All 10 sites parse non-optional `String` columns; the existing `.map(|d| stored_date(col, &d))` + transpose precedent (`deposits.rs`) covers future `Option<String>` sites.
- **D4 — Convention goes in ADR-0005.** One line: `stored_date` is the single parse for stored `YYYY-MM-DD` columns; a failed parse is `Corrupt` (409). Input validation (request bodies → `FieldError`) and deliberate leniency (`.ok()`/`.unwrap_or` defaults) are explicitly different rules, so the boundary is written down, not tribal.

## Site map

| File | Lines | Column | Today |
|---|---|---|---|
| `store/bonds.rs` | ~20, ~387, ~503 | `maturity_date` | hand-rolled `Corrupt` |
| `store/bonds.rs` | ~441, ~515 | `pay_date` | hand-rolled `Corrupt` (one says `"stored date"`) |
| `store/trades.rs` | ~256, ~296 | `trade_date` | hand-rolled `Corrupt` (one says `"stored date"`) |
| `store/aia.rs` | ~359 | `event_date` | hand-rolled `Corrupt` (says `"stored date"`) |
| `store/ibkr.rs` | ~113 | `transfer_date` | hand-rolled `Corrupt` (says `"stored date"`) |
| `store/market_history.rs` | ~73 | `recorded_on` | `map_err` → `sqlx::Error::Decode` |

## Explicitly out of scope

| Site | Why |
|---|---|
| `store/mpf.rs` `.unwrap_or(created_on)` | deliberate fallback default, not an error path |
| `service/overview.rs` `.ok() → None` | deliberate leniency on an optional setting |
| `service/ibkr.rs`, `service/settings.rs`, all `calc/*`, all `routes/*` | request-input validation → `FieldError`; a different rule |
| `store/months.rs` `parse_stored_month` | already a named helper for the `YYYY-MM` month-key variant |
| `#[cfg(test)]` parses (`market_history`, `months`, `mpf` test helpers) | test scaffolding, not stored reads |

## Risks / Trade-offs

- **[Error-message wording changes]** → messages become more specific (`"stored date"` → `"stored transfer_date"`); no test asserts on the old wording (verified against the suite).
- **[market_history corrupt-row surface: decode error → Corrupt 409]** → aligned with the contract; a corrupt stored row is data corruption by definition. No test exercises a corrupt `recorded_on`.
- **[A new store file re-hand-rolls the idiom anyway]** → D4 writes the rule into ADR-0005; enforcement is review, same as the other layer rules.
