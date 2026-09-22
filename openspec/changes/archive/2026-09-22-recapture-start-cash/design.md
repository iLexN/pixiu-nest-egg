# Design

## Context

`upsert` in `backend/src/routes/months.rs` already computes `live_totals_input` when `recapture` is set — the input's `cash_sum` field is exactly the Overview 活期 figure (Σ `manual_assets` kind `cash`), so re-snapshotting 月初 needs no new data source. `LiveTotalsInput` is `Copy`, so the input can be kept for both the totals and `cash_sum`.

## Decisions

- **Capture on recapture only, not on create.** The user asked that 新增月份 take no 月初 input and not auto-snapshot; a created row stores NULL `start_cash` and the column renders `—` until 重新擷取 or a manual edit. The totals still snapshot on create, as before.
- **月初 stays stored-only.** Unlike `total_assets`/`liquid_assets`, there is no live fallback for `start_cash` — past months' 活期 can't be reconstructed — so `改為即時` does not touch it and no `start_cash_live` flag is added.
- **Precedence mirrors the totals**: an explicit `start_cash` in the patch wins over `recapture`, which wins over the stored value. `null` remains indistinguishable from absent (unchanged limitation — 月初 cannot be cleared to NULL via PATCH, same as before).
- **活期 = `cash_sum` only.** IBKR cash positions are excluded, matching the Overview 半流動資金 block where 活期 lists only `cash` manual assets.

## Risks

- A recapture on an old month overwrites its historical 月初 with today's 活期 — same caveat the totals already have, and the button only renders for current-or-later months.
