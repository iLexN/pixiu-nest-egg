# Proposal

## Why

The 年結 → 資產 美股 rows report USD figures only. The owner thinks in HKD — every other card converts USD at the stored rate — so the 成本 and 市值 cells should show the HKD equivalent beneath the USD value instead of making them do the multiplication by hand.

## What Changes

- Each year row's `us_stocks` entry gains two display companions: `cost_hkd` and `now_value_hkd` = the USD figure × the stored USD→HKD rate, absent while either input is missing.
- The 年結 page shows the HKD figure as a muted sub-line under the USD value on the 美股 成本 and 美股 市值 cells.
- No change to the stored figures, the blends, or any other group — the USD rows remain the primary figures.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `year-in-review`: the `us_stocks` entry carries HKD-converted companions for `cost` and `now_value`, and the page renders them as sub-lines.

## Impact

- `backend/src/calc/year_review.rs` — two additive response fields and their derivation; test coverage for the converted and no-rate cases.
- `frontend/src/api.ts` — response type.
- `frontend/src/views/YearReviewView.vue` — the two sub-lines.
- `docs/overview.md` — asset-group description.
