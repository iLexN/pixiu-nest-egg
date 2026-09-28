use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use utoipa_scalar::Servable;
use wealth_backend::db;
use wealth_backend::routes::{AppState, api_router};

const DEFAULT_ADDR: &str = "127.0.0.1:8787";

/// The only cross-origin caller is the Vite dev server; the built frontend is
/// served same-origin. Anything wider would let arbitrary web pages read the
/// API from the browser, since loopback binding does not stop that.
const DEV_ORIGINS: [&str; 2] = ["http://127.0.0.1:5173", "http://localhost:5173"];

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

    let app = build_app(AppState { pool }).unwrap_or_else(|err| {
        tracing::error!("failed to build app: {err}");
        std::process::exit(1);
    });
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Scalar page: the spec is inlined as `$spec`; `data-configuration` disables
/// the Ask AI agent button, which the default bundle enables on loopback.
/// The bundle is pinned and integrity-checked because it runs on the API's
/// origin. To upgrade: fetch the new `dist/browser/standalone.js` and recompute
/// `openssl dgst -sha384 -binary standalone.js | openssl base64 -A`.
const SCALAR_HTML: &str = r#"<!doctype html>
<html>
<head>
    <title>Scalar</title>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
</head>
<body>
<script
        id="api-reference"
        type="application/json"
        data-configuration='{"agentEnabled": false, "agent": {"disabled": true}}'>
    $spec
</script>
<script
        src="https://cdn.jsdelivr.net/npm/@scalar/api-reference@1.72.1/dist/browser/standalone.js"
        integrity="sha384-U11tb2XnKvmwt8RlTvnwUnYgrN+ur4Xyh9htLhjajWNR/Oyl5AX5DEz00qRmlrmK"
        crossorigin="anonymous"></script>
</body>
</html>
"#;

fn build_app(state: AppState) -> anyhow::Result<Router> {
    let dist = frontend_dist();
    // Serving index.html as the fallback keeps client-side routing working.
    let static_files = ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html")));

    // Route paths already carry the /api prefix, so the router merges at the
    // root; the same declaration produces the OpenAPI document.
    let (api, openapi) = api_router(state);
    let spec = openapi.to_json()?;

    let cors = CorsLayer::new()
        .allow_origin(
            DEV_ORIGINS
                .iter()
                .map(|origin| origin.parse::<HeaderValue>())
                .collect::<Result<Vec<_>, _>>()?,
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([CONTENT_TYPE]);

    Ok(Router::new()
        .merge(api)
        .route(
            "/api-docs/openapi.json",
            get(move || {
                let spec = spec.clone();
                async move { ([(CONTENT_TYPE, "application/json")], spec) }
            }),
        )
        .merge(utoipa_scalar::Scalar::with_url("/scalar", openapi).custom_html(SCALAR_HTML))
        .fallback_service(static_files)
        .layer(cors)
        .layer(TraceLayer::new_for_http()))
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
