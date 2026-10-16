//! Blob contract test: a real portal blob feeds the WASM entry points.
//!
//! Picks a dhatu with upasarga stacks, builds its blob from the real table
//! files, then resolves a prefixed surface and a subanta surface through
//! the same entry JSON the portal will pass.

use dhaturupa_wasm::blobs::{self, BlobSources};
use dhaturupa_wasm::forms::{self, KRUT_FAMILIES, TING_FAMILIES, TING_PRAYOGAS};
use std::collections::HashMap;

/// Loaded table files for the blob test.
struct Sources {
    dhatu: dhaturupa_wasm::DhatuFile,
    ting: Vec<(String, String, forms::TingMap)>,
    krut: Vec<(String, forms::KrutMap)>,
    vidyut: Vec<(String, String)>,
}

fn load_sources() -> Sources {
    let dhatu_text = std::fs::read_to_string("data/dhatu_data.json").expect("dhatu data loads");
    let dhatu = dhaturupa_wasm::load_str(&dhatu_text).expect("dhatu data parses");
    let mut ting = Vec::new();
    for family in TING_FAMILIES {
        for prayoga in TING_PRAYOGAS {
            let path = forms::ting_filename(family, prayoga).expect("known family");
            let text = std::fs::read_to_string(&path).expect("ting loads");
            ting.push((
                family.to_string(),
                prayoga.to_string(),
                forms::parse_forms_map(&text).expect("ting parses"),
            ));
        }
    }
    let mut krut = Vec::new();
    for family in KRUT_FAMILIES {
        let path = forms::krut_filename(family).expect("known family");
        let text = std::fs::read_to_string(&path).expect("krut loads");
        krut.push((
            family.to_string(),
            forms::parse_forms_map(&text).expect("krut parses"),
        ));
    }
    Sources {
        dhatu,
        ting,
        krut,
        vidyut: load_vidyut(),
    }
}

fn load_vidyut() -> Vec<(String, String)> {
    let text = std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").expect("vidyut loads");
    dhaturupa_wasm::vidyut::load_tsv_str(&text)
        .into_iter()
        .map(|row| (row.code, row.dhatu))
        .collect()
}

#[test]
fn blob_feeds_wasm_search() {
    let loaded = load_sources();
    let entry = loaded
        .dhatu
        .data
        .iter()
        .find(|item| {
            !item.upasargas.is_empty()
                && loaded
                    .ting
                    .iter()
                    .any(|(_, _, map)| map.contains_key(&item.baseindex))
        })
        .expect("a stacked dhatu with tables");
    let ting_refs: Vec<(String, String, &forms::TingMap)> = loaded
        .ting
        .iter()
        .map(|(family, prayoga, map)| (family.clone(), prayoga.clone(), map))
        .collect();
    let krut_refs: Vec<(String, &forms::KrutMap)> = loaded
        .krut
        .iter()
        .map(|(family, map)| (family.clone(), map))
        .collect();
    let sources = BlobSources {
        ting: ting_refs,
        krut: krut_refs,
        vidyut: loaded
            .vidyut
            .iter()
            .map(|(code, dhatu)| (code.clone(), dhatu.clone()))
            .collect(),
        chandrika: HashMap::new(),
        singles: Vec::new(),
        morphdata_root: None,
        backfill_stats: std::cell::RefCell::new(dhaturupa_wasm::morphdata::BackfillStats::default()),
    };
    let blob = blobs::build_blob(entry, &sources).expect("blob builds");
    assert!(
        blob.payload.to_string().len() < 300_000,
        "blob stays compact"
    );
    let stacks = blobs::stacks_json(&blob.payload);
    assert_ne!(stacks, "[]", "subject has stacks");

    // Prefixed: derive one expected surface under the same context the
    // search uses, then resolve it.
    let kartari = blobs::ting_slice(&blob.payload, "shuddha", "kartari");
    let krut_entry = blobs::krut_slice(&blob.payload, "shuddha");
    let slp1 = blob
        .payload
        .get("slp1")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let category: String =
        serde_json::from_str(&dhaturupa_wasm::wasm::engine_category("shuddha", "ting"))
            .map(|payload: serde_json::Value| {
                payload["category"].as_str().unwrap_or("ting").to_string()
            })
            .expect("category parses");
    let stack: String =
        serde_json::from_str::<Vec<String>>(&stacks).expect("stacks parse")[0].clone();
    let entry_map: std::collections::HashMap<String, String> =
        serde_json::from_str(&kartari).expect("slice parses");
    let variant = entry_map
        .values()
        .flat_map(|joined| {
            joined
                .split(';')
                .flat_map(|slot| slot.split(',').map(str::to_string))
                .collect::<Vec<_>>()
        })
        .find(|variant| !variant.is_empty())
        .expect("a base variant");
    let surfaces: serde_json::Value =
        serde_json::from_str(&dhaturupa_wasm::wasm::prefixed_table_ctx(
            &stack,
            &serde_json::to_string(std::slice::from_ref(&variant)).expect("serializes"),
            slp1,
            &category,
            &entry.dhatu_note,
        ))
        .expect("envelope parses");
    assert_eq!(surfaces["ok"], serde_json::json!(true));
    let surface = surfaces["tables"][0][0]
        .as_str()
        .expect("a surface")
        .to_string();
    let karmani = blobs::ting_slice(&blob.payload, "shuddha", "karmani");
    let dhatu_json = serde_json::json!({
        "family": "shuddha",
        "baseindex": entry.baseindex,
        "slp1": slp1,
        "note": entry.dhatu_note,
    })
    .to_string();
    let hits: serde_json::Value =
        serde_json::from_str(&dhaturupa_wasm::wasm::search_prefixed_dhatu(
            &kartari,
            &karmani,
            &krut_entry,
            &dhatu_json,
            &stacks,
            &surface,
        ))
        .expect("envelope parses");
    assert_eq!(hits["ok"], serde_json::json!(true));
    assert!(
        hits["total"].as_u64().expect("count") >= 1,
        "prefixed surface resolves"
    );

    // Subanta: first declinable cell that builds, then resolve one of its cells.
    let krut_map: std::collections::HashMap<String, String> =
        serde_json::from_str(&krut_entry).expect("slice parses");
    let mut resolved = false;
    let mut krts: Vec<&String> = krut_map.keys().collect();
    krts.sort();
    for krt in krts {
        for cell in forms::parse_krt_rows(&krut_map[krt]) {
            for field in ["m", "f", "n"] {
                let value = match field {
                    "m" => &cell.m,
                    "f" => &cell.f,
                    _ => &cell.n,
                };
                if value.is_empty() {
                    continue;
                }
                let table: serde_json::Value = serde_json::from_str(
                    &dhaturupa_wasm::wasm::krt_subanta("shuddha", krt, field, value),
                )
                .expect("envelope parses");
                if table["ok"] != serde_json::json!(true) {
                    continue;
                }
                let target = table["tables"][0]["cells"][2]["forms"][0]
                    .as_str()
                    .expect("a cell form")
                    .to_string();
                let hits: serde_json::Value =
                    serde_json::from_str(&dhaturupa_wasm::wasm::search_subanta_dhatu(
                        &krut_entry,
                        "shuddha",
                        &entry.baseindex,
                        &target,
                    ))
                    .expect("envelope parses");
                assert!(
                    hits["total"].as_u64().expect("count") >= 1,
                    "subanta cell resolves"
                );
                resolved = true;
                break;
            }
            if resolved {
                break;
            }
        }
        if resolved {
            break;
        }
    }
    assert!(resolved, "some krt cell builds and resolves");
}
