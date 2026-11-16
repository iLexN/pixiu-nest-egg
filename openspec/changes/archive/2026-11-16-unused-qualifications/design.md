# Design

## Context

`unused_qualifications` is allow-by-default in rustc for the same reason
`non_ascii_idents` was — see the archived
`2026-11-16-ascii-identifiers` design for the precedent: warn-level in
`[workspace.lints.rust]`, editor/toolchain parity, `-D warnings` as the
enforcement point. Same decisions apply unchanged.

## Decisions

- **`warn` in the existing workspace table** — one more key beside
  `non_ascii_idents`, same rationale (mirrors the editor; `-D warnings`
  escalates).
- **`cargo fix --lib -p wealth-backend --tests` for the rewrites** — the
  suggestions are machine-applicable (drop path segments only when the tail
  resolves through an existing `use`); review the diff rather than retype
  ten identical edits. Followed by `cargo fmt`, since shortening paths
  reflows wrapped lines.

## Risks / Trade-offs

- [A removed prefix could shadow differently after a future `use` change] →
  rustc re-checks resolution on every build; the lint would flag the new
  situation rather than silently resolve elsewhere.
