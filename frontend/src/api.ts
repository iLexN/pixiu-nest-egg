import type { components } from './api-types'

type S = components['schemas']

export interface SummaryStock extends Stock {
  shares_held: number
  total_buy_cost: number
  weighted_avg_buy_price: number
  current_price: number | null
  market_value: number | null
  unrealized_amount: number | null
  unrealized_return: number | null
  dividends_received: number
  dividend_return: number | null
  net_invested: number
  net_diluted_price: number | null
  real_total_return: number | null
  trade_count: number
}

/** The PATCH/freeze response: the snapshot row keyed by market + year. */
export interface YearSnapshotRow extends YearSnapshot {
  market: Market
  year: number
}

export interface YearlySummary {
  market: Market
  today: string
  years: YearRow[]
}

export interface TradeFilters {
  market?: Market
  stock_id?: number
  code?: string
  from?: string
  to?: string
  order?: 'asc' | 'desc'
}

export interface ReceiveDepositBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Corrects the stored interest to the amount actually received. */
  interest?: number
  /** Bank principal+interest into a cash account. Defaults to true. */
  bank_in?: boolean
  /** The receipt cash_accounts id; falls back to the HKD default. */
  account_id?: number
}

export interface DepositSummary {
  today: string
  upcoming: Deposit[]
  active_totals: { principal: number; interest: number; total: number }
  months: ActiveMonthBucket[]
  banks: BankRollup[]
  years: YearRollup[]
  history_years: number[]
}

export interface DepositFilters {
  status?: 'active' | 'ended'
  year?: number
  order?: 'asc' | 'desc'
}

export interface ReceiveFamilyDepositBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Corrects the stored interest to the amount actually received. */
  interest?: number
}

export interface FamilyDepositFilters {
  status?: 'active' | 'ended'
  holder?: string
  year?: number
  order?: 'asc' | 'desc'
}

export interface DividendSummary {
  today: string
  pending: Dividend[]
  years: DividendYearRollup[]
  history_years: number[]
}

export interface DividendFilters {
  market?: Market
  stock_id?: number
  status?: 'pending' | 'received'
  year?: number
  order?: 'asc' | 'desc'
}

export interface ReceiveBondBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Bank the returned principal into a cash account. Defaults to true. */
  bank_in?: boolean
  /** The receipt cash_accounts id; falls back to the HKD default. */
  account_id?: number
  /** Amount credited; defaults to the bond's principal. */
  credit_amount?: number
}

export interface BondSummary {
  today: string
  totals: { active_principal: number }
  active: BondWithCoupons[]
  matured: BondWithCoupons[]
  upcoming_coupons: UpcomingCoupon[]
}

export interface MpfOverview {
  today: string
  accounts: MpfAccount[]
  totals: MpfTotals
  note: string | null
  history: MpfHistoryRow[]
}

export interface AiaPolicyWithEvents extends AiaPolicy {
  events: AiaEvent[]
}

export interface AiaSummary {
  today: string
  /** Manual USD→HKD rate; HKD figures are null while unset. */
  rate: number | null
  /** Earliest premium-due date still ahead. */
  next_premium_due: string | null
  /** Section free-text note. */
  note: string | null
  totals: AiaTotals
  policies: AiaPolicyWithEvents[]
}

export interface MonthDetail {
  month: MonthStat
  items: MonthItem[]
  suggestions: MonthSuggestion[]
  interest_auto: InterestComponent[]
}

export interface MonthSummary {
  years: MonthYearSummary[]
  running: MonthRunningAverages
  pool_balance: number
  pool_totals: PoolTotals
  pool_rate: number | null
  pool_rate_year: number
}

