use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use crate::cache::Cache;
use crate::sources::denmark;

const POLL_INTERVAL_SECONDS: u64 = 7200;

pub async fn start_polling(cache: Arc<RwLock<Cache>>) {
    let mut interval = time::interval(Duration::from_secs(POLL_INTERVAL_SECONDS));

    poll_once(cache.clone()).await;

    loop {
        interval.tick().await;
        poll_once(cache.clone()).await;
    }
}

async fn poll_once(cache: Arc<RwLock<Cache>>) {
    tracing::info!("Polling Denmark API...");

    match denmark::fetch().await {
        Ok(raw_data) => {
            let forecasts = denmark::transform(raw_data);
            let mut cache_write = cache.write().await;
            cache_write.set("dk".to_string(), forecasts);
            tracing::info!("Successfully updated Denmark cache");
        }
        Err(e) => {
            tracing::error!("Failed to poll Denmark API: {}", e);
        }
    }
}
