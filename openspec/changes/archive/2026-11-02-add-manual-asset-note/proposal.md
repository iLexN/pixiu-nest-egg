# Proposal

## Why

Manual 資產 rows such as HS人壽 are a single typed amount. There is nowhere to record context like the policy number, the valuation date, or how the figure was worked out. The user wants a free-text note on each row, visible where the amount is read.

## What Changes

- Manual asset rows gain an optional free-text `note`. It is returned by `GET /api/manual-assets`, accepted by `POST`, and accepted by `PATCH`, where `null` clears it and an absent field leaves it unchanged. A blank note is stored as absent.
- The 月結 → 設定 → 資產 editor gets a 備註 field, and the 資產 table shows a row's note under its name.
- `GET /api/overview` asset rows carry the manual row's `note`. The 總覽 資產 table shows it as a tooltip when the mouse is over that row.

## Capabilities

### New Capabilities

### Modified Capabilities
- `month-stat`: manual asset rows carry an optional note, which the 資產 editor edits and displays.
- `overview`: asset table rows expose the manual row's note, shown as a hover tooltip on 總覽.

## Impact

- Schema: new migration adding a nullable `manual_assets.note` column.
- API: `ManualAsset`, `NewManualAsset`, `ManualAssetPatch` and `OverviewAssetRow` gain `note`. The change is additive and adds no new endpoints.
- Frontend: `MonthStatView.vue` (settings 資產 editor), `OverviewView.vue` (asset table) and `api.ts` types.
- Docs: `docs/database.md`, `docs/overview.md`.
