# Proposal

## Why

The 年結 → 資產 group labels its stock rows `股票`, but the figures are HK-only by design (US enters only through `invested`/`invested_adjustment`). The label misleads — it reads as HK + US combined. The spec text uses the same `股票` shorthand and must be synced so it stays authoritative.

## What Changes

- Rename the five asset-row labels on the 年結 page: `股票 成本`/`股票 派息`/`股票 率`/`股票 市值`/`股票 市值率` → `港股 …`.
- Update the `year-in-review` spec wording so the asset class is named `港股` everywhere the HK-only figures are meant.
- No API, schema, or calculation changes — field names (`stock_cost`, `stock_dividends`, …) and values are untouched.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `year-in-review`: the per-asset-class block's stock figures are labeled and described as `港股` (HK-only), not `股票`.

## Impact

- `frontend/src/views/YearReviewView.vue` — the 資產 section label strings.
- `backend/src/calc/year_review.rs` — doc comment wording only.
- `docs/overview.md` — year-review asset-group description wording.
- `openspec/specs/year-in-review/spec.md` — wording synced on archive.
