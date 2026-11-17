# Design

## Read path

`calc::build_suggestions(month, events, stored_keys, dismissed_keys)` currently filters out both stored and dismissed keys. Change: stored keys stay filtered (an accepted candidate is a real item, not a ghost), dismissed keys are emitted as `MonthSuggestion { …, dismissed: true }` — the flag replaces suppression. `MonthSuggestion` gains `dismissed: bool` (models, serialized via utoipa schema → regenerated `api-types.ts`).

`routes::months::show` today gates `build_suggestions` behind `suggestions_enabled(month ≥ current month)`. Candidates are pure derivations from dated events, so they exist for any month; the gate only decided whether they were offered. `show` now always builds them and reports `suggestions_actionable = month ≥ first_of_month(previous month)` on `MonthDetailResponse` — the backend owns the action window so the frontend never reimplements the rule. `suggestions_enabled` is replaced by `suggestion_actions_enabled(month, today)` in `calc::suggestions`; its only caller was `show`.

## Undismiss write path

`POST /api/months/{ym}/items/undismiss` `{auto_key}` → `store::months::undismiss_item` (`DELETE FROM month_item_dismissals WHERE month = ? AND auto_key = ?`) → 204. Mirrors `dismiss_item` exactly — single-store write, no service layer. Registered via `utoipa_axum::routes!`; added to the `openapi_documents_every_api_operation` pin list. Accept already clears the tombstone in-transaction (`create_item` → `clear_dismissal`), so 接受 on a dismissed row needs nothing new.

## Frontend

- `suggestionsOf(category)` returns pending first then dismissed (stable sort on `dismissed`).
- Dismissed rows: status cell `忽略（不計）`, dimmed; actions 接受 + 取消忽略 while `detail.suggestions_actionable`. Pending rows: 接受/忽略 while actionable. Non-actionable months render all ghost rows with an empty action cell.
- `api.undismissMonthItem(ym, autoKey)` posts to the new endpoint; `undismissSuggestion` reloads like `dismissSuggestion`.
- The 沒有項目 empty-state check already keys off the combined list.

## Notes

- Dismissed rows never count in totals: they are ghosts, not stored items; `month_item_sums` reads `month_items` only.
- History months may surface stale pending ghosts (events predating tracking); accepted trade-off per the full-reconciliation decision — they are read-only.
- Action-window boundary is `suggestion_actions_enabled` — single point to adjust.
