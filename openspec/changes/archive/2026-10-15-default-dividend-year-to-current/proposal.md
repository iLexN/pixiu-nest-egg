# Proposal

## Why

The 已收記錄 card on the dividend views opens at 全部年份, which forces a manual filter every visit to see the current year's receipts — the view the owner actually wants by default. The deposit views already default their year filter to the current year; dividends should match.

## What Changes

- The 已收記錄 year filter on the dividend views (both 港股 and 美股 — they share the component) SHALL default to the current year, falling back to the latest year with received records when the current year has none. 全部年份 remains an explicit option.
- The market switch resets the filter to the current year again (same fallback).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `stock-dividends`: gains a requirement for the received-records view's year filter default (the existing spec is API-level and says nothing about the UI default).

## Impact

- Frontend only: `frontend/src/views/DividendsView.vue` — `year` ref initial value, the market-switch watcher reset, and a fallback adjustment after the summary loads (same pattern as `DepositHistoryView`).
- `docs/stocks.md`: the one line describing the 已收記錄 card's filters gains the default.
- No backend, schema, or API change.
