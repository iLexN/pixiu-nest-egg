# Tasks

## 1. Create the component

- [x] 1.1 `components/InlineForm.vue` — `title` + `submit-label` props, `submit`/`cancel` emits, default slot, `#actions` slot override, scoped CSS (`:deep()` for slotted `label`/`h4`), `defineExpose` the form element

## 2. Adopt at all 17 sites

- [x] 2.1 `views/AiaView.vue` ×4, `views/BondsView.vue` ×5 — and delete each file's `.inline-form`/`h4`/`label`/`.form-actions` CSS blocks
- [x] 2.2 `views/FamilyDepositsView.vue` ×1, `views/MonthStatView.vue` ×1, `views/MpfView.vue` ×1 (`account-form` drift) — CSS blocks deleted
- [x] 2.3 `components/MonthDetailPanel.vue` ×2 (one via `#actions` slot), `components/MonthSettingsPanel.vue` ×3 — CSS blocks deleted
- [x] 2.4 `components/DepositReceiveForm.vue` ×1 — keep `formEl` scroll-into-view via the expose — CSS block deleted

## 3. Verify

- [x] 3.1 `pnpm build` + `pnpm exec vue-tsc --noEmit` clean; no `.inline-form` CSS remains outside `InlineForm.vue`; every form renders + saves + cancels on the live app (visual spot-check of at least one form per touched file)
