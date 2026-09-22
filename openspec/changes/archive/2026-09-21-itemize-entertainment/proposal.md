# Proposal

## Why

娛樂支出 is the last opaque sum in Month Stat: the sheet's O column is a hand-written expression (`=500 + 4700 + 75`), and amounts that should also be excluded from 生活支出 are typed a second time inside the J formula (`=I42 − 4700`) with no link between the two. The app reproduced this faithfully — a scalar `entertainment` plus a separate `extra_spend` item — so the double-entry survives. Itemizing 娛樂支出 like the other categories, with a per-item 不計入生活支出 flag, lets one entry serve both purposes and keeps the breakdown visible instead of buried in a formula.

## What Changes

- `month_items` gains a fourth category `entertainment` and a boolean `exclude_from_living` — set on an entertainment item, its amount also counts toward the 生活支出 exclusion (the sheet's `=I − …` tail) in addition to the 娛樂支出 total.
- `month_stats.entertainment` is dropped: each month's existing scalar migrates into a single `entertainment` item, and the month's 娛樂支出 becomes a derived sum (`entertainment_sum`), same pattern as 調整.
- `living_spend` gains the flagged items: `month_spend − Σextra_spend − Σ(entertainment where exclude_from_living)`. `extra_spend` stays for non-fun exclusions (TV, tax, AIA premium).
- The 開心Pool chain and yearly 娛樂支出 aggregate read the item sum; no behavior change there.
- The month editor's 娛樂支出 number field becomes a read-only sum; items are added in the items section with a 娛樂 category and a 不計入生活支出 checkbox.
- `import_xlsx` materializes the O cell as a single `entertainment` item (cached total, formula text as note) instead of storing the scalar; the `=I − …` exclusions keep importing as `extra_spend` items — history is not auto-split or re-linked.
- `check_parity` compares the sheet's O against the derived entertainment sum.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `month-stat`: 娛樂支出 becomes itemized (new `entertainment` category + `exclude_from_living` flag); `living_spend` subtracts flagged entertainment; the scalar column and its PATCH field go away.
- `spreadsheet-trade-import`: the O cell imports as an `entertainment` item; parity compares the derived sum.

## Impact

- Backend: migration `0016` rebuilds `month_items` (extended CHECK + `exclude_from_living` column), moves each nonzero `month_stats.entertainment` into an item, and drops the column; `models.rs`, `calc.rs` (item sums, living_spend, pool chain, yearly aggregates), `routes/months.rs` (item validation, response field), `xlsx.rs`/`import.rs`/`parity.rs` updates.
- Frontend: `api.ts` types; `MonthStatView.vue` item editor gains the 娛樂 category and the exclusion checkbox, and the scalar 娛樂支出 field becomes a displayed sum.
- Docs: `docs/DATA_FLOW.md` and `docs/MONTH_STAT_OVERVIEW.md` updates.
- No new dependencies; workbook stays read-only.
