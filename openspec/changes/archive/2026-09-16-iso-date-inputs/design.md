## Context

All six date entry points are native `<input type="date">` elements: `TradeForm.vue` (日期), `TradeTable.vue` (inline edit), `DepositForm.vue` (end date), `DepositTable.vue` (inline edit), and `TradesView.vue` (由/至 filters). Native date inputs render in the browser/OS locale and cannot be reformatted — on this machine they show `dd/mm/yyyy`. The rest of the pipeline is already ISO: form state holds `yyyy-mm-dd` strings, `todayIso()` produces them, the backend validates `%Y-%m-%d`, and tables display the stored strings verbatim.

## Goals / Non-Goals

**Goals:**
- One reusable `DateInput` component used at every date entry point.
- Text entry and display strictly in `yyyy-mm-dd`; a calendar picker remains available via a button.
- No change to the `v-model` contract: the model is still a `yyyy-mm-dd` (or empty) string, so call sites and API payloads are untouched.

**Non-Goals:**
- No date-freedom improvements (e.g. accepting `16/9/2026` and converting) — input must be ISO.
- No backend, API, or workbook changes.
- No changes to date *display* elsewhere (already `yyyy-mm-dd`).

## Decisions

- **Custom `DateInput` component over keeping `type="date"`**: `lang`/CSS cannot force the native control's segment order, so a text input is the only reliable way to show `yyyy-mm-dd`. Alternative considered: a third-party date-picker library — rejected; a dependency is unjustified for one input style in a local app.
- **Keep the picker via a hidden native input + `showPicker()`**: a `type="date"` input rendered but visually hidden (absolute, `opacity:0`, `pointer-events:none` — not `display:none`, which disables `showPicker()`) sits next to a 📅 button; the button calls `showPicker()` and the picker's `change` event writes its already-ISO value into the model. Alternative considered: building a custom calendar popup — rejected as unnecessary complexity.
- **Validation via `pattern="\d{4}-\d{2}-\d{2}"` + `maxlength="10"`**: gives native form-validation blocking on submit and `:invalid` styling for free; calendar correctness (e.g. `2026-13-99`) stays with the existing backend check, which returns a field error the forms already display.
- **Attribute fall-through**: `defineOptions({ inheritAttrs: false })` with `v-bind="$attrs"` on the text input keeps `required` and `@change` (used by the 由/至 filters) working exactly as on the native inputs.
- **Guard the 由/至 filters**: `@change` fires on blur with partial input; `load()` only applies `from`/`to` values that are empty or match `^\d{4}-\d{2}-\d{2}$`, so half-typed values can't silently mis-filter.

## Risks / Trade-offs

- `showPicker()` unsupported in older browsers → text entry still works; the button is simply inert. Acceptable for a local single-user app.
- Users lose native segmented keyboard entry (arrow-key day/month stepping) → the picker button preserves the point-and-click path; typed ISO entry is the intended primary path.
- `pattern` validates shape, not real dates → backend validation already catches impossible dates and returns a named field error.
