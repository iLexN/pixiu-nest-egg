# Frontend API types generated from OpenAPI source

`frontend/src/api-types.ts` is generated — `pnpm gen:api` runs `export_openapi` (builds the utoipa doc over `connect_memory()`) then `openapi-typescript`. Never hand-edit it or the `components['schemas']` aliases in `api.ts`; the `api` object, `request()` error handling, and ~18 frontend-only types stay hand-written. `frontend/openapi.json` is gitignored — regenerated from source each run so codegen can't type against a stale server or a stale committed file. `vue-tsc --noEmit` after regen is the drift check.

### Considered options
- Generating from the running backend — rejected: silently types whatever server happens to be up, including an old binary.
- Generating operations/clients too — rejected: only `components.schemas` are generated; the hand-written `api` object owns request semantics.

Source: `openspec/changes/archive/2026-11-05-openapi-frontend-types`.
