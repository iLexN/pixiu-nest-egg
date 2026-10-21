# Proposal

## Why

The 總覽 目標 card shows each goal's target, remain, years, and ETA but never says which figures a row subtracts or divides by. The 3rd and Final rows can look contradictory — remains far apart, years nearly equal — because 3rd subtracts 流動資產 and paces by `(歷年存+息)×12` while Final subtracts 總數 and paces by `歷年總變化×12`. A reader can't see that from the card; the pace values aren't even exposed to the frontend.

## What Changes

- The 目標 table gains a `基數` column (the figure `remain` subtracts — 現值 for 1st/2nd, 流動資產 for 3rd, 總數 for Final) and a `步伐` column (the pace `years` divides by — 存/年 for 1st/2nd, `(歷年存+息)×12` for 3rd, `歷年總變化×12` for Final). Each cell names the term and shows its live value.
- The `goals` response block echoes `total_change_avg` (the 'Month Stat' C8 running average feeding the Final pace), matching the existing `saved_avg`/`interest_avg` echoes.
- The "shared figures show once in the footer" rule is relaxed so the new columns may restate 現值/存/年 values on milestone rows.
- The existing formula footnotes stay unchanged.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the 目標 card requirement gains the 基數/步伐 columns, and the echoed-fields requirement gains `total_change_avg`.

## Impact

- `backend/src/models/overview.rs`: new `total_change_avg` field on `OverviewGoals` (additive API response field).
- `backend/src/calc/overview.rs`: populate the echo.
- `frontend/src/api.ts`: `OverviewGoals.total_change_avg`.
- `frontend/src/views/OverviewView.vue`: two new columns in the 目標 table.
- `docs/overview.md`: card description updated.
