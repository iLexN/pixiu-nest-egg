## Context

`TradeTable.vue`, `StockTable.vue`, and `DepositTable.vue` each render an identical `.row-actions` cell containing 編輯 and 刪除 `button.link` buttons that emit `edit`/`remove` to their parent views. Tables render directly inside `.card` sections with no `overflow` wrapper, so absolutely positioned overlays are not clipped. Global button styles live in `frontend/src/style.css` (`button.link`, `button.link.danger`).

## Goals / Non-Goals

**Goals:**
- One compact ⋯ button per row that reveals 編輯/刪除 on click.
- No table layout shift when the menu opens or closes.
- Single shared implementation; views' `edit`/`remove` handlers untouched.

**Non-Goals:**
- No changes to edit/delete behavior, confirmation dialogs, or the API.
- No keyboard-navigation/arrow-key menu support beyond Escape-to-close (matches the app's simple utilitarian UI).
- SummaryView's inline 現價 editing and drag handle are unrelated and unchanged.

## Decisions

- **Shared `RowActions.vue` component.** The three cells are identical, so one component emits `edit` and `remove`; each table passes the row's item through its existing emits (`@edit="emit('edit', trade)"` → `@edit="emit('edit', trade)"` via `$event`, i.e. `<RowActions @edit="emit('edit', trade)" @remove="emit('remove', trade)" />`). Alternative: duplicating dropdown state in each table — rejected, triples the open/close logic.
- **Dropdown overlay, not inline swap.** The menu is `position: absolute` inside a `position: relative` cell wrapper, right-aligned under the ⋯ button. An inline swap would resize the auto-layout column and make the whole table jitter on open.
- **Dismissal via fixed transparent backdrop + document Escape listener.** While `open`, a `position: fixed; inset: 0` transparent div under the menu captures outside clicks (including a second click on the ⋯ button itself, closing the menu). A `keydown` listener closes on Escape. Because any outside click closes the current menu, only one row's menu is ever open — no cross-component coordination needed. Listeners are added on open and removed on close/unmount (`onBeforeUnmount`).
- **Menu items reuse the existing link/danger styling.** Items are full-width `button.link` / `button.link.danger` inside a bordered card-styled menu so 刪除 stays red and the look matches the rest of the app.

## Risks / Trade-offs

- [Menu on the last row extends below the card bottom edge] → Acceptable: no `overflow: hidden` ancestor, so it overlays the page background; keep the menu small (two items) so overlap is minor.
- [Two clicks instead of one for every edit/delete] → Intended trade-off requested by the owner; delete still has its `window.confirm` guard.
