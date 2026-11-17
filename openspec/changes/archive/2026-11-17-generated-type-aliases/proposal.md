# Proposal

## Why

`frontend/src/api.ts` hand-declares thirteen response and body interfaces that the OpenAPI generator already emits into `api-types.ts` — `DepositSummary`, `BondSummary`, `DividendSummary`, `MpfOverview`, `AiaSummary`, `AiaPolicyWithEvents`, `MonthDetail`, `MonthSummary`, `YearlySummary`, `SummaryStock`, `YearSnapshotRow`, `ReceiveDepositBody`, `ReceiveBondBody`, `ReceiveFamilyDepositBody`. A field-by-field diff shows every one is an exact respelling of its generated schema (`DepositSummaryResponse`, `StockRow`, `PolicyWithEvents`, …). They are migration residue: the backend response structs only became named `ToSchema` types in `page-read-models-in-service`, and the hand declarations predate that.

This is a second source of truth on the one seam where a lie compiles clean: `request<T>` returns `body as T` unchecked, so when a backend response shape changes, the hand-written type silently stays behind while `vue-tsc` stays green. `dismissed-suggestions-visible` already required a manual patch to `MonthDetail` to keep the copy in sync.

## What Changes

- Each hand interface becomes a one-line alias to its generated schema — `export type DepositSummary = S['DepositSummaryResponse']` — preserving the current frontend names so no call site changes.
- `YearSnapshotRow` aliases `S['YearSnapshot']`, which already carries `market` and `year`.
- The four filter bags (`TradeFilters`, `DepositFilters`, `FamilyDepositFilters`, `DividendFilters`) stay hand-written: they are frontend-only query shapes with no generated schema.
- AGENTS.md gains one line codifying the rule: server response/body shapes in `api.ts` are always `components['schemas']` aliases; only frontend-only types stay hand-written.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None — pure refactor; no spec-level behavior changes.

## Impact

- `frontend/src/api.ts` — the hand interface block shrinks to aliases; the `api` object, `request()`, `queryString()`, `ApiError`, `errorMessage`, `toApiError`, and the filter bags are unchanged.
- `AGENTS.md` — one line under "Frontend API types".
- No API, schema, or behavior change. Verified by `pnpm gen:api`, `pnpm build`, and `pnpm exec vue-tsc --noEmit` — the last is the real check, since it re-types every call site against the generated shape.
