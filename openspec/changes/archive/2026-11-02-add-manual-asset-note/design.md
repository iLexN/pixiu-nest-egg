# Design

## Context

`manual_assets` was last rebuilt in `0029_cash_accounts.sql`. Its columns are `id, label, liquidity, amount, sort_order, updated_at`. Rows flow through `store::months` to the `/api/manual-assets` routes and to `service::overview`. There, each row becomes an `OverviewAssetRow` with `manual_asset_id` set.

## Goals / Non-Goals

**Goals:**
- A note stored per manual asset row, editable in 設定 and readable on 總覽 when the mouse is over the row.

**Non-Goals:**
- Notes on module rows (港股, 債券, …) or on `cash_accounts` rows.
- Editing the note inline on 總覽. 總覽 only shows it.
- Touch-device tooltips.

## Decisions

- **Column:** add it with `ALTER TABLE manual_assets ADD COLUMN note TEXT` in a new migration. It is nullable with no default, so existing rows read NULL and no table rebuild is needed.
- **PATCH semantics:** `note: Option<Option<String>>` with the repo's `nullable` deserializer, matching `StockPatch.note` and `MonthItemPatch.note`. Absent keeps the note and `null` clears it. The route trims the note and maps an empty string to NULL, so the frontend can always send `note.trim() || null`.
- **Overview carries the note:** add `note: Option<String>` to `OverviewAssetRow` instead of having 總覽 make a second `listManualAssets` call. The overview response stays the single source for that page. `asset_row()` (module rows) sets `None`.
- **Tooltip:** a native `title` attribute on the whole `<tr>`, because the user asked for hover on the row. It needs no tooltip component. The label gets a `cursor: help` and dotted underline cue only when a note exists, so it is clear which rows have one.
- **設定 display:** the note goes on a `<small class="muted note">` line under the label, the same pattern as the AIA policy notes and month items.

## Risks / Trade-offs

- [Native `title` tooltips have a delay and do not show on touch devices] → This is acceptable for the local desktop app. A custom tooltip can be added later without changing the API.
- [Adding a migration that writes to the live DB] → The ALTER is additive and nullable, so existing data is untouched.
