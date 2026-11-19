# Proposal

## Why

All app settings live in a collapsible 設定 card inside the 月結 tab, yet almost none of them relate to the month ledger — 活期帳戶/資產 balances, Money Master, 目標 constants, and the 日曆 ID serve 總覽, the forecast, HappyPool, and the deposit calendar links. Settings deserve their own home.

## What Changes

- New top-level 設定 nav group appended after 家人, containing a single 設定 tab backed by a new settings view.
- The 設定 card leaves 月結 entirely; 月結 shows only month data.
- The single all-fields settings form splits into per-section edit forms — 一般 (薪金, 開心Pool 利率, 繳費, 流動目標), Money Master, 目標, 日曆 — each with a read-only summary line and an 編輯 action that PATCHes only that section's fields.
- The 開心Pool 利率 moves to its own section: a table of every stored year's rate （年份 / 利率） with per-row 編輯/刪除 and a 新增 action — deleting a year restores the earlier-year fallback.
- The 活期帳戶 and 資產 editors move unchanged to the new page.
- `GET /api/settings` gains a read-only `pool_rates` list; writes still go through the existing PATCH fields — no endpoint changes otherwise.

## Capabilities

### New Capabilities

- `app-settings`: the dedicated 設定 page — its sections, per-section editing forms, the pool-rate year input, and the 活期帳戶/資產 editors hosted there.

### Modified Capabilities

- `app-navigation`: the top nav gains a 設定 group after 家人; the 月結 sub-tab no longer carries a settings card.
- `month-stat`: the 月結 view loses its settings card; the 資產 editor requirements repoint to the 設定 page.
- `time-deposits`: the Private calendar setting is edited on the 設定 page's 日曆 section, not the 月結 設定 form.

## Impact

- Frontend: new `frontend/src/views/SettingsView.vue`; `frontend/src/components/MonthSettingsPanel.vue` deleted; `frontend/src/views/MonthStatView.vue` drops the panel and its settings/accounts/assets fetches; `frontend/src/App.vue` gains the 設定 group and tab.
- Backend: none — `GET/PATCH /api/settings` already supports partial updates and the OpenAPI surface is unchanged.
- Docs: `docs/overview.md`, `docs/deposits.md`, `docs/DATA_FLOW.md` repoint their "月結 設定" references to the 設定 tab.
