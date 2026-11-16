---
title: "Editor-only warnings (non-ASCII idents, path prefixes) now enforced by workspace lint table"
author: ILexN
date: 2026-11-16
branch: main
id: l-8eac07e8
scope: repo
status: active
replaces: l-d96d218d
src: agent
touches:
  - Cargo.toml
  - backend/Cargo.toml
  - backend/src/calc/candidates.rs
  - backend/src/service/aia.rs
  - backend/src/service/bonds.rs
  - backend/src/service/deposits.rs
  - backend/src/service/months.rs
  - backend/src/service/settings.rs
  - backend/src/service/year_review.rs
  - backend/src/store/bonds.rs
---

## Decided
The editor's inspections (Non-ASCII characters, unnecessary path prefix) are mirrored in the toolchain via the root `[workspace.lints.rust]` table: `non_ascii_idents = "warn"` and `unused_qualifications = "warn"`. Warn, not deny — `cargo clippy -D warnings` is the enforcement point; deny would break `cargo test`/`cargo run` on a warning, stronger than the editor's signal. rust-analyzer reads the lints table through `cargo check`, so editor and CLI now report the same thing.

## Watch out
Both lints are ALLOW-by-default in modern rustc (1.98 here) — only the security subsets `mixed_script_confusables`/`confusable_idents` stay warn-by-default. So without the table, clippy is silent and these warnings exist only in the editor. Chinese in string literals/comments/docs is domain content — `non_ascii_idents` covers identifiers only. For path-prefix fixes, `cargo fix --lib -p wealth-backend --tests` applies all suggestions at once; run `cargo fmt` after, since shortened paths reflow wrapped lines.

In the code: root Cargo.toml `[workspace.lints.rust]`, `[lints] workspace = true` in backend/Cargo.toml. Archived changes: openspec/changes/archive/2026-11-16-ascii-identifiers/ and 2026-11-16-unused-qualifications/.
