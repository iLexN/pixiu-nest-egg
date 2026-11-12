# Design

## Context

- `GLOSSARY.md` exists (created 2026-11-05 during `month-book`) with 2 prose entries; `docs/adr/` does not exist. `docs/agents/domain.md` already instructs agents to read both.
- Skill formats: `.devin/skills/domain-modeling/GLOSSARY-FORMAT.md` (`**Term**: 1–2 sentence def` + `_Avoid_`, grouped under subheadings, domain terms only — no implementation detail) and `ADR-FORMAT.md` (title + 1–3 sentences, `docs/adr/NNNN-slug.md`, sequential numbering, three-criteria test: hard to reverse / surprising / real trade-off).
- Sources: 17 capability specs (`openspec/specs/*/spec.md`) carry the normative vocabulary; 65 archived `design.md` files carry recorded trade-offs ("Alternative rejected" notes). No new decisions are invented — this change transcribes existing ones.

## Goals / Non-Goals

**Goals:**
- Glossary covers every project-specific domain term (~45), grouped by domain area, in the documented format.
- Each ADR cites the archived change it came from and states the rejected alternative where recorded.
- Terms/ADRs remain useful to the `grill-with-docs → openspec-explore → propose → apply → archive` flow: canonical names for spec writing, settled "why"s to stop re-litigation.

**Non-Goals:**
- No code, API, schema, or spec changes.
- Not a complete decision history — only decisions passing all three ADR criteria. The archive itself remains the full history.
- No multi-context split (`GLOSSARY-MAP.md`) — single context.

## Decisions

### Glossary

- **Format per `GLOSSARY-FORMAT.md`.** Existing prose entries ("Month book", "Live totals") are kept as terms but rewritten to the `**Term**:` form and stripped of service-module references — a glossary stays stable as code moves.
- **Headwords are the canonical names the UI/specs use** — Chinese labels where those are primary (收訖, 定期, 月結, 開心Pool) with the code identifier noted inline (`received_at`) where it aids navigation.
- **Grouping subheadings** follow capability clusters: Markets & trades; Dividends & receipts; Month ledger; 開心Pool; Holdings instruments; Cash & accounts; Overview & planning; Year review.

### ADRs (0001–0009)

| # | Decision | Source |
|---|----------|--------|
| 0001 | Local single-user scope: loopback-only, no auth, manual prices, no market-data; workbook retired with no rebuild path | `add-stock-trade-tracker`, `retire-workbook-import` |
| 0002 | Money as f64/REAL, never rounded; display rounding only | `add-stock-trade-tracker` (integer cents rejected) |
| 0003 | Spreadsheet-parity quirks reproduced verbatim (HK fee = total − qty×price; US total = qty×price + fee; 加權平均買入單價 ÷ shares bought; ÷12 averages; deposits at principal) | `add-stock-trade-tracker`, `add-month-stat`, `add-yearly-summary` |
| 0004 | Derive on read, store inputs only; NULL totals = live | `add-stock-trade-tracker`, `add-month-live-totals` |
| 0005 | Horizontal layers routes→service→store + calc; two-tier write seam (store owns single-domain txs; service owns domain workflows; routes never `pool.begin()`) | `layered-backend-architecture`, `service-seam-writes` |
| 0006 | MonthBook is the only month-read path; live totals in `service::wealth` | `month-book` |
| 0007 | Market/MPF history recorded on write, never on read; same-day replace | `record-history-on-write` |
| 0008 | 收訖 is the receipt point of truth; no unreceive action; unreceived stays active (已到期未收) | `remove-unreceive-actions`, `count-unreceived-deposits-as-active` |
| 0009 | Frontend API types generated from OpenAPI source (`pnpm gen:api`), never hand-edited | `openapi-frontend-types` |

- **Rejected as ADRs:** OpenSpec-as-workflow (self-evident from repo), sqlx/Vue stack choice (folded into 0005 context if mentioned), per-endpoint cash-settlement rules (already specified behavior + trail learnings — re-evaluate if a reader could mistake them).
- **Inline grilling:** if a "why" isn't recorded and can't be inferred from the design doc, ask the user rather than inventing rationale.

## Risks / Trade-offs

- [Glossary ~45 terms is large] → Mitigated by grouped subheadings; format keeps entries scannable. It is a one-time catch-up — new terms accrete lazily per change.
- [ADRs duplicate design.md content] → Intended: ADRs are the fast lookup surface; design.md stays the deep record. Each ADR names its source change.
- [Terms mention code identifiers] → Allowed only as inline navigation aids; definitions stay domain-level.
