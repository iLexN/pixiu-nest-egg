use super::nullable;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    pub note: Option<String>,
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
    #[serde(default)]
    pub note: Option<String>,
}

/// Absent fields are left untouched; present fields are written, so `null`
/// clears an optional value.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct ManualAssetPatch {
    pub label: Option<String>,
    pub liquidity: Option<ManualAssetLiquidity>,
    pub amount: Option<f64>,
    #[serde(default, deserialize_with = "nullable")]
    pub note: Option<Option<String>>,
}
