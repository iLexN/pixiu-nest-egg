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
  dividends_received: number
  dividend_return: number | null
  net_invested: number
  net_diluted_price: number | null
  real_total_return: number | null
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
  dividends_received: number
  dividend_return: number | null
  net_invested: number
  net_invested_priced: number
  real_total_return: number | null
}

/** A 未實現報酬率 + 未實現金額 pair for a market's last-month or max figures. */
export interface MarketFigures {
  percent: number | null
  amount: number
}

export interface SummaryResponse {
  market: Market
  average_price_definition: string
  stocks: SummaryStock[]
  sectors: SectorRollup[]
  totals: MarketTotals
  last_month: MarketFigures | null
  max: MarketFigures | null
}

/** The stored frozen figures for one year; a null field means "no override". */
export interface YearSnapshot {
  invested: number | null
  cost: number | null
  market_value: number | null
  sold_pl: number | null
  updated_at: string
}

/** The PATCH/freeze response: the snapshot row keyed by market + year. */
export interface YearSnapshotRow extends YearSnapshot {
  market: Market
  year: number
}

export interface YearRow {
  year: number
  invested: number
  cost: number
  market_value: number | null
  dividends: number
  yield_on_cost: number | null
  yield_on_value: number | null
  monthly_dividend: number
  dividend_yoy: number | null
  invested_yoy: number | null
  sold_pl: number | null
  snapshot: YearSnapshot | null
}

export interface YearlySummary {
  market: Market
  today: string
  years: YearRow[]
}

export interface YearlyPatch {
  invested?: number | null
  cost?: number | null
  market_value?: number | null
  sold_pl?: number | null
}

/** The stored `year_review` record: manual inputs plus the seeded overrides
 * for cells whose history was deleted from the workbook. */
export interface YearReviewRecord {
  income: number | null
  invested_adjustment: number | null
  raise: number | null
  bond_principal: number | null
  bond_interest: number | null
  deposit_principal: number | null
  deposit_interest: number | null
}

/** The A–D ledger group of one YearInReview block. */
export interface YearReviewLedger {
  asset_gain: number | null
  asset_gain_avg: number | null
  spend: number | null
  spend_avg: number | null
  living_avg: number | null
  pool_income: number
  pool_spend: number
  pool_balance: number
  asset_gain_yoy: number | null
  spend_yoy: number | null
  living_yoy: number | null
  pool_income_yoy: number | null
}

/** The E–G investment group of one YearInReview block. */
export interface YearReviewInvestment {
  interest: number
  interest_avg: number
  sold_pl: number | null
  net_investment: number | null
  transferred: number | null
  invested: number | null
  invested_pct: number | null
  irene_pool: number
  raise: number | null
  interest_avg_yoy: number | null
  invested_yoy: number | null
}

/** The H–M asset-returns group of one YearInReview block. */
export interface YearReviewAssets {
  bond_principal: number | null
  bond_interest: number | null
  bond_rate: number | null
  stock_cost: number | null
  stock_dividends: number | null
  stock_rate: number | null
  stock_now_value: number | null
  stock_value_rate: number | null
  deposit_principal: number | null
  deposit_interest: number | null
  income_cost_rate: number | null
  total_value_rate: number | null
  income_value_rate: number | null
  income: number | null
  income_avg: number | null
  income_yoy: number | null
  saved: number | null
  saved_avg: number | null
  saved_pct: number | null
  bond_overridden: boolean
  deposit_overridden: boolean
}

export interface YearReviewRow {
  year: number
  ledger: YearReviewLedger
  investment: YearReviewInvestment
  assets: YearReviewAssets
  record: YearReviewRecord | null
}

export interface YearReviewResponse {
  today: string
  years: YearReviewRow[]
}

export interface YearReviewPatch {
  income?: number | null
  invested_adjustment?: number | null
  raise?: number | null
  sold_pl?: number | null
  bond_principal?: number | null
  bond_interest?: number | null
  deposit_principal?: number | null
  deposit_interest?: number | null
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
  start_date: string | null
  end_date: string
  /** 收訖日; null while the deposit is still on the books (in 未到期定期). */
  received_at: string | null
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
  start_date?: string | null
  end_date: string
  note1?: string | null
  note2?: string | null
}

export interface ReceiveDepositBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Corrects the stored interest to the amount actually received. */
  interest?: number
  /** Cash `manual_assets` id to credit the returned money into. */
  credit_asset_id?: number
  /** Amount credited; defaults to principal + interest. */
  credit_amount?: number
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

