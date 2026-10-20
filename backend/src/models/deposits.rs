use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Derived from `end_date`: `End` once the end date is today or past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum DepositStatus {
    Active,
    End,
}

/// A 定期 deposit record. `rate` (when not stored), `total`, `status`,
/// `end_year` and `end_month` are derived on read, never stored.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Deposit {
    pub id: i64,
    /// The sheet's `id` column: a bank reference like `SC-9632`.
    pub label: Option<String>,
    /// Bank code derived from the label prefix (SC = 渣打, HS = 恒生).
    pub bank: Option<String>,
    /// The sheet's `input` column.
    pub principal: Option<f64>,
    /// Annual rate as a fraction (0.03 = 3%): the stored rate when entered,
    /// else the rate derived on read from interest, principal and the
    /// start→end days.
    pub rate: Option<f64>,
    /// True when `rate` was derived on read (no stored rate) from interest,
    /// principal and the start→end days.
    pub rate_derived: bool,
    /// 利息.
    pub interest: Option<f64>,
    /// When the principal left the bank account; NULL when unknown.
    pub start_date: Option<String>,
    pub end_date: String,
    /// 收訖日: when the user confirmed the money came back; NULL while the
    /// deposit is still on the books (it stays in 未到期定期 until then).
    pub received_at: Option<String>,
    /// The cash account the 收訖 banked into; NULL when nothing was banked.
    pub credited_account_id: Option<i64>,
    /// The amount banked into `credited_account_id`.
    pub credited_amount: Option<f64>,
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
