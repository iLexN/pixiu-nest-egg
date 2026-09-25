use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

/// For PATCH bodies, distinguishes "field absent" from "field present as null".
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum Market {
    Hk,
    Us,
}

impl Market {
    pub fn as_str(self) -> &'static str {
        match self {
            Market::Hk => "HK",
            Market::Us => "US",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "HK" => Some(Market::Hk),
            "US" => Some(Market::Us),
            _ => None,
        }
    }

    /// The input convention each market's statements come in.
    pub fn default_input_mode(self) -> InputMode {
        match self {
            Market::Hk => InputMode::HkTotal,
            Market::Us => InputMode::UsFee,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum TradeType {
    Buy,
    Sell,
}

impl TradeType {
    pub fn as_str(self) -> &'static str {
        match self {
            TradeType::Buy => "BUY",
            TradeType::Sell => "SELL",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_uppercase().as_str() {
            "BUY" => Some(TradeType::Buy),
            "SELL" => Some(TradeType::Sell),
            _ => None,
        }
    }
}

/// Which pair of money figures the user typed; the other pair is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InputMode {
    /// HK style: 單價 + buy total (fee included) given, fee derived.
    HkTotal,
    /// US style: 單價 + fee given, buy total derived.
    UsFee,
}

impl InputMode {
    pub fn as_str(self) -> &'static str {
        match self {
            InputMode::HkTotal => "HK_TOTAL",
            InputMode::UsFee => "US_FEE",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "HK_TOTAL" => Some(InputMode::HkTotal),
            "US_FEE" => Some(InputMode::UsFee),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Stock {
    pub id: i64,
    pub market: Market,
    pub code: String,
    pub ticker: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub manual_price: Option<f64>,
    pub price_updated_at: Option<String>,
    pub pe: Option<f64>,
    pub eps: Option<f64>,
    pub high52: Option<f64>,
    pub low52: Option<f64>,
    pub note: Option<String>,
    pub is_active: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewStock {
    pub market: Market,
    pub code: String,
    pub ticker: Option<String>,
    pub exchange: Option<String>,
    pub sector: Option<String>,
    pub manual_price: Option<f64>,
    pub pe: Option<f64>,
    pub eps: Option<f64>,
    pub high52: Option<f64>,
    pub low52: Option<f64>,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct StockPatch {
    pub code: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub ticker: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub exchange: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub sector: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub manual_price: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub pe: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub eps: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub high52: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub low52: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Trade {
    pub id: i64,
    pub stock_id: i64,
    pub market: Market,
    pub code: String,
    pub trade_type: TradeType,
    pub trade_date: String,
    pub shares: f64,
    pub unit_price: f64,
    pub fee: f64,
    pub total: f64,
    pub input_mode: InputMode,
    pub note: Option<String>,
    /// 平均單價 for this trade: total (fee included) / shares.
    pub unit_price_incl_fee: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewTrade {
    /// Either `stock_id`, or `market` + `code`, identifies the stock.
    pub stock_id: Option<i64>,
    pub market: Option<Market>,
    pub code: Option<String>,
    pub trade_type: String,
    pub trade_date: String,
    pub shares: f64,
    pub unit_price: f64,
    /// HK input: buy total including fee.
    pub total: Option<f64>,
    /// US input: fee charged on top of shares * unit price.
    pub fee: Option<f64>,
    pub input_mode: Option<InputMode>,
    pub note: Option<String>,
}

/// One `{symbol, price}` row from a price file. Fields are optional so a
/// malformed entry can be reported instead of failing the whole upload.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RawPriceEntry {
    pub symbol: Option<String>,
    pub price: Option<f64>,
}

/// A price that was applied to a stored stock.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PriceUpdate {
    pub market: Market,
    pub code: String,
    pub symbol: String,
    pub price: f64,
}

/// A file entry that was skipped: missing symbol or non-positive price.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InvalidPriceEntry {
    pub symbol: Option<String>,
    pub price: Option<f64>,
    pub reason: String,
}

/// Result of a bulk 現價 upload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PriceReport {
    pub updated: Vec<PriceUpdate>,
    /// File symbols that matched no stored stock.
    pub unmatched: Vec<String>,
    /// File entries that could not be applied.
    pub invalid: Vec<InvalidPriceEntry>,
    /// Stored stocks (as `MARKET code`) with no entry in the file.
    pub not_updated: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct TradePatch {
    pub stock_id: Option<i64>,
    pub trade_type: Option<String>,
    pub trade_date: Option<String>,
    pub shares: Option<f64>,
    pub unit_price: Option<f64>,
    pub total: Option<f64>,
    pub fee: Option<f64>,
    pub input_mode: Option<InputMode>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// Derived from `end_date`: `End` once the end date is today or past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DepositStatus {
    Active,
    End,
}

/// A 定期 deposit record. `total`, `status`, `end_year` and `end_month` are
/// derived on read, never stored.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Deposit {
    pub id: i64,
    /// The sheet's `id` column: a bank reference like `SC-9632`.
    pub label: Option<String>,
    /// Bank code derived from the label prefix (SC = 渣打, HS = 恒生).
    pub bank: Option<String>,
    /// The sheet's `input` column.
    pub principal: Option<f64>,
    /// Annual rate as a fraction (0.03 = 3%).
    pub rate: Option<f64>,
    /// 利息.
    pub interest: Option<f64>,
    /// When the principal left the bank account; NULL when unknown.
    pub start_date: Option<String>,
    pub end_date: String,
    /// 收訖日: when the user confirmed the money came back; NULL while the
    /// deposit is still on the books (it stays in 未到期定期 until then).
    pub received_at: Option<String>,
    pub note1: Option<String>,
    pub note2: Option<String>,
    pub sort_order: i64,
    /// principal + interest, blanks counting as 0.
    pub total: f64,
    /// `END` once 收訖; `ACTIVE` while unreceived (even past `end_date`).
    pub status: DepositStatus,
    pub end_year: i32,
    pub end_month: u32,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewDeposit {
    pub label: Option<String>,
    pub bank: Option<String>,
    pub principal: Option<f64>,
    pub rate: Option<f64>,
    pub interest: Option<f64>,
    pub start_date: Option<String>,
    pub end_date: String,
    pub note1: Option<String>,
    pub note2: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct DepositPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub label: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bank: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub rate: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub interest: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub start_date: Option<Option<String>>,
    pub end_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub note1: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note2: Option<Option<String>>,
}

/// A 家人 定期 record: a deposit held on behalf of a family member, kept in
/// its own table so it never feeds the user's own totals. `total`,
/// `status`, `end_year` and `end_month` are derived on read, never stored.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FamilyDeposit {
    pub id: i64,
    /// The family member the deposit belongs to, e.g. `媽媽`, `Irene`.
    pub holder: String,
    /// The bank reference, e.g. `SC-9179`.
    pub label: Option<String>,
    pub bank: Option<String>,
    pub principal: Option<f64>,
    /// 利息.
    pub interest: Option<f64>,
    /// When the principal left the bank account; NULL when unknown.
    pub start_date: Option<String>,
    pub end_date: String,
    /// 收訖日; NULL while the deposit is still on the books.
    pub received_at: Option<String>,
    /// Free text, e.g. the bank's stepped-rate schedule.
    pub note: Option<String>,
    pub sort_order: i64,
    /// principal + interest, blanks counting as 0.
    pub total: f64,
    /// `END` once 收訖; `ACTIVE` while unreceived (even past `end_date`).
    pub status: DepositStatus,
    pub end_year: i32,
    pub end_month: u32,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewFamilyDeposit {
    pub holder: String,
    pub label: Option<String>,
    pub bank: Option<String>,
    pub principal: Option<f64>,
    pub interest: Option<f64>,
    pub start_date: Option<String>,
    pub end_date: String,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct FamilyDepositPatch {
    pub holder: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub label: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bank: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub interest: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub start_date: Option<Option<String>>,
    pub end_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// Pending until `received_amount` is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DividendStatus {
    Pending,
    Received,
}

/// A 派息 record. `shares_held`, `buy_cost` and `received_price` are
/// point-in-time snapshots stored at write time; `status`, `amount`,
/// `yield_on_cost`, `yield_on_price` and `variance` are derived on read.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Dividend {
    pub id: i64,
    pub stock_id: i64,
    pub market: Market,
    pub code: String,
    /// The sheet's K column: the pay date.
    pub pay_date: String,
    /// Announced 每股派息.
    pub per_share: Option<f64>,
    /// 股數 snapshot: ΣBUY − ΣSELL shares on or before pay_date.
    pub shares_held: Option<f64>,
    /// 總買入成本 snapshot on or before pay_date (the sheet's L denominator).
    pub buy_cost: Option<f64>,
    /// 預期派息.
    pub estimated_amount: Option<f64>,
    /// 實收派息; NULL while pending.
    pub received_amount: Option<f64>,
    /// 現價 snapshot at receipt (the sheet's N denominator).
    pub received_price: Option<f64>,
    pub note: Option<String>,
    pub status: DividendStatus,
    /// received_amount when present, else estimated_amount.
    pub amount: Option<f64>,
    /// amount ÷ buy_cost (the sheet's rate column).
    pub yield_on_cost: Option<f64>,
    /// amount ÷ (received_price × shares_held).
    pub yield_on_price: Option<f64>,
    /// received_amount − estimated_amount when both are present.
    pub variance: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewDividend {
    /// Either `stock_id`, or `market` + `code`, identifies the stock.
    pub stock_id: Option<i64>,
    pub market: Option<Market>,
    pub code: Option<String>,
    pub pay_date: String,
    pub per_share: Option<f64>,
    pub estimated_amount: Option<f64>,
    /// Snapshot overrides; derived from trades <= pay_date when absent.
    pub shares_held: Option<f64>,
    pub buy_cost: Option<f64>,
    pub note: Option<String>,
}

/// Derived from `maturity_date`: `Matured` once maturity is today or past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum BondStatus {
    Active,
    Matured,
}

/// 待定 until `annual_rate`/`per_10k` are fixed, then `Pending` until
/// `received_amount` is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CouponStatus {
    PendingFix,
    Pending,
    Received,
}

/// A 債券 record. `status` and `next_pay_date` are derived on read.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Bond {
    pub id: i64,
    /// The sheet's label column, e.g. `silver bond`.
    pub label: String,
    /// 發行編號, e.g. `03GB2710R`.
    pub issue_no: Option<String>,
    pub principal: f64,
    /// The sheet's `end` column.
    pub maturity_date: String,
    /// 收訖日: when the user confirmed the principal came back; NULL while
    /// matured-but-unreceived (the coupon lifecycle is separate).
    pub received_at: Option<String>,
    pub note: Option<String>,
    pub sort_order: i64,
    /// `MATURED` once `maturity_date` is reached (sectioning only — receipt is
    /// tracked by `received_at`).
    pub status: BondStatus,
    /// Earliest pay_date among this bond's non-received coupons.
    pub next_pay_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewBond {
    pub label: String,
    pub issue_no: Option<String>,
    pub principal: f64,
    pub maturity_date: String,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct BondPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub issue_no: Option<Option<String>>,
    pub principal: Option<f64>,
    pub maturity_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// One scheduled coupon of a bond. `annual_rate`/`per_10k` stay NULL while the
/// rate is 待定; `status`, `expected` and `variance` are derived on read.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BondCoupon {
    pub id: i64,
    pub bond_id: i64,
    /// The sheet's 付息日.
    pub pay_date: String,
    /// The sheet's 利息釐定日.
    pub fixing_date: Option<String>,
    /// 年息率 as a fraction (0.04 = 4%); NULL = 待定.
    pub annual_rate: Option<f64>,
    /// 每1萬港元債券利息; NULL = 待定.
    pub per_10k: Option<f64>,
    /// 實收利息; NULL while unreceived.
    pub received_amount: Option<f64>,
    pub note: Option<String>,
    pub status: CouponStatus,
    /// per_10k × bond principal ÷ 10000 (the sheet's interest column).
    pub expected: Option<f64>,
    /// received_amount − expected when both are present.
    pub variance: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewBondCoupon {
    pub bond_id: i64,
    pub pay_date: String,
    pub fixing_date: Option<String>,
    pub annual_rate: Option<f64>,
    pub per_10k: Option<f64>,
    pub received_amount: Option<f64>,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value — clearing `received_amount` marks the coupon
/// unreceived, clearing `annual_rate`/`per_10k` returns it to 待定.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct BondCouponPatch {
    pub bond_id: Option<i64>,
    pub pay_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub fixing_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub annual_rate: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub per_10k: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub received_amount: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    /// On 收訖 (received_amount NULL → set), bank the amount into the HS cash
    /// row. Defaults to on; only meaningful during the receipt transition.
    #[serde(default)]
    pub bank_in: Option<bool>,
}

/// A rate + net gain pair. `rate` is empty when contributions are zero.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct MpfFigures {
    pub rate: Option<f64>,
    pub gain: f64,
}

/// A 未實現報酬率 + 未實現金額 pair for a market's last-month or max figures.
/// `percent` is empty when the recorded `buy_cost_priced` is zero.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct MarketFigures {
    pub percent: Option<f64>,
    pub amount: f64,
}

/// An MPF (強積金) account. `rate`, `gain`, `last_month` and `max` are derived
/// on read from the stored values plus the history rows; `seed_max_*` are the
/// imported high-water marks that act as a floor for the reported maxima.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfAccount {
    pub id: i64,
    /// The sheet's account name, e.g. `new type`, `強積金個人帳戶`.
    pub label: String,
    pub trustee: Option<String>,
    /// 總供款額.
    pub contributions: f64,
    /// 帳戶結存.
    pub balance: f64,
    pub plan_name: Option<String>,
    pub member_no: Option<String>,
    pub sort_order: i64,
    /// (balance − contributions) ÷ contributions; empty when contributions = 0.
    pub rate: Option<f64>,
    /// balance − contributions.
    pub gain: f64,
    /// Latest history row in the previous calendar month, if any.
    pub last_month: Option<MpfFigures>,
    /// All-time maxima over the seed, every history row, and current values.
    /// Rate and gain are independent and may come from different moments.
    pub max: MpfFigures,
}

/// One recorded account state. Synthetic rows backfill months with no update.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfHistoryRow {
    pub id: i64,
    pub account_id: i64,
    /// YYYY-MM-DD.
    pub recorded_on: String,
    pub contributions: f64,
    pub balance: f64,
    pub synthetic: bool,
    pub rate: Option<f64>,
    pub gain: f64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewMpfAccount {
    pub label: String,
    pub trustee: Option<String>,
    pub contributions: Option<f64>,
    pub balance: Option<f64>,
    pub plan_name: Option<String>,
    pub member_no: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. Changing `contributions` or `balance` records a
/// history row; metadata-only edits do not.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MpfAccountPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub trustee: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub contributions: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub balance: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub plan_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub member_no: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MpfNotePatch {
    pub note: Option<String>,
}

/// Stored frozen figures for one (market, year) in the yearly summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct YearSnapshot {
    pub market: Market,
    pub year: i32,
    /// Manual override for the year's net invested; NULL falls back to
    /// Σ BUY − Σ SELL of the year.
    pub invested: Option<f64>,
    /// Frozen 年末總成本; NULL falls back to cumulative Σ BUY total.
    pub cost: Option<f64>,
    /// Frozen 年末總市值; NULL stays live for the current year, empty for
    /// past years.
    pub market_value: Option<f64>,
    /// 賣出損益: the year's realized sell P/L, entered by hand — the
    /// workbook never recorded SELL trades. NULL reports it absent.
    pub sold_pl: Option<f64>,
    pub updated_at: String,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears a stored value and the column falls back to the computed figure.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct YearlyPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub invested: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub cost: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub market_value: Option<Option<f64>>,
    /// Unlike the other figures, sold P/L may be negative.
    #[serde(default, deserialize_with = "nullable")]
    pub sold_pl: Option<Option<f64>>,
}

