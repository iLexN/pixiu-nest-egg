# Design

## Context

`/api/summary` already returns `note` per stock (`SummaryStock extends Stock`), and `PATCH /api/stocks/{id}` already trims a note string and clears it on empty/null. All work is frontend-only in `SummaryView.vue`, plus small textbox/style tweaks in `StockForm.vue` and `StockTable.vue`. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Expandable, full-width note sub-row per stock row in 持倉總覽, with inline textarea editing.
- Expand/collapse-all control; expanded state resets on market switch.
- Multi-line note entry and rendering (pre-wrap) in the 管理 form and table.

**Non-Goals:**
- Persisting expanded state across reloads or market switches.
- Backend or API changes.

## Decisions

- **Sub-row inside the same `<tbody>`** via `<template v-for>`: keeps the existing main `<tr>` and its drag/drop handlers untouched; the note row carries no drag handlers so reordering only targets stock rows. Alternative (a second table or detail panel) loses the row-to-note association.
- **Note row = empty drag-handle `<td>` + `<td colspan="15">`**: spans the full table width without disturbing the 16-column layout.
- **Expand state as a `Set<number>` of stock ids**, replaced immutably so Vue reactivity triggers; toggles per row and an all-or-nothing button in `.summary-actions`.
- **Save via `api.updateStock(id, { note: trimmed || null })` then `load()`**: reuses the existing PATCH semantics (empty clears) and keeps the table authoritative. Errors surface in `error` like `savePrice`.
- **Muted toggle when a stock has no note** so the affordance exists without drawing attention.

## Risks / Trade-offs

- [Expanded rows during drag-reorder could confuse the drop target] → Note rows have no drag/drop handlers; only stock rows are valid drop targets.
- [Long notes widen vertical space] → Sub-row uses a smaller font and subtle background; collapsible by default.
