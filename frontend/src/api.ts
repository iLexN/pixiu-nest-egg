export type Market = 'HK' | 'US'
export type TradeType = 'BUY' | 'SELL'
export type InputMode = 'HK_TOTAL' | 'US_FEE'

export interface FieldError {
  field: string
  message: string
}

export interface Stock {
  id: number
  market: Market
  code: string
  ticker: string | null
  exchange: string | null
  sector: string | null
  manual_price: number | null
  price_updated_at: string | null
  pe: number | null
  eps: number | null
  high52: number | null
  low52: number | null
  note: string | null
  is_active: boolean
  sort_order: number
}

export interface Trade {
  id: number
  stock_id: number
  market: Market
  code: string
  trade_type: TradeType
  trade_date: string
  shares: number
  unit_price: number
  fee: number
  total: number
  input_mode: InputMode
  note: string | null
  unit_price_incl_fee: number | null
}

export interface SummaryStock extends Stock {
  shares_held: number
  total_buy_cost: number
  weighted_avg_buy_price: number
  current_price: number | null
  market_value: number | null
  unrealized_amount: number | null
  unrealized_return: number | null
  trade_count: number
}

export interface SectorRollup {
  sector: string
  buy_cost: number
  buy_cost_priced: number
  market_value: number | null
  share_of_market_value: number | null
  percent_change: number | null
}

export interface MarketTotals {
  buy_cost: number
  buy_cost_priced: number
  market_value: number | null
  net_amount: number | null
  net_percent: number | null
  excluded_codes: string[]
}

export interface SummaryResponse {
  market: Market
  average_price_definition: string
  stocks: SummaryStock[]
  sectors: SectorRollup[]
  totals: MarketTotals
}

export interface NewStock {
  market: Market
  code: string
  ticker?: string | null
  exchange?: string | null
  sector?: string | null
  manual_price?: number | null
  pe?: number | null
  eps?: number | null
  high52?: number | null
  low52?: number | null
  note?: string | null
}

export interface NewTrade {
  stock_id?: number
  market?: Market
  code?: string
  trade_type: TradeType
  trade_date: string
  shares: number
  unit_price: number
  total?: number | null
  fee?: number | null
  input_mode?: InputMode
  note?: string | null
}

export interface PriceUpdate {
  market: Market
  code: string
  symbol: string
  price: number
}

export interface InvalidPriceEntry {
  symbol: string | null
  price: number | null
  reason: string
}

export interface PriceReport {
  updated: PriceUpdate[]
  unmatched: string[]
  invalid: InvalidPriceEntry[]
  not_updated: string[]
}

export interface TradeFilters {
  market?: Market
  stock_id?: number
  code?: string
  from?: string
  to?: string
  order?: 'asc' | 'desc'
}

export type DepositStatus = 'ACTIVE' | 'END'

export interface Deposit {
  id: number
  label: string | null
  bank: string | null
  principal: number | null
  rate: number | null
  interest: number | null
  end_date: string
  note1: string | null
  note2: string | null
  sort_order: number
  total: number
  status: DepositStatus
  end_year: number
  end_month: number
}

export interface NewDeposit {
  label?: string | null
  bank?: string | null
  principal?: number | null
  rate?: number | null
  interest?: number | null
  end_date: string
  note1?: string | null
  note2?: string | null
}

export interface ActiveMonthBucket {
  year: number
  month: number
  principal: number
  interest: number
  total: number
}

export interface BankRollup {
  bank: string
  principal: number
  interest: number
  total: number
}

export interface YearMonthRow {
  month: number
  interest: number
  payout: number
  total: number
}

export interface YearRollup {
  year: number
  months: YearMonthRow[]
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

export type DividendStatus = 'PENDING' | 'RECEIVED'

export interface Dividend {
  id: number
  stock_id: number
  market: Market
  code: string
  pay_date: string
  per_share: number | null
  shares_held: number | null
  buy_cost: number | null
  estimated_amount: number | null
  received_amount: number | null
  received_price: number | null
  note: string | null
  status: DividendStatus
  amount: number | null
  yield_on_cost: number | null
  yield_on_price: number | null
  variance: number | null
}

export interface NewDividend {
  stock_id?: number
  market?: Market
  code?: string
  pay_date: string
  per_share?: number | null
  estimated_amount?: number | null
  shares_held?: number | null
  buy_cost?: number | null
  note?: string | null
}

export interface DividendPatch {
  stock_id?: number
  pay_date?: string
  per_share?: number | null
  shares_held?: number | null
  buy_cost?: number | null
  estimated_amount?: number | null
  received_amount?: number | null
  received_price?: number | null
  note?: string | null
  refresh_snapshots?: boolean
}

export interface DividendStockTotal {
  code: string
  received: number
}

export interface DividendYearRollup {
  year: number
  total: number
  stocks: DividendStockTotal[]
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

/** Carries the server's field-level messages so forms can show them inline. */
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
  const body: unknown = text ? JSON.parse(text) : null
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
  depositSummary(): Promise<DepositSummary> {
    return request('/deposits/summary')
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
}
