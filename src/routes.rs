use axum::{
    routing::get,
    Router,
    Json,
    response::{IntoResponse, Response},
    extract::{Path, State, Query},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::{OpenApi, ToSchema, IntoParams};
use utoipa_scalar::{Scalar, Servable};
use crate::cache::Cache;
use crate::errors::ApiError;
use crate::pollen_types;
use crate::sources::denmark;
use crate::models::*;

pub type SharedCache = Arc<RwLock<Cache>>;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Pollen API",
        description = "Clean REST API relay for Nordic pollen data. Data sourced from astma-allergi.dk and served with caching, filtering, and multilingual support.",
        version = "0.2.0",
        license(name = "MIT"),
        contact(name = "Allan Kimmer Jensen", email = "hi@akj.io")
    ),
    paths(
        api_info,
        health,
        country_info,
        list_regions,
        list_pollen_types,
        get_forecast,
    ),
    components(schemas(
        ApiInfo,
        HealthResponse,
        CountryInfo,
        RegionsResponse,
        Region,
        PollenTypesResponse,
        PollenType,
        ForecastResponse,
        PollenForecast,
        ApiError,
    )),
    tags(
        (name = "info", description = "API metadata and discovery"),
        (name = "health", description = "Health and cache status"),
        (name = "pollen", description = "Pollen data and forecasts"),
    )
)]
struct ApiDoc;

pub fn create_router(cache: SharedCache) -> Router {
    Router::new()
        .route("/", get(api_info))
        .route("/health", get(health))
        .route("/v1/{country}", get(country_info))
        .route("/v1/{country}/regions", get(list_regions))
        .route("/v1/{country}/pollen-types", get(list_pollen_types))
        .route("/v1/{country}/{region}/forecast", get(get_forecast))
        .route("/openapi.json", get(openapi_spec))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .with_state(cache)
}

#[derive(serde::Serialize, ToSchema)]
struct ApiInfo {
    name: String,
    version: String,
    countries: Vec<String>,
    languages: Vec<String>,
    docs: String,
    openapi_spec: String,
    example: String,
    repository: String,
}

#[utoipa::path(
    get,
    path = "/",
    tag = "info",
    responses(
        (status = 200, description = "API metadata", body = ApiInfo)
    )
)]
async fn api_info() -> Json<ApiInfo> {
    Json(ApiInfo {
        name: "Pollen API Relay".to_string(),
        version: "0.2.0".to_string(),
        countries: vec!["dk".to_string()],
        languages: vec!["en".to_string(), "da".to_string()],
        docs: "/docs".to_string(),
        openapi_spec: "/openapi.json".to_string(),
        example: "/v1/dk/copenhagen/forecast?lang=en&types=grass,birch".to_string(),
        repository: "https://github.com/Saturate/pollen".to_string(),
    })
}

async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

const POLL_INTERVAL_SECONDS: i64 = 7200;

#[derive(serde::Serialize, ToSchema)]
struct HealthResponse {
    /// "ok", "degraded", or "no_data"
    status: String,
    cache_age_seconds: Option<i64>,
    last_updated: Option<String>,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Cache health and staleness info", body = HealthResponse)
    )
)]
async fn health(State(cache): State<SharedCache>) -> Json<HealthResponse> {
    let cache_read = cache.read().await;
    let cached = cache_read.get("dk");

    match cached {
        Some(data) => {
            let age = chrono::Utc::now()
                .signed_duration_since(data.last_updated)
                .num_seconds();
            let status = if age > POLL_INTERVAL_SECONDS * 2 {
                "degraded"
            } else {
                "ok"
            };
            Json(HealthResponse {
                status: status.to_string(),
                cache_age_seconds: Some(age),
                last_updated: Some(data.last_updated.to_rfc3339()),
            })
        }
        None => Json(HealthResponse {
            status: "no_data".to_string(),
            cache_age_seconds: None,
            last_updated: None,
        }),
    }
}

#[derive(Deserialize, IntoParams)]
struct LangQuery {
    /// Language code (en, da). Defaults to en.
    lang: Option<String>,
}

