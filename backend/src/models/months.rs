use super::Market;
use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Stored frozen figures for one (market, year) in the yearly summary.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct YearSnapshot {
    pub market: Market,
    pub year: i32,
    /// Manual override for the year's net invested; NULL falls back to
    /// Σ BUY − Σ SELL of the year.
    pub invested: Option<f64>,
    /// Frozen 年末總成本; NULL falls back to cumulative Σ BUY total.
    pub cost: Option<f64>,
    /// Frozen 年末總市值; NULL stays live for the current year, empty for
    /// past years.
    pub market_value: Option<f64>,
    /// 賣出損益: the year's realized sell P/L, entered by hand — the
    /// workbook never recorded SELL trades. NULL reports it absent.
    pub sold_pl: Option<f64>,
    pub updated_at: String,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears a stored value and the column falls back to the computed figure.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct YearlyPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub invested: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub cost: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub market_value: Option<Option<f64>>,
    /// Unlike the other figures, sold P/L may be negative.
    #[serde(default, deserialize_with = "nullable")]
    pub sold_pl: Option<Option<f64>>,
}

/// `PATCH /api/year-review/:year` body. Absent fields are left untouched;
/// `null` clears an override so the figure derives live again. `sold_pl`
/// writes the year's HK `year_snapshots` row instead of `year_review`.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct YearReviewPatch {
    #[serde(default, deserialize_with = "nullable")]
    pub income: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub invested_adjustment: Option<Option<f64>>,
    /// 月薪增幅 override; unlike income it may be negative. `null` restores
    /// the salary-derived figure.
    #[serde(default, deserialize_with = "nullable")]
    pub raise: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub sold_pl: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bond_principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub bond_interest: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deposit_principal: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub deposit_interest: Option<Option<f64>>,
}

/// The month-item categories replacing the sheet's opaque sum columns:
/// `adjustment` is the G column (bank in/out that must not count as 支出),
/// `extra_spend` the extras subtracted inside J, `income` the extras added
/// inside L, `entertainment` the O 娛樂支出 items, `interest` the hand-kept
/// part of N 利息 the auto events do not cover (bank 活期 interest, promos).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MonthItemCategory {
    Adjustment,
    ExtraSpend,
    Income,
    Entertainment,
    Interest,
}

impl MonthItemCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            MonthItemCategory::Adjustment => "adjustment",
            MonthItemCategory::ExtraSpend => "extra_spend",
            MonthItemCategory::Income => "income",
            MonthItemCategory::Entertainment => "entertainment",
            MonthItemCategory::Interest => "interest",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "adjustment" => Some(MonthItemCategory::Adjustment),
            "extra_spend" => Some(MonthItemCategory::ExtraSpend),
            "income" => Some(MonthItemCategory::Income),
            "entertainment" => Some(MonthItemCategory::Entertainment),
            "interest" => Some(MonthItemCategory::Interest),
            _ => None,
        }
    }
}

/// One month of the Month Stat ledger (`YYYY-MM-01`). Only entered figures are
/// stored; `interest`, `end_cash`, `month_spend`, `living_spend`, `saved`,
/// `total_change` and `liquid_change` are derived on read. `total_assets`/`liquid_assets`
/// report the effective value — stored when frozen, live-derived when NULL —
/// and the `_live` flags tell the UI which.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthStat {
    pub month: String,
    /// F 月初(出糧後): the 活期 total right after salary lands.
    pub start_cash: Option<f64>,
    /// The salary in effect that month, snapshotted at creation.
    pub salary: Option<f64>,
    /// B 總數 (effective: frozen value or live-derived).
    pub total_assets: Option<f64>,
    /// D 流動資產 (effective: frozen value or live-derived).
    pub liquid_assets: Option<f64>,
    /// True while `total_assets` is live-derived rather than stored.
    pub total_assets_live: bool,
    /// True while `liquid_assets` is live-derived rather than stored.
    pub liquid_assets_live: bool,
    /// N 利息 (derived): auto events + Σ interest items.
    pub interest: f64,
    /// P Irene + 開心 Pool.
    pub pool_input: f64,
    /// Hand-frozen H 月尾 value for months the `=F(n+1) − salary` chain cannot
    /// reproduce (e.g. the first ledger month's typed bank balance).
    pub end_cash_override: Option<f64>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// H 月尾(出糧前): the stored override, else next row's start_cash −
    /// salary; absent on the latest row.
    pub end_cash: Option<f64>,
    /// I 月支出: start_cash + Σadjustment − end_cash.
    pub month_spend: Option<f64>,
    /// J 生活支出: month_spend − Σextra_spend − Σ flagged entertainment.
    pub living_spend: Option<f64>,
    /// L 存: salary − month_spend + Σincome.
    pub saved: Option<f64>,
    /// C Changed: next row's total_assets − this row's.
    pub total_change: Option<f64>,
    /// E Changed: next row's liquid_assets − this row's.
    pub liquid_change: Option<f64>,
    /// Sheet column K: `(J − J a year earlier) / J` — YoY living-spend change.
    pub living_yoy: Option<f64>,
    /// G 調整: Σ adjustment items.
    pub adjustment_sum: f64,
    /// Σ extra_spend items (I − J on the sheet).
    pub extra_spend_sum: f64,
    /// Σ income items (the L tail beyond salary − I).
    pub income_sum: f64,
    /// O 娛樂支出: Σ entertainment items.
    pub entertainment_sum: f64,
}

