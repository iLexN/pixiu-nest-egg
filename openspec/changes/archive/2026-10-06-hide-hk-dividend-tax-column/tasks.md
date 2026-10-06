# Tasks

## 1. DividendTable — market prop and conditional Tax column

- [x] 1.1 In `frontend/src/components/DividendTable.vue`, add `market: Market` to `defineProps` (import `Market` from `../api`); verify `pnpm exec vue-tsc --noEmit` flags the two `DividendTable` usages in `DividendsView.vue` as missing the prop.
- [x] 1.2 Gate the Tax `<th>` and its `fmtMoney(dividend.withholding_tax)` `<td>` on `props.market === 'US'` inside the existing `v-if="!props.receivable"` blocks, and update `columns` to `props.receivable ? 9 : props.market === 'US' ? 12 : 11`; verify by inspection that header, cells, and empty-row colspan agree for all four combinations (pending/received × HK/US).

## 2. DividendsView — pass market

- [x] 2.1 Pass `:market="props.market"` to both `<DividendTable>` usages in `frontend/src/views/DividendsView.vue`; verify `pnpm exec vue-tsc --noEmit` passes clean.

## 3. Verification

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; verify both succeed.
- [x] 3.2 Restart the running backend per AGENTS.md (`lsof -ti :8787 | xargs kill`, then `cargo run -p wealth-backend --bin wealth-backend` in the background, confirm the API answers) so :8787 serves the rebuilt frontend.
- [x] 3.3 Manual check on http://127.0.0.1:8787: 港股 → 派息 → 已收記錄 has no Tax column; 美股 → 派息 → 已收記錄 still shows Tax with the real withholding-tax values; the empty 已收記錄 state renders `沒有已收派息記錄` spanning the row.
