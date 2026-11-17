# Proposal

## Why

Dismissing a 月結 suggestion writes a tombstone and the candidate is filtered out of `GET /api/months/{ym}` entirely — the row vanishes. There is no way to see what was ignored or to undo a mis-tap; the tombstone is invisible forever.

## What Changes

- `GET /api/months/{ym}` returns dismissed candidates flagged `dismissed` instead of dropping them, and returns candidates (pending and dismissed) for **every** stored month — including months before the current month, which previously returned no candidates at all.
- The month detail response gains `suggestions_actionable`: accept/dismiss/undismiss actions apply only to the previous month and later; candidates in earlier months are informational.
- New `POST /api/months/{ym}/items/undismiss` removes a dismissal tombstone, restoring the candidate to pending.
- 月結 item editor: dismissed candidates render after pending ones with a `忽略（不計）` status and 接受/取消忽略 actions where actionable; ghost rows outside the action window show no actions.

## Capabilities

### Modified Capabilities

- `month-stat`: the candidate-visibility and dismiss semantics requirements change — dismissal no longer hides the candidate, candidates are returned for every month, and actions are gated to the previous month and later. The item-editor view requirement gains the dismissed-row presentation and the undismiss action.

## Impact

- Backend: `models/months.rs` (`MonthSuggestion.dismissed`), `calc/suggestions.rs` (emit flagged, action-window predicate), `routes/months.rs` (`show` always builds candidates + response flag + `undismiss_item` route), `store/months.rs` (`undismiss_item`), `routes/mod.rs` registration and the OpenAPI pin test.
- Frontend: `api.ts` (`undismissMonthItem`), `api-types.ts` regenerated, `MonthStatView.vue` ordering/status/gating.
- Docs: `GLOSSARY.md` Candidate entry, `docs/overview.md` suggestions paragraph, `docs/database.md` dismissals line.
