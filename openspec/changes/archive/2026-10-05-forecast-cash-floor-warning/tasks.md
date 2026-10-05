# Tasks

## 1. Display rule

- [x] 1.1 In `frontend/src/views/OverviewView.vue`, add `:class="{ negative: month.cash !== null && month.cash < 20000 }"` to the 活期 row's `<td>` in the 預測 grid (the `null` guard is required — `null < 20000` coerces true and would paint `—` cells red); verify `cd frontend && pnpm exec vue-tsc --noEmit` passes
- [x] 1.2 Update `docs/overview.md`'s 預測 section so the `活期` bullet documents the red-below-`20000` display rule alongside the `ref check` red-while-negative note; verify the wording matches the delta spec

## 2. Integration check

- [x] 2.1 Run `cd frontend && pnpm build`, restart the backend on :8787 (`lsof -ti :8787 | xargs kill` then `cargo run -p wealth-backend --bin wealth-backend` in background), and verify on the live 總覽 page that 活期 cells below `20000` render red while cells at `20000`, above it, or `—` render normally
