# Proposal

## Why

The `Overview` sheet's `F3:G10` block — 過去 12 個月平均 (trailing-12-month averages of Month Stat's Changed/支出/生活支出/存/利息 columns), the live 開心Pool balance, and the `H6` living-spend budget — is still spreadsheet-side. Every input is already stored (month rows + items) or computed (pool chain, liquid assets), so the block is fully derivable. `H6` is also the input the upcoming 預測 forecast table consumes, including its red-flag rule `H6 < 流動資產 × 0.0001 × 30 + 9000`.

## What Changes

- `GET /api/overview` gains an `averages` block: trailing averages of 總數增加 / 支出 / 生活支出 / 存 / 利息 over the 12 most recent months before the current one, the live pool balance, the 生活預算 figure `ROUNDUP(生活支出_avg × 1.05, −2)`, and a `living_budget_low` flag when the budget falls below `流動資產 × 0.0001 × 30 + 9000`.
- The 總覽 page gains a "過去 12 個月平均" card listing the block; 生活預算 renders red while below the threshold.
- `xlsx.rs` parses the cached cells `G4:G8`, `H6`, `G10`; `check_parity`'s Overview section compares them (informational — the sheet's OFFSET anchor is manual and month data is user-edited).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the aggregation endpoint and 總覽 page gain the trailing-12-month averages block.
- `spreadsheet-trade-import`: the Overview parity report also compares the averages block cells `G4:G8`, `H6`, `G10` as informational outcomes.

## Impact

- Backend: `xlsx.rs` cached-cell parsing, `calc.rs` `trailing_averages`, `models.rs`/`routes/overview.rs` response extension, `parity.rs` new comparisons.
- Frontend: `api.ts` types, `OverviewView.vue` averages card.
- Docs: `docs/DATA_FLOW.md`, `docs/MONTH_STAT_OVERVIEW.md`, `AGENTS.md`.
- No schema or dependency changes.
