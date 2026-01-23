use serde::Deserialize;
use anyhow::Result;
use std::collections::HashMap;
use crate::pollen_types;

const DANISH_API_URL: &str = "https://www.astma-allergi.dk/umbraco/Api/PollenApi/GetPollenFeed";

#[derive(Debug, Deserialize)]
pub struct FirestoreDocument {
    pub fields: HashMap<String, FirestoreValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirestoreValue {
    pub map_value: Option<FirestoreMap>,
    pub string_value: Option<String>,
    pub integer_value: Option<String>,
    pub boolean_value: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FirestoreMap {
    pub fields: HashMap<String, FirestoreValue>,
}

pub async fn fetch() -> Result<FirestoreDocument> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(DANISH_API_URL).send().await?;
    let json_string: String = response.json().await?;
    let document: FirestoreDocument = serde_json::from_str(&json_string)?;
    Ok(document)
}

pub fn transform(raw: FirestoreDocument) -> Vec<crate::models::PollenForecast> {
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

fn convert_danish_date(danish_date: &str) -> String {
    let parts: Vec<&str> = danish_date.split('-').collect();
    if parts.len() == 3 {
        format!("{}-{}-{}", parts[2], parts[1], parts[0])
    } else {
        danish_date.to_string()
    }
}
