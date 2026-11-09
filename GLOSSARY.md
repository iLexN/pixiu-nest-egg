# Glossary

Domain terms as this codebase uses them — the local wealth-report app that
replaced `財富分析報告.xlsx`. Code, specs, docs, and proposals should use these
names rather than synonyms. Chinese UI labels are canonical where the app
shows them; code identifiers are noted inline where they aid navigation.

## Markets & trades

**Market**: the stock universe a record belongs to — exactly one of `HK` (港股)
or `US` (美股). Codes are unique within a market, not across markets.

**股票代碼 (stock code)**: a stock's identifier within its market — the Chinese
name for HK stocks, the ticker for US stocks.
_Avoid_: symbol, ticker alone, name

**現價 (current price)**: a stock's manually maintained latest price; no
market-data feed exists. Stocks without 現價 are excluded from market-value
totals and listed in `totals.excluded_codes`.
_Avoid_: price, last price, quote

**類別**: a trade's side — `BUY` or `SELL`.

**總買入成本 (buy cost)**: Σ BUY `total` for a stock; a SELL never reduces it.

**加權平均買入單價 (weighted average buy price)**: `總買入成本 ÷ Σ BUY 股數` —
divides by shares *bought*, not shares held (spreadsheet-compatible quirk,
deliberate).
_Avoid_: average cost, cost basis, avg buy

**平均單價**: a trade's per-share price including fee — `total ÷ 股數`; empty
for zero-share rows.

**未實現報酬率 / 未實現金額**: a market's unrealized pair —
`(market_value − buy_cost_priced) ÷ buy_cost_priced` and the difference itself.

**上月 / 最高**: a market summary's last-month and all-time-max unrealized
figure pairs, derived on read from market history, seed marks, and current.

**Seed mark**: a per-market stored floor for 最高 — a workbook-era high the
running max can never drop below.

**Hidden stock**: a stock flagged inactive — omitted from 持倉總覽 rows but
still counted in totals and sector rollups (display-only).

**持倉總覽**: the per-market portfolio summary view/table.

## Dividends & receipts

**派息**: a dividend record — `estimated_amount` while pending,
`received_amount` once paid.

**收訖 (received)**: the user-confirmed receipt mark on a deposit, coupon,
dividend, or family deposit (`received_at`/`received_amount`). It is the
single point of truth that flips activity, credits cash, and feeds month 利息.
_Avoid_: confirmed, collected, completed

**待定**: a pending coupon or dividend with no fixed amount — previews in
breakdowns but is never suggested as a month item.
_Avoid_: pending, unknown

**Withholding tax**: US dividend tax withheld at source (`withholding_tax`);
`received_amount` stores the net (gross − tax).

## Month ledger (月結)

**月結**: the month-by-month cash ledger — spending falls out of bank-balance
differences; individual transactions are never logged.
_Avoid_: monthly statement, transaction log

**月初 (start_cash)**: 月初出糧後 — the 活期 bank total measured right after
payday salary lands.

**月尾 (end_cash)**: 月尾出糧前 — the bank total just before the next payday;
derived as next month's 月初 − this month's salary unless `end_cash_override`
is stored.

**月支出 (month_spend)**: `月初 + Σ 調整 items − 月尾`.

**生活支出 (living_spend)**: `月支出 − Σ extra_spend − Σ excluded 娛樂 items` —
the everyday living cost.

**存 (saved)**: `salary − 月支出 + Σ income items`.

**利息 (interest)**: a month's interest figure — Σ *received* auto components
(deposit interest, bond coupons, HK dividends; US dividends stay inside IBKR
and are never counted) + Σ `interest` items.

**娛樂支出 (entertainment)**: Σ `entertainment` items — fun spending counted by
the 開心Pool.

**Month item**: a labeled line on a month in one of five categories:
`adjustment`, `extra_spend`, `income`, `entertainment`, `interest`.

**調整 (adjustment item)**: a bank in/out that must not count as spending —
定期 start/end, stock buys/sells, received dividends/coupons, IBKR 轉入.
_Avoid_: transfer, non-spend

**extra_spend item**: real spending excluded from 生活支出 — a TV, an AIA
premium, tax, doctor.

**即時 (live)**: a month whose `total_assets`/`liquid_assets` is NULL — every
read derives the figure from the current portfolio; marked 即時 until frozen.
_Avoid_: realtime, computed

**重新擷取 (recapture)**: the freeze step — snapshots live totals and 月初
(the live 活期 sum) into the month row; clearing the fields returns it to 即時.
_Avoid_: refresh, snapshot

**總數 / 流動資產**: the month's stored-or-live asset totals (`total_assets`,
`liquid_assets` — the sheet's `Overview!B1`/`H1`).

**Candidate**: an event-derived month-item suggestion carrying a stable
`auto_key` — accept stores it as an item, dismiss records the key so it never
reappears. Suggested only for the current month and later.
_Avoid_: suggestion, auto-item

**Month book**: the assembled 月結 ledger for reads — all stored month rows,
their items, the derived stat rows, live totals, the live tail, pool rates,
and HK sold P/L. Every month-derived read opens the book (`MonthBook::open`)
and asks it questions — `months`, `month`, `year_summaries`,
`running_averages`, `trailing_averages`, `pool_balance`, `pool_rate` — never
its raw parts.
_Avoid_: pipeline, month load

**Live totals**: the 總資產/流動資產 aggregate computed from current data
(HK + US market value, active 定期/債券 principal, AIA, MPF, manual assets,
活期, IBKR cash, minus/plus 開心Pool) — "live" because it reflects now, not a
stored figure. Month NULL totals fill from it at/after the current month.
_Avoid_: totals, overview totals

