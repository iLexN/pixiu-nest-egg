//! Service layer: composition across stores and calculations. Functions here
//! return `StoreError`/`ServiceError` and may call `store::*` and `calc::*`,
//! never `routes::*`. Routes own HTTP concerns and map errors to `ApiError`.

use crate::store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("validation failed")]
    Validation(Vec<crate::calc::FieldError>),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
}

impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError::Store(StoreError::Database(err))
    }
}

pub mod bonds;
pub mod cash_accounts;
pub mod deposits;
pub mod dividends;
pub mod months;
pub mod mpf;
pub mod overview;
pub mod receipts;
pub mod settings;
pub mod summary;
pub mod trades;
pub mod year_review;
pub mod yearly;
