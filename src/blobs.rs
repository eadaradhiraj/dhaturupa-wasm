//! Per-dhatu portal blobs: one compact JSON per dhatu for lazy fetching.
//!
//! Shape (only present families/prayogas are included):
//!
//! ```json
//! {"baseindex": "01.0001", "meta": {…`DhatuEntry`…}, "slp1": "BU",
//!  "stacks": ["प्र", …],
//!  "ting": {"shuddha": {"kartari": {"plat": "…;…"}, "karmani": {…}}, …},
//!  "krut": {"shuddha": {"ल्युट्": "base,m,f,n"}, …}}
//! ```
//!
//! Table slices are verbatim `dhatuforms_*` entries — exactly what the
//! `wasm::search_prefixed_dhatu` / `search_subanta_dhatu` entry params and
//! the `ting_table` / `krut_table_json` maps accept. Measured ≈115KB raw,
//! ≈10KB gzipped per dhatu.

use crate::forms::{KrutMap, TingMap, KRUT_FAMILIES, TING_FAMILIES, TING_PRAYOGAS};
use std::collections::HashMap;

/// Assembled blob for one dhatu.
#[derive(Debug, Clone)]
pub struct DhatuBlob {
    /// Canonical baseindex (also the `<baseindex>.json` filename).
    pub baseindex: String,
    /// Portal payload (see module docs).
    pub payload: serde_json::Value,
}

