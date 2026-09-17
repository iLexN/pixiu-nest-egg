# Tasks

## 1. Grouped nav in App.vue

- [x] 1.1 In `frontend/src/App.vue` script: replace `STOCK_TABS` with a `NAV` const array (`{ id, label, tabs: [{ id, label }] }[]`) — group `stock` (label 股票) with tabs `summary`→總覽, `trades`→交易記錄, `dividends`→派息, `stocks`→管理; group `deposit` (label 定期) with tabs `deposits`→總覽, `depositHistory`→記錄. Verify the file compiles (`cd frontend && pnpm exec vue-tsc --noEmit`).
- [x] 1.2 Change `tab` default to `'summary'`; add `group` computed (group whose tabs contain `tab`), `activeGroup` computed, and `selectGroup(g)` that sets `tab` to `g.tabs[0].id`. Verify `vue-tsc --noEmit` still passes.
- [x] 1.3 Update the header template: `nav.groups` renders `v-for` group buttons (active = current `group`, click = `selectGroup`); `nav.markets` renders unchanged but gated on `group === 'stock'`; `nav.tabs` renders `v-for` over `activeGroup.tabs` (active = current `tab`). Verify in `pnpm dev` that each button shows the right view.
- [x] 1.4 Update scoped styles: `.tabs { flex-basis: 100% }` so sub-tabs sit on their own row; `.tabs button { background: var(--surface-alt) }` for a secondary look; keep `nav button.active` accent on both rows. Verify the two rows render distinctly and the active states highlight correctly.

## 2. Docs

- [x] 2.1 Update tab-name references in `docs/DATA_FLOW.md` to the new group paths (e.g. `DepositsView (定期 tab)` → `(定期 → 總覽)`, `定期記錄 tab` → `定期 → 記錄`, `股票管理` → `股票 → 管理`, `持倉總覽` nav references → `股票 → 總覽`). Verify no stale nav-label references remain (`grep -n "持倉總覽\|股票管理\|定期記錄" docs/DATA_FLOW.md` only matches in-page heading references, if any).

## 3. Verify

- [x] 3.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — all pass.
- [x] 3.2 Manual check via `pnpm dev`: app opens on 股票 → 總覽; clicking 定期 shows 總覽/記錄 sub-tabs and hides the market toggle; returning to 股票 lands on 總覽 and restores the 港股/美股 toggle; market selection persists across stock sub-tabs.
