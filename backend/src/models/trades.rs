use super::Market;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
