---
title: "Frontend contract types are generated: run pnpm gen:api after backend model changes, never hand-edit api.ts interfaces"
author: ILexN
date: 2026-11-05
branch: main
id: l-be71be3c
scope: repo
status: active
src: agent
touches:
  - frontend/package.json
  - frontend/pnpm-lock.yaml
  - frontend/src/api.ts
---

## Decided
- `frontend/src/api-types.ts` is generated from the utoipa OpenAPI doc: `cd frontend && pnpm gen:api` runs `cargo run -p wealth-backend --bin export_openapi -- openapi.json` then `openapi-typescript`. The `api` object, `request()` error handling, and 18 frontend-only types (filters, bespoke response shapes) stay hand-written in `api.ts`; the 113 model interfaces are `components['schemas']` aliases.
- `frontend/openapi.json` is gitignored — regenerated from source each run so it can't drift by commit. `api-types.ts` IS committed; `vue-tsc` after regen is the drift check.

## Watch out
- After editing `backend/src/models/*.rs` (or any `#[utoipa::path]` response type), run `pnpm gen:api` then `pnpm build` — do NOT hand-edit the alias section of `api.ts`.
- `typescript` stays pinned `~6.0.x` (vue-tsc shim) — unrelated to `openapi-typescript`, which is fine at 7.x.
