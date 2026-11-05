# Design

## Decisions

- **Spec from source, not the server.** `export_openapi` builds the `OpenApi` doc over `connect_memory()` and writes it — codegen can't silently type against a stale running backend, and the JSON is gitignored so it can't drift by commit.
- **Types only.** `openapi-typescript` generates `components.schemas`; the hand-written `api` object stays (it owns `request()`'s error handling). Aliases preserve `api.ts`'s public names; 18 frontend-only types (filters, bespoke response shapes) stay hand-written.
- **Interface is the test surface.** `vue-tsc --noEmit` after regen is the drift check — any schema/view mismatch becomes a compile error.
