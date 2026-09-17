# Design

## Context

`frontend/src/App.vue` is the entire app shell. There is no router: a `tab` ref switches between six views rendered with `v-if`/`v-else-if`, and a `market` ref feeds the four stock-scoped views. The header currently renders a `nav.markets` (港股/美股, gated by `STOCK_TABS.includes(tab)`) and a flat `nav.tabs` with six buttons. Button styling comes from global `style.css`; `nav button.active` in the scoped block applies the accent fill.

## Goals / Non-Goals

**Goals:**

- Declarative nav model so group/sub-tab structure lives in one place.
- Two visually distinct nav rows: group buttons, then sub-tabs of the active group.
- Keep the market toggle bound to the active group, not a tab list.

**Non-Goals:**

- No vue-router, no URL routing, no persisted nav state — page switching stays a `tab` ref.
- No changes to view internals, headings, APIs, or the backend.

## Decisions

1. **Keep the `tab` ref; derive the group.** Store a single `tab` value (`'summary' | 'trades' | 'dividends' | 'stocks' | 'deposits' | 'depositHistory'`) and compute `group` by finding which group's tab list contains it. Alternative considered: separate `group` + `subTab` refs — rejected because it adds a second source of truth that can drift (e.g., group says stock while tab is deposits).

2. **Group buttons navigate to the group's first tab.** `selectGroup(g)` sets `tab` to `g.tabs[0].id`. No per-group memory. Alternative: remember last sub-tab per group — rejected as unneeded complexity for six pages; spec reflects the simpler behavior.

3. **Nav model as a const array.** A `NAV` constant (`{ id, label, tabs: [{ id, label }] }[]`) drives both nav rows with `v-for`, so adding a page later is a one-line change. Group ids (`'stock'` / `'deposit'`) are distinct from tab ids to avoid the collision where tab `'stocks'` lives in group `'stock'`.

4. **Two rows via a `.subnav` wrapper, not a nested component.** A `div.subnav` (`flex-basis: 100%`) holds the sub-tabs and — while the 股票 group is active — the 港股/美股 toggle, so markets sit on the same row as the sub-tabs they affect. Sub-tab buttons use `var(--surface-alt)` background to read as secondary; the shared `nav button.active` accent style marks the active item on both rows. Alternative considered: dropdown menus — rejected (extra interaction and CSS for two groups).

5. **Labels only change in the nav.** Views keep their internal headings (持倉總覽, 定期記錄, etc.); only nav labels are renamed to 總覽 / 管理 / 記錄. `docs/DATA_FLOW.md` references are updated to the new group → sub-tab paths.

## Risks / Trade-offs

- Duplicate sub-tab label 總覽 in both groups → harmless, only one group's sub-tabs render at a time.
- `tab` ids and view components stay 1:1, so the existing `v-if`/`v-else-if` chain in `<main>` is untouched.
- Docs (`DATA_FLOW.md`, `AGENTS.md`) mention old tab names → update `DATA_FLOW.md` in this change; AGENTS.md mentions 持倉總覽 as a page name which still matches the in-page heading, so it can stay.
