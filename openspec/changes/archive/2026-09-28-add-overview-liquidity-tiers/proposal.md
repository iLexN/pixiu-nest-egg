# Proposal

## Why

The `Overview` sheet's 策略 block (`J3:K7`) re-partitions 總數 into four liquidity tiers — what can be spent normally, the untouchable 6-month salary buffer, and what could be recovered short- vs long-term — and is one of the cash-planning figures the user still reads off the workbook. Every input it needs (半流動資金, salary, the asset rows) is already derived by `GET /api/overview`, so it is the cheapest remaining Overview block to migrate; the only gap is that the app has no way to say whether a manual asset row (Irene vs HS人壽) is short- or long-term recoverable.

## What Changes

- `GET /api/overview` gains a `liquidity_tiers` block mirroring `J3:K7`: `can_use` (`K4 = B14 − K5`), `cannot_use` (`K5 = salary × 6`, the sheet's `N6`), `short_term` (`K6` = 港股 + 債券 + IBKR + short-term manual assets), `long_term` (`K7` = 基金 + MPF + long-term manual assets). `can_use`/`cannot_use` are absent without a stored salary; `short_term`/`long_term` are absent without the USD→HKD rate, matching the module's existing rule.
- `manual_assets` rows gain a `liquidity` attribute (`short` | `long`, default `long`) so `asset` rows can be placed in `K6` or `K7`. Exposed on `GET/POST/PATCH /api/manual-assets`; editable in the existing 資產 editor (Month Stat settings). Irrelevant for `cash` rows, which already sit inside 半流動資金.
- Import seeds Irene as `short` and HS人壽 as `long` under the existing seed-once rule; the parity report pins `K4:K7`.
- The 總覽 page shows a 策略 card with the four tiers.
- The 6-month multiplier is a constant, not a setting — the sheet's `N5`/`N6` are fixed labels the user treats as static.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `overview`: new `liquidity_tiers` block on the aggregation endpoint and a 策略 card on the 總覽 page.
- `month-stat`: manual `asset` balances carry a `liquidity` classification (`short`/`long`), settable through the manual-assets endpoints and the 資產 editor.
- `spreadsheet-trade-import`: seeding assigns the sheet's Irene/HS人壽 rows their liquidity; the Overview parity report also compares `K4:K7`.

## Impact

- **Schema**: new migration `0025_manual_assets_liquidity.sql` adding `manual_assets.liquidity TEXT NOT NULL DEFAULT 'long' CHECK (liquidity IN ('short','long'))`. Existing rows (only `HS 人壽` in the live database) default correctly.
- **Backend**: `models.rs` (`ManualAsset`, `NewManualAsset`, `ManualAssetPatch`, new `ManualAssetLiquidity` enum, new `LiquidityTiers` struct on `OverviewResponse`), `routes/months.rs` (manual-assets CRUD reads/writes the column), `routes/overview.rs` (block derivation), `xlsx.rs` (`SheetManualAsset.liquidity`, `OverviewCached` K4:K7), `import.rs` (seed), `parity.rs` (pins). No new endpoints, so `openapi_documents_every_api_operation` is unchanged.
- **Frontend**: `api.ts` types, `OverviewView.vue` (策略 card), `MonthStatView.vue` (流動性 select on `asset` rows).
- **Docs**: `docs/database.md` (`manual_assets`), `docs/overview.md` (策略 block), `docs/MONTH_STAT_OVERVIEW.md` header note.
- **Compatibility**: `POST /api/manual-assets` accepts `liquidity` optionally (defaults `long`), so existing callers keep working. No breaking changes.