/// `PATCH /api/year-review/:year` body. Absent fields are left untouched;
/// `null` clears an override so the figure derives live again. `sold_pl`
/// writes the year's HK `year_snapshots` row instead of `year_review`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct YearReviewPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub income: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub invested_adjustment: Option<Option<f64>>,
    /// 月薪增幅 override; unlike income it may be negative. `null` restores
    /// the salary-derived figure.
    #[serde(default, deserialize_with = "nullable")]
    pub raise: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub sold_pl: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bond_principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bond_interest: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deposit_principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deposit_interest: Option<Option<f64>>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. `refresh_snapshots` re-derives shares_held and
/// buy_cost from trades on or before the (possibly edited) pay_date.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct DividendPatch {
    pub stock_id: Option<i64>,
    pub pay_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub per_share: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub shares_held: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub buy_cost: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub estimated_amount: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub received_amount: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub received_price: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    #[serde(default)]
    pub refresh_snapshots: bool,
    /// On 收訖 (received_amount NULL → set), bank the amount: HK dividends go
    /// to the HS cash row, US to the IBKR USD cash meta value. Defaults to on;
    /// only meaningful during the receipt transition.
    #[serde(default)]
    pub bank_in: Option<bool>,
}

/// An AIA policy row. `balance_pct` is derived on read; the totals flags
/// reproduce the sheet's two sums: `excluded` rows are in the account but not
/// the user's money (irene 20%), `in_account` rows count in `display_value`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AiaPolicy {
    pub id: i64,
    /// Plan or group name, e.g. `年金 - 2024 - 2029`.
    pub label: String,
    /// e.g. `B632611401`.
    pub policy_no: Option<String>,
    /// Next premium-due date.
    pub next_pay_date: Option<String>,
    /// The sheet's `buy usd`.
    pub premium_usd: f64,
    /// The sheet's `now usd`.
    pub value_usd: f64,
    pub value_updated_at: Option<String>,
    pub remaining_years: Option<f64>,
    /// The sheet's `Withdrew`, cumulative.
    pub withdrew_usd: f64,
    pub note: Option<String>,
    /// The remark column's hyperlink target (manual entry).
    pub link: Option<String>,
    /// Not counted in the portfolio totals.
    pub excluded: bool,
    /// Counted in `display_value`.
    pub in_account: bool,
    pub sort_order: i64,
    /// (value_usd + withdrew_usd − premium_usd) ÷ premium_usd; absent at 0.
    pub balance_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewAiaPolicy {
    pub label: String,
    pub policy_no: Option<String>,
    pub next_pay_date: Option<String>,
    pub premium_usd: f64,
    pub value_usd: f64,
    pub remaining_years: Option<f64>,
    pub withdrew_usd: f64,
    pub note: Option<String>,
    pub link: Option<String>,
    #[serde(default)]
    pub excluded: bool,
    #[serde(default = "default_true")]
    pub in_account: bool,
}

