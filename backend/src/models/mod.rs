//! The API request/response and stored-row types, split by domain. All types
//! are re-exported flat — `crate::models::X` keeps working everywhere.

mod aia;
mod bonds;
mod cash;
mod deposits;
mod dividends;
mod family;
mod forecast;
mod months;
mod mpf;
mod overview;
mod settings;
mod stocks;
mod trades;

pub use aia::*;
pub use bonds::*;
pub use cash::*;
pub use deposits::*;
pub use dividends::*;
pub use family::*;
pub use forecast::*;
pub use months::*;
pub use mpf::*;
pub use overview::*;
pub use settings::*;
pub use stocks::*;
pub use trades::*;

use serde::{Deserialize, Deserializer};

/// For PATCH bodies, distinguishes "field absent" from "field present as null".
fn nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}
