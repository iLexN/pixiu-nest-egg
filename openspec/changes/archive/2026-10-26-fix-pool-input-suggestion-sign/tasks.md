# Tasks

## 1. Pool Input Suggestion Sign

- [x] 1.1 Change `pool-input` candidate `amount` in `backend/src/calc/suggestions.rs::build_suggestions` from `-events.pool_input` to `events.pool_input`, update `suggestions_cover_each_event_kind` to assert `4142.66`, and verify with `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check`
- [x] 1.2 Update `docs/overview.md` (`Month Stat (總覽 → 月結)` suggestions note) from `` `pool-input` −pool_input `` to `` `pool-input` +pool_input `` and restart the running `wealth-backend` process so `GET /api/months/2026-10` returns `+454.95`
