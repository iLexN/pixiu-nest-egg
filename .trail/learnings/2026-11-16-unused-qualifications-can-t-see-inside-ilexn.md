---
title: "unused_qualifications can't see inside macros — editor still flags matches!/assert! paths"
author: ILexN
date: 2026-11-16
branch: main
id: l-2e998c3e
scope: repo
status: active
src: agent
touches:
  - Cargo.toml
  - backend/Cargo.toml
  - backend/src/calc/candidates.rs
  - backend/src/routes/forecast.rs
  - backend/src/service/aia.rs
  - backend/src/service/bonds.rs
  - backend/src/service/deposits.rs
  - backend/src/service/months.rs
  - backend/src/service/settings.rs
  - backend/src/service/year_review.rs
---

## Watch out
rustc's `unused_qualifications` (warn via our workspace table) does not inspect paths inside macro invocations like `matches!`/`assert!` — hygiene gap. The editor's inspection still flags them, so expect stragglers clippy can't see. Example: `Err(crate::store::StoreError::NotFound(_))` inside `matches!` in routes/forecast.rs tests — clippy clean, editor warned.

Fix = import the type and write it bare (`use crate::store::StoreError`), not shorten through a module alias: `store::StoreError` fails with E0603 because the `use` inside `store::forecast` is private. When auditing for remaining sites, grep for `crate::` inside matches!/assert! — inline paths that AREN'T imported elsewhere (e.g. `crate::calc::approx_eq`, `crate::models::BondStatus`) are legitimate and stay.

In the code: backend/src/routes/forecast.rs tests — `use crate::store::StoreError` + `Err(StoreError::NotFound(_))`.