export type Market = S['Market']
export type TradeType = S['TradeType']
export type InputMode = S['InputMode']
export type Currency = S['Currency']
export type CashRole = S['CashRole']
export type CashAccount = S['CashAccount']
export type FxRate = S['FxRate']
export type FxRateEntry = S['FxRateEntry']
export type NewCashAccount = S['NewCashAccount']
export type CashAccountPatch = S['CashAccountPatch']
export type FieldError = S['FieldError']
export type Stock = S['Stock']
export type Trade = S['Trade']
export type SectorRollup = S['SectorRollup']
export type MarketTotals = S['MarketTotals']
export type MarketFigures = S['MarketFigures']
export type SummaryResponse = S['SummaryResponse']
export type YearSnapshot = S['YearSnapshot']
export type YearRow = S['YearRow']
export type YearlyPatch = S['YearlyPatch']
export type YearReviewRecord = S['YearReviewRecord']
export type YearReviewLedger = S['YearReviewLedger']
export type YearReviewInvestment = S['YearReviewInvestment']
export type YearReviewAssets = S['YearReviewAssets']
export type YearReviewRow = S['YearReviewRow']
export type YearReviewResponse = S['YearReviewResponse']
export type YearReviewPatch = S['YearReviewPatch']
export type NewStock = S['NewStock']
export type NewTrade = S['NewTrade']
export type PriceUpdate = S['PriceUpdate']
export type InvalidPriceEntry = S['InvalidPriceEntry']
export type PriceReport = S['PriceReport']
export type DepositStatus = S['DepositStatus']
export type Deposit = S['Deposit']
export type NewDeposit = S['NewDeposit']
export type ActiveMonthBucket = S['ActiveMonthBucket']
export type BankRollup = S['BankRollup']
export type YearMonthRow = S['YearMonthRow']
export type YearRollup = S['YearRollup']
export type FamilyDeposit = S['FamilyDeposit']
export type NewFamilyDeposit = S['NewFamilyDeposit']
export type FamilyDepositPatch = S['FamilyDepositPatch']
export type FamilyHolderSummary = S['FamilyHolderSummary']
export type FamilyDepositSummary = S['FamilyDepositSummary']
export type DividendStatus = S['DividendStatus']
export type Dividend = S['Dividend']
export type NewDividend = S['NewDividend']
export type DividendPatch = S['DividendPatch']
export type DividendStockTotal = S['DividendStockTotal']
export type DividendYearRollup = S['DividendYearRollup']
export type BondStatus = S['BondStatus']
export type CouponStatus = S['CouponStatus']
export type Bond = S['Bond']
export type NewBond = S['NewBond']
export type BondCoupon = S['BondCoupon']
export type NewBondCoupon = S['NewBondCoupon']
export type BondCouponPatch = S['BondCouponPatch']
export type BondWithCoupons = S['BondWithCoupons']
export type UpcomingCoupon = S['UpcomingCoupon']
export type MpfFigures = S['MpfFigures']
export type MpfAccount = S['MpfAccount']
export type MpfHistoryRow = S['MpfHistoryRow']
export type MpfTotals = S['MpfTotals']
export type NewMpfAccount = S['NewMpfAccount']
export type MpfAccountPatch = S['MpfAccountPatch']
export type AiaEventKind = S['AiaEventKind']
export type AiaPolicy = S['AiaPolicy']
export type NewAiaPolicy = S['NewAiaPolicy']
export type AiaPolicyPatch = S['AiaPolicyPatch']
export type AiaEvent = S['AiaEvent']
export type NewAiaEvent = S['NewAiaEvent']
export type AiaTotals = S['AiaTotals']
export type MonthItemCategory = S['MonthItemCategory']
export type ManualAssetLiquidity = S['ManualAssetLiquidity']
export type MonthStat = S['MonthStat']
export type MonthItem = S['MonthItem']
export type MonthSuggestion = S['MonthSuggestion']
export type InterestComponent = S['InterestComponent']
export type MonthYearSummary = S['MonthYearSummary']
export type MonthRunningAverages = S['MonthRunningAverages']
export type PoolTotals = S['PoolTotals']
export type AppSettings = S['AppSettings']
export type MonthStatPatch = S['MonthStatPatch']
export type NewMonthItem = S['NewMonthItem']
export type MonthItemPatch = S['MonthItemPatch']
export type AppSettingsPatch = S['AppSettingsPatch']
export type ForecastItemKind = S['ForecastItemKind']
export type ForecastItem = S['ForecastItem']
export type NewForecastItem = S['NewForecastItem']
export type ForecastItemPatch = S['ForecastItemPatch']
export type ForecastPlacement = S['ForecastPlacement']
export type ForecastTrade = S['ForecastTrade']
export type ForecastCell = S['ForecastCell']
export type ForecastMonth = S['ForecastMonth']
export type ForecastResponse = S['ForecastResponse']
export type ManualAsset = S['ManualAsset']
export type NewManualAsset = S['NewManualAsset']
export type ManualAssetPatch = S['ManualAssetPatch']
export type IbkrBlock = S['IbkrBlock']
export type IbkrPatch = S['IbkrPatch']
export type ForecastIbkrTransfer = S['ForecastIbkrTransfer']
export type IbkrTransfer = S['IbkrTransfer']
export type NewIbkrTransfer = S['NewIbkrTransfer']
export type OverviewAssetRow = S['OverviewAssetRow']
export type SemiLiquid = S['SemiLiquid']
export type TwelveMonthAverages = S['TwelveMonthAverages']
export type InvestTargetRow = S['InvestTargetRow']
export type InvestTargets = S['InvestTargets']
export type LiquidityTiers = S['LiquidityTiers']
export type MoneyMaster = S['MoneyMaster']
export type GoalMilestone = S['GoalMilestone']
export type FormulaGoal = S['FormulaGoal']
export type OverviewGoals = S['OverviewGoals']
export type OverviewResponse = S['OverviewResponse']