/// Upsert body for a month row. Absent fields are left untouched (or defaulted
/// on create); `null` on `total_assets`/`liquid_assets` restores live
/// derivation, and `recapture` re-snapshots the live totals.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MonthStatPatch {
    pub start_cash: Option<f64>,
    pub salary: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub total_assets: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub liquid_assets: Option<Option<f64>>,
    pub pool_input: Option<f64>,
    /// `null` clears the stored 月尾 and returns to the derived chain value.
    #[serde(default, deserialize_with = "nullable")]
    pub end_cash_override: Option<Option<f64>>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
    pub recapture: Option<bool>,
}

/// One labeled line item of a month.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthItem {
    pub id: i64,
    pub month: String,
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Entertainment items only: also subtract from 生活支出.
    pub exclude_from_living: bool,
    /// Links the item to the app event it was suggested from; NULL = manual.
    pub auto_key: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewMonthItem {
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Entertainment items only: also subtract from 生活支出.
    #[serde(default)]
    pub exclude_from_living: bool,
    /// Set when accepting a suggestion.
    pub auto_key: Option<String>,
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MonthItemPatch {
    pub category: Option<MonthItemCategory>,
    #[serde(default, deserialize_with = "nullable")]
    pub label: Option<Option<String>>,
    pub amount: Option<f64>,
    pub exclude_from_living: Option<bool>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}

/// How quickly a manual `asset` row can be recovered: `short` rows count in
/// the Overview 策略 block's K6 短期可取回, `long` rows in K7 長期可取回.
/// `cash` rows carry the column but it has no effect on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManualAssetLiquidity {
    Short,
    #[default]
    Long,
}

impl ManualAssetLiquidity {
    pub fn as_str(self) -> &'static str {
        match self {
            ManualAssetLiquidity::Short => "short",
            ManualAssetLiquidity::Long => "long",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "short" => Some(ManualAssetLiquidity::Short),
            "long" => Some(ManualAssetLiquidity::Long),
            _ => None,
        }
    }
}

/// A named manual balance (the Overview cells B7/B8/B16/B17).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ManualAsset {
    pub id: i64,
    pub label: String,
    pub liquidity: ManualAssetLiquidity,
    pub amount: f64,
    pub sort_order: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewManualAsset {
    pub label: String,
    /// Absent defaults to `long`.
    #[serde(default)]
    pub liquidity: Option<ManualAssetLiquidity>,
    pub amount: f64,
}

/// Absent fields are left untouched; present fields are written.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct ManualAssetPatch {
    pub label: Option<String>,
    pub liquidity: Option<ManualAssetLiquidity>,
    pub amount: Option<f64>,
}

/// A computed candidate item for a month, never stored until accepted.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MonthSuggestion {
    pub auto_key: String,
    pub category: MonthItemCategory,
    pub label: Option<String>,
    pub amount: f64,
    /// Which app event produced it: deposit, trade, dividend, coupon, aia, pool.
    pub source: String,
}

/// One auto interest component of a month: a 定期 ending, a received bond
/// coupon, or a received HK dividend dated in the month. Deposit components
/// are `received: false` while the deposit is not yet 收訖 — they preview in
/// the breakdown but do not count in the derived 利息.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InterestComponent {
    /// `deposit`, `coupon` or `dividend`.
    pub source: String,
    pub label: Option<String>,
    /// The received amount, else the expected/estimated figure while pending;
    /// NULL when nothing is known yet (待定 coupon, estimate-less dividend).
    pub amount: Option<f64>,
    pub received: bool,
}
