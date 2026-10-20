use super::DepositStatus;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
