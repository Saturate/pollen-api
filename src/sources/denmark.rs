use serde::Deserialize;
use anyhow::Result;
use std::collections::HashMap;
use crate::pollen_types;

const DANISH_API_URL: &str = "https://www.astma-allergi.dk/umbraco/Api/PollenApi/GetPollenFeed";

#[derive(Debug, Deserialize)]
pub struct PollenFeedResponse {
    pub fields: HashMap<String, PollenFeedValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollenFeedValue {
    pub map_value: Option<PollenFeedMap>,
    pub string_value: Option<String>,
    pub integer_value: Option<String>,
    pub boolean_value: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PollenFeedMap {
    pub fields: HashMap<String, PollenFeedValue>,
}

pub async fn fetch() -> Result<PollenFeedResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(DANISH_API_URL).send().await?;
    let json_string: String = response.json().await?;
    let document: PollenFeedResponse = serde_json::from_str(&json_string)?;
    Ok(document)
}

pub fn transform(raw: PollenFeedResponse) -> Vec<crate::models::PollenForecast> {
    let mut forecasts = Vec::new();
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    for (region_id, region_value) in raw.fields {
        if let Some(region_map) = &region_value.map_value {
            if let Some(data_value) = region_map.fields.get("data") {
                if let Some(data_map) = &data_value.map_value {
                    for (pollen_id, pollen_value) in &data_map.fields {
                        if let Some(canonical_id) = pollen_types::denmark_id_to_canonical(pollen_id) {
                            let pollen_name = pollen_types::get_pollen_name(&canonical_id, "en")
                                .unwrap_or_else(|| canonical_id.clone());

                            if let Some(pollen_map) = &pollen_value.map_value {
                                if let Some(level_value) = pollen_map.fields.get("level") {
                                    if let Some(level_str) = &level_value.integer_value {
                                        if let Ok(level) = level_str.parse::<i32>() {
                                            let normalized_level = if level == -1 { 0 } else { level };
                                            forecasts.push(crate::models::PollenForecast {
                                                region: region_id.clone(),
                                                pollen_type: canonical_id.clone(),
                                                pollen_name: pollen_name.clone(),
                                                date: today.clone(),
                                                level: normalized_level,
                                                is_forecast: false,
                                            });
                                        }
                                    }
                                }

                                if let Some(predictions_value) = pollen_map.fields.get("predictions") {
                                    if let Some(predictions_map) = &predictions_value.map_value {
                                        for (date_str, prediction_value) in &predictions_map.fields {
                                            if let Some(prediction_map) = &prediction_value.map_value {
                                                if let Some(pred_value) = prediction_map.fields.get("prediction") {
                                                    if let Some(pred_str) = &pred_value.string_value {
                                                        if let Ok(level) = pred_str.parse::<i32>() {
                                                            let normalized_level = if level == -1 { 0 } else { level };

                                                            let iso_date = convert_danish_date(date_str);

                                                            forecasts.push(crate::models::PollenForecast {
                                                                region: region_id.clone(),
                                                                pollen_type: canonical_id.clone(),
                                                                pollen_name: pollen_name.clone(),
                                                                date: iso_date,
                                                                level: normalized_level,
                                                                is_forecast: true,
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    forecasts
}

pub(crate) fn convert_danish_date(danish_date: &str) -> String {
    let parts: Vec<&str> = danish_date.split('-').collect();
    if parts.len() == 3 {
        format!("{}-{}-{}", parts[2], parts[1], parts[0])
    } else {
        danish_date.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_danish_date_valid() {
        assert_eq!(convert_danish_date("22-03-2026"), "2026-03-22");
        assert_eq!(convert_danish_date("01-12-2025"), "2025-12-01");
    }

    #[test]
    fn convert_danish_date_malformed_returns_input() {
        assert_eq!(convert_danish_date("2026-03-22"), "22-03-2026");
        assert_eq!(convert_danish_date("garbage"), "garbage");
        assert_eq!(convert_danish_date(""), "");
    }

    fn make_integer_value(val: &str) -> PollenFeedValue {
        PollenFeedValue {
            map_value: None,
            string_value: None,
            integer_value: Some(val.to_string()),
            boolean_value: None,
        }
    }

    fn make_string_value(val: &str) -> PollenFeedValue {
        PollenFeedValue {
            map_value: None,
            string_value: Some(val.to_string()),
            integer_value: None,
            boolean_value: None,
        }
    }

    fn make_map_value(fields: HashMap<String, PollenFeedValue>) -> PollenFeedValue {
        PollenFeedValue {
            map_value: Some(PollenFeedMap { fields }),
            string_value: None,
            integer_value: None,
            boolean_value: None,
        }
    }

    fn build_pollen_entry(level: &str) -> PollenFeedValue {
        let pollen_fields = HashMap::from([
            ("level".to_string(), make_integer_value(level)),
        ]);
        make_map_value(pollen_fields)
    }

    fn build_pollen_entry_with_prediction(level: &str, pred_date: &str, pred_level: &str) -> PollenFeedValue {
        let prediction_inner = HashMap::from([
            ("prediction".to_string(), make_string_value(pred_level)),
        ]);
        let predictions = HashMap::from([
            (pred_date.to_string(), make_map_value(prediction_inner)),
        ]);
        let pollen_fields = HashMap::from([
            ("level".to_string(), make_integer_value(level)),
            ("predictions".to_string(), make_map_value(predictions)),
        ]);
        make_map_value(pollen_fields)
    }

    #[test]
    fn transform_basic_level() {
        // pollen id "7" = birch
        let data_fields = HashMap::from([
            ("7".to_string(), build_pollen_entry("3")),
        ]);
        let region_fields = HashMap::from([
            ("data".to_string(), make_map_value(data_fields)),
        ]);
        let raw = PollenFeedResponse {
            fields: HashMap::from([
                ("48".to_string(), make_map_value(region_fields)),
            ]),
        };

        let forecasts = transform(raw);
        assert_eq!(forecasts.len(), 1);
        assert_eq!(forecasts[0].region, "48");
        assert_eq!(forecasts[0].pollen_type, "birch");
        assert_eq!(forecasts[0].level, 3);
        assert!(!forecasts[0].is_forecast);
    }

    #[test]
    fn transform_normalizes_negative_one_to_zero() {
        let data_fields = HashMap::from([
            ("28".to_string(), build_pollen_entry("-1")),
        ]);
        let region_fields = HashMap::from([
            ("data".to_string(), make_map_value(data_fields)),
        ]);
        let raw = PollenFeedResponse {
            fields: HashMap::from([
                ("49".to_string(), make_map_value(region_fields)),
            ]),
        };

        let forecasts = transform(raw);
        assert_eq!(forecasts.len(), 1);
        assert_eq!(forecasts[0].level, 0);
        assert_eq!(forecasts[0].pollen_type, "grass");
    }

    #[test]
    fn transform_skips_unknown_pollen_ids() {
        let data_fields = HashMap::from([
            ("999".to_string(), build_pollen_entry("2")),
        ]);
        let region_fields = HashMap::from([
            ("data".to_string(), make_map_value(data_fields)),
        ]);
        let raw = PollenFeedResponse {
            fields: HashMap::from([
                ("48".to_string(), make_map_value(region_fields)),
            ]),
        };

        let forecasts = transform(raw);
        assert!(forecasts.is_empty());
    }

    #[test]
    fn transform_includes_predictions_as_forecasts() {
        let data_fields = HashMap::from([
            ("1".to_string(), build_pollen_entry_with_prediction("2", "24-03-2026", "4")),
        ]);
        let region_fields = HashMap::from([
            ("data".to_string(), make_map_value(data_fields)),
        ]);
        let raw = PollenFeedResponse {
            fields: HashMap::from([
                ("48".to_string(), make_map_value(region_fields)),
            ]),
        };

        let forecasts = transform(raw);
        assert_eq!(forecasts.len(), 2);

        let current = forecasts.iter().find(|f| !f.is_forecast).unwrap();
        assert_eq!(current.level, 2);
        assert_eq!(current.pollen_type, "alder");

        let prediction = forecasts.iter().find(|f| f.is_forecast).unwrap();
        assert_eq!(prediction.level, 4);
        assert_eq!(prediction.date, "2026-03-24");
    }
}