export class ApiError extends Error {
  status: number
  fields: FieldError[]

  constructor(status: number, message: string, fields: FieldError[]) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.fields = fields
  }

  fieldMessage(field: string): string | undefined {
    return this.fields.find((f) => f.field === field)?.message
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api${path}`, {
    headers: init?.body ? { 'content-type': 'application/json' } : undefined,
    ...init,
  })
  if (response.status === 204) {
    return undefined as T
  }
  const text = await response.text()
  // The proxy can answer with an HTML error page when the backend is down;
  // don't let a parse failure mask the real HTTP status.
  let body: unknown = null
  try {
    body = text ? JSON.parse(text) : null
  } catch {
    /* non-JSON body */
  }
  if (!response.ok) {
    const payload = (body ?? {}) as { message?: string; fields?: FieldError[] }
    throw new ApiError(
      response.status,
      payload.message ?? `request failed with status ${response.status}`,
      payload.fields ?? [],
    )
  }
  return body as T
}

function queryString(filters: Record<string, string | number | undefined>): string {
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(filters)) {
    if (value !== undefined && value !== '') {
      params.set(key, String(value))
    }
  }
  const query = params.toString()
  return query ? `?${query}` : ''
}

export const api = {
  listStocks(market?: Market): Promise<Stock[]> {
    return request(`/stocks${queryString({ market })}`)
  },
  createStock(stock: NewStock): Promise<Stock> {
    return request('/stocks', { method: 'POST', body: JSON.stringify(stock) })
  },
  updateStock(id: number, patch: Partial<NewStock> & { is_active?: boolean }): Promise<Stock> {
    return request(`/stocks/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteStock(id: number): Promise<void> {
    return request(`/stocks/${id}`, { method: 'DELETE' })
  },
  uploadPrices(fileText: string): Promise<PriceReport> {
    return request('/stocks/prices', { method: 'POST', body: fileText })
  },
  reorderStocks(market: Market, stockIds: number[]): Promise<Stock[]> {
    return request('/stocks/order', {
      method: 'POST',
      body: JSON.stringify({ market, stock_ids: stockIds }),
    })
  },
  listTrades(filters: TradeFilters = {}): Promise<Trade[]> {
    return request(`/trades${queryString({ ...filters })}`)
  },
  createTrade(trade: NewTrade): Promise<Trade> {
    return request('/trades', { method: 'POST', body: JSON.stringify(trade) })
  },
  updateTrade(id: number, patch: Partial<NewTrade>): Promise<Trade> {
    return request(`/trades/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteTrade(id: number): Promise<void> {
    return request(`/trades/${id}`, { method: 'DELETE' })
  },
  summary(market: Market): Promise<SummaryResponse> {
    return request(`/summary${queryString({ market })}`)
  },
  yearlySummary(market: Market): Promise<YearlySummary> {
    return request(`/summary/yearly${queryString({ market })}`)
  },
  updateYearly(market: Market, year: number, patch: YearlyPatch): Promise<YearSnapshotRow> {
    return request(`/summary/yearly/${market}/${year}`, {
      method: 'PATCH',
      body: JSON.stringify(patch),
    })
  },
  freezeYearly(market: Market, year: number): Promise<YearSnapshotRow> {
    return request(`/summary/yearly/${market}/${year}/freeze`, { method: 'POST' })
  },
  yearReview(): Promise<YearReviewResponse> {
    return request('/year-review')
  },
  updateYearReview(year: number, patch: YearReviewPatch): Promise<YearReviewRow> {
    return request(`/year-review/${year}`, {
      method: 'PATCH',
      body: JSON.stringify(patch),
    })
  },
  listDeposits(filters: DepositFilters = {}): Promise<Deposit[]> {
    return request(`/deposits${queryString({ ...filters })}`)
  },
  createDeposit(deposit: NewDeposit): Promise<Deposit> {
    return request('/deposits', { method: 'POST', body: JSON.stringify(deposit) })
  },
  updateDeposit(id: number, patch: Partial<NewDeposit>): Promise<Deposit> {
    return request(`/deposits/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteDeposit(id: number): Promise<void> {
    return request(`/deposits/${id}`, { method: 'DELETE' })
  },
  /** 收訖: mark received, optionally credit principal+interest to a cash
   * manual asset, and record the month's dep-end adjustment item. */
  receiveDeposit(id: number, body: ReceiveDepositBody = {}): Promise<Deposit> {
    return request(`/deposits/${id}/receive`, { method: 'POST', body: JSON.stringify(body) })
  },
  depositSummary(): Promise<DepositSummary> {
    return request('/deposits/summary')
  },
  listFamilyDeposits(filters: FamilyDepositFilters = {}): Promise<FamilyDeposit[]> {
    return request(`/family/deposits${queryString({ ...filters })}`)
  },
  createFamilyDeposit(deposit: NewFamilyDeposit): Promise<FamilyDeposit> {
    return request('/family/deposits', { method: 'POST', body: JSON.stringify(deposit) })
  },
  updateFamilyDeposit(id: number, patch: FamilyDepositPatch): Promise<FamilyDeposit> {
    return request(`/family/deposits/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteFamilyDeposit(id: number): Promise<void> {
    return request(`/family/deposits/${id}`, { method: 'DELETE' })
  },
  /** 收訖: mark received and optionally correct the interest. Unlike 定期,
   * no cash row is credited and no month item is recorded. */
  receiveFamilyDeposit(
    id: number,
    body: ReceiveFamilyDepositBody = {},
  ): Promise<FamilyDeposit> {
    return request(`/family/deposits/${id}/receive`, {
      method: 'POST',
      body: JSON.stringify(body),
    })
  },
  familyDepositSummary(): Promise<FamilyDepositSummary> {
    return request('/family/deposits/summary')
  },
  /** The per-holder free-text note; `null`/empty clears it. */
  updateFamilyHolderNote(
    holder: string,
    note: string | null,
  ): Promise<{ holder: string; note: string | null }> {
    return request(`/family/holders/${encodeURIComponent(holder)}/note`, {
      method: 'PUT',
      body: JSON.stringify({ note }),
    })
  },
  listDividends(filters: DividendFilters = {}): Promise<Dividend[]> {
    return request(`/dividends${queryString({ ...filters })}`)
  },
  createDividend(dividend: NewDividend): Promise<Dividend> {
    return request('/dividends', { method: 'POST', body: JSON.stringify(dividend) })
  },
  updateDividend(id: number, patch: DividendPatch): Promise<Dividend> {
    return request(`/dividends/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteDividend(id: number): Promise<void> {
    return request(`/dividends/${id}`, { method: 'DELETE' })
  },
  dividendSummary(market: Market): Promise<DividendSummary> {
    return request(`/dividends/summary${queryString({ market })}`)
  },
  listBonds(status?: 'active' | 'matured'): Promise<Bond[]> {
    return request(`/bonds${queryString({ status })}`)
  },
  createBond(bond: NewBond): Promise<Bond> {
    return request('/bonds', { method: 'POST', body: JSON.stringify(bond) })
  },
  updateBond(id: number, patch: Partial<NewBond>): Promise<Bond> {
    return request(`/bonds/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteBond(id: number): Promise<void> {
    return request(`/bonds/${id}`, { method: 'DELETE' })
  },
  /** 收訖 the principal return: mark received, optionally credit the principal
   * to a cash manual asset, and record the maturity month's bond-end item. */
  receiveBond(id: number, body: ReceiveBondBody = {}): Promise<Bond> {
    return request(`/bonds/${id}/receive`, { method: 'POST', body: JSON.stringify(body) })
  },
  bondSummary(): Promise<BondSummary> {
    return request('/bonds/summary')
  },
  listCoupons(bondId?: number): Promise<BondCoupon[]> {
    return request(`/coupons${queryString({ bond_id: bondId })}`)
  },
  createCoupon(coupon: NewBondCoupon): Promise<BondCoupon> {
    return request('/coupons', { method: 'POST', body: JSON.stringify(coupon) })
  },
  updateCoupon(id: number, patch: BondCouponPatch): Promise<BondCoupon> {
    return request(`/coupons/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteCoupon(id: number): Promise<void> {
    return request(`/coupons/${id}`, { method: 'DELETE' })
  },
  mpfOverview(): Promise<MpfOverview> {
    return request('/mpf')
  },
  createMpfAccount(account: NewMpfAccount): Promise<MpfAccount> {
    return request('/mpf/accounts', { method: 'POST', body: JSON.stringify(account) })
  },
  updateMpfAccount(id: number, patch: MpfAccountPatch): Promise<MpfAccount> {
    return request(`/mpf/accounts/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteMpfAccount(id: number): Promise<void> {
    return request(`/mpf/accounts/${id}`, { method: 'DELETE' })
  },
  updateMpfNote(note: string | null): Promise<{ note: string | null }> {
    return request('/mpf/note', { method: 'PATCH', body: JSON.stringify({ note }) })
  },
  deleteMpfHistory(id: number): Promise<void> {
    return request(`/mpf/history/${id}`, { method: 'DELETE' })
  },
  aiaSummary(): Promise<AiaSummary> {
    return request('/aia/summary')
  },
  createAiaPolicy(policy: NewAiaPolicy): Promise<AiaPolicy> {
    return request('/aia/policies', { method: 'POST', body: JSON.stringify(policy) })
  },
  updateAiaPolicy(id: number, patch: AiaPolicyPatch): Promise<AiaPolicy> {
    return request(`/aia/policies/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteAiaPolicy(id: number): Promise<void> {
    return request(`/aia/policies/${id}`, { method: 'DELETE' })
  },
  updateAiaNote(note: string | null): Promise<{ note: string | null }> {
    return request('/aia/note', { method: 'PATCH', body: JSON.stringify({ note }) })
  },
  listAiaEvents(policyId?: number): Promise<AiaEvent[]> {
    return request(`/aia/events${queryString({ policy_id: policyId })}`)
  },
  createAiaEvent(event: NewAiaEvent): Promise<AiaEvent> {
    return request('/aia/events', { method: 'POST', body: JSON.stringify(event) })
  },
  deleteAiaEvent(id: number): Promise<void> {
    return request(`/aia/events/${id}`, { method: 'DELETE' })
  },

  listMonths(year?: number): Promise<MonthStat[]> {
    return request(`/months${queryString({ year })}`)
  },
  monthSummary(): Promise<MonthSummary> {
    return request('/months/summary')
  },
  monthDetail(ym: string): Promise<MonthDetail> {
    return request(`/months/${ym}`)
  },
  patchMonth(ym: string, patch: MonthStatPatch): Promise<MonthStat> {
    return request(`/months/${ym}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteMonth(ym: string): Promise<void> {
    return request(`/months/${ym}`, { method: 'DELETE' })
  },
  createMonthItem(ym: string, item: NewMonthItem): Promise<MonthItem> {
    return request(`/months/${ym}/items`, { method: 'POST', body: JSON.stringify(item) })
  },
  updateMonthItem(id: number, patch: MonthItemPatch): Promise<MonthItem> {
    return request(`/month-items/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteMonthItem(id: number): Promise<void> {
    return request(`/month-items/${id}`, { method: 'DELETE' })
  },
  dismissMonthItem(ym: string, autoKey: string): Promise<void> {
    return request(`/months/${ym}/items/dismiss`, {
      method: 'POST',
      body: JSON.stringify({ auto_key: autoKey }),
    })
  },
  getSettings(): Promise<AppSettings> {
    return request('/settings')
  },
  updateSettings(patch: AppSettingsPatch): Promise<AppSettings> {
    return request('/settings', { method: 'PATCH', body: JSON.stringify(patch) })
  },
  listCashAccounts(): Promise<CashAccount[]> {
    return request('/cash-accounts')
  },
  createCashAccount(account: NewCashAccount): Promise<CashAccount> {
    return request('/cash-accounts', { method: 'POST', body: JSON.stringify(account) })
  },
  updateCashAccount(id: number, patch: CashAccountPatch): Promise<CashAccount> {
    return request(`/cash-accounts/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteCashAccount(id: number): Promise<void> {
    return request(`/cash-accounts/${id}`, { method: 'DELETE' })
  },
  listFxRates(): Promise<FxRate[]> {
    return request('/fx-rates')
  },
  /** A number stores the rate; `null` clears it so HKD totals drop out. */
  updateFxRate(currency: Currency, rate: number | null): Promise<FxRateEntry> {
    return request(`/fx-rates/${currency}`, {
      method: 'PATCH',
      body: JSON.stringify({ rate }),
    })
  },
  listManualAssets(): Promise<ManualAsset[]> {
    return request('/manual-assets')
  },
  createManualAsset(asset: NewManualAsset): Promise<ManualAsset> {
    return request('/manual-assets', { method: 'POST', body: JSON.stringify(asset) })
  },
  updateManualAsset(id: number, patch: ManualAssetPatch): Promise<ManualAsset> {
    return request(`/manual-assets/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteManualAsset(id: number): Promise<void> {
    return request(`/manual-assets/${id}`, { method: 'DELETE' })
  },
  overview(): Promise<OverviewResponse> {
    return request('/overview')
  },
  forecast(): Promise<ForecastResponse> {
    return request('/forecast')
  },
  createForecastItem(ym: string, item: NewForecastItem): Promise<ForecastItem> {
    return request(`/forecast/${ym}/items`, { method: 'POST', body: JSON.stringify(item) })
  },
  updateForecastItem(id: number, patch: ForecastItemPatch): Promise<ForecastItem> {
    return request(`/forecast-items/${id}`, { method: 'PATCH', body: JSON.stringify(patch) })
  },
  deleteForecastItem(id: number): Promise<void> {
    return request(`/forecast-items/${id}`, { method: 'DELETE' })
  },
  /** 轉為定期: create the deposit and consume the plan in one transaction. */
  convertForecastItem(id: number, deposit: NewDeposit): Promise<Deposit> {
    return request(`/forecast-items/${id}/convert`, {
      method: 'POST',
      body: JSON.stringify(deposit),
    })
  },
  ibkr(): Promise<IbkrBlock> {
    return request('/ibkr')
  },
  updateIbkr(patch: IbkrPatch): Promise<IbkrBlock> {
    return request('/ibkr', { method: 'PATCH', body: JSON.stringify(patch) })
  },
  listIbkrTransfers(): Promise<IbkrTransfer[]> {
    return request('/ibkr/transfers')
  },
  createIbkrTransfer(body: NewIbkrTransfer): Promise<IbkrTransfer> {
    return request('/ibkr/transfers', { method: 'POST', body: JSON.stringify(body) })
  },
  deleteIbkrTransfer(id: number): Promise<void> {
    return request(`/ibkr/transfers/${id}`, { method: 'DELETE' })
  },
}