fn default_true() -> bool {
    true
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. Changing `value_usd` refreshes
/// `value_updated_at`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AiaPolicyPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub policy_no: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub next_pay_date: Option<Option<String>>,
    pub premium_usd: Option<f64>,
    pub value_usd: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub remaining_years: Option<Option<f64>>,
    pub withdrew_usd: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub link: Option<Option<String>>,
    pub excluded: Option<bool>,
    pub in_account: Option<bool>,
}

/// One recorded premium payment or withdrawal. `prev_next_pay_date` /
/// `prev_remaining_years` snapshot the policy fields a payment touched so
/// deleting the event restores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiaEventKind {
    Payment,
    Withdrawal,
}

impl AiaEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AiaEventKind::Payment => "payment",
            AiaEventKind::Withdrawal => "withdrawal",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "payment" => Some(AiaEventKind::Payment),
            "withdrawal" => Some(AiaEventKind::Withdrawal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AiaEvent {
    pub id: i64,
    pub policy_id: i64,
    pub kind: AiaEventKind,
    pub event_date: String,
    pub amount_usd: f64,
    pub note: Option<String>,
    pub prev_next_pay_date: Option<String>,
    pub prev_remaining_years: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewAiaEvent {
    pub policy_id: i64,
    pub kind: AiaEventKind,
    pub event_date: String,
    pub amount_usd: f64,
    pub note: Option<String>,
    /// Explicit new next premium-due date for payments; defaults to the
    /// policy's current `next_pay_date` plus one year.
    pub next_pay_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiaRatePatch {
    pub rate: Option<f64>,
}

/// The month-item categories replacing the sheet's opaque sum columns:
/// `adjustment` is the G column (bank in/out that must not count as 支出),
/// `extra_spend` the extras subtracted inside J, `income` the extras added
/// inside L, `entertainment` the O 娛樂支出 items, `interest` the hand-kept
/// part of N 利息 the auto events do not cover (bank 活期 interest, promos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MonthItemCategory {
    Adjustment,
    ExtraSpend,
    Income,
    Entertainment,
    Interest,
}

impl MonthItemCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            MonthItemCategory::Adjustment => "adjustment",
            MonthItemCategory::ExtraSpend => "extra_spend",
            MonthItemCategory::Income => "income",
            MonthItemCategory::Entertainment => "entertainment",
            MonthItemCategory::Interest => "interest",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "adjustment" => Some(MonthItemCategory::Adjustment),
            "extra_spend" => Some(MonthItemCategory::ExtraSpend),
            "income" => Some(MonthItemCategory::Income),
            "entertainment" => Some(MonthItemCategory::Entertainment),
            "interest" => Some(MonthItemCategory::Interest),
            _ => None,
        }
    }
}

