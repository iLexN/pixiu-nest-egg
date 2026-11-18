# Proposal

## Why

`store::stored_date(column, raw)` encodes one rule — a stored `YYYY-MM-DD` that won't parse is data corruption (`Corrupt`, served as 409), never a request error. Adoption is partial: `deposits`, `forecast`, `family`, `dividends` (and `service/aia`) use it, but ~10 sites still hand-roll the same `parse_from_str → Corrupt` map — and the wording already drifts (`"stored maturity_date {x}"` vs `"stored date {x}"` vs `"stored pay_date {x}"`). One site (`store/market_history.rs`) pre-dates the helper entirely and maps a bad stored `recorded_on` to `sqlx::Error::Decode` — same concept, different vehicle, different error surface. A started pattern with no finish is a rule nobody can see.

## What Changes

- **9 verbatim swaps** — sites already doing `parse_from_str → Corrupt` call `stored_date` instead, passing the stored column's name: `store/bonds.rs` ×5 (`maturity_date` ×3, `pay_date` ×2), `store/trades.rs` ×2 (`trade_date`), `store/aia.rs` ×1 (`event_date`), `store/ibkr.rs` ×1 (`transfer_date`). Two sites that today say the vaguer `"stored date {x}"` get the column's real name, matching adopter messages.
- **1 vehicle swap** — `store/market_history.rs`'s stored `recorded_on` parse moves from `map_err(|err| sqlx::Error::Decode(..))` to `stored_date("recorded_on", ..)`: a corrupt row surfaces as `Corrupt` (409) per the documented contract instead of a decode error.
- **Convention recorded** — one line in ADR-0005 naming `stored_date` the single parse for stored `YYYY-MM-DD` columns, so the next store file doesn't re-hand-roll it.
- **Untouched** — deliberately lenient reads stay lenient: `store/mpf.rs`'s `.unwrap_or(created_on)` fallback and `service/overview.rs`'s `.ok() → None` are defaults, not error paths. All request-input parses in `calc/`, `routes/`, and `service/` (→ `FieldError`/validation) are a different rule entirely.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — no specified flow changes; the only behavioral nuance is the `market_history` corruption path surfacing 409 instead of a decode error, which is a fix to an unspecified edge, not a spec change. `skip_specs: true` is set.
