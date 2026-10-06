# Design

## Context

The MPF page already implements exactly this feature: a page-level free-text note persisted in the `app_meta` key-value table under `mpf.note`, returned by `GET /api/mpf`, updated by `PATCH /api/mpf/note` (`MpfNotePatch` → `NoteResponse`), and rendered as a click-to-edit paragraph swapping to a textarea. The AIA route module already consumes the same `mpf::meta_get`/`meta_put` helpers for `aia.usd_hkd_rate`, so all persistence machinery is shared. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**

- One free-text note per AIA section, stored under `aia.note`, exposed on the summary and editable from the page.

**Non-Goals:**

- Per-policy or per-event notes — already exist.
- Import/parity seeding — the workbook's AIA sheet has no section-level note cell, and the MPF note is likewise unseeded.
- Schema migration — `app_meta` already exists.

## Decisions

- **Reuse the MPF note pattern verbatim** rather than a per-row column or a new table: same `app_meta` key namespace (`aia.note`), same blank-deletes-key semantics via `meta_put`, same endpoint shape (`PATCH` with `{ note }` → `{ note }`), same frontend interaction. Alternative considered: a `note` column on a singleton/section table — rejected, `app_meta` is the established place for section-level state (`docs/database.md`).
- **`NOTE_KEY` lives in `routes/aia.rs`** beside `RATE_KEY`, not in `mpf.rs`: the key namespacing is per-section and aia.rs already owns its `aia.*` keys.
- **Reuse `routes::mpf::NoteResponse`** as the handler's response body instead of declaring `AiaNoteResponse`: utoipa schemas are keyed by type name, so a second `NoteResponse` struct would collide in the generated OpenAPI document. A new `AiaNotePatch` request struct is still declared in `models.rs` for symmetry with `AiaRatePatch`.

## Risks / Trade-offs

- [Two features sharing `mpf::meta_*` couples aia.rs to the mpf module] → Already the case for `aia.usd_hkd_rate`; extracting a `meta` module is a refactor outside this change's scope.
