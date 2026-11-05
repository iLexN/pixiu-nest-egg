# Proposal

## Why

`frontend/src/api.ts` hand-mirrored ~1,410 lines of `models/*.rs` type definitions — the hottest frontend file, drifting silently whenever a model changed without a matching edit. The backend already emits a utoipa OpenAPI doc, so the contract had a real seam with only one consumer (Scalar).

## What Changes

- New `backend/src/bin/export_openapi` dumps the spec from source to `frontend/openapi.json` (gitignored, regenerable).
- `pnpm gen:api` runs the bin + `openapi-typescript` → `frontend/src/api-types.ts` (committed; `vue-tsc` then catches drift as compile errors).
- `api.ts` keeps `request()`/`queryString`/the `api` object and 18 frontend-only types; 113 interfaces became aliases to generated schemas. 1,763 → ~450 lines.

## Capabilities

None — tooling refactor; `skip_specs: true`.

## Impact

`backend/src/bin/export_openapi.rs` (new), `frontend/package.json` (+ `openapi-typescript` devDep, `gen:api` script), `frontend/src/api.ts`, `frontend/src/api-types.ts` (new), `.gitignore`.
