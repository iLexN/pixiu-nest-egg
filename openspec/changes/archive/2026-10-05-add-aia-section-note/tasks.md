# Tasks

## 1. Backend

- [x] 1.1 Add `AiaNotePatch { note: Option<String> }` to `backend/src/models.rs` beside `AiaRatePatch` and verify `cargo check` passes
- [x] 1.2 In `backend/src/routes/aia.rs`: add `pub const NOTE_KEY: &str = "aia.note"` beside `RATE_KEY`, add `note: Option<String>` to `AiaSummaryResponse`, load it in `summary()` via `crate::mpf::meta_get(&state.pool, NOTE_KEY)`, and verify `cargo check` passes
- [x] 1.3 In `backend/src/routes/aia.rs`: add `update_note` handling `PATCH /api/aia/note` (body `AiaNotePatch`, returns `super::mpf::NoteResponse`, writes via `mpf::meta_put`/`meta_get` like `mpf::update_note`) with the `#[utoipa::path]` annotation, and verify `cargo check` passes
- [x] 1.4 Register the route in `backend/src/routes/mod.rs` via `utoipa_axum::routes!(aia::update_note)` and add `("/api/aia/note", "patch")` to the pinned list in `openapi_documents_every_api_operation` (sorted between `/api/aia/events/{id}` and `/api/aia/policies`); verify `cargo test openapi_documents` passes
- [x] 1.5 Update `docs/database.md` `app_meta` key list with `aia.note` and verify the entry matches the implemented key

## 2. Frontend

- [x] 2.1 In `frontend/src/api.ts`: add `note: string | null` to `AiaSummary` and `updateAiaNote(note: string | null)` issuing `PATCH /aia/note`; verify `pnpm exec vue-tsc --noEmit` passes
- [x] 2.2 In `frontend/src/views/AiaView.vue`: add the 備註 card replicating MpfView's note block — `noteDraft`/`noteLoaded` refs, `startNote`/`saveNote` (sends `noteDraft.trim() || null`, then reloads), click-to-edit `<p class="note-text">` swapping to a `<textarea>` with 儲存/取消, placed after the 記錄 card, plus `.note-text` and `textarea` scoped styles; verify `pnpm build` succeeds and the note saves/clears against the running backend

## 3. Docs and verification

- [x] 3.1 Add the note flow line to the AIA section of `docs/investments.md` (mirroring the MPF note line) and update `docs/DATA_FLOW.md` if the AIA row needs it
- [x] 3.2 Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `pnpm build`, `pnpm exec vue-tsc --noEmit`, and `openspec validate add-aia-section-note`; all pass
- [x] 3.3 Restart the backend on :8787 and verify `PATCH /api/aia/note` + `GET /api/aia/summary` round-trip via the Scalar UI or curl
