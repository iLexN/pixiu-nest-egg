# Design

## Context

The 目標 card derives all four rows from `overview_goals` (`backend/src/calc/overview.rs`) and renders them in `frontend/src/views/OverviewView.vue`. The block already echoes `saved_avg`/`interest_avg` so the card can expand `存/年` without a second fetch; `total_change_avg` (running C8, the Final pace's base) is computed for `final_goal.years` but never returned. `averages.total_change` is a different figure — the trailing-12-month average — and must not be reused for this label.

## Goals / Non-Goals

**Goals:**

- Each goal row visibly reads `target − 基數 = remain ÷ 步伐 = years → eta`.
- The Final pace shows its true running-average value, not the trailing average.

**Non-Goals:**

- Restructuring or collapsing the existing formula footnotes.
- Changing any calculation, stored data, or endpoint shape beyond the additive field.

## Decisions

- **Echo `total_change_avg` on `goals`** rather than have the card call the months summary endpoint. Matches the established `saved_avg`/`interest_avg` echo pattern and keeps the card single-fetch. Alternative rejected: deriving the pace client-side as `remain ÷ years` — it breaks when `years` is absent (reached or missing inputs) and hides the real input.
- **Cell = term name + live value** (name on one line, muted value beneath) rather than name-only or value-only. Name-only leaves the Final pace value homeless (no footer row for it); value-only doesn't explain the number. Milestone cells restating 現值/存/年 values is deliberate — consistency across rows beats the old "footer-only" rule, which the delta relaxes.
- **Labels name the running window**: `歷年總變化×12` / `(歷年存+息)×12`, matching the `歷年存×12` wording already used in the footnote — distinct from the 平均 card's 近12月 figures.

## Risks / Trade-offs

- [Two added columns widen the table] → `goals-card` already spans two grid columns and scrolls horizontally on overflow.
- [`歷年總變化` (≈59k/mo) vs 平均 `total_change` (近12月, ≈73k/mo) could confuse] → the label keeps the 歷年 prefix; the value is the echoed running average, not `averages.total_change`.
