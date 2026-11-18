//! The 月結 module: `book` is the assembled ledger every month-derived read
//! opens; `read` holds the page envelopes; `write` holds the mutation
//! workflows.

mod book;
mod read;
mod write;

#[cfg(test)]
pub(crate) use book::load_events;
pub use book::{MonthBook, live_from};
pub(crate) use book::{load_interest_events, stat_rows_no_live};
pub use read::{MonthDetailResponse, MonthSummaryResponse, detail, list, summary};
pub use write::{create_item, update_item, upsert};