## 開心Pool

**開心Pool (HappyPool)**: the shared fun-money pool — fed by year's interest ×
pool rate plus contributions, spent on 娛樂支出.

**Pool rate (娛樂 rate)**: one stored rate per year (`pool_rate`); a year with
none falls back to the latest earlier year's rate.

**Pool income**: `Σ interest × pool rate` for the year — iLex's contribution
(`ilex_input`).

**Pool input (Irene + 開心 Pool)**: manual contributions to the pool
(`pool_input`, `irene_input`).

**Pool balance**: chained yearly — `prev closing + pool income −
Σ 娛樂支出 + Σ pool_input`.

**Irene payable**: `ilex_input ÷ 2 − irene_input` — what Irene still owes to
keep the split even; may be negative.

## Holdings instruments

**定期**: a time deposit — label, bank, principal, rate, interest, start/end
dates; active until 收訖.

**已到期未收**: an unreceived deposit past its `end_date` — still active in
aggregates; activity is `!received`, not the date.
_Avoid_: matured, overdue

**End**: a deposit's terminal state — set by 收訖, never by the end date alone.

**Derived rate**: a deposit's annual rate computed on read —
`interest ÷ principal × 365 ÷ days` — only when no rate is stored; flagged
as derived, never persisted.

**債券**: a bond — principal, coupons, maturity.

**Coupon (票面息)**: a bond's scheduled interest payment with `pay_date`;
待定 until its rate is fixed.

**MPF account**: a Mandatory Provident Fund account — trustee, contributions,
帳戶結存 (balance).

**AIA policy**: an AIA insurance/annuity line — `premium_usd`, `value_usd`,
`withdrew_usd`, `remaining_years`, `next_pay_date`.

**繳費 / 提取**: AIA payment and withdrawal events — each atomically updates
the policy and stores the prior values, so deleting the event undoes it.

**display_value**: the AIA account-display figure — only `in_account` policies
count.

**excluded / in_account**: AIA row flags — `excluded` drops the row from
portfolio totals, `in_account=false` drops it from `display_value` only.

**家人 deposit**: a family member's deposit record — record-only, no cash or
month-item side effects.

## Cash & accounts

**活期**: demand-account money — the `bank` cash accounts behind 月初
(HKD-only).
_Avoid_: checking, savings

**Cash account**: a stored money balance — role `bank` (HKD, e.g. HS/渣打) or
`broker` (IBKR HKD/USD).

**IBKR**: the Interactive Brokers account — US stock value + HKD/USD broker
cash + the 轉入 log.

**轉入**: an IBKR transfer — moves HKD from a bank cash account into the IBKR
broker account atomically; log-only rows (no `from_account_id`) never suggest
month items.
_Avoid_: deposit, funding

**Manual asset (資產)**: a non-cash stored balance (e.g. Irene, HS人壽) with
`liquidity` `short`/`long` — drives 策略 tier placement.
_Avoid_: asset row, misc asset

**USD fx rate**: the stored USD→HKD rate; USD-denominated figures are absent
while it is unset.

## Overview & planning

**總覽**: the overview page — headline totals, asset allocation, IBKR block,
averages, 投資目標, 目標, 策略, 預測, Money Master.

**半流動資金 (semi-liquid funds)**: assets liquid within days — tracked
against a configurable target ratio.

**投資目標 (investment target)**: the savings/investment goal formula —
target/remain/growth derived from income, 月薪增幅, and running averages.

**目標 (goals)**: the milestone cards — formula footnotes and reached flags.

**策略 (liquidity tiers)**: the 短期可取回 / 長期可取回 breakdown — manual
assets placed by their `liquidity` field.
_Avoid_: strategy

**預測 (forecast)**: the month-window cash projection grid — salary/spend,
deposit finish, bill, interest, plan rows, trade lines, locked chain.
_Avoid_: projection

**Locked chain**: the forecast's committed 定期+SC anchor path — events that
must happen, so later columns chain from them.

**Forecast cell**: one (month, plan row) square of the 預測 grid as the
backend returns it — the month's plan items of that kind, its read-only
placement/trade/IBKR lines, and the `figure` the grid shows. Routing a line
to its row and summing the figure happen in the backend only.
_Avoid_: cell sum, bucket

**Plan item**: a user-planned forecast cell (e.g. a deposit placement or
轉為定期 conversion) that edits the projection without moving money yet.

**Bill**: the forecast's recurring-bill amount setting.

**Money Master**: the bank-app savings challenge block — counters
(`start_date`/`target_months`/`target_amount`/`month_now`) copied from the
bank app, not derivations of tracked data.

## Year review (年結)

**年結**: the per-year cross-domain review — ledger aggregates, investment
summary, per-asset-class returns with blended rates and YoY deltas.
_Avoid_: yearly summary (that's the per-market stock sheet), annual review

**投資純利 (net investment)**: `利息 + HK sold_pl` — US sold P/L never feeds it.

**月薪增幅 (raise)**: the year's salary increment feeding 投資目標 — stored
override, else `max(0, lastSalary(Y) − lastSalary(Y−1))`.

**Year snapshot**: a year's frozen 年末總成本/總市值/`sold_pl`
(`year_snapshots`); absent → live derivation.

**Blended rate**: the year review's cross-class yields — `income_cost_rate`,
`total_value_rate`, `income_value_rate`; USD figures blend at the fx rate and
are absent while it is unset.
