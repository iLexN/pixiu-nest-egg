use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Absent fields are left untouched; `null` clears a field. Transfers live
/// on their own resource — `POST /api/ibkr/transfers`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct IbkrPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub now_value: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub hkd_cash: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub usd_cash: Option<Option<f64>>,
}

/// One `ibkr_transfers` log row. `from_account_id`/`broker_account_id`
/// record the accounts a cash-moving transfer touched — NULL on log-only
/// legacy rows and, via `ON DELETE SET NULL`, on a deleted account.
/// `from_account_label` is the joined source label (NULL like the id).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IbkrTransfer {
    pub id: i64,
    /// `YYYY-MM-DD` — the day the cash moved.
    pub transfer_date: String,
    /// The transferred amount in HKD.
    pub amount_hkd: f64,
    /// The debited bank account; NULL on log-only rows.
    pub from_account_id: Option<i64>,
    /// The source account's label; NULL on log-only rows or a deleted source.
    pub from_account_label: Option<String>,
    /// The credited IBKR HKD broker account; NULL on log-only rows.
    pub broker_account_id: Option<i64>,
    pub created_at: String,
}

/// `POST /api/ibkr/transfers` body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewIbkrTransfer {
    /// Finite HKD amount, greater than 0.
    pub amount_hkd: f64,
    /// Optional `YYYY-MM-DD`, not after today; defaults to today.
    #[serde(default)]
    pub transfer_date: Option<String>,
    /// Optional source account id; defaults to the `HS` bank account.
    #[serde(default)]
    pub from_account_id: Option<i64>,
}
