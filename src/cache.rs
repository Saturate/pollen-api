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