#[derive(Deserialize, IntoParams)]
struct ForecastQuery {
    /// Language code (en, da). Defaults to en.
    lang: Option<String>,
    /// Comma-separated pollen type filter (e.g. grass,birch)
    types: Option<String>,
}

fn require_dk(country: &str) -> Result<(), ApiError> {
    if country != "dk" {
        return Err(ApiError::not_found(format!(
            "Country '{}' is not supported. Available: dk",
            country
        )));
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/v1/{country}",
    tag = "pollen",
    params(
        ("country" = String, Path, description = "Country code (e.g. dk)")
    ),
    responses(
        (status = 200, description = "Country information", body = CountryInfo),
        (status = 404, description = "Country not supported", body = ApiError)
    )
)]
async fn country_info(Path(country): Path<String>) -> Result<Json<CountryInfo>, ApiError> {
    require_dk(&country)?;

    Ok(Json(CountryInfo {
        code: "dk".to_string(),
        name: "Denmark".to_string(),
        regions: vec!["copenhagen".to_string(), "viborg".to_string()],
    }))
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

#[utoipa::path(
    get,
    path = "/v1/{country}/regions",
    tag = "pollen",
    params(
        ("country" = String, Path, description = "Country code (e.g. dk)")
    ),
    responses(
        (status = 200, description = "Available regions", body = RegionsResponse),
        (status = 404, description = "Country not supported", body = ApiError)
    )
)]
async fn list_regions(Path(country): Path<String>) -> Result<Json<RegionsResponse>, ApiError> {
    require_dk(&country)?;

    Ok(Json(RegionsResponse {
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
    }))
}

#[utoipa::path(
    get,
    path = "/v1/{country}/pollen-types",
    tag = "pollen",
    params(
        ("country" = String, Path, description = "Country code (e.g. dk)"),
        LangQuery,
    ),
    responses(
        (status = 200, description = "Available pollen types", body = PollenTypesResponse),
        (status = 404, description = "Country not supported", body = ApiError)
    )
)]
async fn list_pollen_types(
    Path(country): Path<String>,
    Query(params): Query<LangQuery>,
) -> Result<Json<PollenTypesResponse>, ApiError> {
    require_dk(&country)?;

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

    Ok(Json(PollenTypesResponse {
        country: "dk".to_string(),
        pollen_types: types,
    }))
}

