# Tasks

## 1. Backend model and calc

- [x] 1.1 `models/months.rs`: add `dismissed: bool` to `MonthSuggestion`.
- [x] 1.2 `calc/suggestions.rs`: `build_suggestions` emits dismissed candidates flagged `dismissed` (stored keys still filtered); replace `suggestions_enabled` with `suggestion_actions_enabled(month, today)` = month ≥ first of previous month; update affected tests.

## 2. Backend routes and store

- [x] 2.1 `store/months.rs`: `undismiss_item` deletes the tombstone row.
- [x] 2.2 `routes/months.rs`: `show` always builds suggestions and returns `suggestions_actionable` on `MonthDetailResponse`; new `undismiss_item` route `POST /api/months/{ym}/items/undismiss`.
- [x] 2.3 `routes/mod.rs`: register route via `utoipa_axum::routes!`; add to `openapi_documents_every_api_operation` pin list.
- [x] 2.4 Tests: history month returns pending + dismissed flagged; undismiss removes the tombstone; dismiss keeps the row in the response.

## 3. Frontend

- [x] 3.1 `pnpm gen:api` to regenerate `api-types.ts`; add `api.undismissMonthItem`.
- [x] 3.2 `MonthStatView.vue`: pending-then-dismissed ordering, `忽略（不計）` status cell, gated actions via `suggestions_actionable`, `undismissSuggestion` handler, dimmed dismissed styling.

## 4. Docs

- [x] 4.1 `GLOSSARY.md` Candidate entry: dismissal stays visible flagged, uncounted, restorable.
- [x] 4.2 `docs/overview.md` suggestions paragraph + `docs/database.md` `month_item_dismissals` line.

## 5. Verify

- [x] 5.1 `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
- [x] 5.2 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`
- [x] 5.3 Restart backend; manual check: dismiss → `忽略（不計）` row; 取消忽略 → pending; 接受 on dismissed → item; old month → read-only ghosts.
