# Tasks

## 1. Navigation

- [x] 1.1 In `frontend/src/App.vue`, drop `'months'` and `'year'` from the `Group` union and remove the 月結 and 年結 entries from `NAV`; extend the `overview` group's `tabs` to `[{ id: 'overview', label: '總覽' }, { id: 'months', label: '月結' }, { id: 'yearReview', label: '年結' }]`. Leave the `Tab` union, `tab` default (`'overview'`), the `<main>` render branches, and the `market` toggle condition unchanged. Verify with `cd frontend && pnpm exec vue-tsc --noEmit && pnpm build`.
- [x] 1.2 Run the app (`cargo run -p wealth-backend --bin wealth-backend` + `pnpm dev`) and confirm in the browser: the first-level nav is 總覽, 股票, 定期, MPF, 債券, AIA, 家人; the landing page is 總覽 with sub-tabs 總覽 | 月結 | 年結; 月結 shows the Month Stat page and 年結 shows the year review page; the 港股/美股 toggle stays hidden on all three 總覽 sub-tabs; clicking 總覽 again from 年結 → 定期 → 總覽 lands on 總覽 → 總覽.

## 2. Documentation

- [x] 2.1 Update `docs/DATA_FLOW.md`: rename the "Load the 年結 → 回顧 view" heading and the two "年結 → 回顧" references in the year-end steps to "總覽 → 年結", and retitle the "Month Stat (月結)" section to mention it lives at 總覽 → 月結. Verify with `grep -n "年結 → 回顧" docs/DATA_FLOW.md` returning nothing.
- [x] 2.2 Update `AGENTS.md`: in the Migration roadmap paragraph change "YearInReview 年結 → 回顧" to "YearInReview 總覽 → 年結" and note in the Month Stat / Overview sections that 月結 and 年結 are sub-tabs of 總覽. Verify with `grep -n "年結 → 回顧" AGENTS.md` returning nothing.

## 3. Final check

- [x] 3.1 Run `openspec validate move-month-year-under-overview --strict` and `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`; both succeed.
