use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An AIA policy row. `balance_pct` is derived on read; the totals flags
/// reproduce the sheet's two sums: `excluded` rows are in the account but not
/// the user's money (irene 20%), `in_account` rows count in `display_value`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AiaPolicy {
    pub id: i64,
    /// Plan or group name, e.g. `年金 - 2024 - 2029`.
    pub label: String,
    /// e.g. `B632611401`.
    pub policy_no: Option<String>,
    /// Next premium-due date.
    pub next_pay_date: Option<String>,
    /// The sheet's `buy usd`.
    pub premium_usd: f64,
    /// The sheet's `now usd`.
    pub value_usd: f64,
    pub value_updated_at: Option<String>,
    pub remaining_years: Option<f64>,
    /// The sheet's `Withdrew`, cumulative.
    pub withdrew_usd: f64,
    pub note: Option<String>,
    /// The remark column's hyperlink target (manual entry).
    pub link: Option<String>,
    /// Not counted in the portfolio totals.
    pub excluded: bool,
    /// Counted in `display_value`.
    pub in_account: bool,
    pub sort_order: i64,
    /// (value_usd + withdrew_usd − premium_usd) ÷ premium_usd; absent at 0.
    pub balance_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewAiaPolicy {
    pub label: String,
    pub policy_no: Option<String>,
    pub next_pay_date: Option<String>,
    pub premium_usd: f64,
    pub value_usd: f64,
    pub remaining_years: Option<f64>,
    pub withdrew_usd: f64,
    pub note: Option<String>,
    pub link: Option<String>,
    #[serde(default)]
    pub excluded: bool,
    #[serde(default = "default_true")]
    pub in_account: bool,
}

fn default_true() -> bool {
    true
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. Changing `value_usd` refreshes
/// `value_updated_at`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AiaPolicyPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub policy_no: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub next_pay_date: Option<Option<String>>,
    pub premium_usd: Option<f64>,
    pub value_usd: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub remaining_years: Option<Option<f64>>,
    pub withdrew_usd: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub link: Option<Option<String>>,
    pub excluded: Option<bool>,
    pub in_account: Option<bool>,
}

/// One recorded premium payment or withdrawal. `prev_next_pay_date` /
/// `prev_remaining_years` snapshot the policy fields a payment touched so
/// deleting the event restores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AiaEventKind {
    Payment,
    Withdrawal,
}

impl AiaEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AiaEventKind::Payment => "payment",
            AiaEventKind::Withdrawal => "withdrawal",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "payment" => Some(AiaEventKind::Payment),
            "withdrawal" => Some(AiaEventKind::Withdrawal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AiaEvent {
    pub id: i64,
    pub policy_id: i64,
    pub kind: AiaEventKind,
    pub event_date: String,
    pub amount_usd: f64,
    pub note: Option<String>,
    pub prev_next_pay_date: Option<String>,
    pub prev_remaining_years: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewAiaEvent {
    pub policy_id: i64,
    pub kind: AiaEventKind,
    pub event_date: String,
    pub amount_usd: f64,
    pub note: Option<String>,
    /// Explicit new next premium-due date for payments; defaults to the
    /// policy's current `next_pay_date` plus one year.
    pub next_pay_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AiaNotePatch {
    pub note: Option<String>,
}
