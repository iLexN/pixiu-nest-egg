# Tasks

## 1. Frontend

- [x] 1.1 `frontend/src/components/IbkrCard.vue`: new shared card — `ibkr: IbkrBlock` prop, `saved: [IbkrBlock]` emit; display table (2-pair rows) plus the 編輯 form (transfer delta + date, App 現值, HKD/USD 現金) saving through `api.updateIbkr`; local error display; `.ibkr-form`/`.ibkr-actions`/`.ibkr-preview` styles moved here.
- [x] 1.2 `frontend/src/views/SummaryView.vue`: replace the inline IBKR section with `<IbkrCard>` (`@saved="ibkr = $event"`); remove the moved state/handlers/styles.
- [x] 1.3 `frontend/src/views/OverviewView.vue`: replace the read-only IBKR card with `<IbkrCard :ibkr="data.ibkr" @saved="load" />`; drop the "於 股票 → 美股 → 總覽 編輯" note.

## 2. Docs

- [x] 2.1 `docs/overview.md`: the IBKR edit flow lives on both 總覽 and 美股 → 總覽 via the shared card.

## 3. Verify

- [x] 3.1 `cd frontend && pnpm build && pnpm exec vue-tsc --noEmit`.
- [x] 3.2 Restart the backend and confirm the 總覽 IBKR card matches the 美股 one and edits/saves from both pages.
