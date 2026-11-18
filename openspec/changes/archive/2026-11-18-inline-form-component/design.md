# Design

## Decisions (from grilling)

- **D1 — Presentational shell only, no state composable.** The uniform repeated thing is the shell + CSS; the state side is *less* uniform — sentinels differ (`'new'` vs object vs `true`), drafts copy different fields, save bodies differ. A `useInlineEdit` composable would have a wider, mushier interface for less uniform repetition. `editing` refs, drafts, `run()` calls, and `v-if` gates all stay in parents.
- **D2 — Plain `title` prop, not sentinel-aware.** Three title shapes exist: `editing === 'new' ? '新增X' : '編輯 …'` (edit-forms), context titles (`收訖 — ${pay_date}`), and static (`設定`). A `title` string prop covers all three uniformly; making the component understand `editing` internals buys nothing.
- **D3 — Default actions + `#actions` escape hatch.** Default `.form-actions` renders `<button type="submit">{{ submitLabel }}</button>` + `<button class="link" @click="emit('cancel')">取消</button>` — covering 16 of 17 sites. `MonthDetailPanel`'s month-header form has conditional extra buttons, so a named `#actions` slot replaces the default entirely when provided.
- **D4 — Scoped CSS with `:deep()`.** Slot content (the `<label>`s) is parent-owned DOM, so `InlineForm`'s scoped styles need `:deep()` to reach them — `.inline-form :deep(label)`. The 7 per-file CSS blocks are deleted; the component's copy is the only home.

## The component contract

```vue
<InlineForm v-if="editingX" title="編輯 債券" submit-label="儲存" @submit="saveX" @cancel="editingX = null">
  <label>名稱 <input v-model="draft.label" /></label>
  …
  <template #actions>…conditional buttons, only where needed…</template>
</InlineForm>
```

- `submit` emits on `@submit.prevent` — parents keep their `saveX` signature unchanged.
- `cancel` emits from the `取消` link — parents wire it to `editingX = null` / `emit('cancelled')` as today.
- `DepositReceiveForm` keeps its `formEl` scroll-into-view: `InlineForm` exposes the `<form>` element via `defineExpose`; the parent refs the component and reads `.$el`/`formEl` from the expose.

## Site map

| File | Sites | Notes |
|---|---|---|
| `views/AiaView.vue` | 4 | editingPolicy, editingRate, paying, withdrawing |
| `views/BondsView.vue` | 5 | editingBond, editingCoupon, fixing, receiving, receivingBond |
| `views/FamilyDepositsView.vue` | 1 | `receiving` — its `card deposit-form` is a different shell, out |
| `views/MonthStatView.vue` | 1 | addMonth (always rendered) |
| `components/MonthDetailPanel.vue` | 2 | month-header form (needs `#actions` slot), one other; per-row `editingItem` inline-edit is a different shape — out |
| `components/MonthSettingsPanel.vue` | 3 | editingAccount, editingAsset, editingSettings (boolean sentinel) |
| `components/DepositReceiveForm.vue` | 1 | always rendered; `formEl` expose |
| `views/MpfView.vue` | 1 | `account-form` class — same protocol under a drifted name (byte-identical CSS); folded in during implementation |

## Explicitly out of scope

- `views/FamilyDepositsView.vue`'s `form.card.deposit-form` — a different visual shell (`card`), not `inline-form`.
- `MonthDetailPanel`'s per-row `editingItem` editing — inline table-row editing, not a standalone form block.
- The dedicated form components (`DepositForm`, `StockForm`, `TradeForm`, `DividendForm`, `DividendReceiveForm`) — they use their own shells, no `.inline-form` markup.
- The `editing === 'new' ? 新增 : 編輯` title idiom — stays a one-liner at call sites via the `title` prop.

## Risks / Trade-offs

- **[:deep() selector misses a field element]** → styles target `.inline-form :deep(label)`/`h4`/`input`; verified visually on the live app — every form renders in place.
- **[A form's markup diverges from the shell]** → `#actions` slot covers button variance; a form diverging beyond that (different class) isn't an `inline-form` and stays out — same rule the file map already applies.
- **[Vue emits/types plumbing]** → `vue-tsc --noEmit` + `pnpm build` are the checks; no unit tests exist on the frontend.
