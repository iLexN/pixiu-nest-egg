# Design

## Context

Rustc's `non_ascii_idents` lint is allow-by-default since rustc relaxed it
(the security-relevant subsets — `mixed_script_confusables`,
`confusable_idents` — remain warn-by-default and are untouched). The editor's
"Non-ASCII characters" inspection still flags non-ASCII identifiers, so
toolchain and editor disagreed. See proposal.md for the rename list.

## Goals / Non-Goals

**Goals:**

- ASCII-only identifiers across the backend.
- `cargo check`/`cargo clippy` warn on non-ASCII identifiers; `-D warnings`
  fails verification.
- rust-analyzer reports the same warning, so editor and CLI agree.

**Non-Goals:**

- No change to Chinese in string literals, comments, or docs — domain
  content, not identifiers.
- No enforcement of confusable/homoglyph rules beyond rustc's defaults.
- No frontend linting (out of scope; the flagged code is Rust test code).

## Decisions

- **`[workspace.lints.rust]` in the root `Cargo.toml`** over a
  `.cargo/config.toml` rustflags entry: the lints table is the edition-2024
  mechanism, travels with the package, and rust-analyzer honors it through
  `cargo check`. Alternative considered: `#[warn(non_ascii_idents)]` crate
  attribute in `lib.rs` — rejected because a lint table keeps style config
  out of source files.
- **`warn` over `deny`**: mirrors the editor's warning semantics; the repo's
  documented `cargo clippy --all-targets -- -D warnings` already escalates
  warn-level lints to errors, so `deny` would only differ by breaking
  `cargo test`/`cargo run` on a warning — stronger than the editor's signal.
- **ASCII names carry the literal's meaning, not the literal**: e.g.
  `..._falls_back_to_the_default_label` names what the fallback is; the
  test's `assert_eq!(label, "定期")` and a doc comment keep the stored
  string pinned.

## Risks / Trade-offs

- [Renames lose the literal label from the name] → doc comment + in-test
  assertion keep `定期`/`IBKR 轉入` pinned verbatim.
- [Future non-ASCII identifier breaks `-D warnings` verification] →
  intended: that is the enforcement point.