/// One month of the Month Stat ledger (`YYYY-MM-01`). Only entered figures are
/// stored; `interest`, `end_cash`, `month_spend`, `living_spend`, `saved`,
/// `total_change` and `liquid_change` are derived on read. `total_assets`/`liquid_assets`
/// report the effective value — stored when frozen, live-derived when NULL —
/// and the `_live` flags tell the UI which.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthStat {
    pub month: String,
    /// F 月初(出糧後): the 活期 total right after salary lands.
    pub start_cash: Option<f64>,
    /// The salary in effect that month, snapshotted at creation.
    pub salary: Option<f64>,
    /// B 總數 (effective: frozen value or live-derived).
    pub total_assets: Option<f64>,
    /// D 流動資產 (effective: frozen value or live-derived).
    pub liquid_assets: Option<f64>,
    /// True while `total_assets` is live-derived rather than stored.
    pub total_assets_live: bool,
    /// True while `liquid_assets` is live-derived rather than stored.
    pub liquid_assets_live: bool,
    /// N 利息 (derived): auto events + Σ interest items.
    pub interest: f64,
    /// P Irene + 開心 Pool.
    pub pool_input: f64,
    /// Hand-frozen H 月尾 value for months the `=F(n+1) − salary` chain cannot
    /// reproduce (e.g. the first ledger month's typed bank balance).
    pub end_cash_override: Option<f64>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// H 月尾(出糧前): the stored override, else next row's start_cash −
    /// salary; absent on the latest row.
    pub end_cash: Option<f64>,
    /// I 月支出: start_cash + Σadjustment − end_cash.
    pub month_spend: Option<f64>,
    /// J 生活支出: month_spend − Σextra_spend − Σ flagged entertainment.
    pub living_spend: Option<f64>,
    /// L 存: salary − month_spend + Σincome.
    pub saved: Option<f64>,
    /// C Changed: next row's total_assets − this row's.
    pub total_change: Option<f64>,
    /// E Changed: next row's liquid_assets − this row's.
    pub liquid_change: Option<f64>,
    /// Sheet column K: `(J − J a year earlier) / J` — YoY living-spend change.
    pub living_yoy: Option<f64>,
    /// G 調整: Σ adjustment items.
    pub adjustment_sum: f64,
    /// Σ extra_spend items (I − J on the sheet).
    pub extra_spend_sum: f64,
    /// Σ income items (the L tail beyond salary − I).
    pub income_sum: f64,
    /// O 娛樂支出: Σ entertainment items.
    pub entertainment_sum: f64,
}