/** A 家人 定期 record — family money, kept out of the user's own totals. */
export interface FamilyDeposit {
  id: number
  /** The family member the deposit belongs to, e.g. 媽媽 / Irene. */
  holder: string
  /** Bank reference, e.g. SC-9179. */
  label: string | null
  bank: string | null
  principal: number | null
  interest: number | null
  start_date: string | null
  end_date: string
  /** 收訖日; null while the deposit is still on the books. */
  received_at: string | null
  /** Free text, e.g. the bank's stepped-rate schedule. */
  note: string | null
  sort_order: number
  total: number
  status: DepositStatus
  end_year: number
  end_month: number
}

export interface NewFamilyDeposit {
  holder: string
  label?: string | null
  bank?: string | null
  principal?: number | null
  interest?: number | null
  start_date?: string | null
  end_date: string
  note?: string | null
}

export type FamilyDepositPatch = Partial<NewFamilyDeposit>

export interface ReceiveFamilyDepositBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Corrects the stored interest to the amount actually received. */
  interest?: number
}

export interface FamilyHolderSummary {
  holder: string
  note: string | null
  /** Unreceived deposits, earliest maturity first. */
  upcoming: FamilyDeposit[]
  /** Σ principal over deposits ending in the future. */
  active_principal: number
}

export interface FamilyDepositSummary {
  today: string
  /** Every holder with deposits or a stored note, sorted by name. */
  holders: FamilyHolderSummary[]
  history_years: number[]
}

