# Proposal

## Why

The inline-editing form protocol repeats **17 times** across 7 files — `form.inline-form` + `h4` title + field rows + `.form-actions` (submit + `取消`) — and every one of those files also carries a **byte-identical ~21-line CSS block** (`.inline-form`, `h4`, `label`, `.form-actions`). It is the same shape `page.ts` fixed for the page lifecycle: an interface with 17 implementations and no implementation home. Adding a form today means copying the shell and the whole CSS block into yet another file.

## What Changes

- **`components/InlineForm.vue`** — a presentational shell: props `title` and `submit-label` (default `儲存`), emits `submit`/`cancel`, default slot for field rows, named `#actions` slot as the escape hatch for the one site with conditional extra buttons. It owns `.inline-form`/`h4`/`label`/`.form-actions` styles once (scoped + `:deep` so they reach slotted fields).
- **All 17 sites adopt it** — AiaView ×4, BondsView ×5, FamilyDepositsView ×1, MonthStatView ×1, MonthDetailPanel ×2, MonthSettingsPanel ×3, DepositReceiveForm ×1. Each parent's `v-if` gate, `editing` refs, drafts, and save logic stay put — the component owns markup + CSS only. `DepositReceiveForm`'s `formEl` scroll-into-view ref is forwarded via `defineExpose`.
- **~150 duplicated CSS lines deleted** + ~70 shell-markup lines → ~4-line component usage per site.
- **Behavior unchanged** — same forms, same fields, same labels, same styling; `skip_specs: true` is set.

## Capabilities

### New Capabilities

None — pure refactor; `skip_specs: true` is set.

### Modified Capabilities

None — markup and styling render identically; `skip_specs: true` is set.
