use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use crate::models::PollenForecast;

#[derive(Clone)]
pub struct CachedData {
    pub forecasts: Vec<PollenForecast>,
    pub last_updated: DateTime<Utc>,
}

pub struct Cache {
    data: HashMap<String, CachedData>,
}

impl Cache {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Cache {
            data: HashMap::new(),
        }))
    }

    pub fn get(&self, source: &str) -> Option<CachedData> {
        self.data.get(source).cloned()
    }

    pub fn set(&mut self, source: String, forecasts: Vec<PollenForecast>) {
        self.data.insert(source, CachedData {
            forecasts,
            last_updated: Utc::now(),
        });
    }

    pub fn is_stale(&self, source: &str, ttl_seconds: i64) -> bool {
        match self.data.get(source) {
            None => true,
            Some(cached) => {
                let age = Utc::now().signed_duration_since(cached.last_updated);
                age.num_seconds() > ttl_seconds
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PollenForecast;

    fn sample_forecast() -> PollenForecast {
        PollenForecast {
            region: "48".to_string(),
            pollen_type: "grass".to_string(),
            pollen_name: "Grass".to_string(),
            date: "2026-03-23".to_string(),
            level: 3,
            is_forecast: false,
        }
    }

    #[test]
    fn set_and_get_roundtrip() {
        let cache = Cache::new();
        let mut cache = cache.try_write().unwrap();
        cache.set("dk".to_string(), vec![sample_forecast()]);

        let cached = cache.get("dk").expect("should have data");
        assert_eq!(cached.forecasts.len(), 1);
        assert_eq!(cached.forecasts[0].pollen_type, "grass");
    }

    #[test]
    fn get_missing_returns_none() {
        let cache = Cache::new();
        let cache = cache.try_read().unwrap();
        assert!(cache.get("xx").is_none());
    }

    #[test]
    fn is_stale_with_no_data() {
        let cache = Cache::new();
        let cache = cache.try_read().unwrap();
        assert!(cache.is_stale("dk", 3600));
    }

    #[test]
    fn is_stale_fresh_data() {
        let cache = Cache::new();
        let mut cache = cache.try_write().unwrap();
        cache.set("dk".to_string(), vec![sample_forecast()]);
        assert!(!cache.is_stale("dk", 3600));
    }
}
