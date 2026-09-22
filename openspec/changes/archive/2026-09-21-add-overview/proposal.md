# Proposal

## Why

`Overview` is the last core sheet still living in `財富分析報告.xlsx`. Its `A3:C18` block — the asset-allocation table (港股/債券/基金/MPF/Irene/HS人壽/IBKR/Sum plus share-of-total) and the 半流動資金 block (已定期/HS/渣打/活期) — is now fully derivable: every source module is migrated, and the live-totals aggregation built for Month Stat already computes the headline 總數/流動資產. The one missing input is the IBKR account block on the 美股 sheet (`A1:B5`): the cash positions feeding `Overview!B9`, plus the transferred-in/now-value cross-check the user reads off the IBKR app (its implied FX rate differs from the manual `aia.usd_hkd_rate`, so it stays manual).

Migrating this block also corrects a live-totals mismatch: the app currently counts active deposits at principal + expected interest, but the sheet's `已定期` (`定期!B1 = sum(F:F)` on `input`) is principal only — interest lands in 活期 at maturity instead of being accrued.

## What Changes

- New `GET /api/overview` aggregating the sheet's `A3:C18`: headline 總數 (B1), 流動資產 (H1), J1 = 流動資產 ÷ (salary×100); asset rows 港股/債券/基金/MPF + every manual `asset` row + IBKR with per-row share of Sum; the 半流動資金 block (已定期 principal, each manual `cash` row, 活期, 半流動 total, `C14` = 半流動 − 25%×流動資產) and the `A13` ratio 半流動 ÷ (港股+債券+半流動+IBKR). Rate-dependent rows are absent while `aia.usd_hkd_rate` is unset.
- New `GET/PATCH /api/ibkr` for the 美股 `A1:B5` manual inputs stored in `app_meta` (`ibkr.transferred_hkd`, `ibkr.now_value`, `ibkr.hkd_cash`, `ibkr.usd_cash`), returning the derived block: computed total `(US market value + USD cash) × rate + HKD cash` (the sheet's `美股!B7` → `Overview!B9`), net = now − transferred, net%, and computed − now_value difference.
- **BREAKING** (behavior): live totals (`total_assets`/`liquid_assets`) now count active 定期 at principal only and include IBKR cash — the US term becomes the full `美股!B7` figure. This makes the app match the workbook exactly; Month Stat live figures shift accordingly.
- New 總覽 nav group (first) with a dashboard view: headline strip, asset table with inline amount editing for manual rows (existing `/api/manual-assets` endpoints), 半流動資金 block, and the IBKR readout.
- US 持倉總覽 gains an IBKR card editing the four inputs with the derived figures beside it (the sheet puts them on the 美股 sheet).
- `import_xlsx` seeds the four `ibkr.*` keys from the 美股 cached cells once (never overwritten); `check_parity` gains an Overview section comparing B3:B10, A13, B14:B18, C shares, C14, B1/H1/J1, and 美股 B1/B2/B4/B5/B7.
- Not in this change: Overview averages/budget (F/G/H), liquidity tiers (J/K), rates block (M/N), targets, the 預測 forecast grid, and YearInReview/YYYY回報率 generation.

## Capabilities

### New Capabilities

- `overview`: the all-asset dashboard — `GET /api/overview` aggregation, the IBKR account block (`GET/PATCH /api/ibkr`), the 總覽 page with inline manual-asset editing, and the IBKR card on the US summary.

### Modified Capabilities

- `month-stat`: live `total_assets`/`liquid_assets` count active 定期 at principal only (not principal + interest) and include IBKR cash positions via the full 美股!B7-style IBKR total.
- `spreadsheet-trade-import`: the import seeds the IBKR `app_meta` keys from the 美股 cached cells idempotently; parity gains the Overview block comparison.
- `app-navigation`: a 總覽 group is added before 股票, containing a single 總覽 tab; the market toggle stays hidden while it is active.
- `stock-portfolio-summary`: the 美股 持倉總覽 page gains the IBKR edit card.

## Impact

- Backend: new `routes/overview.rs`; `calc.rs` live-totals signature change; `models.rs` response/patch structs; `xlsx.rs` reads the Overview block + 美股 account cells; `import.rs`/`parity.rs` extensions; `routes/mod.rs` registration.
- Frontend: new `OverviewView.vue`, IBKR card in `SummaryView.vue`, `api.ts` types/methods, `App.vue` nav wiring.
- Docs: `docs/DATA_FLOW.md` (Overview flow + live-totals correction), `AGENTS.md` roadmap, `docs/MONTH_STAT_OVERVIEW.md` annotation.
- No new dependencies; `財富分析報告.xlsx` stays read-only. IBKR transfers remain manual Month Stat 調整 items — `ibkr.transferred_hkd` is a cumulative figure, not dated events.
