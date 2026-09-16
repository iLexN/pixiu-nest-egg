## 1. DateInput component

- [x] 1.1 Create `frontend/src/components/DateInput.vue`: `defineModel<string>({ default: '' })`, `defineOptions({ inheritAttrs: false })`, a `type="text"` input with `v-bind="$attrs"`, `placeholder="YYYY-MM-DD"`, `pattern="\d{4}-\d{2}-\d{2}"`, `maxlength="10"`, `inputmode="numeric"`, `autocomplete="off"`; a 📅 `type="button"` that calls `showPicker()` on a hidden-but-rendered `type="date"` input (absolute, `opacity:0`, `pointer-events:none` — not `display:none`); picker `change` writes its value into the model; sync the picker's value from the model before opening. Verify the file compiles via `pnpm exec vue-tsc --noEmit`.

## 2. Swap call sites

- [x] 2.1 `frontend/src/components/TradeForm.vue`: replace the 日期 `type="date"` input with `<DateInput v-model="form.trade_date" required />`. Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 2.2 `frontend/src/components/TradeTable.vue`: replace the inline-edit `type="date"` input with `<DateInput v-model="draft.trade_date" />` and update the `.editing input[type='date']` width rule to fit the new control (~9rem). Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 2.3 `frontend/src/components/DepositForm.vue`: replace the end date `type="date"` input with `<DateInput v-model="form.end_date" required />`. Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 2.4 `frontend/src/components/DepositTable.vue`: replace the inline-edit `type="date"` input with `<DateInput v-model="draft.end_date" />` and update the `.editing input[type='date']` width rule similarly. Verify `pnpm exec vue-tsc --noEmit` passes.
- [x] 2.5 `frontend/src/views/TradesView.vue`: replace both 由/至 `type="date"` inputs with `<DateInput v-model="from" @change="load" />` / `to`, and in `load()` only pass `from`/`to` when empty or matching `/^\d{4}-\d{2}-\d{2}$/`. Verify `pnpm exec vue-tsc --noEmit` passes.

## 3. Verification

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and confirm no errors.
- [x] 3.2 Run `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` to confirm the untouched backend still passes.
- [x] 3.3 Manual check in the dev UI: every date field shows `yyyy-mm-dd`; typing `16/09/2026` is blocked by form validation; the 📅 button opens a calendar and picking a date fills `yyyy-mm-dd`; submitting a trade and a deposit succeeds; the 由/至 filters apply only complete dates.
- [x] 3.4 Update `docs/DATA_FLOW.md` and `AGENTS.md` if either documents the date input format.
