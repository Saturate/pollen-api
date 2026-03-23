use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use crate::cache::Cache;
use crate::sources::denmark;

const POLL_INTERVAL_SECONDS: u64 = 7200;
const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_SECS: u64 = 2;

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

    for attempt in 0..=MAX_RETRIES {
        match denmark::fetch().await {
            Ok(raw_data) => {
                let forecasts = denmark::transform(raw_data);
                let mut cache_write = cache.write().await;
                cache_write.set("dk".to_string(), forecasts);
                tracing::info!("Successfully updated Denmark cache");
                return;
            }
            Err(e) => {
                if attempt < MAX_RETRIES {
                    let delay = BASE_RETRY_DELAY_SECS * 4u64.pow(attempt);
                    tracing::warn!(
                        "Failed to poll Denmark API (attempt {}/{}): {}. Retrying in {}s...",
                        attempt + 1,
                        MAX_RETRIES + 1,
                        e,
                        delay
                    );
                    time::sleep(Duration::from_secs(delay)).await;
                } else {
                    tracing::error!(
                        "Failed to poll Denmark API after {} attempts: {}",
                        MAX_RETRIES + 1,
                        e
                    );
                }
            }
        }
    }
}
