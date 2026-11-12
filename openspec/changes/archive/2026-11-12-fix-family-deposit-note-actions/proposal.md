# Proposal

## Why

On 家庭定期 (Family Deposits), each holder card's note block renders `隱藏備註` and `新增備註`/`編輯備註` as two `.link` buttons with zero separation — the scoped `.note-block .link { padding: 0 }` removes all horizontal padding and Vue's whitespace condensing drops the newline between them, so when a holder has no note the two actions appear glued together and are hard to read.

## What Changes

- Wrap the holder card's note actions (the show/hide toggle and the add/edit-note link) in a `.note-actions` flex row with `gap: 0.5rem`, matching the existing `.ibkr-actions` pattern in `IbkrCard.vue`.
- Apply the same class to the 儲存/取消 row shown while editing a note.
- With a note present and expanded, `編輯備註` moves from below the note text up beside `隱藏備註`; the note itself still renders below the actions row.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
<!-- None — pure visual fix; the family-deposits spec specifies note behavior, not pixel layout. skip_specs: true is set in .openspec.yaml. -->

## Impact

- Frontend only: `frontend/src/views/FamilyDepositsView.vue` (note-block markup + one new scoped CSS rule).
- No backend, API, spec, or doc changes.
