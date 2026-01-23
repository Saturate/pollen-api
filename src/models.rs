use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub slug: String,
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollenType {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollenForecast {
    pub region: String,
    pub pollen_type: String,
    pub pollen_name: String,
    pub date: String,
    pub level: i32,
    pub is_forecast: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryInfo {
    pub code: String,
    pub name: String,
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionsResponse {
    pub country: String,
    pub regions: Vec<Region>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollenTypesResponse {
    pub country: String,
    pub pollen_types: Vec<PollenType>,
}
