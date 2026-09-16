## 1. 交易記錄 — TradesView.vue

- [x] 1.1 Add `const showForm = ref(false)` and render a 新增交易 `<button type="button">` at the top of the `<section>` that toggles `showForm`; verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 1.2 Gate the existing `<TradeForm>` with `v-if="showForm"` (no prop/emit changes) so it mounts fresh each open; verify the form does not render on page load and appears on click.
- [x] 1.3 Set `showForm.value = false` inside `onSaved` so the form auto-collapses after a successful save; verify a saved trade collapses the form and appears in the reloaded `TradeTable`.

## 2. 定期記錄 — DepositHistoryView.vue

- [x] 2.1 Add `const showForm = ref(false)` and a 新增定期 toggle button at the top of the `<section>`; gate `<DepositForm>` with `v-if="showForm"`; verify `vue-tsc` passes and the form starts hidden.
- [x] 2.2 Set `showForm.value = false` inside `onSaved`; verify a saved deposit collapses the form and the history list reloads.

## 3. 股票管理 — StocksView.vue

- [x] 3.1 Add `const showForm = ref(false)` and a toggle button whose label is `新增港股股票` / `新增美股股票` based on `props.market`; verify the label follows the 港股/美股 market buttons.
- [x] 3.2 Gate the `<form class="card">` with `v-if="showForm"`; make the toggle handler call `resetForm()` when closing so in-progress input is discarded; verify `vue-tsc` passes.
- [x] 3.3 Set `showForm.value = false` after a successful `api.createStock` in `save()` and inside the `watch(() => props.market)` handler; verify the form collapses after 新增 and when switching markets.
- [x] 3.4 Confirm the `message`/`error` paragraphs in the table card still render for add, inline-edit save, and delete while the form is collapsed (no markup move needed — they already sit above `<StockTable>`).

## 4. Docs and verification

- [x] 4.1 Update `docs/DATA_FLOW.md` so the "Add a trade in 交易記錄", "Add a stock in 股票管理", and "Add a deposit in 定期記錄" flows note the form is revealed by the 新增… button; verify wording matches the new UI.
- [x] 4.2 Run `cd frontend && pnpm exec vue-tsc --noEmit && pnpm build`; verify both succeed with no warnings.
- [x] 4.3 Manual check: on 交易記錄, 股票管理 (HK and US), and 定期記錄, the form is hidden on load; clicking the 新增… button shows it; saving collapses it and the new row appears; clicking the button again hides it; an interrupted form reopens empty.
