//! Ting-table coverage audit: nothing upstream ships may be dropped, and the
//! per-dhātu पद coverage is counted so gaps are a NUMBER, not a surprise.
//!
//! Checks the raw→artifact pipeline: every non-empty (baseindex, lakāra,
//! row, field) cell in the upstream ting files must appear in the shipped
//! per-dhātu blobs (what the detail page renders), for all families and
//! prayogas. Also reports how many dhātus ship परस्मैपदम्-only kartari
//! tables (upstream lacks आत्मनेपदम् for them — e.g. जि 01.1096 is
//! परस्मैपदी, so no `alat` section exists unprefixed).

use dhaturupa_wasm::forms::{self, TING_FAMILIES, TING_PRAYOGAS};
use std::collections::HashMap;
use std::sync::OnceLock;

fn blobs() -> &'static HashMap<String, serde_json::Value> {
    static SHARED: OnceLock<HashMap<String, serde_json::Value>> = OnceLock::new();
    SHARED.get_or_init(|| {
        let dir = std::path::Path::new("dist/data/blobs");
        std::fs::read_dir(dir)
            .expect("blobs dir")
            .filter_map(std::result::Result::ok)
            // manifest.json carries the freshness stamp, not a dhatu blob
            .filter(|e| {
                e.path().extension().is_some_and(|x| x == "json")
                    && e.path().file_name().is_some_and(|n| n != "manifest.json")
            })
            .map(|e| {
                let text = std::fs::read_to_string(e.path()).expect("blob loads");
                let v: serde_json::Value = serde_json::from_str(&text).expect("blob parses");
                (v["baseindex"].as_str().expect("baseindex").to_string(), v)
            })
            .collect()
    })
}

/// The blob's ting cell for a key, or "".
fn blob_cell(
    blob: &serde_json::Value,
    family: &str,
    prayoga: &str,
    key: &str,
    slot: usize,
) -> String {
    blob["ting"][family][prayoga][key]
        .as_str()
        .and_then(|rows| rows.split(';').nth(slot))
        .unwrap_or("")
        .to_string()
}

#[test]
fn ting_coverage_raw_to_blob() {
    let mut checked = 0_u64;
    let mut dropped: Vec<String> = Vec::new();
    let mut kartari_parasmai_only = 0_u64;
    let mut kartari_both_padas = 0_u64;
    let mut kartari_atmam_only = 0_u64;

    for family in TING_FAMILIES {
        for prayoga in TING_PRAYOGAS {
            let path = forms::ting_filename(family, prayoga).expect("known family");
            let text = std::fs::read_to_string(&path).expect("ting file loads");
            let map = forms::parse_forms_map(&text).expect("ting file parses");
            for (id, entries) in &map {
                let Some(blob) = blobs().get(id) else {
                    dropped.push(format!("{id}: no blob"));
                    continue;
                };
                for (key, rows) in entries {
                    let slots = rows.split(';').filter(|s| !s.is_empty()).count();
                    for slot in 0..slots {
                        let want = rows.split(';').nth(slot).unwrap_or("");
                        if want.trim().is_empty() {
                            continue;
                        }
                        checked += 1;
                        let got = blob_cell(blob, family, prayoga, key, slot);
                        if got.trim().is_empty() {
                            dropped.push(format!("{id} {family}/{prayoga} {key}[{slot}]"));
                        }
                    }
                }
            }
        }
        // Count the kartari पद coverage per dhātu (a\* = आत्मनेपदम्).
        let path = forms::ting_filename(family, "kartari").expect("known family");
        let text = std::fs::read_to_string(&path).expect("ting file loads");
        let map = forms::parse_forms_map(&text).expect("ting file parses");
        for entries in map.values() {
            let has_p = entries.keys().any(|k| k.starts_with('p'));
            let has_a = entries.keys().any(|k| k.starts_with('a'));
            match (has_p, has_a) {
                (true, true) => kartari_both_padas += 1,
                (true, false) => kartari_parasmai_only += 1,
                (false, true) => kartari_atmam_only += 1,
                _ => {}
            }
        }
    }

    eprintln!(
        "ting-coverage: checked={checked} dropped={} \
         kartari(parasmai-only)={kartari_parasmai_only} \
         kartari(both-padas)={kartari_both_padas} kartari(atmane-only)={kartari_atmam_only}",
        dropped.len()
    );
    assert!(
        dropped.is_empty(),
        "dropped:\n{}",
        dropped
            .iter()
            .take(25)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    // कर्मणी files are a*-only by design (upstream ships no parasmaipada कर्मणी).
    assert!(checked > 100_000, "audit covered too little: {checked}");
}
