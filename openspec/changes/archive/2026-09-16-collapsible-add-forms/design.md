## Context

See proposal.md for motivation. Current state:

- `TradesView.vue` mounts `<TradeForm>` (a self-contained component that owns its form state) directly above the filters + `TradeTable` card.
- `StocksView.vue` owns a `reactive` form object plus `resetForm()`/`save()` inline; the form is add-only since `inline-stock-row-editing` moved row editing into `StockTable`. `message`/`error` feedback paragraphs already live in the table card.
- `DepositHistoryView.vue` mounts `<DepositForm>` (same self-contained pattern as `TradeForm`) above the 定期記錄 card; `message`/`error` paragraphs sit at the bottom of the section.

## Goals / Non-Goals

**Goals:**

- Each view shows a labeled 新增… button by default; the form appears only after it is clicked.
- A successful save collapses the form.
- Closing the panel discards in-progress input — the next open shows an empty form.
- Feedback (`已新增…`, errors) remains visible after the form collapses.

**Non-Goals:**

- No modal/dialog pattern — the form stays an inline `.card` in the same position, just conditionally rendered.
- No changes to `TradeForm`/`DepositForm` internals, `StockTable`, APIs, or persistence.
- No persistence of the open/closed state across page loads (always starts closed).

## Decisions

- **`v-if` over `v-show` for `TradeForm`/`DepositForm`.** Both components build their form state in `<script setup>`; `v-if` remounts on each open, giving a guaranteed-empty form for free. `v-show` would keep stale input alive. For `StocksView`'s inline form there is no component boundary, so closing calls `resetForm()` instead.
- **Auto-collapse on save.** In `TradesView.onSaved` and `DepositHistoryView.onSaved`, set `showForm = false` before reloading; in `StocksView.save()`, set it after `api.createStock` succeeds. The new row in the reloaded table is the confirmation, matching the owner's preference that history is the primary surface.
- **Button placement: above the existing cards, inside the `<section>`.** Keeps the existing vertical order (button → form → table card) so the form still opens where it used to sit; no layout rework. Plain `button` styling already exists globally; no new CSS is needed.
- **`StocksView` also collapses on market switch.** The existing `watch(() => props.market)` already calls `resetForm()`; adding `showForm = false` prevents an open HK add-form carrying over to 美股.
- **No form changes inside the components.** `TradeForm` and `DepositForm` keep their own submit buttons and error display; the view only controls mounting.

## Risks / Trade-offs

- [User opens the form, then it vanishes after save while they expected to enter another row] → Accepted trade-off per owner decision (auto-collapse); reopening is one click and costs nothing since the form resets anyway.
- [`StocksView` success/error text would be hidden with the form] → Already mitigated: `message`/`error` paragraphs live in the table card, not the form.
- [Draft input lost if the user misclicks the toggle] → Accepted; the form is short and the owner prefers a clean default state.
