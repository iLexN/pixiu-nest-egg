pub mod aia;
pub mod bonds;
pub mod deposits;
pub mod dividends;
pub mod family;
pub mod forecast;
pub mod months;
pub mod mpf;
pub mod overview;
pub mod stocks;
pub mod summary;
pub mod trades;
pub mod year_review;
pub mod yearly;

use axum::Router;
use sqlx::SqlitePool;
use utoipa::OpenApi;

use crate::error::ApiError;
use crate::models::Market;
use chrono::Datelike;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

#[derive(utoipa::OpenApi)]
#[openapi(info(title = "Wealth Report API", version = "0.1.0"))]
struct ApiDoc;

/// Build the API router and the OpenAPI document describing it. Routes MUST be
/// registered via `routes!` — a plain `.route()` on an `OpenApiRouter` compiles
/// but silently leaves the endpoint out of the doc.
pub fn api_router(state: AppState) -> (Router, utoipa::openapi::OpenApi) {
    let (router, api) = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(stocks::list, stocks::create))
        .routes(utoipa_axum::routes!(stocks::reorder))
        .routes(utoipa_axum::routes!(stocks::upload_prices))
        .routes(utoipa_axum::routes!(stocks::update, stocks::remove))
        .routes(utoipa_axum::routes!(trades::list, trades::create))
        .routes(utoipa_axum::routes!(trades::update, trades::remove))
        .routes(utoipa_axum::routes!(summary::show))
        .routes(utoipa_axum::routes!(yearly::list))
        .routes(utoipa_axum::routes!(yearly::update))
        .routes(utoipa_axum::routes!(yearly::freeze))
        .routes(utoipa_axum::routes!(year_review::list))
        .routes(utoipa_axum::routes!(year_review::update))
        .routes(utoipa_axum::routes!(deposits::list, deposits::create))
        .routes(utoipa_axum::routes!(deposits::summary))
        .routes(utoipa_axum::routes!(deposits::update, deposits::remove))
        .routes(utoipa_axum::routes!(deposits::receive))
        .routes(utoipa_axum::routes!(family::list, family::create))
        .routes(utoipa_axum::routes!(family::summary))
        .routes(utoipa_axum::routes!(family::update, family::remove))
        .routes(utoipa_axum::routes!(family::receive))
        .routes(utoipa_axum::routes!(family::update_note))
        .routes(utoipa_axum::routes!(dividends::list, dividends::create))
        .routes(utoipa_axum::routes!(dividends::summary))
        .routes(utoipa_axum::routes!(dividends::update, dividends::remove))
        .routes(utoipa_axum::routes!(bonds::list, bonds::create))
        .routes(utoipa_axum::routes!(bonds::summary))
        .routes(utoipa_axum::routes!(bonds::update, bonds::remove))
        .routes(utoipa_axum::routes!(bonds::receive))
        .routes(utoipa_axum::routes!(
            bonds::list_coupons,
            bonds::create_coupon
        ))
        .routes(utoipa_axum::routes!(
            bonds::update_coupon,
            bonds::remove_coupon
        ))
        .routes(utoipa_axum::routes!(mpf::overview))
        .routes(utoipa_axum::routes!(mpf::create))
        .routes(utoipa_axum::routes!(mpf::update, mpf::remove))
        .routes(utoipa_axum::routes!(mpf::update_note))
        .routes(utoipa_axum::routes!(mpf::remove_history))
        .routes(utoipa_axum::routes!(aia::list, aia::create))
        .routes(utoipa_axum::routes!(aia::summary))
        .routes(utoipa_axum::routes!(aia::update_rate))
        .routes(utoipa_axum::routes!(aia::update_note))
        .routes(utoipa_axum::routes!(aia::update, aia::remove))
        .routes(utoipa_axum::routes!(aia::list_events, aia::create_event))
        .routes(utoipa_axum::routes!(aia::remove_event))
        .routes(utoipa_axum::routes!(months::list))
        .routes(utoipa_axum::routes!(months::summary))
        .routes(utoipa_axum::routes!(
            months::settings,
            months::update_settings
        ))
        .routes(utoipa_axum::routes!(
            months::show,
            months::upsert,
            months::remove
        ))
        .routes(utoipa_axum::routes!(months::create_item))
        .routes(utoipa_axum::routes!(months::dismiss_item))
        .routes(utoipa_axum::routes!(
            months::update_item,
            months::remove_item
        ))
        .routes(utoipa_axum::routes!(
            months::list_assets,
            months::create_asset
        ))
        .routes(utoipa_axum::routes!(
            months::update_asset,
            months::remove_asset
        ))
        .routes(utoipa_axum::routes!(overview::overview))
        .routes(utoipa_axum::routes!(overview::ibkr, overview::update_ibkr))
        .routes(utoipa_axum::routes!(forecast::forecast))
        .routes(utoipa_axum::routes!(forecast::create_item))
        .routes(utoipa_axum::routes!(
            forecast::update_item,
            forecast::remove_item
        ))
        .routes(utoipa_axum::routes!(forecast::convert))
        .split_for_parts();
    (router.with_state(state), api)
}

