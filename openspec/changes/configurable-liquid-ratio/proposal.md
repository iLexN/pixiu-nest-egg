# Proposal

## Why

The liquidity buffer threshold — 半流動資金 vs a quarter of 流動資產 — is hardcoded at 25% in two places, mirroring literals inside the workbook's `C14` and `N7` formulas. The user wants to tune that target without a code change, the same way `forecast.bill_amount` already lets them adjust the quarterly 差餉 charge.

## What Changes

- New `app_meta` setting `overview.semi_liquid_target` — the share of 流動資產 that 半流動資金 should cover, stored as a fraction, effective default `0.25` while unset (matching the sheet's literal). Editable through `PATCH /api/months/settings` alongside salary / pool rate / 差餉; `null` resets to the default.
- `GET /api/overview`'s `semi_liquid.vs_quarter_liquid` becomes `total − configured ratio × liquid_assets` instead of the hardcoded `0.25`; the 總覽 block's red/green threshold and footer label follow the configured percent.
- The forecast's `ref check` becomes `semi_liquid − configured ratio × liquid_assets` (currently `÷ 4`), sharing the same setting — the sheet's `N7` (`H1 ÷ 4`) and `C14` (`0.25 × H1`) are one threshold written two ways.
- The overview response carries the effective ratio so the `與…%流動相差` footer label can render the configured percent.
- `check_parity` keeps verifying the workbook's own formula: it compares `semi_liquid.total − 0.25 × liquid_assets` (recomputed with the sheet's literal) against cached `C14`, independent of the configured display ratio — so tuning the setting never turns the parity report noisy.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `overview`: the 半流動資金 block's `vs_quarter_liquid` and the 總覽 page's red threshold switch from a literal 25% to the configured ratio; the forecast `ref_check` follows the same setting; a new requirement covers the setting itself (endpoint field, validation, default, reset).
- `spreadsheet-trade-import`: the Overview parity report's `C14` comparison is redefined as a check of the sheet's literal formula (`total − 25% × liquid_assets`) rather than the response field, which now follows the user's configured ratio.

## Impact

- **Backend**: `backend/src/routes/months.rs` (new `app_meta` key + constant, `MonthSettings`/`MonthSettingsPatch` field, `update_settings` validation), `backend/src/models.rs`, `backend/src/routes/overview.rs`, `backend/src/routes/forecast.rs` + `backend/src/calc.rs` (`ForecastInput`/`forecast_months` take the ratio), `backend/src/parity.rs` (`C14` recomputed with literal `0.25`). No migration — `app_meta` is generic key-value; no new endpoints, so the OpenAPI pin test is untouched.
- **Frontend**: `frontend/src/api.ts` types, `frontend/src/views/MonthStatView.vue` settings form (percent input like pool rate), `frontend/src/views/OverviewView.vue` dynamic label.
- **Docs**: `docs/overview.md` updates the `C14` and `ref check` descriptions.
