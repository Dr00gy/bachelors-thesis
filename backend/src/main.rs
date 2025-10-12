use axum::{routing::post, Router};
use std::sync::Arc;

mod xmap;
mod api;

#[tokio::main]
async fn main() {
    let cache = Arc::new(xmap::XmapCache::new());

    let app = Router::new()
        .route("/api/match", post(api::stream_xmap_matches))
        .with_state(cache);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:8080");

    axum::serve(listener, app).await.unwrap();
}