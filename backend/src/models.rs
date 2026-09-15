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
