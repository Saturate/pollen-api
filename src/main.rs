mod cache;
mod models;
mod pollen_types;
mod poller;
mod routes;
mod sources;

use std::net::SocketAddr;
use cache::Cache;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cache = Cache::new();

    let poller_cache = cache.clone();
    tokio::spawn(async move {
        poller::start_polling(poller_cache).await;
    });

    let app = routes::create_router(cache);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3060));
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to port 3060");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
