# Design

## Context

See proposal.md (Why) for the drift.

Candidate construction today lives inline in `calc::build_suggestions`, which covers nine sources. Recording lives in each `service::*` write path, which calls `store::months::record_receipt_item` with a hand-built `format!("<prefix>:{id}")` key and a `format!("{}-01", &date[..7])` month. Removal goes through either `delete_auto_item(month, key)` or `delete_auto_items_by_key(key)`.

The store already maps each source row to an event shape for suggestions:

| Source | Mapping |
|---|---|
| deposits | `deposits::suggestion_events` → `SuggestionDeposit` |
| dividends | `dividends::suggestion_receipts` → `SuggestionReceipt` (HK only) |
| coupons | `bonds::suggestion_receipts` → `SuggestionReceipt` |
| 轉入 | `ibkr::transfers_in_range` → `SuggestionIbkrTransfer` |
| HK trades | `trades::hk_suggestion_trades` |
| AIA payments | `aia::suggestion_payments` |

Layer rules (ADR-0005) apply: `calc/` is pure, `store/` owns SQL, and `service/` composes the two.

## Goals / Non-Goals

**Goals:**
- One definition per Candidate source, used by both the suggestion side and the write side.
- One record operation and one retire operation for keyed month items.
- Parity by construction. A write path builds its Candidate from the same event shape the suggestion uses, loaded from the same store mapping.

**Non-Goals:**
- 預測 lines (`service::forecast`) keep their own placement, trade and 轉入 lines. They aren't Candidates. Moving them over is a possible follow-up.
- Cash movement (`service::receipts`, the debit/transfer/settlement steps) is untouched. That's the follow-up change `cash-movement-module`.
- Accept and dismiss keep taking and storing the client's `auto_key` string unparsed.

## Decisions

### D1: The pure catalogue is `calc/candidates.rs`

- `CandidateKey` is an enum with one variant per source: `DepStart(id)`, `DepEnd(id)`, `BondEnd(id)`, `Coupon(id)`, `Div(id)`, `Ibkr(id)`, `Trade(id)`, `AiaPay(id)` and `PoolInput`.
  - Its `Display` impl is the only place that renders a stored key. Output is byte-identical to today's strings.
  - `source()` gives the `MonthSuggestion.source` string.
- `Candidate` holds `{ key, month: NaiveDate (first of month), category, label: String, amount: f64 }`.
- There is one builder per source, taking that source's event shape and returning `Option<Candidate>`:

  | Builder | Event shape | Returns `None` when |
  |---|---|---|
  | `dep_start` | `SuggestionDeposit` | there's no `start_date` |
  | `dep_end` | `SuggestionDeposit` | always `Some` |
  | `bond_end` | new `SuggestionBond` | always `Some` |
  | `coupon` | `SuggestionReceipt` | the amount is unknown |
  | `dividend` | `SuggestionReceipt` | the amount is unknown |
  | `ibkr` | `SuggestionIbkrTransfer` | always `Some` |
  | `trade` | `SuggestionTrade` | always `Some` |
  | `aia_pay` | `SuggestionAiaPayment` | there's no fx rate |
  | `pool_input` | the month's `pool_input` | `pool_input` is 0 |

  Each builder owns its key, month, sign and label. The deposit label falls back to `定期`.
- `build_suggestions` keeps its signature. It becomes "for each event, build; keep the ones in this month; drop stored and dismissed keys". `MonthSuggestion.label` stays `Option<String>` on the wire but is always `Some`, so `api-types.ts` doesn't change.
- `interest_components` is unchanged in shape. It keeps reading the event shapes directly, because 利息 isn't a Candidate.
- *Alternative considered:* builders taking loose parameters (`id, date, label, amount`). Rejected because each writer would pass its own mapping, which is how the drift happened in the first place.

### D2: The SQL half is `store::months::{record_candidate, retire_candidate}`

- `record_candidate(conn, &Candidate)` behaves like `record_receipt_item`, but the category comes from the Candidate instead of being hard-coded to `adjustment`:
  - refresh the label and amount of an item stored for (month, key);
  - otherwise insert, if the month row exists;
  - in both cases, clear that month's dismissal for the key.
- `retire_candidate(conn, &CandidateKey)` deletes the key's item in any month, plus all its dismissals. This is today's `delete_auto_items_by_key`.
- `record_receipt_item`, `delete_auto_item` and `delete_auto_items_by_key` are deleted, not kept alongside the new pair.

### D3: Writers load the event shape by id inside their transaction, after the write

Each store gets a by-id variant of its existing mapping that runs on `&mut SqliteConnection`:
- `deposits::suggestion_event`
- `bonds::coupon_event`
- `bonds::bond_end_event`
- `dividends::suggestion_receipt`
- `ibkr::transfer_event`

The existing range queries and the by-id variants share one row→event function per store.

After a write, the service loads the event, calls the builder, then calls `record_candidate`. The Candidate therefore reflects stored post-write facts:
- **dep-end:** amount = principal + the corrected interest.
- **coupon and dividend:** amount = `received_amount`.
- **bond-end:** amount = `credited_amount` when credited, else principal.
- **dep-start and 轉入:** amount = the event's own amount.

Removal paths call `retire_candidate`. That covers deposit delete (dep-start), 轉入 delete, coupon clear and dividend clear.

The cost is one extra indexed read per write. *Alternative considered:* building the Candidate from the request body. Rejected because it bypasses the shared mapping.

### D4: bond-end gets a new event source

- `SuggestionBond` holds `{ id, maturity_date, principal, credited_amount: Option<f64>, label }`.
- `store::bonds::bond_end_events(pool, range)` selects bonds by `maturity_date` in the range.
- `SuggestionEvents` gains `bonds`, and `service::months::load_events` fills it.
- The builder's amount is `credited_amount.unwrap_or(principal)`. Before 收訖 that's the principal. After 收訖 it's whatever was credited, which is exactly what the write recorded.
- If a bond was received with `bank_in=false`, the principal is used. That matches the write, which also records the principal in that case (bonds delta spec).

### D5: The US-dividend rule stays at the source

`dividends::suggestion_receipt` is HK-only, as the range query already is. For a US dividend, the by-id variant returns `None`, so its 收訖 records no Candidate. This replaces today's `if input.market == Market::Hk` check in `service::dividends`.

## Risks / Trade-offs

- **[Retire now also clears dismissals on coupon and dividend un-receive]** → After clearing a receipt, a pending coupon or dividend that the user had dismissed earlier can be suggested again. This is accepted, and specified in month-stat's "Retiring a candidate item": un-receiving returns the event to pending, and a pending event is a fresh Candidate.
- **[A new bond-end suggestion appears for bonds maturing this month or later that aren't yet stored]** → This is intended. Past months never suggest. In the live DB, any bond already received in a month that has its row stores the item, so it's suppressed.
- **[Key strings must stay byte-identical]** → The `CandidateKey` `Display` tests pin every prefix against the strings stored in the live DB today: `coupon`, `dep-end`, `dep-start`, `div`, `pool-input` and `trade`, plus `ibkr`, `aia-pay` and the new `bond-end`.

## Migration Plan

No schema change and no data rewrite. Rollback is a code revert. A bond-end item stored by the new code stays valid under the old code, which already records `bond-end:<id>`.
