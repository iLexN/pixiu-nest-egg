use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The month-stat settings held in `app_meta`: the current salary, the
/// per-year 開心Pool rate, and the forecast's 差餉 bill amount.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AppSettings {
    pub salary: Option<f64>,
    /// The rate in effect for `pool_rate_year`.
    pub pool_rate: Option<f64>,
    pub pool_rate_year: i32,
    /// `forecast.bill_amount` — the quarterly 差餉 charge in the forecast's
    /// 繳費 row; the effective value (2158 while unset).
    pub bill_amount: f64,
    /// `overview.semi_liquid_target` — the share of 流動資產 the 半流動資金
    /// buffer targets; the effective value (0.25 while unset).
    pub semi_liquid_target: f64,
    /// `money_master.start_date` — the bank challenge's start `YYYY-MM-DD`;
    /// `month_now` derives from it.
    pub money_master_start_date: Option<String>,
    /// `money_master.saved` — the bank app's saved figure (L31).
    pub money_master_saved: Option<f64>,
    /// `money_master.target_months`/`target_amount` — the challenge params
    /// (K32/L32).
    pub money_master_target_months: Option<f64>,
    pub money_master_target_amount: Option<f64>,
    /// `money_master.month_now`/`coming_save` — optional overrides pinning
    /// the derived figures while set.
    pub money_master_month_now: Option<f64>,
    pub money_master_coming_save: Option<f64>,
    /// `overview.goal1_amount`/`overview.goal2_amount` — the 1st/2nd 目標
    /// milestone targets (Overview!K12/K18); `null` while unset.
    pub goal1_amount: Option<f64>,
    pub goal2_amount: Option<f64>,
    /// `overview.goal3_inflation`/`overview.goal3_years` — the 3rd target's
    /// constants (the sheet's `1.07` and `25+5`); the effective values.
    pub goal3_inflation: f64,
    pub goal3_years: f64,
    /// `overview.final_replacement` — the Final Target's salary-replacement
    /// ratio (the sheet's `0.7`); the effective value.
    pub final_replacement: f64,
}

/// Absent fields are left untouched; `null` clears `salary`/`pool_rate`/
/// `bill_amount`/`semi_liquid_target` (a cleared bill amount or target falls
/// back to the default) and every `money_master_*` field — clearing
/// `month_now`/`coming_save` returns that figure to its derivation.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct AppSettingsPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub salary: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub pool_rate: Option<Option<f64>>,
    /// The year `pool_rate` applies to; defaults to the current year.
    pub pool_rate_year: Option<i32>,
    #[serde(default, deserialize_with = "nullable")]
    pub bill_amount: Option<Option<f64>>,
    /// `null` resets the buffer target to the 25% default.
    #[serde(default, deserialize_with = "nullable")]
    pub semi_liquid_target: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_start_date: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_saved: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_target_months: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_target_amount: Option<Option<f64>>,
    /// `null` clears the override so `month_now` derives from `start_date`.
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_month_now: Option<Option<f64>>,
    /// `null` clears the override so `coming_save` derives again.
    #[serde(default, deserialize_with = "nullable")]
    pub money_master_coming_save: Option<Option<f64>>,
    /// `null` clears the milestone amount, blanking that milestone's figures.
    #[serde(default, deserialize_with = "nullable")]
    pub goal1_amount: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub goal2_amount: Option<Option<f64>>,
    /// `null` resets the constant to its default (1.07 / 30 / 0.7).
    #[serde(default, deserialize_with = "nullable")]
    pub goal3_inflation: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub goal3_years: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub final_replacement: Option<Option<f64>>,
}
