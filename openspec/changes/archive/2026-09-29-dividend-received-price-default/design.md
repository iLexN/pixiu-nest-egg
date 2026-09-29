# Design

## Context

`PATCH /api/dividends/{id}` (`backend/src/routes/dividends.rs::update`) merges the patch over the stored row, then handles the `received_amount` NULL→Some transition inside a transaction: it banks the amount (HS cash row for HK, `ibkr.usd_cash` meta for US) and records the `div:<id>` month item. `DividendPatch.received_price` is `Option<Option<f64>>` — absent keeps the stored value, `null` clears it, a number sets it. The 收訖派息 form sends `received_price: null` when the field is blank, so the merged value after a blank submit is NULL.

## Goals / Non-Goals

**Goals:**
- A receipt submitted with no `received_price` stores `stocks.manual_price` as the price snapshot.
- Explicit values and explicit clears on non-transition updates keep today's semantics.

**Non-Goals:**
- No frontend changes — the form's blank → `null` payload already reaches the new rule.
- No new API fields, no payload change, no schema migration.
- No auto-fill on create or on non-transition edits; the import path (which recovers `received_price` from the sheet's N-rate) is untouched.

## Decisions

**Resolve the default in the backend, inside the existing receipt transaction.** After the merged `received_price` is computed, when the `(None, Some(amount))` transition applies and the merged price is NULL, `SELECT manual_price FROM stocks WHERE id = ?` and use that value for the UPDATE. Alternatives considered:

- *Frontend prefill/fetch-on-submit*: leaks a stored-value rule into the UI, and other callers (the edit form's receive path, future clients) would each need to reimplement it. The backend is the source of truth for what is stored.
- *Pre-fill via the dividend list payload* (`s.manual_price` on `DIVIDEND_SELECT`): useful only for display; the user confirmed silent auto-fill is enough.

**Blank and explicit null both auto-fill, but only on the transition.** The form sends `null` for blank, so distinguishing `Some(None)` from `None` would require a frontend change for no practical gain: the only thing lost is "receive a dividend with genuinely no price while the stock has one", which remains reachable by clearing `received_price` afterwards on the already-received record (a `Some→Some` merge — no transition — so no re-fill).

**No staleness check on `manual_price`.** The user wants the current 現價 unconditionally; a weeks-old price still yields a correct historical `yield_on_price` relative to what they last recorded.

## Risks / Trade-offs

- [A receipt now always carries a price when the stock has one, so `yield_on_price` appears where it previously stayed empty] → Intended; the dividend table's second rate column simply fills in.
- [The snapshot reflects the price at 收訖 time, not the pay_date price] → Same as a hand-typed value today; users who want a different-day price can still type it.
