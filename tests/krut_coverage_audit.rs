//! Kṛdanta-table coverage audit: nothing upstream ships may be dropped.
//!
//! Mirror of `ting_coverage_audit.rs`: every non-empty
//! (baseindex, kṛt, row, field) cell in the upstream krut files must appear
//! in the shipped per-dhātu blobs (what the detail page renders), for all
//! five families. Counts the indeclinable-by-design cells separately
//! (`tṛc.n`, verbal citations) so the report distinguishes drop from design.

use dhaturupa_wasm::forms::{self, KRUT_FAMILIES};
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

fn blob_cell(
    blob: &serde_json::Value,
    family: &str,
    krt: &str,
    row: usize,
    field: usize,
) -> String {
    blob["krut"][family][krt]
        .as_str()
        .and_then(|rows| rows.split(';').nth(row))
        .and_then(|r| r.split(',').nth(field))
        .unwrap_or("")
        .to_string()
}

#[test]
fn krut_coverage_raw_to_blob() {
    let mut checked = 0_u64;
    let mut dropped: Vec<String> = Vec::new();
    let mut empty_by_upstream = 0_u64;

    for family in KRUT_FAMILIES {
        let path = forms::krut_filename(family).expect("known family");
        let text = std::fs::read_to_string(&path).expect("krut file loads");
        let map = forms::parse_forms_map(&text).expect("krut file parses");
        for (id, krts) in &map {
            let Some(blob) = blobs().get(id) else {
                dropped.push(format!("{id}: no blob"));
                continue;
            };
            for (krt, cells) in krts {
                for (row, cell) in forms::parse_krt_rows(cells).iter().enumerate() {
                    for (field, value) in [&cell.base, &cell.m, &cell.f, &cell.n].iter().enumerate()
                    {
                        if value.trim().is_empty() {
                            empty_by_upstream += 1;
                            continue;
                        }
                        checked += 1;
                        let got = blob_cell(blob, family, krt, row, field);
                        if got.trim().is_empty() {
                            dropped.push(format!("{id} {family} {krt}[{row}]"));
                        }
                    }
                }
            }
        }
    }

    eprintln!(
        "krut-coverage: checked={checked} dropped={} empty-by-upstream={empty_by_upstream}",
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
    assert!(checked > 100_000, "audit covered too little: {checked}");
}
