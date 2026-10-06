# Proposal

## Why

The AIA page has no free-text area for section-level remarks — policy `note` fields are per-row, so there is nowhere to keep scratch observations about the section as a whole (e.g. pending surrender decisions, premium reminders across policies). The MPF page already solves this with a page-level 備註 stored in `app_meta`; AIA should get the same.

## What Changes

- New `app_meta` key `aia.note` holding one free-text note for the whole AIA section, stored via the existing `meta_get`/`meta_put` helpers — an empty note deletes the key rather than storing a blank row.
- `GET /api/aia/summary` gains a `note` field returning the stored text.
- New `PATCH /api/aia/note` endpoint updating (or clearing) the note.
- The AIA view gains a 備註 card: the stored note shown as a click-to-edit paragraph that swaps to a multiline textarea with 儲存/取消 — the same interaction as the MPF page's note block.
- No schema migration (`app_meta` already exists) and no import/parity changes (the workbook's AIA sheet has no section-level note cell; nothing is seeded).

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `aia-policies`: adds a section-level free-text note — returned by the summary, updated through a new endpoint, and shown editable on the AIA view.

## Impact

- Backend: `models.rs` (`AiaNotePatch`), `routes/aia.rs` (`NOTE_KEY`, `note` on the summary response, `update_note` handler), `routes/mod.rs` (route registration + the pinned OpenAPI path list).
- Frontend: `api.ts` (`AiaSummary.note`, `updateAiaNote`), `AiaView.vue` (備註 card).
- Docs: `docs/investments.md` AIA section, `docs/database.md` `app_meta` key list.
- No new dependencies; no migration; the workbook stays untouched.