#[derive(serde::Serialize, ToSchema)]
struct ForecastResponse {
    forecasts: Vec<PollenForecast>,
    last_updated: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/{country}/{region}/forecast",
    tag = "pollen",
    params(
        ("country" = String, Path, description = "Country code (e.g. dk)"),
        ("region" = String, Path, description = "Region slug (copenhagen, viborg) or alias (east, west)"),
        ForecastQuery,
    ),
    responses(
        (status = 200, description = "Pollen forecast data", body = ForecastResponse),
        (status = 404, description = "Country or region not found", body = ApiError),
        (status = 500, description = "Upstream fetch failed", body = ApiError)
    )
)]
async fn get_forecast(
    State(cache): State<SharedCache>,
    Path((country, region_slug)): Path<(String, String)>,
    Query(params): Query<ForecastQuery>,
) -> Result<Response, ApiError> {
    require_dk(&country)?;

    let region_id = resolve_region_slug(&country, &region_slug).ok_or_else(|| {
        ApiError::not_found(format!(
            "Region '{}' not found. Available: copenhagen, viborg (aliases: east, west)",
            region_slug
        ))
    })?;

    let lang = params.lang.as_deref().unwrap_or("en");

    let requested_types: Option<Vec<String>> = params.types.as_ref().map(|types_str| {
        types_str.split(',').map(|s| s.trim().to_string()).collect()
    });

    let cache_read = cache.read().await;
    let cached_data = cache_read.get(&country);
    drop(cache_read);

    let (forecasts, last_updated) = match cached_data {
        Some(cached) => (cached.forecasts, Some(cached.last_updated)),
        None => {
            tracing::warn!("Cache miss for country: {}, fetching...", country);
            match denmark::fetch().await {
                Ok(raw_data) => (denmark::transform(raw_data), None),
                Err(e) => {
                    tracing::error!("Failed to fetch {} data: {}", country, e);
                    return Err(ApiError::internal("Failed to fetch pollen data from upstream"));
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

    let last_updated_str = last_updated.map(|lu| lu.to_rfc3339());

    let mut response = Json(ForecastResponse {
        forecasts: filtered,
        last_updated: last_updated_str.clone(),
    })
    .into_response();

    if let Some(lu) = &last_updated {
        let age = chrono::Utc::now().signed_duration_since(*lu).num_seconds();
        let headers = response.headers_mut();
        if let Ok(val) = format!("public, max-age={}", POLL_INTERVAL_SECONDS).parse() {
            headers.insert("cache-control", val);
        }
        if let Ok(val) = age.to_string().parse() {
            headers.insert("x-data-age", val);
        }
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use crate::cache::Cache;
    use crate::models::PollenForecast;

    fn setup_cache_with_data() -> SharedCache {
        let cache = Cache::new();
        let forecasts = vec![
            PollenForecast {
                region: "48".to_string(),
                pollen_type: "grass".to_string(),
                pollen_name: "Grass".to_string(),
                date: "2026-03-23".to_string(),
                level: 3,
                is_forecast: false,
            },
            PollenForecast {
                region: "48".to_string(),
                pollen_type: "birch".to_string(),
                pollen_name: "Birch".to_string(),
                date: "2026-03-23".to_string(),
                level: 1,
                is_forecast: false,
            },
            PollenForecast {
                region: "49".to_string(),
                pollen_type: "grass".to_string(),
                pollen_name: "Grass".to_string(),
                date: "2026-03-23".to_string(),
                level: 2,
                is_forecast: false,
            },
        ];
        {
            let mut c = cache.try_write().unwrap();
            c.set("dk".to_string(), forecasts);
        }
        cache
    }

    async fn get_body(response: axum::http::Response<Body>) -> String {
        let body = response.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["cache_age_seconds"].is_number());
    }

    #[tokio::test]
    async fn health_no_data() {
        let cache = Cache::new();
        let app = create_router(cache);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["status"], "no_data");
    }

    #[tokio::test]
    async fn country_info_dk() {
        let cache = Cache::new();
        let app = create_router(cache);

        let response = app
            .oneshot(Request::builder().uri("/v1/dk").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn country_info_unknown_returns_404_with_error() {
        let cache = Cache::new();
        let app = create_router(cache);

        let response = app
            .oneshot(Request::builder().uri("/v1/xx").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("not supported"));
        assert_eq!(json["code"], 404);
    }

    #[tokio::test]
    async fn forecast_filters_by_region() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/copenhagen/forecast")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let forecasts = json["forecasts"].as_array().unwrap();
        assert_eq!(forecasts.len(), 2);
        assert!(forecasts.iter().all(|f| f["region"] == "48"));
    }

    #[tokio::test]
    async fn forecast_filters_by_type() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/copenhagen/forecast?types=grass")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let forecasts = json["forecasts"].as_array().unwrap();
        assert_eq!(forecasts.len(), 1);
        assert_eq!(forecasts[0]["pollen_type"], "grass");
    }

    #[tokio::test]
    async fn forecast_lang_da() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/copenhagen/forecast?lang=da&types=grass")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let forecasts = json["forecasts"].as_array().unwrap();
        assert_eq!(forecasts[0]["pollen_name"], "Græs");
    }

    #[tokio::test]
    async fn forecast_unknown_region_returns_404() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/aarhus/forecast")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn forecast_has_cache_headers() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/copenhagen/forecast")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.headers().get("cache-control").is_some());
        assert!(response.headers().get("x-data-age").is_some());
    }

    #[tokio::test]
    async fn forecast_region_alias_east() {
        let cache = setup_cache_with_data();
        let app = create_router(cache);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/dk/east/forecast")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = get_body(response).await;
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        let forecasts = json["forecasts"].as_array().unwrap();
        // east = copenhagen = region 48
        assert!(forecasts.iter().all(|f| f["region"] == "48"));
    }

    #[tokio::test]
    async fn docs_returns_html() {
        let cache = Cache::new();
        let app = create_router(cache);

        let response = app
            .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.contains("text/html"));
    }
}
