//! Storage layer: the only place that issues SQL. Functions here return
//! `StoreError`, which carries no HTTP semantics; the routes layer maps it to
//! the HTTP error type.

pub mod aia;
pub mod bonds;
pub mod cash_accounts;
pub mod deposits;
pub mod dividends;
pub mod family;
pub mod forecast;
pub mod fx;
pub mod ibkr;
pub mod market_history;
pub mod months;
pub mod mpf;
pub mod settings;
pub mod stocks;
pub mod trades;
pub mod year_review;
pub mod yearly;

pub(super) mod meta;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Corrupt(String),
    /// A state conflict the caller maps to HTTP 409 (e.g. deleting a
    /// protected cash account).
    #[error("{0}")]
    Conflict(String),
}

/// A recorded receipt credit: the cash account it banked into and the amount
/// added there, so a reversal subtracts from the same account.
#[derive(Debug, Clone, Copy)]
pub struct Credit {
    pub account_id: i64,
    pub amount: f64,
}

pub(crate) fn now_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Local today: what deposit status derives from.
pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}
