## Context

After `inline-stock-row-editing` (archived 2026-09-16), all three tables — `StockTable`, `TradeTable`, `DepositTable` — edit rows inline: each carries `editingId`, a string-valued `draft`, `rowError`, `saving`, and duplicates its entity's field markup plus a reduced preview (`TradeTable` re-implements the fee/total preview that `TradeForm` already has; `DepositTable` re-implements the principal+interest total). Meanwhile `TradeForm` and `DepositForm` are add-only forms with full-size labeled inputs, per-field error display, and the real preview logic; stock adding lives in an inline form inside `StocksView`. See proposal.md — Why for the motivation.

## Goals / Non-Goals

**Goals:**

- One form per entity serves both add and edit: field markup, per-field errors, and preview exist exactly once.
- Tables are read-only; 編輯/刪除 emit `edit`/`remove` and the view owns `editing` state.
- Editing a row surfaces the same validation and preview UX as adding — no hidden field ceiling.
- All three entities behave identically, resolving the original inconsistency.

**Non-Goals:**

- No backend, API, spec, or validation changes.
- No new fields added to any form (e.g. `is_active` stays unexposed — this change only removes the ceiling, it doesn't fill it).
- No modal/drawer; the form stays where it already renders (top of the view for stocks/trades, toggle/reveal for deposits).

## Decisions

- **Form-based edit over inline cells (option D from exploration).** Rationale: the form is the richer surface (labels, per-field `error.fieldMessage`, live preview) and shares one markup copy between add and edit; inline rows forced a second, poorer copy inside every table. Alternatives considered: keep inline (couples editable fields to columns, keeps the duplication); expandable detail row (edit stays at the row but still needs a second full form — worst of both for this app).
- **`editing` state lives in each view, not the form or table.** Views already coordinate `saved`/`remove`/`error`; adding `editing = ref<Entity | null>` and passing it down keeps the existing event flow: `table @edit → view.editing → form :editing → form @saved/@cancelled → view clears + reloads`.
- **Edit mode reuses the form's existing submit path with an `editing` branch**, mirroring how `StocksView` previously did it: title flips to `編輯 <x>`, submit calls `update*` with the entity id, a 取消 button appears and emits `cancelled`. `watch(() => props.editing)` fills the form; when `editing` is null the form resets to `emptyForm()` so the same form cleanly returns to add mode.
- **`TradeForm` resolves `stock_id` from the selected `code`** (`stocks.find(s => s.code === form.code)?.id`) on update, and sends `input_mode: editing.input_mode` rather than re-deriving it — the trade's stored mode is authoritative.
- **`DepositForm` gets shared across both deposit views.** `DepositsView` (未到期) has no form today, so it renders `<DepositForm v-if="editing">` while a row is being edited — same component, new placement. `DepositHistoryView` reuses its existing `showForm` toggle: `@edit` sets `editing` and opens the form.
- **Tables keep their read-only extras**: `TradeTable` keeps the `toggle-order` header and `totalCost` footer; `DepositTable` keeps `showStatus`/`emptyText`/`columns`; the `bank-codes` datalist moves out of `DepositTable` (a copy already exists in `DepositForm`).
- **No shared base form component.** The three forms differ in fields, payloads, and preview rules; the "sharing" is within each entity (add ↔ edit), not across entities. A generic entity-form abstraction would cost more than it saves.

## Risks / Trade-offs

- [Edit returns to the top of the page, away from the row — the original complaint in reverse] → Accepted by user decision; mitigated by the 編輯 <code> title making context explicit. If it proves annoying, a `scrollIntoView` on edit is a one-line follow-up.
- [`DepositsView` gains a conditional form — new visual element on that page] → Only renders while editing; `v-if` keeps the page unchanged otherwise.
- [Editing state can go stale if the entity list reloads (market switch, filter change, year change)] → Views clear `editing` on reload triggers where the edited row may vanish (market watch in StocksView/TradesView, year change in DepositHistoryView); worst case the form keeps stale values until submit, which errors cleanly.
- [Two places could 編輯 simultaneously? No — `editing` is a single ref per view] → Only one edit session per view, same as before.