/// Upsert body for a month row. Absent fields are left untouched (or defaulted
/// on create); `null` on `total_assets`/`liquid_assets` restores live
/// derivation, and `recapture` re-snapshots the live totals.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MonthStatPatch {
    pub start_cash: Option<f64>,
    pub salary: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub total_assets: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub liquid_assets: Option<Option<f64>>,
    pub pool_input: Option<f64>,
    /// `null` clears the stored 月尾 and returns to the derived chain value.
    #[serde(default, deserialize_with = "nullable")]
    pub end_cash_override: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    pub recapture: Option<bool>,
}

/// One labeled line item of a month.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthItem {
    pub id: i64,
    pub month: String,
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Entertainment items only: also subtract from 生活支出.
    pub exclude_from_living: bool,
    /// Links the item to the app event it was suggested from; NULL = manual.
    pub auto_key: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewMonthItem {
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Entertainment items only: also subtract from 生活支出.
    #[serde(default)]
    pub exclude_from_living: bool,
    /// Set when accepting a suggestion.
    pub auto_key: Option<String>,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MonthItemPatch {
    pub category: Option<MonthItemCategory>,
    #[serde(default, deserialize_with = "nullable")]
    pub label: Option<Option<String>>,
    pub amount: Option<f64>,
    pub exclude_from_living: Option<bool>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// The kind of a manual balance: `cash` rows are the 活期 behind 月初 and the
/// liquid totals; `asset` rows feed only 總數.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManualAssetKind {
    Cash,
    Asset,
}

impl ManualAssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ManualAssetKind::Cash => "cash",
            ManualAssetKind::Asset => "asset",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cash" => Some(ManualAssetKind::Cash),
            "asset" => Some(ManualAssetKind::Asset),
            _ => None,
        }
    }
}

