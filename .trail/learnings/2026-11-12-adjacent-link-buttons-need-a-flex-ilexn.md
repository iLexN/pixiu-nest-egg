---
title: Adjacent link buttons need a flex-gap wrapper; whitespace between them is not rendered
author: ILexN
date: 2026-11-12
branch: main
id: l-c4fa6d7c
scope: repo
status: active
src: agent
touches:
  - frontend/src/views/FamilyDepositsView.vue
  - frontend/src/components/IbkrCard.vue
---

## Decided

When two or more `button.link` actions sit side by side, wrap them in a flex `<div>` with `gap` (`.ibkr-actions`, `.note-actions` pattern). Do not rely on the whitespace/newline between elements for separation.

## Tried and ruled out

- Newline between `<button>` elements: Vue 3's default whitespace condensing removes whitespace-only text nodes containing newlines, so the buttons render glued together.
- `padding` on `button.link`: the global `0 0.25rem` is often overridden to `0` in scoped styles (e.g. `.note-block .link`), removing even that small gap.
- `button.link + button.link { margin-left }`: works only while the buttons are DOM-adjacent; a conditional `<p>` rendered between them silently breaks the rule.

## Watch out

The same glued-rendering applies to any inline elements separated only by a source newline in a Vue template.

In the code:
- Pattern: `frontend/src/components/IbkrCard.vue` `.ibkr-actions`, `frontend/src/views/FamilyDepositsView.vue` `.note-actions`
- Fixed instance: 家庭定期 holder card 隱藏備註 + 新增備註/編輯備註, openspec change `2026-11-12-fix-family-deposit-note-actions`
