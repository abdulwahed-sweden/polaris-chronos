mod handlers;
mod state;
mod static_files;

use axum::Router;
use axum::routing::get;
use axum::http::HeaderValue;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;

/// Application version, read from Cargo.toml at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn build_router() -> Router {
    let state = Arc::new(AppState::new());

    // API routes with no-cache + version headers
    let api_routes = Router::new()
        .route("/api/resolve", get(handlers::resolve))
        .route("/api/times", get(handlers::prayer_times))
        .route("/api/month", get(handlers::month_times))
        .route("/api/cities", get(handlers::city_list))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate, max-age=0"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::header::PRAGMA,
            HeaderValue::from_static("no-cache"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            axum::http::HeaderName::from_static("x-polaris-version"),
            HeaderValue::from_static(VERSION),
        ));

    Router::new()
        .route("/", get(handlers::index))
        .route("/day", get(handlers::index))
        .route("/docs", get(handlers::index))
        .route("/style.css", get(handlers::style))
        .route("/app.js", get(handlers::script))
        .merge(api_routes)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn start(host: &str, port: u16) {
    let app = build_router();
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Error: Cannot bind to {}: {}", addr, e);
            std::process::exit(1);
        });

    let base = format!("http://{}", addr);

    eprintln!();
    eprintln!("--------------------------------------------------");
    eprintln!("  Polaris Chronos Server v{}", VERSION);
    eprintln!("  Cache: fresh (in-memory, 6h TTL)");
    eprintln!();
    eprintln!("  Local:     {}", base);
    eprintln!("  Docs:      {}/docs", base);
    eprintln!();
    eprintln!("  API:");
    eprintln!("    {}/api/resolve?query=stockholm", base);
    eprintln!("    {}/api/times?city=stockholm", base);
    eprintln!("    {}/api/month?city=stockholm", base);
    eprintln!("    {}/api/cities", base);
    eprintln!();
    eprintln!("  Press Ctrl+C to stop.");
    eprintln!("--------------------------------------------------");
    eprintln!();

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        });
}
