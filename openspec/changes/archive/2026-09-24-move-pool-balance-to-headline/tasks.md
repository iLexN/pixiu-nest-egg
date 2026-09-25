# Tasks

## 1. Move the pool tile

- [x] 1.1 In `frontend/src/views/OverviewView.vue`, delete the 開心 Pool row from the 過去 12 個月平均 table and add a fourth `.total-card` ("開心 Pool", `fmtMoney(data.averages.pool_balance)`) to the headline `totals-grid`; verify the page builds and the tile renders beside 流動資產 ÷ 薪金×100
- [x] 1.2 Update `docs/DATA_FLOW.md` so the headline strip and the 過去 12 個月平均 card descriptions match the new layout; verify the doc names the pool figure in the strip and not the card

## 2. Verify

- [x] 2.1 Run `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` and confirm both pass
