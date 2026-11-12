# Tasks

## 1. Glossary backfill

- [x] 1.1 Rewrite `GLOSSARY.md` in `GLOSSARY-FORMAT.md` format: `**Term**:` + 1–2 sentence definition + `_Avoid_` where synonyms exist; keep "Month book" and "Live totals" as terms but strip service-module references
- [x] 1.2 Add terms grouped under subheadings — Markets & trades (market 港股/美股, 股票代碼, 現價, 類別, 總買入成本, 加權平均買入單價, 平均單價, 未實現報酬率/未實現金額, 上月/最高, hidden stock, 持倉總覽); Dividends & receipts (派息, 收訖, 待定, estimated vs received amount, withholding tax); Month ledger (月結, 月初, 月尾, 月支出, 生活支出, 存, 利息, 娛樂支出, month-item categories, 調整, 即時, 重新擷取, 總數/流動資產, candidate/auto_key); 開心Pool (pool rate, income/spend/balance/input, irene_payable); Holdings instruments (定期 active/已到期未收/End, derived rate, 債券, coupon, MPF 帳戶結存, AIA policy terms, 家人); Cash & accounts (活期, bank/broker cash accounts, IBKR, 轉入, manual asset liquidity short/long, USD fx rate); Overview & planning (總覽, 半流動資金, 投資目標, 目標, 策略 tiers, 預測/locked chain/plan items/bill, Money Master); Year review (年結, 投資純利, 月薪增幅, year snapshot, blended rates)
- [x] 1.3 Verify every term traces to `openspec/specs/` or the two existing entries; no implementation details beyond inline code-identifier navigation aids

## 2. ADR backfill

- [x] 2.1 Create `docs/adr/` and write `0001-local-single-user-scope.md` — loopback-only, no auth, manual prices, no market data; workbook retired, no rebuild path (from `add-stock-trade-tracker`, `retire-workbook-import`)
- [x] 2.2 Write `0002-f64-money.md` — f64/REAL, never rounded, display rounding only (integer cents rejected)
- [x] 2.3 Write `0003-spreadsheet-parity-quirks.md` — HK/US fee derivation directions, 加權平均買入單價 ÷ shares bought, ÷12 averages, deposits at principal — reproduced verbatim, not "fixed"
- [x] 2.4 Write `0004-derive-on-read.md` — store inputs only; NULL totals derive live (stored aggregates rejected)
- [x] 2.5 Write `0005-layered-backend-seams.md` — routes→service→store + calc; two-tier write seam; routes never `pool.begin()` (from `layered-backend-architecture`, `service-seam-writes`)
- [x] 2.6 Write `0006-monthbook-read-path.md` — MonthBook is the only month-read path; live totals in `service::wealth` (lazy accessors / parallel public helpers rejected)
- [x] 2.7 Write `0007-history-on-write.md` — market/MPF history recorded on write, never on read; same-day replace (in-transaction recording rejected)
- [x] 2.8 Write `0008-shouqi-receipt-truth.md` — 收訖 is receipt truth; no unreceive; unreceived stays active 已到期未收 (from `remove-unreceive-actions`, `count-unreceived-deposits-as-active`)
- [x] 2.9 Write `0009-generated-api-types.md` — frontend `api-types.ts` generated via `pnpm gen:api` from OpenAPI source, never hand-edited (spec-from-source over running server)

## 3. Review & commit

- [x] 3.1 Grill the user where a "why" is unrecorded (e.g. whether a 10th ADR on cash-settlement semantics is warranted) rather than inventing rationale — user approved; `0010-cash-movement-semantics.md` written
- [x] 3.2 Self-review: ADR numbering sequential, each cites its source change, glossary definitions ≤2 sentences, `_Avoid_` lists real synonyms
- [x] 3.3 `git status` / `git diff` review; direct commit (docs-only, per AGENTS.md precedent — suggest a one-line message)
