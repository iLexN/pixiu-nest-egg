use serde::{Deserialize, Deserializer, Serialize};

/// For PATCH bodies, distinguishes "field absent" from "field present as null".
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct RawPriceEntry {
    pub symbol: Option<String>,
    pub price: Option<f64>,
}

/// A price that was applied to a stored stock.
#[derive(Debug, Clone, Serialize)]
pub struct PriceUpdate {
    pub market: Market,
    pub code: String,
    pub symbol: String,
    pub price: f64,
}

/// A file entry that was skipped: missing symbol or non-positive price.
#[derive(Debug, Clone, Serialize)]
pub struct InvalidPriceEntry {
    pub symbol: Option<String>,
    pub price: Option<f64>,
    pub reason: String,
}

/// Result of a bulk 現價 upload.
#[derive(Debug, Clone, Serialize)]
pub struct PriceReport {
    pub updated: Vec<PriceUpdate>,
    /// File symbols that matched no stored stock.
    pub unmatched: Vec<String>,
    /// File entries that could not be applied.
    pub invalid: Vec<InvalidPriceEntry>,
    /// Stored stocks (as `MARKET code`) with no entry in the file.
    pub not_updated: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DepositStatus {
    Active,
    End,
}

/// A 定期 deposit record. `total`, `status`, `end_year` and `end_month` are
/// derived on read, never stored.
#[derive(Debug, Clone, Serialize)]
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
    pub end_date: String,
    pub note1: Option<String>,
    pub note2: Option<String>,
    pub sort_order: i64,
    /// principal + interest, blanks counting as 0.
    pub total: f64,
    pub status: DepositStatus,
    pub end_year: i32,
    pub end_month: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewDeposit {
    pub label: Option<String>,
    pub bank: Option<String>,
    pub principal: Option<f64>,
    pub rate: Option<f64>,
    pub interest: Option<f64>,
    pub end_date: String,
    pub note1: Option<String>,
    pub note2: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize)]
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
    pub end_date: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub note1: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note2: Option<Option<String>>,
}

/// Pending until `received_amount` is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum DividendStatus {
    Pending,
    Received,
}

/// A 派息 record. `shares_held`, `buy_cost` and `received_price` are
/// point-in-time snapshots stored at write time; `status`, `amount`,
/// `yield_on_cost`, `yield_on_price` and `variance` are derived on read.
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. `refresh_snapshots` re-derives shares_held and
/// buy_cost from trades on or before the (possibly edited) pay_date.
#[derive(Debug, Clone, Default, Deserialize)]
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
}
