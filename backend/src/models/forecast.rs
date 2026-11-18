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
    /// Every kind in the sheet's plan-row order — the order of a month's
    /// `cells`.
    pub const ALL: [ForecastItemKind; 7] = [
        ForecastItemKind::HsDeposit,
        ForecastItemKind::ScDeposit,
        ForecastItemKind::Interest,
        ForecastItemKind::Tax,
        ForecastItemKind::Stock,
        ForecastItemKind::Bill,
        ForecastItemKind::Other,
    ];

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

    /// The plan row a real deposit placement lands in: bank `HS` → 定期 HS,
    /// any other bank (or none) → SC高息馬拉松 — the reverse of
    /// [`Self::default_bank`].
    pub fn placement_row(bank: Option<&str>) -> ForecastItemKind {
        match bank {
            Some("HS") => ForecastItemKind::HsDeposit,
            _ => ForecastItemKind::ScDeposit,
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
