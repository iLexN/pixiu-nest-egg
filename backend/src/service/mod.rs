//! Service layer: composition across stores and calculations. Functions here
//! return `StoreError` and may call `store::*` and `calc::*`, never
//! `routes::*`. Routes own HTTP concerns and map `StoreError` to `ApiError`.

pub mod bonds;
pub mod dividends;
pub mod months;
pub mod mpf;
pub mod overview;
pub mod summary;
pub mod trades;
pub mod year_review;
pub mod yearly;
