# Tasks

## 1. Backend: store and manual-asset API

- [x] 1.1 Add `backend/migrations/0033_manual_asset_note.sql` (`ALTER TABLE manual_assets ADD COLUMN note TEXT;`) and document `note` under `manual_assets` in `docs/database.md`. Verify by running `cargo test`, which applies the migrations.
- [x] 1.2 Add `note` to `ManualAsset` and `NewManualAsset` (`Option<String>`, serde default) and `ManualAssetPatch` (`#[serde(default, deserialize_with = "nullable")] Option<Option<String>>`) in `backend/src/models/months.rs`. Thread it through `ASSET_COLUMNS`, `row_to_asset`, `insert_asset` and `update_asset` in `backend/src/store/months.rs`. Verify with `cargo build`.
- [x] 1.3 In `create_asset`/`update_asset` (`backend/src/routes/months.rs`), trim the note and map blank to NULL. On PATCH, absent keeps the existing note and `null` clears it. Extend the manual-asset route test to cover set, kept-on-amount-only patch, cleared by `null`, and cleared by whitespace. Verify with `cargo test`.

## 2. Backend: overview response

- [x] 2.1 Add `note: Option<String>` to `OverviewAssetRow` (`backend/src/models/overview.rs`). Fill it from the manual row in `backend/src/service/overview.rs` and set `None` in `asset_row()`. Update any test fixtures. Mention the tooltip in `docs/overview.md`. Verify with `cargo test`, then check that a manual row's note appears in `GET /api/overview`.

## 3. Frontend

- [x] 3.1 Add `note` to `ManualAsset`, `NewManualAsset`, `ManualAssetPatch` and `OverviewAssetRow` in `frontend/src/api.ts`. Verify with `pnpm exec vue-tsc --noEmit`.
- [x] 3.2 In `MonthStatView.vue`'s 設定 資產 editor, add `note` to `assetDraft`, add a 備註 `<input>`, send `note: draft.note.trim() || null`, and render `<small v-if="asset.note" class="muted note">` under the label. Verify with `pnpm build`, then check by hand that saving shows the note.
- [x] 3.3 In `OverviewView.vue`, set `:title="row.note ?? undefined"` on the asset `<tr>` and add a `cursor: help` / dotted-underline cue on the label when a note exists. Verify with `pnpm build`, then hover the HS人壽 row by hand.

## 4. Integration

- [x] 4.1 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`. Then restart the backend per AGENTS.md and confirm `GET /api/manual-assets` returns `note`.

## Workflow follow-up

- Archive the change once the user confirms the behavior on the live app.
