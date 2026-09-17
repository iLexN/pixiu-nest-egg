# Proposal

## Why

The top nav is a flat row of six tabs (交易記錄 / 持倉總覽 / 股票管理 / 派息 / 定期 / 定期記錄) that mixes stock-scoped and deposit-scoped pages, and the app opens on 交易記錄 rather than the most useful landing page. Grouping the tabs by domain makes the nav shorter and clearer, and puts 持倉總覽 first as the default page.

## What Changes

- Reorganize the top nav in `App.vue` into a two-row grouped layout:
  - Top row: group buttons 股票 / 定期.
  - Second row: sub-tabs of the active group, plus the existing 港股/美股 market toggle (shown only while the 股票 group is active).
- 股票 group sub-tabs: 總覽 (was 持倉總覽), 交易記錄, 派息, 管理 (was 股票管理).
- 定期 group sub-tabs: 總覽 (was 定期), 記錄 (was 定期記錄).
- Default page changes from 交易記錄 to 股票 → 總覽. **BREAKING** (landing page only; no API or data change).
- Clicking a group button opens that group's first sub-tab (its 總覽); the last-visited sub-tab is not remembered.
- Update tab-name references in `docs/DATA_FLOW.md` to the new group → sub-tab paths.

## Capabilities

### New Capabilities

- `app-navigation`: the grouped top-nav structure — which pages belong to which group, the labels and order of sub-tabs, when the market toggle is visible, and which page the app opens on.

### Modified Capabilities

<!-- None: existing capability specs describe per-page behavior, not the nav shell. -->

## Impact

- `frontend/src/App.vue` — nav model, template, and styles (the only code change; there is no router, the `tab` ref drives everything).
- `docs/DATA_FLOW.md` — references to old tab names (持倉總覽, 股票管理, 定期, 定期記錄) updated to the new group paths.
- In-page headings inside the views (持倉總覽, 未到期定期, 定期記錄, etc.) stay unchanged — only nav labels change.
- No backend, API, or database impact.
