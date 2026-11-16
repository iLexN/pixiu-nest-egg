# Proposal

## Why

The editor flags `merge_num` in `routes/patch.rs` — "match expression can be
replaced with a method call" — clippy's `option_if_let_else` (a nursery lint,
off by default). Same editor/toolchain gap as the earlier lint-parity
changes; the user opted to enable the lint and rewrite every site it flags.

## What Changes

- Add `option_if_let_else = "warn"` under a new `[workspace.lints.clippy]`
  table in the root `Cargo.toml`.
- Rewrite all 16 Option-match sites it flags:
  - Simple `match` → `unwrap_or`/`map_or`/`is_some_and`: patch.rs ×2,
    service/months.rs, service/mpf.rs (identical `merge_text` copies),
    routes/stocks.rs, service/wealth.rs, calc/suggestions.rs.
  - Nested/tuple arms → `map_or_else` with `zip`/`and_then` inside:
    calc/months.rs ×2, routes/mod.rs, service/ibkr.rs.
  - Vec get-or-insert (`match find() { Some(b)=>b, None=>{push;last_mut} }`)
    → `iter().position().unwrap_or_else(push-and-index)`:
    calc/deposits.rs ×3, calc/dividends.rs, calc/stocks.rs.

No behavior changes; `skip_specs: true`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor plus tooling configuration; `skip_specs: true` is set.

## Impact

- 12 backend files touched; `Cargo.toml` gains a `[workspace.lints.clippy]`
  table. `option_if_let_else` is a nursery lint — warn-level, still failed by
  `cargo clippy -D warnings`.
