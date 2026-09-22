# Design

## Context

`Overview!F3:G10` reads `AVERAGE(OFFSET('Month Stat'!<col>31, 0, 0, 12))` — a hand-maintained 12-row window anchored at `C31` = 2025-09, i.e. the 12 months *before* the current month. The current month's row is deliberately excluded: its spend/save columns are not final until the next payday. `G10` links the live pool balance (`'Month Stat'!M4`). `H6` derives a living-spend budget `ROUNDUP(G6 × 1.05, −2)` = 14,900 and feeds the 預測 block's red-flag rule `H6 < H1 × 0.01% × 30 + 9000`.

The app's existing `month_running_averages` averages *all* months (the sheet's row-8 cells `AVERAGE(C10:C1008)`) — a different window, so a separate derivation is needed.

## Goals / Non-Goals

**Goals:**
- Trailing averages over the 12 most recent month rows with `month < current month`; per column, months with no value are skipped (AVERAGE ignores blanks).
- `H6` budget + red-flag rule surfaced to the UI for the future 預測 table.
- Parity coverage of the cached cells.

**Non-Goals:**
- The 預測 forecast grid and the J–M strategy block (目標, saving/year, FIRE date) that consume these figures.
- The all-time running averages already live in Month Stat — unchanged.

## Decisions

- **Window = latest 12 completed month rows, not a calendar range.** The sheet's OFFSET takes 12 consecutive rows; `month < current_month`, sorted descending, take 12 reproduces it and tolerates a missing month row the same way.
- **Per-column skip-empty filtering.** AVERAGE ignores blank cells, so each column averages only the months where its derived figure exists (`interest` keeps the existing non-empty rule: `start_cash.is_some() || interest != 0`).
- **`ROUNDUP(x, −2)` = `ceil(x / 100) × 100`** for positive x — living spend is always positive.
- **The floor threshold is computed, not stored:** `living_budget_floor = liquid_assets × 0.0001 × 30 + 9000`. The UI colors 生活預算 green while the budget stays under the floor and red while it exceeds it — the floor is what liquid assets can sustain, so exceeding it is the warning state. The response exposes `living_budget_low` plus the floor value so the UI can explain the coloring.
- **Parity compares these cells as informational.** The sheet's anchor is bumped by hand and months are user-edited, so drift is expected — same convention as the rest of `check_overview`.
- **`G10` reuses `live_totals_input.pool_balance`** — the live chain already computes `'Month Stat'!M4`.

## Risks / Trade-offs

- [Sheet anchor lags the app's auto-tracking window at month boundaries] → Informational parity outcome; resolves when the user bumps the anchor.
- [A month with no stored row inside the window shrinks the divisor for every column] → Intentional: matches AVERAGE-over-rows, and 12 contiguous rows is the normal case.
