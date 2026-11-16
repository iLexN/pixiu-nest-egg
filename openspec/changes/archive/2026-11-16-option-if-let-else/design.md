# Design

## Context

Third editor-parity lint after `non_ascii_idents` and
`unused_qualifications`. `option_if_let_else` differs: it's clippy *nursery*,
not rustc — its suggestions are not machine-applicable (`cargo clippy --fix`
applies none) and can be worse than the `match` for complex arms. User chose
full conversion anyway.

## Decisions

- **`[workspace.lints.clippy]`** — new table alongside `[lints.rust]`; same
  warn-not-deny rationale as the archived design (`-D warnings` enforces).
- **`unwrap_or`/`is_some_and` where they'd beat the lint's literal suggestion**
  — `merge_num`'s match is `Option<Option<f64>>`, so `unwrap_or(existing)`
  is exact and matches sibling `merge`; `map_or(false, ..)` would trip
  default-on `unnecessary_map_or`, so suggestions.rs uses `is_some_and`.
- **`position().unwrap_or_else` for the five Vec get-or-insert buckets** —
  `map_or_else` cannot compile there: the `iter_mut().find()` borrow stays
  alive across the call while the None closure needs `&mut buckets` again.
  `iter().position()` returns an owned `usize`, so the `unwrap_or_else`
  closure may push freely; `&mut buckets[index]` yields the element.
- **Side-effect and nested arms keep their inner `match`** inside
  `map_or_else` closures (ibkr.rs validation, parse_status, months.rs
  changes): the lint only sees the outer Option match, and the inner match
  stays readable.

## Risks / Trade-offs

- [Nursery lint may gain/lose heuristics across clippy versions] → warn
  level limits fallout to a warning; `-D warnings` still catches it.
- [Double scan in the position pattern vs original single find] → buckets
  are ≤12 elements; negligible.
