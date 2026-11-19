---
title: trail check conflicts on moved or re-shaped code are usually fuzzy false positives
author: ILexN
date: 2026-11-19
branch: main
id: l-9557040c
scope: repo
status: active
src: agent
touches:
  - backend/src/service/bonds.rs
  - backend/src/service/months.rs
---

## Decided
- When `trail check` flags a hard conflict on a line a refactor moved or merely re-exported, verify the flagged code against the learning's `## Decided` section before touching anything. If the flagged code IS the decided shape, the hit is a token-level false positive — the sanctioned fix is a one-line justification in the commit message or PR description, not an edit.
- Two hard conflicts on the Nov 19 push were both this: `service/bonds.rs` doc comments co-locating "matured"/"received" (the API status stayed MATURED, exactly what the learning decided) and `service/months.rs` re-exporting `live_from` + `pub(crate) stat_rows_no_live` (exactly the single-book design the learning decided — `live_from` is a NaiveDate helper, not a stat-row loader).

## Tried and ruled out
- Rewording doc comments or renaming `live_from` to break the token match: contorting correct, spec'd code to silence a fuzzy matcher costs more than writing the justification.

## Watch out
- `trail check` only diffs the branch vs origin's default branch, so conflicts on already-merged work vanish after push ("nothing to check", exit 0). Reproduce an old report with `trail check --base <pre-push-sha>`.
- Only `conflict` verdicts (✗) fail the check; `lead` verdicts (●) are advisory and can't fail it.

In the code: backend/src/service/bonds.rs:44-55 (BondSummaryResponse docs), backend/src/service/months.rs:9-12 (re-exports), .trail/learnings/2026-11-05-monthbook-*, .trail/learnings/2026-11-12-bonds-*.
