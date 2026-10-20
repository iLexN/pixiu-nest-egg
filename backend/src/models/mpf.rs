use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A rate + net gain pair. `rate` is empty when contributions are zero.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct MpfFigures {
    pub rate: Option<f64>,
    pub gain: f64,
}

/// A 未實現報酬率 + 未實現金額 pair for a market's last-month or max figures.
/// `percent` is empty when the recorded `buy_cost_priced` is zero.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct MarketFigures {
    pub percent: Option<f64>,
    pub amount: f64,
}

/// An MPF (強積金) account. `rate`, `gain`, `last_month` and `max` are derived
/// on read from the stored values plus the history rows; `seed_max_*` are the
/// imported high-water marks that act as a floor for the reported maxima.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfAccount {
    pub id: i64,
    /// The sheet's account name, e.g. `new type`, `強積金個人帳戶`.
    pub label: String,
    pub trustee: Option<String>,
    /// 總供款額.
    pub contributions: f64,
    /// 帳戶結存.
    pub balance: f64,
    pub plan_name: Option<String>,
    pub member_no: Option<String>,
    pub sort_order: i64,
    /// (balance − contributions) ÷ contributions; empty when contributions = 0.
    pub rate: Option<f64>,
    /// balance − contributions.
    pub gain: f64,
    /// Latest history row in the previous calendar month, if any.
    pub last_month: Option<MpfFigures>,
    /// All-time maxima over the seed, every history row, and current values.
    /// Rate and gain are independent and may come from different moments.
    pub max: MpfFigures,
}

/// One recorded account state. Synthetic rows backfill months with no update.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MpfHistoryRow {
    pub id: i64,
    pub account_id: i64,
    /// YYYY-MM-DD.
    pub recorded_on: String,
    pub contributions: f64,
    pub balance: f64,
    pub synthetic: bool,
    pub rate: Option<f64>,
    pub gain: f64,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewMpfAccount {
    pub label: String,
    pub trustee: Option<String>,
    pub contributions: Option<f64>,
    pub balance: Option<f64>,
    pub plan_name: Option<String>,
    pub member_no: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value. Changing `contributions` or `balance` records a
/// history row; metadata-only edits do not.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MpfAccountPatch {
    pub label: Option<String>,
    #[serde(default, deserialize_with = "nullable")]
    pub trustee: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub contributions: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub balance: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub plan_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub member_no: Option<Option<String>>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MpfNotePatch {
    pub note: Option<String>,
}