export interface FamilyDepositFilters {
  status?: 'active' | 'ended'
  holder?: string
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
  /** On 收訖, bank the amount (HK → HS cash row, US → IBKR USD cash).
   * Defaults to on. */
  bank_in?: boolean
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

export type BondStatus = 'ACTIVE' | 'MATURED'
export type CouponStatus = 'PENDING_FIX' | 'PENDING' | 'RECEIVED'

export interface Bond {
  id: number
  label: string
  /** 發行編號, e.g. 03GB2710R */
  issue_no: string | null
  principal: number
  maturity_date: string
  /** 收訖日 for the principal return; null while matured-but-unreceived. */
  received_at: string | null
  note: string | null
  sort_order: number
  status: BondStatus
  next_pay_date: string | null
}

export interface ReceiveBondBody {
  /** 收訖日; defaults to today server-side. */
  received_at?: string
  /** Cash `manual_assets` id to credit the returned principal into. */
  credit_asset_id?: number
  /** Amount credited; defaults to the bond's principal. */
  credit_amount?: number
}

export interface NewBond {
  label: string
  issue_no?: string | null
  principal: number
  maturity_date: string
  note?: string | null
}

export interface BondCoupon {
  id: number
  bond_id: number
  /** 付息日 */
  pay_date: string
  /** 利息釐定日 */
  fixing_date: string | null
  /** 年息率; null = 待定 */
  annual_rate: number | null
  /** 每1萬港元債券利息; null = 待定 */
  per_10k: number | null
  received_amount: number | null
  note: string | null
  status: CouponStatus
  /** per_10k × principal ÷ 10000 */
  expected: number | null
  variance: number | null
}

export interface NewBondCoupon {
  bond_id: number
  pay_date: string
  fixing_date?: string | null
  annual_rate?: number | null
  per_10k?: number | null
  received_amount?: number | null
  note?: string | null
}

export interface BondCouponPatch {
  bond_id?: number
  pay_date?: string
  fixing_date?: string | null
  annual_rate?: number | null
  per_10k?: number | null
  received_amount?: number | null
  note?: string | null
  /** On 收訖, bank the amount into the HS cash row. Defaults to on. */
  bank_in?: boolean
}

export interface BondWithCoupons extends Bond {
  coupons: BondCoupon[]
}

export interface UpcomingCoupon extends BondCoupon {
  bond_label: string
}

export interface BondSummary {
  today: string
  totals: { active_principal: number }
  active: BondWithCoupons[]
  matured: BondWithCoupons[]
  upcoming_coupons: UpcomingCoupon[]
}

/** A rate + net gain pair; `rate` is null when contributions are zero. */
export interface MpfFigures {
  rate: number | null
  gain: number
}

export interface MpfAccount {
  id: number
  label: string
  trustee: string | null
  /** 總供款額 */
  contributions: number
  /** 帳戶結存 */
  balance: number
  plan_name: string | null
  member_no: string | null
  sort_order: number
  rate: number | null
  gain: number
  last_month: MpfFigures | null
  max: MpfFigures
}

export interface MpfHistoryRow {
  id: number
  account_id: number
  recorded_on: string
  contributions: number
  balance: number
  /** true for month-end rows backfilled automatically, not real updates. */
  synthetic: boolean
  rate: number | null
  gain: number
}

export interface MpfTotals {
  buy: number
  now: number
  rate: number | null
  gain: number
  last_month: MpfFigures | null
  max: MpfFigures
}

export interface MpfOverview {
  today: string
  accounts: MpfAccount[]
  totals: MpfTotals
  note: string | null
  history: MpfHistoryRow[]
}

export interface NewMpfAccount {
  label: string
  trustee?: string | null
  contributions?: number | null
  balance?: number | null
  plan_name?: string | null
  member_no?: string | null
}

export interface MpfAccountPatch {
  label?: string
  trustee?: string | null
  contributions?: number | null
  balance?: number | null
  plan_name?: string | null
  member_no?: string | null
}

export type AiaEventKind = 'payment' | 'withdrawal'

export interface AiaPolicy {
  id: number
  /** Plan/group name, e.g. 年金 - 2024 - 2029 */
  label: string
  policy_no: string | null
  /** Next premium-due date */
  next_pay_date: string | null
  premium_usd: number
  value_usd: number
  value_updated_at: string | null
  remaining_years: number | null
  withdrew_usd: number
  note: string | null
  link: string | null
  /** Excluded from the portfolio totals */
  excluded: boolean
  /** Counted in the AIA account display value */
  in_account: boolean
  sort_order: number
  /** (value + withdrew − premium) ÷ premium; null at zero premium */
  balance_pct: number | null
}

export interface NewAiaPolicy {
  label: string
  policy_no?: string | null
  next_pay_date?: string | null
  premium_usd: number
  value_usd: number
  remaining_years?: number | null
  withdrew_usd: number
  note?: string | null
  link?: string | null
  excluded?: boolean
  in_account?: boolean
}

export interface AiaPolicyPatch {
  label?: string
  policy_no?: string | null
  next_pay_date?: string | null
  premium_usd?: number
  value_usd?: number
  remaining_years?: number | null
  withdrew_usd?: number
  note?: string | null
  link?: string | null
  excluded?: boolean
  in_account?: boolean
}

export interface AiaEvent {
  id: number
  policy_id: number
  kind: AiaEventKind
  event_date: string
  amount_usd: number
  note: string | null
  prev_next_pay_date: string | null
  prev_remaining_years: number | null
}

export interface NewAiaEvent {
  policy_id: number
  kind: AiaEventKind
  event_date: string
  amount_usd: number
  note?: string | null
  /** New next premium-due date for payments; defaults to current +1 year. */
  next_pay_date?: string | null
}

export interface AiaTotals {
  premium: number
  value: number
  withdrew: number
  balance_pct: number | null
  display_value: number
  premium_hkd: number | null
  value_hkd: number | null
  withdrew_hkd: number | null
  /** The sheet's B5: now − buy − drew in HKD (net position change). */
  net_change_hkd: number | null
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
  totals: AiaTotals
  policies: AiaPolicyWithEvents[]
}


export type MonthItemCategory =
  | 'adjustment'
  | 'extra_spend'
  | 'income'
  | 'entertainment'
  | 'interest'
export type ManualAssetKind = 'cash' | 'asset'

export interface MonthStat {
  month: string
  start_cash: number | null
  salary: number | null
  total_assets: number | null
  liquid_assets: number | null
  total_assets_live: boolean
  liquid_assets_live: boolean
  interest: number
  pool_input: number
  end_cash_override: number | null
  note: string | null
  created_at: string
  updated_at: string
  end_cash: number | null
  month_spend: number | null
  living_spend: number | null
  saved: number | null
  total_change: number | null
  liquid_change: number | null
  living_yoy: number | null
  adjustment_sum: number
  extra_spend_sum: number
  income_sum: number
  entertainment_sum: number
}

export interface MonthItem {
  id: number
  month: string
  category: MonthItemCategory
  label: string | null
  amount: number
  exclude_from_living: boolean
  auto_key: string | null
  note: string | null
  created_at: string
}

/** A computed, not-yet-stored item candidate for the month. */
export interface MonthSuggestion {
  auto_key: string
  category: MonthItemCategory
  label: string | null
  amount: number
  source: string
}

/** One auto interest component: a deposit ending, a received coupon, or a
 * received HK dividend dated in the month. */
export interface InterestComponent {
  source: string
  label: string | null
  /** Received amount, or the expected/estimated figure while pending; null
   * when nothing is known yet (待定 coupon, estimate-less dividend). */
  amount: number | null
  /** Components are false until 收訖 — preview only, not counted. */
  received: boolean
}

export interface MonthDetail {
  month: MonthStat
  items: MonthItem[]
  suggestions: MonthSuggestion[]
  interest_auto: InterestComponent[]
}

export interface MonthYearSummary {
  year: number
  total_change_sum: number | null
  total_change_avg: number | null
  spend_sum: number | null
  spend_avg: number | null
  living_avg: number | null
  entertainment_sum: number
  interest_sum: number
  interest_avg: number
  pool_income: number
  pool_balance: number
  pool_input_sum: number
  months: number
  /** 投資純利 — null until HK sold P/L is computed. */
  net_investment: number | null
}

export interface MonthRunningAverages {
  total_change_avg: number | null
  liquid_change_avg: number | null
  saved_avg: number | null
  interest_avg: number | null
}

export interface MonthSummary {
  years: MonthYearSummary[]
  running: MonthRunningAverages
  pool_balance: number
  pool_rate: number | null
  pool_rate_year: number
}

export interface MonthSettings {
  salary: number | null
  pool_rate: number | null
  pool_rate_year: number
}

export interface MonthStatPatch {
  start_cash?: number | null
  salary?: number | null
  total_assets?: number | null
  liquid_assets?: number | null
  pool_input?: number
  end_cash_override?: number | null
  note?: string | null
  recapture?: boolean
}

export interface NewMonthItem {
  category: MonthItemCategory
  label?: string | null
  amount: number
  exclude_from_living?: boolean
  auto_key?: string | null
  note?: string | null
}

export interface MonthItemPatch {
  category?: MonthItemCategory
  label?: string | null
  amount?: number
  exclude_from_living?: boolean
  note?: string | null
}

export interface MonthSettingsPatch {
  salary?: number | null
  pool_rate?: number | null
  pool_rate_year?: number
}

export interface ManualAsset {
  id: number
  label: string
  kind: ManualAssetKind
  amount: number
  sort_order: number
  updated_at: string
}

export interface NewManualAsset {
  label: string
  kind: ManualAssetKind
  amount: number
}

export interface ManualAssetPatch {
  label?: string
  kind?: ManualAssetKind
  amount?: number
}

/** The 美股 sheet's IBKR account block: manual inputs + derived cross-checks. */
export interface IbkrBlock {
  transferred_hkd: number | null
  now_value: number | null
  hkd_cash: number | null
  usd_cash: number | null
  stock_value_usd: number | null
  computed_total_hkd: number | null
  net: number | null
  net_pct: number | null
  vs_now_value: number | null
}

export interface IbkrPatch {
  /** A bank→IBKR transfer delta in HKD; negative records a withdrawal. */
  transfer_hkd?: number
  /** Optional `YYYY-MM-DD`; defaults to today. */
  transfer_date?: string
  now_value?: number | null
  hkd_cash?: number | null
  usd_cash?: number | null
}

export interface OverviewAssetRow {
  key: string
  label: string
  amount: number | null
  share: number | null
  manual_asset_id: number | null
}

export interface SemiLiquid {
  deposits: number
  cash_rows: ManualAsset[]
  cash_sum: number
  total: number
  vs_quarter_liquid: number
  share: number | null
}

export interface TwelveMonthAverages {
  total_change: number | null
  month_spend: number | null
  living_spend: number | null
  living_budget: number | null
  living_budget_low: boolean
  living_budget_floor: number
  saved: number | null
  interest: number | null
  pool_balance: number
  window_start: string | null
  window_end: string | null
}

/** The 投資目標 block (Overview!J22:N27). */
export interface InvestTargetRow {
  year: number
  invested: number | null
  raise: number | null
  target: number | null
  remain: number | null
  growth: number | null
}

export interface InvestTargets {
  avg_invested: number | null
  rows: InvestTargetRow[]
}

export interface OverviewResponse {
  today: string
  rate: number | null
  salary: number | null
  total_assets: number
  liquid_assets: number
  liquid_ratio: number | null
  assets: OverviewAssetRow[]
  assets_sum: number
  semi_liquid: SemiLiquid
  ibkr: IbkrBlock
  averages: TwelveMonthAverages
  invest_targets: InvestTargets
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
  unreceiveDeposit(id: number): Promise<Deposit> {
    return request(`/deposits/${id}/unreceive`, { method: 'POST' })
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
  unreceiveFamilyDeposit(id: number): Promise<FamilyDeposit> {
    return request(`/family/deposits/${id}/unreceive`, { method: 'POST' })
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
  unreceiveBond(id: number): Promise<Bond> {
    return request(`/bonds/${id}/unreceive`, { method: 'POST' })
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
  updateAiaRate(rate: number | null): Promise<{ rate: number | null }> {
    return request('/aia/rate', { method: 'PATCH', body: JSON.stringify({ rate }) })
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
  monthSettings(): Promise<MonthSettings> {
    return request('/months/settings')
  },
  updateMonthSettings(patch: MonthSettingsPatch): Promise<MonthSettings> {
    return request('/months/settings', { method: 'PATCH', body: JSON.stringify(patch) })
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
  ibkr(): Promise<IbkrBlock> {
    return request('/ibkr')
  },
  updateIbkr(patch: IbkrPatch): Promise<IbkrBlock> {
    return request('/ibkr', { method: 'PATCH', body: JSON.stringify(patch) })
  },
}
