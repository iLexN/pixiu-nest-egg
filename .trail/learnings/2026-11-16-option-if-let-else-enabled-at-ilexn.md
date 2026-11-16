---
title: "option_if_let_else enabled at warn; Vec get-or-insert needs position().unwrap_or_else, not map_or_else"
author: ILexN
date: 2026-11-16
branch: main
id: l-19591937
scope: repo
status: active
src: agent
touches:
  - Cargo.toml
  - backend/Cargo.toml
  - backend/src/calc/candidates.rs
  - backend/src/calc/deposits.rs
  - backend/src/calc/dividends.rs
  - backend/src/calc/months.rs
  - backend/src/calc/stocks.rs
  - backend/src/calc/suggestions.rs
  - backend/src/routes/forecast.rs
  - backend/src/routes/mod.rs
---

## Decided
`[workspace.lints.clippy] option_if_let_else = "warn"` — third editor-parity lint (it's clippy NURSERY, not rustc, and its suggestions are NOT machine-applicable: `cargo clippy --fix` applies none, all 16 sites were hand-rewritten). When the lint's literal `map_or` suggestion would trip the default-on `unnecessary_map_or` lint, use `is_some_and`/`is_none_or`/`unwrap_or` instead.

## Watch out
`match buckets.iter_mut().find(..) { Some(b)=>b, None=>{push;last_mut()} }` can't become `map_or_else` — the find borrow stays alive across the call so the None closure can't `&mut buckets` again (E0499). Working shape: `iter().position(..).unwrap_or_else(|| {push(..); len()-1})` then `&mut buckets[index]` — position returns an owned usize so the closure may borrow freely. Same for `match &opt { Some=>side-effects, None=>default }`: `opt.as_deref().map_or_else(default_fn, |v| match ..)` keeps the inner match (lint only checks the outer Option match).

In the code: archived change openspec/changes/archive/2026-11-16-option-if-let-else/; five position+unwrap_or_else sites in calc/deposits.rs, calc/dividends.rs, calc/stocks.rs.
