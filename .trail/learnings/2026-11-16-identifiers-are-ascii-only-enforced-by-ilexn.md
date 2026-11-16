---
title: "Identifiers are ASCII-only; enforced by workspace non_ascii_idents warn, not deny"
author: ILexN
date: 2026-11-16
branch: main
id: l-d96d218d
scope: repo
status: active
src: agent
touches:
  - Cargo.toml
  - backend/Cargo.toml
  - backend/src/calc/candidates.rs
  - backend/src/service/deposits.rs
---

## Decided
Rust identifiers (fn/let/const/type names) stay ASCII-only. `non_ascii_idents = "warn"` is set in the root `[workspace.lints.rust]` table so `cargo check`/`clippy` and rust-analyzer flag it — matching the editor's "Non-ASCII characters" inspection, which is where code review happens. Warn (not deny) because `cargo clippy -D warnings` already escalates it in verification; deny would break `cargo test`/`cargo run` on a warning, stronger than the editor's signal.

## Watch out
`non_ascii_idents` is ALLOW-by-default in modern rustc (1.98 here) — the security subsets `mixed_script_confusables`/`confusable_idents` are the ones still warn-by-default, and they don't fire on CJK. So without the workspace table, clippy is silent and the warning only exists in the editor. Chinese in string literals/comments/docs (定期, 轉入, etc.) is domain content — the lint covers identifiers only; test assertions keep the literal strings pinned.

In the code: `[workspace.lints.rust] non_ascii_idents = "warn"` in root Cargo.toml, `[lints] workspace = true` in backend/Cargo.toml. Archived change: openspec/changes/archive/2026-11-16-ascii-identifiers/.
