# Tasks

## 1. Pipeline

- [x] 1.1 `export_openapi` bin dumps the spec; `gen:api` script + `openapi-typescript` devDep; `openapi.json` gitignored; verify spec generates.
- [x] 1.2 Replace hand-written interfaces in `api.ts` with schema aliases; keep frontend-only types; verify `vue-tsc --noEmit` passes.