/// A named manual balance (the Overview cells B7/B8/B16/B17).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ManualAsset {
    pub id: i64,
    pub label: String,
    pub kind: ManualAssetKind,
    pub amount: f64,
    pub sort_order: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewManualAsset {
    pub label: String,
    pub kind: ManualAssetKind,
    pub amount: f64,
}

/// Absent fields are left untouched; present fields are written.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct ManualAssetPatch {
    pub label: Option<String>,
    pub kind: Option<ManualAssetKind>,
    pub amount: Option<f64>,
}

/// The month-stat settings held in `app_meta`: the current salary and the
/// per-year 開心Pool rate.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthSettings {
    pub salary: Option<f64>,
    /// The rate in effect for `pool_rate_year`.
    pub pool_rate: Option<f64>,
    pub pool_rate_year: i32,
}

/// Absent fields are left untouched; `null` clears `salary`/`pool_rate`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MonthSettingsPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub salary: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub pool_rate: Option<Option<f64>>,
    /// The year `pool_rate` applies to; defaults to the current year.
    pub pool_rate_year: Option<i32>,
}

/// A computed candidate item for a month, never stored until accepted.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthSuggestion {
    pub auto_key: String,
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Which app event produced it: deposit, trade, dividend, coupon, aia, pool.
    pub source: String,
}

