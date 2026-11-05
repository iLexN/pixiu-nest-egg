//! Dump the OpenAPI JSON to a path (arg 1) or stdout — the frontend's
//! `pnpm gen:api` runs this so codegen always types the current source.

use wealth_backend::db;
use wealth_backend::routes::{AppState, api_router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = db::connect_memory().await?;
    let (_, openapi) = api_router(AppState { pool });
    let json = openapi.to_json()?;
    match std::env::args().nth(1) {
        Some(path) => std::fs::write(path, format!("{json}\n"))?,
        None => println!("{json}"),
    }
    Ok(())
}
