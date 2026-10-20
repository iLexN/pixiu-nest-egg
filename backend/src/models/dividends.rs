use super::Market;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pending until `received_amount` is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DividendStatus {
    Pending,
    Received,
}

/// A 派息 record. `shares_held`, `buy_cost` and `received_price` are
/// point-in-time snapshots stored at write time; `status`, `amount`,
/// `yield_on_cost`, `yield_on_price`, `variance` and `gross_amount` are
/// derived on read.
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
    /// 實收派息 (net of withholding tax); NULL while pending.
    pub received_amount: Option<f64>,
    /// 現價 snapshot at receipt (the sheet's N denominator).
    pub received_price: Option<f64>,
    /// Tax withheld from the gross 派息; NULL when none was recorded.
    pub withholding_tax: Option<f64>,
    /// 派息 gross: received_amount + withholding_tax when both are present.
    pub gross_amount: Option<f64>,
    /// The cash account the dividend banked into; NULL when nothing was banked.
    pub credited_account_id: Option<i64>,
    /// The amount banked into `credited_account_id`.
    pub credited_amount: Option<f64>,
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
    /// Input-only 派息 gross; resolves to received_amount = gross − tax.
    #[serde(default)]
    pub gross_amount: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub withholding_tax: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    #[serde(default)]
    pub refresh_snapshots: bool,
    /// On 收訖 (received_amount NULL → set), bank the amount: HK dividends go
    /// to the default HKD account, US to the default USD (IBKR) account.
    /// Defaults to on; only meaningful during the receipt transition.
    #[serde(default)]
    pub bank_in: Option<bool>,
    /// The chosen receipt account; falls back to the currency default.
    #[serde(default)]
    pub account_id: Option<i64>,
}
