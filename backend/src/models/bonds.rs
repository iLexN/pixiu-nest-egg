use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    /// The cash account the principal banked into; NULL when nothing was banked.
    pub credited_account_id: Option<i64>,
    /// The amount banked into `credited_account_id`.
    pub credited_amount: Option<f64>,
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
    /// The cash account the coupon banked into; NULL when nothing was banked.
    pub credited_account_id: Option<i64>,
    /// The amount banked into `credited_account_id`.
    pub credited_amount: Option<f64>,
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
    /// On 收訖 (received_amount NULL → set), bank the amount into a cash
    /// account. Defaults to on; only meaningful during the receipt transition.
    #[serde(default)]
    pub bank_in: Option<bool>,
    /// The chosen receipt account; falls back to the currency default.
    #[serde(default)]
    pub account_id: Option<i64>,
}