/// Loaded source tables for blob building.
pub struct BlobSources<'a> {
    /// `(family, prayoga, map)` for the 10 ting files.
    pub ting: Vec<(String, String, &'a TingMap)>,
    /// `(family, map)` for the 5 krut files.
    pub krut: Vec<(String, &'a KrutMap)>,
    /// `baseindex -> SLP1 dhatu` join (`vidyut_dhatupatha_5.tsv`).
    pub vidyut: HashMap<String, String>,
    /// Global single-upasarga names across the corpus (no `+` combos),
    /// mirroring the portal's unified prefix dropdown. Together with each
    /// dhātu's listed stacks these are all selectable stacks — all backfilled.
    pub singles: Vec<String>,
    /// skt-morph-data root for prefixed-tiṅanta backfill (`None` = skip;
    /// portal then engine-derives unlisted stacks live).
    pub morphdata_root: Option<std::path::PathBuf>,
    /// Backfill accounting across `build_all`.
    pub backfill_stats: std::cell::RefCell<crate::morphdata::BackfillStats>,
    /// `baseindex -> [(upasarga name, entry)]` (chandrika; possibly empty).
    pub chandrika: HashMap<String, Vec<(String, crate::chandrika::ChandrikaEntry)>>,
}

/// Build the blob for one entry (`None` when the dhatu has no tables at all,
/// e.g. ganasutra rows).
#[must_use]
pub fn build_blob(entry: &crate::DhatuEntry, sources: &BlobSources<'_>) -> Option<DhatuBlob> {
    let baseindex = entry.baseindex.clone();
    let mut ting = serde_json::Map::new();
    for family in TING_FAMILIES {
        let mut prayogas = serde_json::Map::new();
        for prayoga in TING_PRAYOGAS {
            let found = sources
                .ting
                .iter()
                .find(|(fam, pra, _)| fam == family && pra == prayoga)
                .and_then(|(_, _, map)| map.get(&baseindex));
            if let Some(tables) = found {
                prayogas.insert(
                    prayoga.to_string(),
                    serde_json::to_value(tables).unwrap_or(serde_json::Value::Null),
                );
            }
        }
        if !prayogas.is_empty() {
            ting.insert(family.to_string(), serde_json::Value::Object(prayogas));
        }
    }
    let mut krut = serde_json::Map::new();
    for family in KRUT_FAMILIES {
        let found = sources
            .krut
            .iter()
            .find(|(fam, _)| fam == family)
            .and_then(|(_, map)| map.get(&baseindex));
        if let Some(tables) = found {
            krut.insert(
                family.to_string(),
                serde_json::to_value(tables).unwrap_or(serde_json::Value::Null),
            );
        }
    }
    if ting.is_empty() && krut.is_empty() {
        return None;
    }
    let stacks: Vec<serde_json::Value> = entry
        .upasargas
        .iter()
        .map(|item| serde_json::Value::String(item.name.clone()))
        .collect();
    // Prefixed backfill, one selectable stack at a time: corpus values
    // (byte-exact site parity). Selectable = listed stacks + global
    // singles, exactly mirroring the portal dropdown — everything the
    // user can pick renders backfilled cells; live paths serve only
    // no-corpus builds and corpus-missing pairs.
    let mut offered: Vec<String> = entry
        .upasargas
        .iter()
        .map(|item| item.name.clone())
        .collect();
    for single in &sources.singles {
        if !offered.contains(single) {
            offered.push(single.clone());
        }
    }
    let mut ting_prefixed = serde_json::Map::new();
    let mut krut_prefixed = serde_json::Map::new();
    if let Some(root) = sources.morphdata_root.as_deref() {
        let mut stats = sources.backfill_stats.borrow_mut();
        for name in &offered {
            if let Some(tables) =
                crate::morphdata::backfill_prefixed_ting(root, name, &baseindex, &mut stats)
            {
                ting_prefixed.insert(name.clone(), tables);
            }
            if let Some(tables) = crate::morphdata::backfill_prefixed_krut(root, name, &baseindex) {
                krut_prefixed.insert(name.clone(), tables);
            }
        }
    }
    let meta = serde_json::to_value(entry).unwrap_or(serde_json::Value::Null);
    let slp1 = sources
        .vidyut
        .get(&baseindex)
        .map_or(serde_json::Value::Null, |dhatu| {
            serde_json::Value::String(dhatu.clone())
        });
    let chandrika: Vec<serde_json::Value> =
        sources
            .chandrika
            .get(&baseindex)
            .map_or(Vec::new(), |entries| {
                entries
                    .iter()
                    .map(|(upasarga, entry)| {
                        serde_json::json!({
                            "upasarga": upasarga,
                            "artha": entry.artha,
                            "examples": entry.examples,
                        })
                    })
                    .collect()
            });
    Some(DhatuBlob {
        baseindex: baseindex.clone(),
        payload: serde_json::json!({
            "baseindex": baseindex,
            "meta": meta,
            "slp1": slp1,
            "stacks": stacks,
            "ting": ting,
            "krut": krut,
            "ting_prefixed": ting_prefixed,
            "krut_prefixed": krut_prefixed,
            "chandrika": chandrika,
        }),
    })
}

/// Build blobs for every entry that has tables; returns `(built, skipped)`.
pub fn build_all(file: &crate::DhatuFile, sources: &BlobSources<'_>) -> (Vec<DhatuBlob>, usize) {
    let mut blobs = Vec::new();
    let mut skipped = 0_usize;
    for entry in &file.data {
        match build_blob(entry, sources) {
            Some(blob) => blobs.push(blob),
            None => skipped += 1,
        }
    }
    (blobs, skipped)
}

/// Slice helpers matching the `wasm` entry params (portal contract):
/// missing families/prayogas yield `{}`/`[]` so calls stay total.
#[must_use]
pub fn ting_slice(payload: &serde_json::Value, family: &str, prayoga: &str) -> String {
    payload
        .get("ting")
        .and_then(|ting| ting.get(family))
        .and_then(|fam| fam.get(prayoga))
        .map_or("{}".to_string(), std::string::ToString::to_string)
}

/// Krut entry slice for `family` (`{}` when the dhatu lacks it).
#[must_use]
pub fn krut_slice(payload: &serde_json::Value, family: &str) -> String {
    payload
        .get("krut")
        .and_then(|krut| krut.get(family))
        .map_or("{}".to_string(), std::string::ToString::to_string)
}

/// Upasarga stack names for the dhatu (`[]` when none).
#[must_use]
pub fn stacks_json(payload: &serde_json::Value) -> String {
    payload
        .get("stacks")
        .map_or("[]".to_string(), std::string::ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_tableless_entries() {
        let file = crate::DhatuFile {
            name: "dhatu".to_string(),
            data: vec![],
        };
        let sources = BlobSources {
            ting: vec![],
            krut: vec![],
            vidyut: HashMap::new(),
            chandrika: HashMap::new(),
            singles: Vec::new(),
            morphdata_root: None,
            backfill_stats: std::cell::RefCell::new(crate::morphdata::BackfillStats::default()),
        };
        let (blobs, skipped) = build_all(&file, &sources);
        assert!(blobs.is_empty());
        assert_eq!(skipped, 0);
    }
}
