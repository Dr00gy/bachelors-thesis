use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use axum::extract::DefaultBodyLimit;
use tower_http::cors::{CorsLayer, Any};

mod xmap;
mod api;

/// Root endpoint handler
///
/// # Returns
/// * `&'static str` - Server status message
async fn root() -> &'static str {
    "XMAP Backend Server is running!\n\nEndpoints:\n- GET /\n- POST /api/match"
}

/// Main application entry point
///
/// # Setup
/// * Creates shared XmapCache
/// * Configures CORS for local development
/// * Sets up routes with state sharing
/// * Starts server on port 8080
#[tokio::main]
async fn main() {
    let cache = Arc::new(xmap::XmapCache::new());

    let cors = CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),
            "http://127.0.0.1:5173".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(root))
        .route("/api/match", post(api::stream_xmap_matches))
        .with_state(cache)
        .layer(cors)
        .layer(DefaultBodyLimit::disable());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    println!("Server running on http://0.0.0.0:8080");

    axum::serve(listener, app).await.unwrap();
}