/// Resolve the stock a trade or dividend refers to — by `stock_id`, or by
/// market + 股票代碼 when the caller only has the code. `service::trades`
/// does the lookup; the NotFound carries the message this maps to a field
/// error.
pub async fn resolve_stock(
    pool: &SqlitePool,
    stock_id: Option<i64>,
    market: Option<Market>,
    code: Option<&str>,
) -> Result<(i64, Market), ApiError> {
    crate::service::trades::resolve_stock(pool, stock_id, market, code)
        .await
        .map_err(|err| match err {
            crate::store::StoreError::NotFound(message) => {
                let field = if stock_id.is_some() {
                    "stock_id"
                } else if market.is_some() && code.is_some_and(|c| !c.trim().is_empty()) {
                    "code"
                } else {
                    "stock_id"
                };
                ApiError::field(field, message)
            }
            other => ApiError::from(other),
        })
}

/// `:ym` accepts `YYYY-MM` or `YYYY-MM-01` and normalises to `YYYY-MM-01`.
pub(crate) fn parse_ym(ym: &str) -> Result<chrono::NaiveDate, ApiError> {
    let candidate = if ym.len() == 7 {
        format!("{ym}-01")
    } else {
        ym.to_string()
    };
    let date = chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d")
        .map_err(|_| ApiError::field("ym", "month must be in YYYY-MM form"))?;
    if date.day() != 1 {
        return Err(ApiError::field("ym", "month must be in YYYY-MM form"));
    }
    Ok(date)
}

/// Kept next to the router so both stock and trade handlers share it.
pub fn parse_market(value: &str) -> Result<Market, ApiError> {
    Market::parse(&value.to_ascii_uppercase())
        .ok_or_else(|| ApiError::field("market", "market must be HK or US"))
}

pub fn now_timestamp() -> String {
    chrono::Local::now().to_rfc3339()
}

