# Design

## Context

See proposal.md — Why. Today every deposit-derived figure reads the whole `deposits` table: `routes/deposits.rs::summary`, `routes/months.rs::live_totals_input` (已定期 in live 總數/流動資產 and Overview 半流動資金), `routes/months.rs::load_deposit_events` (利息 auto, dep-start/dep-end suggestions), `routes/year_review.rs::deposit_facts`, and `parity.rs::check_deposits` via `deposits::load_all`. `deposits::receive` additionally credits a cash `manual_assets` row and records a `dep-end` `month_items` row. The MPF page already stores a free-text page note in `app_meta` through `mpf::meta_get`/`meta_put`.

The `Mum`/`Dad` sheets hold one rolling principal per person; each row is a rollover with a bank ref, principal, end date (Dad also has start), a stepped-rate schedule as text, interest paid, and an `end` marker. The user will re-key these by hand — no import.

## Goals / Non-Goals

**Goals:**

- Same day-to-day record-keeping as 定期 (form, list, inline edit, 收訖/取消收訖, history by year) with a holder dimension and a per-holder note.
- Structural isolation: no code path that computes the user's figures can see family rows.
- Reuse existing validation and rollup math rather than re-deriving it.

**Non-Goals:**

- Importing `Mum`/`Dad`, parity for them, or any bank-in / Month Stat / suggestion side effects.
- Numeric rates or stepped-rate modelling (note text only).
- A fixed holder enum or holder management UI beyond the free-text field.

## Decisions

1. **Separate `family_deposits` table, not a `holder` column on `deposits`.**
   *Why*: five aggregate readers plus parity all `SELECT ... FROM deposits`; a flag would need a `WHERE holder IS NULL` at every site, and one miss silently inflates 總數 by hundreds of thousands. A separate table makes the boundary a grep: no file outside `routes/family.rs` may mention `family_deposits`. *Alternative rejected*: flag column with a shared filtered loader — smaller diff but the failure mode is silent.

2. **Own route module `routes/family.rs`, modelled on `routes/deposits.rs` minus bank-in/month-item code.** Endpoints under `/api/family/`:
   - `GET /deposits?status=active|ended&holder=&year=&order=asc|desc`, `POST /deposits`, `PATCH /deposits/:id`, `DELETE /deposits/:id`
   - `POST /deposits/:id/receive` `{received_at?, interest?}` (409 if received), `POST /deposits/:id/unreceive` (409 if not)
   - `GET /deposits/summary` → `{ today, holders: [{ holder, note, upcoming[], active_principal }], history_years }`
   - `PUT /holders/:holder/note` `{ note: string|null }` → `{ holder, note }`
   The module MUST NOT import `manual_assets`/`month_items` helpers (`record_receipt_item`) — reviewers check this.

3. **Schema** (`0024_family_deposits.sql`): `id, holder TEXT NOT NULL, label, bank, principal REAL, interest REAL, start_date, end_date TEXT NOT NULL, received_at, note, sort_order INTEGER NOT NULL, created_at, updated_at`; index `(holder, end_date)`. No `rate`, no `credited_*` columns — they have no meaning here. Holder is stored trimmed as typed; grouping is by exact string.

4. **Validation reuses `calc::validate_deposit`** with `rate: None` and the existing `DepositInput` shape (label/bank/principal/interest/start/end), plus a holder non-empty check. Keeps the date/negative/at-least-one-of rules identical to 定期 without duplicating them.

5. **Summary math reuses `calc::DepositFacts` / `active_totals`** per holder. Month/bank/year rollups are not exposed (one rolling principal per person makes them noise); `bank` is still stored for display.

6. **Per-holder note in `app_meta`** under key `family.note.<holder>` via `mpf::meta_get`/`meta_put`. *Why*: matches the MPF page note; no holder table needed. Holders in the summary = distinct `holder` in `family_deposits` ∪ holders with a non-empty note key (`SELECT key FROM app_meta WHERE key LIKE 'family.note.%'`), so a note survives deleting the last deposit. The `:holder` path segment is URL-encoded by the client.

7. **No auto-receive on past end dates.** `deposits` auto-marks past `end_date` as received on create/import to preserve the old date-based interest rule; family rows have no such rule, so the user marks the sheet's `end` explicitly via 收訖. This is a deliberate divergence, recorded in the spec.

8. **Frontend**: `FamilyDepositsView.vue` renders holder sections (chips to filter to one holder), each with active principal, click-to-edit note (textarea, 儲存/取消 — same UX as MPF note), the upcoming table with 收訖/取消收訖 (dialog with 收訖日 + 利息 only), and inline edit/delete via `RowActions`. A 新增 form mirrors `DepositForm.vue` with a `holder` input backed by a `<datalist>` of existing holders, no rate field, and a note textarea. Year-filtered history table below. `App.vue` gains group `家人` → tab `family-deposits`, market toggle hidden.

## Risks / Trade-offs

- [Duplicated CRUD vs `deposits.rs`] → Accepted for isolation; the shared pieces (validation, rollup math, date input, row actions) are reused so the duplication is route plumbing only.
- [Free-text holder typos create a new group and orphan a note key] → datalist of existing holders in the form; grouping is exact-string so a typo is visible immediately and fixable via inline edit.
- [Accidental future coupling — someone adds `family_deposits` to a total] → spec requirement "Isolation from the user's own figures" plus an API test asserting 定期 summary / Overview / `interest_auto` are unchanged after creating and receiving a family deposit.
- [`app_meta` key with arbitrary holder text] → keys are only ever read by prefix and exact match inside `family.rs`; no other consumer enumerates `app_meta`.

## Migration Plan

Additive migration only (new table). Rollback = drop the table and delete `family.note.%` meta keys; no existing data is touched. Frontend gains a new group; nothing existing moves.
