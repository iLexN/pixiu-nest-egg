# Proposal

## Why

The user wants to feed their wealth data to an external AI for analysis. Today the only way out of `wealth.db` is reading the UI pages one by one. A single curated JSON snapshot — liquid assets, cash-flow momentum, the seven-month forecast with rigid expenses, and per-stock holdings — gives an AI everything it needs in one download.

## What Changes

- New `GET /api/export/analysis` endpoint that returns a derived-on-read JSON snapshot with `Content-Disposition: attachment; filename="wealth-export-<today>.json"`. No new stored data and no schema change; every section is composed from existing read models (`service::overview`, `service::forecast`, `service::summary`, `MonthBook` averages).
- New `service::export` module that composes those read models into one serializable `AnalysisExport` struct.
- A new `Export` link in the 總覽 group's sub-tab row (rendered after the `HappyPool` tab) that downloads the snapshot.

## Capabilities

### New Capabilities

- `analysis-export`: the analysis snapshot endpoint — response sections, field semantics (rigid expenses from forecast plan items plus the synthesized quarterly bill, per-holding dividend yield, goal-3 liquid-assets target), and the attachment download contract.

### Modified Capabilities

- `app-navigation`: the 總覽 group's sub-tab row gains an `Export` link after the `HappyPool` tab that downloads the analysis export.

## Impact

- Backend: new `backend/src/service/export.rs` and `backend/src/routes/export.rs`; `summarize_market` visibility widened in `service::summary`; route registered in `api_router`; the OpenAPI operation-list test updated.
- Frontend: `frontend/src/App.vue` — a `links` entry on the `overview` nav group.
- No database migrations, no frontend state changes, no new dependencies.