/// Local today: what deposit status derives from.
pub fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect_memory;
    use std::collections::BTreeSet;

    /// The doc is generated from the same `routes!` registrations that serve
    /// requests, so an undocumented endpoint is impossible — this instead pins
    /// the exact surface so a route change always updates this list.
    #[tokio::test]
    async fn openapi_documents_every_api_operation() {
        let pool = connect_memory().await.expect("memory db");
        let (_router, api) = api_router(AppState { pool });

        let doc: serde_json::Value =
            serde_json::from_str(&api.to_json().expect("openapi serializes")).unwrap();
        let actual: BTreeSet<(String, String)> = doc["paths"]
            .as_object()
            .expect("paths object")
            .iter()
            .flat_map(|(path, item)| {
                item.as_object()
                    .expect("path item")
                    .keys()
                    .map(|method| (path.clone(), method.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        let expected: BTreeSet<(String, String)> = [
            ("/api/aia/events", "get"),
            ("/api/aia/events", "post"),
            ("/api/aia/events/{id}", "delete"),
            ("/api/aia/note", "patch"),
            ("/api/aia/policies", "get"),
            ("/api/aia/policies", "post"),
            ("/api/aia/policies/{id}", "delete"),
            ("/api/aia/policies/{id}", "patch"),
            ("/api/aia/rate", "patch"),
            ("/api/aia/summary", "get"),
            ("/api/bonds", "get"),
            ("/api/bonds", "post"),
            ("/api/bonds/summary", "get"),
            ("/api/bonds/{id}", "delete"),
            ("/api/bonds/{id}", "patch"),
            ("/api/bonds/{id}/receive", "post"),
            ("/api/coupons", "get"),
            ("/api/coupons", "post"),
            ("/api/coupons/{id}", "delete"),
            ("/api/coupons/{id}", "patch"),
            ("/api/deposits", "get"),
            ("/api/deposits", "post"),
            ("/api/deposits/summary", "get"),
            ("/api/deposits/{id}", "delete"),
            ("/api/deposits/{id}", "patch"),
            ("/api/deposits/{id}/receive", "post"),
            ("/api/dividends", "get"),
            ("/api/dividends", "post"),
            ("/api/dividends/summary", "get"),
            ("/api/dividends/{id}", "delete"),
            ("/api/dividends/{id}", "patch"),
            ("/api/family/deposits", "get"),
            ("/api/family/deposits", "post"),
            ("/api/family/deposits/summary", "get"),
            ("/api/family/deposits/{id}", "delete"),
            ("/api/family/deposits/{id}", "patch"),
            ("/api/family/deposits/{id}/receive", "post"),
            ("/api/family/holders/{holder}/note", "put"),
            ("/api/forecast", "get"),
            ("/api/forecast-items/{id}", "delete"),
            ("/api/forecast-items/{id}", "patch"),
            ("/api/forecast-items/{id}/convert", "post"),
            ("/api/forecast/{ym}/items", "post"),
            ("/api/ibkr", "get"),
            ("/api/ibkr", "patch"),
            ("/api/manual-assets", "get"),
            ("/api/manual-assets", "post"),
            ("/api/manual-assets/{id}", "delete"),
            ("/api/manual-assets/{id}", "patch"),
            ("/api/month-items/{id}", "delete"),
            ("/api/month-items/{id}", "patch"),
            ("/api/months", "get"),
            ("/api/months/settings", "get"),
            ("/api/months/settings", "patch"),
            ("/api/months/summary", "get"),
            ("/api/months/{ym}", "delete"),
            ("/api/months/{ym}", "get"),
            ("/api/months/{ym}", "patch"),
            ("/api/months/{ym}/items", "post"),
            ("/api/months/{ym}/items/dismiss", "post"),
            ("/api/mpf", "get"),
            ("/api/mpf/accounts", "post"),
            ("/api/mpf/accounts/{id}", "delete"),
            ("/api/mpf/accounts/{id}", "patch"),
            ("/api/mpf/history/{id}", "delete"),
            ("/api/mpf/note", "patch"),
            ("/api/overview", "get"),
            ("/api/stocks", "get"),
            ("/api/stocks", "post"),
            ("/api/stocks/order", "post"),
            ("/api/stocks/prices", "post"),
            ("/api/stocks/{id}", "delete"),
            ("/api/stocks/{id}", "patch"),
            ("/api/summary", "get"),
            ("/api/summary/yearly", "get"),
            ("/api/summary/yearly/{market}/{year}", "patch"),
            ("/api/summary/yearly/{market}/{year}/freeze", "post"),
            ("/api/trades", "get"),
            ("/api/trades", "post"),
            ("/api/trades/{id}", "delete"),
            ("/api/trades/{id}", "patch"),
            ("/api/year-review", "get"),
            ("/api/year-review/{year}", "patch"),
        ]
        .iter()
        .map(|(path, method)| (path.to_string(), method.to_string()))
        .collect();

        assert_eq!(actual, expected);
    }
}
