use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A currency that cash accounts and fx rates support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum Currency {
    #[serde(rename = "HKD")]
    Hkd,
    #[serde(rename = "USD")]
    Usd,
}

impl Currency {
    pub fn as_str(self) -> &'static str {
        match self {
            Currency::Hkd => "HKD",
            Currency::Usd => "USD",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "HKD" => Some(Currency::Hkd),
            "USD" => Some(Currency::Usd),
            _ => None,
        }
    }
}

/// `bank` = a 活期 account (HKD only); `broker` = the IBKR cash in that
/// currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CashRole {
    Bank,
    Broker,
}

impl CashRole {
    pub fn as_str(self) -> &'static str {
        match self {
            CashRole::Bank => "bank",
            CashRole::Broker => "broker",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "bank" => Some(CashRole::Bank),
            "broker" => Some(CashRole::Broker),
            _ => None,
        }
    }
}

/// One cash account — a 活期 bank row or the IBKR broker cash.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CashAccount {
    pub id: i64,
    pub label: String,
    pub currency: Currency,
    pub role: CashRole,
    pub amount: f64,
    /// The bank-in target for the currency's receipts; at most one per currency.
    pub is_default_receipt: bool,
    pub sort_order: i64,
    pub updated_at: String,
}

/// A stored exchange rate to HKD (`fx_rates`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FxRate {
    pub currency: Currency,
    /// HKD per one unit of `currency`.
    pub rate_to_hkd: f64,
    pub updated_at: String,
}

/// `PATCH /api/fx-rates/{currency}`: a number stores the rate, `null`/absent
/// clears it.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct FxRatePatch {
    #[serde(default)]
    pub rate: Option<f64>,
}

/// The fx rate entry a PATCH returns: `rate_to_hkd`/`updated_at` are null
/// right after a clear.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FxRateEntry {
    pub currency: Currency,
    pub rate_to_hkd: Option<f64>,
    pub updated_at: Option<String>,
}

/// `POST /api/cash-accounts` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewCashAccount {
    pub label: String,
    pub currency: Currency,
    pub role: CashRole,
    #[serde(default)]
    pub amount: Option<f64>,
    #[serde(default)]
    pub is_default_receipt: Option<bool>,
}

/// `PATCH /api/cash-accounts/{id}`: any subset of the fields.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct CashAccountPatch {
    pub label: Option<String>,
    pub currency: Option<Currency>,
    pub role: Option<CashRole>,
    pub amount: Option<f64>,
    pub is_default_receipt: Option<bool>,
}
