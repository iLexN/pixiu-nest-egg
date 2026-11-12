# Proposal

## Why

`grill-with-docs` (grilling + domain-modeling) was adopted today and produced `GLOSSARY.md` with only 2 terms and no `docs/adr/`. The project's full domain vocabulary and the rationale for its architectural decisions already exist — scattered across `openspec/specs/` and 65 archived `design.md` files — but agents can't consume them as a canonical glossary or decision record, so terms drift and settled decisions get re-litigated.

## What Changes

- Rewrite `GLOSSARY.md` in the domain-modeling skill's format (`**Term**: def` / `_Avoid_`), backfilled with the full domain vocabulary mined from `openspec/specs/` (~45 terms, grouped by domain area). The two existing entries ("Month book", "Live totals") are kept, tightened to domain-only definitions.
- Create `docs/adr/` with 9 numbered ADRs (0001–0009), each sourced from a recorded trade-off in an archived change's `design.md`: local single-user scope, f64 money, spreadsheet-parity quirks, derive-on-read, layered backend + two-tier write seam, MonthBook read path, history-on-write, 收訖/no-unreceive, generated API types.
- Docs-only change — no code, no API, no schema, no behavior change.

## Capabilities

### New Capabilities
(none — `skip_specs: true`)

### Modified Capabilities
(none)

## Impact

- `GLOSSARY.md` — rewritten and expanded
- `docs/adr/0001..0009-*.md` — new directory and files
- Consumed by `docs/agents/domain.md` readers (already references both files) and by future `grill-with-docs` / `openspec-explore` sessions as the vocabulary + decision baseline.
- No runtime impact.