/// One auto interest component of a month: a 定期 ending, a received bond
/// coupon, or a received HK dividend dated in the month. Deposit components
/// are `received: false` while the deposit is not yet 收訖 — they preview in
/// the breakdown but do not count in the derived 利息.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InterestComponent {
    /// `deposit`, `coupon` or `dividend`.
    pub source: String,
    pub label: Option<String>,
    /// The received amount, else the expected/estimated figure while pending;
    /// NULL when nothing is known yet (待定 coupon, estimate-less dividend).
    pub amount: Option<f64>,
    pub received: bool,
}

/// The 美股 sheet's IBKR account block (A1:B5 + B7): four manual inputs plus
/// the derived cross-checks. `now_value` is the account total as the IBKR app
/// displays it — its implied FX rate differs from `aia.usd_hkd_rate`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IbkrBlock {
    /// 美股!B1: cumulative bank→IBKR transfers — Σ `ibkr_transfers`.
    pub transferred_hkd: Option<f64>,
    /// 美股!B2: the account total shown in the IBKR app.
    pub now_value: Option<f64>,
    /// 美股!B4/B5: the account's cash positions.
    pub hkd_cash: Option<f64>,
    pub usd_cash: Option<f64>,
    /// US stock market value in USD (美股!E2).
    pub stock_value_usd: Option<f64>,
    /// 美股!B7 = (stock_value_usd + usd_cash) × rate + hkd_cash — the figure
    /// Overview!B9 links to.
    pub computed_total_hkd: Option<f64>,
    /// C1 = now_value − transferred_hkd; C2 = net ÷ transferred.
    pub net: Option<f64>,
    pub net_pct: Option<f64>,
    /// computed_total_hkd − now_value: the FX-gap cross-check.
    pub vs_now_value: Option<f64>,
}

/// Absent fields are left untouched; `null` clears a field. `transfer_hkd`
/// appends a dated row to `ibkr_transfers` instead — a positive value is a
/// bank→IBKR transfer, a negative one a withdrawal/correction.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct IbkrPatch {
    #[serde(default)]
    pub transfer_hkd: Option<f64>,
    /// Optional `YYYY-MM-DD` for the transfer; defaults to today.
    #[serde(default)]
    pub transfer_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub now_value: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub hkd_cash: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub usd_cash: Option<Option<f64>>,
}

