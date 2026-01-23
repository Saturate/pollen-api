use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PollenTypeInfo {
    pub id: String,
    pub name_en: String,
    pub name_da: String,
}

pub fn get_pollen_types() -> Vec<PollenTypeInfo> {
    vec![
        PollenTypeInfo {
            id: "alder".to_string(),
            name_en: "Alder".to_string(),
            name_da: "El".to_string(),
        },
        PollenTypeInfo {
            id: "hazel".to_string(),
            name_en: "Hazel".to_string(),
            name_da: "Hassel".to_string(),
        },
        PollenTypeInfo {
            id: "elm".to_string(),
            name_en: "Elm".to_string(),
            name_da: "Elm".to_string(),
        },
        PollenTypeInfo {
            id: "birch".to_string(),
            name_en: "Birch".to_string(),
            name_da: "Birk".to_string(),
        },
        PollenTypeInfo {
            id: "grass".to_string(),
            name_en: "Grass".to_string(),
            name_da: "Græs".to_string(),
        },
        PollenTypeInfo {
            id: "mugwort".to_string(),
            name_en: "Mugwort".to_string(),
            name_da: "Bynke".to_string(),
        },
        PollenTypeInfo {
            id: "alternaria".to_string(),
            name_en: "Alternaria".to_string(),
            name_da: "Alternaria".to_string(),
        },
        PollenTypeInfo {
            id: "cladosporium".to_string(),
            name_en: "Cladosporium".to_string(),
            name_da: "Cladosporium".to_string(),
        },
    ]
}

pub fn denmark_id_to_canonical(danish_id: &str) -> Option<String> {
    let mapping: HashMap<&str, &str> = [
        ("1", "alder"),
        ("2", "hazel"),
        ("4", "elm"),
        ("7", "birch"),
        ("28", "grass"),
        ("31", "mugwort"),
        ("44", "alternaria"),
        ("45", "cladosporium"),
    ].iter().cloned().collect();

    mapping.get(danish_id).map(|s| s.to_string())
}

pub fn get_pollen_name(canonical_id: &str, lang: &str) -> Option<String> {
    let types = get_pollen_types();
    types.iter().find(|t| t.id == canonical_id).map(|t| {
        match lang {
            "da" => t.name_da.clone(),
            _ => t.name_en.clone(),
        }
    })
}
