use super::InterestComponent;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Which 預測 row a forecast item belongs to. `hs_deposit`/`sc_deposit` are
/// planned lockups that schedule a derived return; the rest are plain signed
/// lines (`bill` items replace the quarter-month default rather than stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForecastItemKind {
    /// Sheet row 29: planned 定期 at HS (returns +3 months).
    HsDeposit,
    /// Sheet row 30: planned SC高息馬拉松 (returns +4 months).
    ScDeposit,
    /// Sheet row 31 tail: extra known interest beyond the auto receipts.
    Interest,
    /// Sheet row 32: Tax/基金/醫療保險.
    Tax,
    /// Sheet row 33: planned stock buys/sells.
    Stock,
    /// Sheet row 34: 繳費 — replaces the quarterly default when present.
    Bill,
    /// Sheet row 36: TBC.
    Other,
}

impl ForecastItemKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ForecastItemKind::HsDeposit => "hs_deposit",
            ForecastItemKind::ScDeposit => "sc_deposit",
            ForecastItemKind::Interest => "interest",
            ForecastItemKind::Tax => "tax",
            ForecastItemKind::Stock => "stock",
            ForecastItemKind::Bill => "bill",
            ForecastItemKind::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "hs_deposit" => Some(ForecastItemKind::HsDeposit),
            "sc_deposit" => Some(ForecastItemKind::ScDeposit),
            "interest" => Some(ForecastItemKind::Interest),
            "tax" => Some(ForecastItemKind::Tax),
            "stock" => Some(ForecastItemKind::Stock),
            "bill" => Some(ForecastItemKind::Bill),
            "other" => Some(ForecastItemKind::Other),
            _ => None,
        }
    }

    /// The default months until a planned lockup's principal returns (the
    /// sheet's row-35 lags); `None` for non-deposit kinds.
    pub fn deposit_lag(self) -> Option<u32> {
        match self {
            ForecastItemKind::HsDeposit => Some(3),
            ForecastItemKind::ScDeposit => Some(4),
            _ => None,
        }
    }

    /// The bank code a convert writes when the deposit form leaves it blank.
    pub fn default_bank(self) -> Option<&'static str> {
        match self {
            ForecastItemKind::HsDeposit => Some("HS"),
            ForecastItemKind::ScDeposit => Some("SC"),
            _ => None,
        }
    }
}

/// One stored 預測 plan row (the sheet's manual cells as a list).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ForecastItem {
    pub id: i64,
    /// `YYYY-MM-01`: the month the cash line lands in.
    pub month: String,
    pub kind: ForecastItemKind,
    /// Signed amount (planned lockups are negative).
    pub amount: f64,
    /// `YYYY-MM-01` override for the return month; NULL uses the kind's lag.
    pub return_month: Option<String>,
    pub note: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewForecastItem {
    pub kind: ForecastItemKind,
    pub amount: f64,
    /// `YYYY-MM` or `YYYY-MM-01`; deposit kinds only.
    pub return_month: Option<String>,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears `return_month`/`note`. Switching a deposit kind to a non-deposit
/// kind clears the stored `return_month`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct ForecastItemPatch {
    pub kind: Option<ForecastItemKind>,
    pub amount: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub return_month: Option<Option<String>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// A real deposit placed in the month (`start_date` inside it) — shown
/// read-only in its bank's plan row alongside the editable `plan_items`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ForecastPlacement {
    /// The `deposits` row id.
    pub deposit_id: i64,
    /// The deposit's label (`SC-9632`-style), absent while unset.
    pub label: Option<String>,
    /// The deposit's bank; `HS` maps to the 定期 HS row, anything else to
    /// SC高息馬拉松.
    pub bank: Option<String>,
    pub principal: f64,
    /// `YYYY-MM-DD` — the day the money left 活期.
    pub start_date: String,
}

/// One month column of the Overview 預測 grid (sheet columns B:H).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ForecastMonth {
    /// `YYYY-MM-01`, the first of the projected month.
    pub month: String,
    /// Row 25 start: the first column's 月初(出糧後), later the prior column's
    /// `cash`.
    pub start: Option<f64>,
    /// Row 26: `overview.salary`; absent while unset.
    pub salary: Option<f64>,
    /// Row 27 支出: −生活預算; absent while the trailing window is empty.
    pub spend: Option<f64>,
    /// Row 28 定期 finish: Σ principal of deposits ending in the month.
    pub deposit_finish: f64,
    /// Row 31 利息: deposit interest ending + expected/received HK dividends
    /// and bond coupons in the month + `interest` items.
    pub interest: f64,
    /// The auto receipts behind `interest` (deposit/coupon/dividend) — unlike
    /// the month stat's breakdown, pending components DO count here at their
    /// expected figure; `received` marks confirmed ones.
    pub interest_components: Vec<InterestComponent>,
    /// Row 34 繳費: −bill_amount on Jan/Apr/Jul/Oct, or Σ `bill` items when
    /// any exist; absent otherwise.
    pub bill: Option<f64>,
    /// The month's stored forecast items — the sheet's editable cells.
    pub plan_items: Vec<ForecastItem>,
    /// The month's real deposit placements (`start_date` inside it), read-only
    /// lines in the deposit rows.
    pub deposit_placements: Vec<ForecastPlacement>,
    /// Row 35 TBC - 定期 end: Σ principal returning from earlier plans.
    pub deposit_return: f64,
    /// Row 23 活期 = Σ(start..deposit_return) − deposit placements; absent
    /// while `start` is.
    pub cash: Option<f64>,
    /// Row 24 定期+SC: prior `locked` − finish − plans + placements − returns,
    /// anchored on the deposits held on the window's first day.
    pub locked: Option<f64>,
    /// Row 21 半流動 = `cash` + `locked`.
    pub semi_liquid: Option<f64>,
    /// Row 20 = `semi_liquid` − 流動資產 ÷ 4; absent while either side is.
    pub ref_check: Option<f64>,
}

/// `GET /api/forecast`: seven columns from the current month.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ForecastResponse {
    /// The effective quarterly bill amount (`forecast.bill_amount` or 2158).
    pub bill_amount: f64,
    pub months: Vec<ForecastMonth>,
}