/// One row of the 總覽 asset table (Overview!A3:C9): a module total or a
/// manual `asset` row, with its share of the sum (the C column).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OverviewAssetRow {
    pub key: String,
    pub label: String,
    /// The row's HKD amount; absent while rate-dependent and no rate is set.
    pub amount: Option<f64>,
    /// amount ÷ assets_sum.
    pub share: Option<f64>,
    /// Set when the row is a manual balance, enabling inline edits.
    pub manual_asset_id: Option<i64>,
}

/// The Overview!A14:C18 半流動資金 block plus the A13 ratio.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SemiLiquid {
    /// 已定期 (B15): Σ principal over active deposits.
    pub deposits: f64,
    /// The manual `cash` rows (B16/B17).
    pub cash_rows: Vec<ManualAsset>,
    /// 活期 (B18) = Σ cash rows.
    pub cash_sum: f64,
    /// 半流動資金 (B14) = deposits + cash_sum.
    pub total: f64,
    /// C14 = total − 25% × liquid_assets.
    pub vs_quarter_liquid: f64,
    /// A13 = total ÷ (港股 + 債券 + total + IBKR); absent while a term is missing.
    pub share: Option<f64>,
}

/// `Overview!F3:G10` (+`H6`): trailing averages over the 12 completed months
/// before the current one, the live pool balance, and the living budget with
/// its 預測 red-flag floor.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TwelveMonthAverages {
    /// G4 總數增加.
    pub total_change: Option<f64>,
    /// G5 支出.
    pub month_spend: Option<f64>,
    /// G6 生活支出.
    pub living_spend: Option<f64>,
    /// H6 生活預算 = ROUNDUP(living_spend × 1.05, −2).
    pub living_budget: Option<f64>,
    /// The 預測 check: true while `living_budget < living_budget_floor`.
    pub living_budget_low: bool,
    /// `liquid_assets × 0.0001 × 30 + 9000` — the floor the budget is
    /// compared against.
    pub living_budget_floor: f64,
    /// G7 存.
    pub saved: Option<f64>,
    /// G8 利息.
    pub interest: Option<f64>,
    /// G10 開心Pool — the live balance.
    pub pool_balance: f64,
    /// The window's first/last months (e.g. "2025-09-01").
    pub window_start: Option<String>,
    pub window_end: Option<String>,
}

/// The 投資目標 block (Overview!J22:N27): the J22 three-year invested average
/// plus one row per year-review year under the unified target formula.
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct InvestTargets {
    /// J22: mean `invested` over the last three completed years, skip-absent.
    pub avg_invested: Option<f64>,
    pub rows: Vec<InvestTargetRow>,
}

/// One 投資目標 row (J:N): invested, the effective raise, and the derived
/// target/remain/growth.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InvestTargetRow {
    pub year: i32,
    /// K column: the year-review `invested`; absent while none.
    pub invested: Option<f64>,
    /// The year's effective 月薪增幅 (override or salary-derived).
    pub raise: Option<f64>,
    /// L column: the year's target; absent without a prior-year `invested`.
    pub target: Option<f64>,
    /// M column: target − invested, on the current year only.
    pub remain: Option<f64>,
    /// N column: invested YoY for completed years; target vs last invested
    /// for the current year.
    pub growth: Option<f64>,
}

/// `GET /api/overview`: the sheet's A3:C18 block plus the B1/H1/J1 headline.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OverviewResponse {
    pub today: String,
    /// `aia.usd_hkd_rate`; absent while unset.
    pub rate: Option<f64>,
    /// `overview.salary`; feeds J1.
    pub salary: Option<f64>,
    /// B1 總數 = assets_sum + 半流動資金.
    pub total_assets: f64,
    /// H1 流動資產 = 港股 + 半流動資金 + 債券 + IBKR − 開心Pool.
    pub liquid_assets: f64,
    /// J1 = liquid_assets ÷ (salary × 100); absent without a salary.
    pub liquid_ratio: Option<f64>,
    pub assets: Vec<OverviewAssetRow>,
    /// B10 Sum = Σ present asset rows.
    pub assets_sum: f64,
    pub semi_liquid: SemiLiquid,
    pub ibkr: IbkrBlock,
    /// F3:G10 + H6.
    pub averages: TwelveMonthAverages,
    /// J22:N27.
    pub invest_targets: InvestTargets,
}
