//! Upasargartha Chandrika: per-(upasarga, dhatu) meanings with examples.
//!
//! Source: `kosha/upasargartha_chandrika.json`
//! (`{"data": [{"num", "name": "प्र", "data": [{"dhatu", "artha",
//! "baseindex", "examples": [...]}]}]}`). Joins to dhatus by `baseindex`
//! (658 dhatus covered). Detail scope only — general kosha search stays
//! out of scope (see README).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level file shape.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChandrikaFile {
    /// One entry per upasarga or stack (236: `प्र`, `प्र + सम्`, …).
    #[serde(default)]
    pub data: Vec<ChandrikaUpasarga>,
}

/// One upasarga section.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChandrikaUpasarga {
    /// Order number.
    #[serde(default)]
    pub num: u32,
    /// Upasarga or stack name (`प्र`, `प्र + सम्`).
    #[serde(default)]
    pub name: String,
    /// Per-dhatu entries.
    #[serde(default)]
    pub data: Vec<ChandrikaEntry>,
}

/// One (upasarga, dhatu) meaning with literary examples.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChandrikaEntry {
    /// Dhatu as listed (`डुदाञ्`).
    #[serde(default)]
    pub dhatu: String,
    /// Meaning (`डुदाञ् दाने`).
    #[serde(default)]
    pub artha: String,
    /// Join key to [`crate::DhatuEntry::baseindex`].
    #[serde(default)]
    pub baseindex: String,
    /// Examples with `<hl>`-marked spans.
    #[serde(default)]
    pub examples: Vec<String>,
}

/// Parse the file payload.
pub fn load_str(text: &str) -> Result<ChandrikaFile, serde_json::Error> {
    serde_json::from_str(text)
}

/// `baseindex -> [(upasarga name, entry)]`, file order preserved.
#[must_use]
pub fn baseindex_map(file: &ChandrikaFile) -> HashMap<&str, Vec<(&str, &ChandrikaEntry)>> {
    let mut map: HashMap<&str, Vec<(&str, &ChandrikaEntry)>> = HashMap::new();
    for upasarga in &file.data {
        for entry in &upasarga.data {
            if entry.baseindex.is_empty() {
                continue;
            }
            map.entry(entry.baseindex.as_str())
                .or_default()
                .push((upasarga.name.as_str(), entry));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{"data":[
      {"num": 1, "name": "प्र", "data": [
        {"dhatu": "अज्", "artha": "अज गतिक्षेपणयोः", "baseindex": "01.0262",
         "examples": ["<hl>प्राजितः</hl> कालितः।"]}
      ]}
    ]}"#;

    #[test]
    fn loads_and_joins() {
        let file = load_str(FIXTURE).expect("parses");
        assert_eq!(file.data.len(), 1);
        let map = baseindex_map(&file);
        let entries = map.get("01.0262").expect("joined");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "प्र");
        assert_eq!(entries[0].1.examples.len(), 1);
        assert!(!map.contains_key("99.9999"));
    }
}
