use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::nullable;
use super::{Currency, InputMode};

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

    /// The currency the market's money is held in.
    pub fn currency(self) -> Currency {
        match self {
            Market::Hk => Currency::Hkd,
            Market::Us => Currency::Usd,
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
