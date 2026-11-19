# Tasks

## 1. 總覽 投資目標 hint

- [x] 1.1 Delete the `<p v-if="targetNoteTerms" class="muted">月薪增幅於 年結 → 投資 覆寫</p>` line in `frontend/src/views/OverviewView.vue`; verify the six-term `note-formula` footnote above it is untouched.

## 2. Stale 月結設定 pointers

- [x] 2.1 In `frontend/src/views/YearReviewView.vue` change `月結設定更新` to `設定更新` in the year-end guide note; verify no other "月結設定" remains (`grep -rn "月結設定" frontend/src` returns nothing).
- [x] 2.2 In `frontend/src/views/DepositsView.vue` change `月結設定更新` to `設定更新` in the muted checklist note; same grep verification.

## 3. Verification

- [x] 3.1 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit` — clean build and typecheck.
- [x] 3.2 Restart the backend (`lsof -ti :8787 | xargs kill`, then detached `cargo run -p wealth-backend --bin wealth-backend`) and confirm the API answers on `http://127.0.0.1:8787`.

## Workflow follow-up

- `openspec archive` the change (no spec sync needed — `skip_specs`), then suggest a one-line commit message.
