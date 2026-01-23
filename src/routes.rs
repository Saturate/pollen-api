use axum::{
    routing::get,
    Router,
    Json,
    http::StatusCode,
    response::IntoResponse,
    extract::{Path, State, Query},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::cache::Cache;
use crate::pollen_types;
use crate::sources::denmark;
use crate::models::*;

pub type SharedCache = Arc<RwLock<Cache>>;

pub fn create_router(cache: SharedCache) -> Router {
    Router::new()
        .route("/", get(api_info))
        .route("/health", get(|| async { "OK" }))
        .route("/v1/{country}", get(country_info))
        .route("/v1/{country}/regions", get(list_regions))
        .route("/v1/{country}/pollen-types", get(list_pollen_types))
        .route("/v1/{country}/{region}/forecast", get(get_forecast))
        .with_state(cache)
}

#[derive(serde::Serialize)]
struct ApiInfo {
    name: String,
    version: String,
    countries: Vec<String>,
    languages: Vec<String>,
    example: String,
    repository: String,
}

async fn api_info() -> Json<ApiInfo> {
    Json(ApiInfo {
        name: "Pollen API Relay".to_string(),
        version: "0.1.0".to_string(),
        countries: vec!["dk".to_string()],
        languages: vec!["en".to_string(), "da".to_string()],
        example: "/v1/dk/copenhagen/forecast?lang=en&types=grass,birch".to_string(),
        repository: "https://github.com/Saturate/pollen".to_string(),
    })
}

#[derive(Deserialize)]
struct LangQuery {
    lang: Option<String>,
}

#[derive(Deserialize)]
struct ForecastQuery {
    lang: Option<String>,
    types: Option<String>,
}

async fn country_info(Path(country): Path<String>) -> impl IntoResponse {
    if country != "dk" {
        return (StatusCode::NOT_FOUND, Json(None));
    }

    (StatusCode::OK, Json(Some(CountryInfo {
        code: "dk".to_string(),
        name: "Denmark".to_string(),
        regions: vec!["copenhagen".to_string(), "viborg".to_string()],
    })))
}

fn resolve_region_slug(country: &str, slug: &str) -> Option<String> {
    if country != "dk" {
        return None;
    }

    match slug {
        "copenhagen" | "east" => Some("48".to_string()),
        "viborg" | "west" => Some("49".to_string()),
        _ => None,
    }
}

async fn list_regions(Path(country): Path<String>) -> impl IntoResponse {
    if country != "dk" {
        return (StatusCode::NOT_FOUND, Json(None));
    }

    (StatusCode::OK, Json(Some(RegionsResponse {
        country: "dk".to_string(),
        regions: vec![
            Region {
                slug: "copenhagen".to_string(),
                name: "Copenhagen Area (East of Great Belt)".to_string(),
                aliases: vec!["east".to_string()],
            },
            Region {
                slug: "viborg".to_string(),
                name: "Viborg Area (West of Great Belt)".to_string(),
                aliases: vec!["west".to_string()],
            },
        ],
    })))
}

async fn list_pollen_types(
    Path(country): Path<String>,
    Query(params): Query<LangQuery>,
) -> impl IntoResponse {
    if country != "dk" {
        return (StatusCode::NOT_FOUND, Json(None));
    }

    let lang = params.lang.as_deref().unwrap_or("en");

    let types = pollen_types::get_pollen_types()
        .into_iter()
        .map(|t| PollenType {
            id: t.id.clone(),
            name: match lang {
                "da" => t.name_da,
                _ => t.name_en,
            },
        })
        .collect();

    (StatusCode::OK, Json(Some(PollenTypesResponse {
        country: "dk".to_string(),
        pollen_types: types,
    })))
}

async fn get_forecast(
    State(cache): State<SharedCache>,
    Path((country, region_slug)): Path<(String, String)>,
    Query(params): Query<ForecastQuery>,
) -> impl IntoResponse {
    if country != "dk" {
        return (StatusCode::NOT_FOUND, Json(Vec::<PollenForecast>::new()));
    }

    let region_id = match resolve_region_slug(&country, &region_slug) {
        Some(id) => id,
        None => {
            return (StatusCode::NOT_FOUND, Json(Vec::<PollenForecast>::new()));
        }
    };

    let lang = params.lang.as_deref().unwrap_or("en");

    let requested_types: Option<Vec<String>> = params.types.as_ref().map(|types_str| {
        types_str.split(',').map(|s| s.trim().to_string()).collect()
    });

    let cache_read = cache.read().await;
    let cached_data = cache_read.get(&country);
    drop(cache_read);

    let forecasts = match cached_data {
        Some(cached) => cached.forecasts,
        None => {
            tracing::warn!("Cache miss for country: {}, fetching...", country);
            match denmark::fetch().await {
                Ok(raw_data) => denmark::transform(raw_data),
                Err(e) => {
                    tracing::error!("Failed to fetch {} data: {}", country, e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(Vec::<PollenForecast>::new()));
                }
            }
        }
    };

    let filtered: Vec<PollenForecast> = forecasts
        .into_iter()
        .filter(|f| f.region == region_id)
        .filter(|f| {
            if let Some(ref types) = requested_types {
                types.contains(&f.pollen_type)
            } else {
                true
            }
        })
        .map(|mut f| {
            f.pollen_name = pollen_types::get_pollen_name(&f.pollen_type, lang)
                .unwrap_or_else(|| f.pollen_type.clone());
            f
        })
        .collect();

    (StatusCode::OK, Json(filtered))
}
