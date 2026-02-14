mod handlers;
mod state;
mod static_files;

use axum::Router;
use axum::routing::get;
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::location::LocationResolver;
use std::sync::Mutex;

pub fn build_router() -> Router {
    let state = Arc::new(AppState {
        resolver: Mutex::new(LocationResolver::new()),
    });

    Router::new()
        .route("/", get(handlers::index))
        .route("/style.css", get(handlers::style))
        .route("/app.js", get(handlers::script))
        .route("/api/times", get(handlers::prayer_times))
        .route("/api/cities", get(handlers::city_list))
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

    eprintln!("  Polaris Chronos server listening on http://{}", addr);
    eprintln!("  Press Ctrl+C to stop.");

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        });
}
