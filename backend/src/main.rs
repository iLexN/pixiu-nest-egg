use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use wealth_backend::db;
use wealth_backend::routes::{api_router, AppState};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wealth_backend=info,tower_http=info".into()),
        )
        .init();

    let db_path = std::env::var("WEALTH_DB").unwrap_or_else(|_| db::DEFAULT_DB_PATH.to_string());
    let pool = db::connect(&db_path).await?;
    tracing::info!("database ready at {db_path}");

    let addr: SocketAddr = std::env::var("WEALTH_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string())
        .parse()?;

    let app = build_app(AppState { pool });
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn build_app(state: AppState) -> Router {
    let dist = frontend_dist();
    // Serving index.html as the fallback keeps client-side routing working.
    let static_files = ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html")));

    Router::new()
        .nest("/api", api_router(state))
        .fallback_service(static_files)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

fn frontend_dist() -> PathBuf {
    if let Ok(path) = std::env::var("WEALTH_FRONTEND_DIST") {
        return PathBuf::from(path);
    }
    let workspace_relative = PathBuf::from("frontend/dist");
    if workspace_relative.exists() {
        return workspace_relative;
    }
    PathBuf::from("../frontend/dist")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
