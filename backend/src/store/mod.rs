//! Storage layer: the only place that issues SQL. Functions here return
//! `StoreError`, which carries no HTTP semantics; the routes layer maps it to
//! the HTTP error type.

pub mod aia;
pub mod bonds;
pub mod deposits;
pub mod dividends;
pub mod family;
pub mod forecast;
pub mod market_history;
pub mod meta;
pub mod months;
pub mod mpf;
pub mod overview;
pub mod stocks;
pub mod trades;
pub mod year_review;
pub mod yearly;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Corrupt(String),
}

pub(crate) fn now_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Local today: what deposit status derives from.
pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}
