# Tasks

## 1. Alias server shapes to generated schemas

- [x] 1.1 In `frontend/src/api.ts`, replace the hand interface block with one-line `components['schemas']` aliases keeping the current names: `SummaryStock = S['StockRow']`, `YearSnapshotRow = S['YearSnapshot']`, `YearlySummary = S['YearlyResponse']`, `ReceiveDepositBody = S['ReceiveDeposit']`, `DepositSummary = S['DepositSummaryResponse']`, `ReceiveFamilyDepositBody = S['ReceiveFamilyDeposit']`, `DividendSummary = S['DividendSummaryResponse']`, `ReceiveBondBody = S['ReceiveBond']`, `BondSummary = S['BondSummaryResponse']`, `MpfOverview = S['MpfResponse']`, `AiaPolicyWithEvents = S['PolicyWithEvents']`, `AiaSummary = S['AiaSummaryResponse']`, `MonthDetail = S['MonthDetailResponse']`, `MonthSummary = S['MonthSummaryResponse']`. Preserve each doc comment where the hand version carries one not covered by the generated `@description`.
- [x] 1.2 Leave `TradeFilters`, `DepositFilters`, `FamilyDepositFilters`, `DividendFilters` hand-written — frontend-only query shapes with no generated schema.

## 2. Codify the rule

- [x] 2.1 Add one line to `AGENTS.md` under "Frontend API types": server response/body shapes in `api.ts` are always `components['schemas']` aliases; only frontend-only types (filters, `YearSnapshotRow`-style conveniences) stay hand-written.

## 3. Verify

- [x] 3.1 `cd frontend && pnpm gen:api && pnpm build && pnpm exec vue-tsc --noEmit` — vue-tsc is the drift check: it re-types every view against the generated shape. Fix any call site that was relying on a hand field the server does not send.
