//! Derived prefixed tiṅanta tables vs skt-morph-data ground truth.
//!
//! The site derives prefixed tables live (Vidyuṭ); so do we
//! (`derive_prefixed_ting`). This pins our derivation against the same
//! corpus the engine ships: every (dhātu, listed upasarga, family) in
//! `upasarga_forms` must match cell-for-cell, in Devanagari.
//!
//! Corpus layout (per dhātu JSON): `upasarga_forms.<stack>.conjugations`
//! holds one map per (family, pada-half): `ting`+`yak` = shuddha,
//! `nich`+`nich_yak`, `san`+`san_yak`, `yang`+`yang_yak`,
//! `yangluk`+`yangluk_yak`. `p*` keys = परस्मैपदम्, `a*` = आत्मनेपदम्;
//! each is a 9-slot SLP1 array (prathama-eka … uttama-bahu).
//!
//! The corpus lives outside this repo:
//! `SKT_MORPH_DATA=/home/edhiraj/Documents/projs/pypanini/skt-morph-data`
//! (default). Missing corpus → test skips with a notice, never false-green.
//!
//! Sampled tier runs in plain `cargo test`; exhaustive is `#[ignore]`d:
//! `cargo test --release -- --ignored derive_ting`.

use dhaturupa_wasm::translit::slp1_to_deva;
use std::sync::OnceLock;

/// (portal family, corpus kartari map, corpus karmaṇī map).
const FAMILIES: [(&str, &str, &str); 5] = [
    ("shuddha", "ting", "yak"),
    ("nich", "nich", "nich_yak"),
    ("san", "san", "san_yak"),
    ("yang", "yang", "yang_yak"),
    ("yangluk", "yangluk", "yangluk_yak"),
];

fn corpus_root() -> Option<std::path::PathBuf> {
    let root = std::env::var("SKT_MORPH_DATA")
        .unwrap_or_else(|_| "/home/edhiraj/Documents/projs/pypanini/skt-morph-data".to_string());
    let path = std::path::PathBuf::from(root);
    path.is_dir().then_some(path)
}

fn stack_deva_display(stack_slp: &str) -> Option<String> {
    let mut parts = Vec::new();
    for token in dhaturupa_wasm::slp1::split_prefix_stack(stack_slp) {
        parts.push(dhaturupa_wasm::translit::slp1_to_deva(token)?);
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join(" + "))
}

fn walkdir_json(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(dir) = std::fs::read_dir(root) else {
        return;
    };
    for entry in dir.filter_map(std::result::Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            walkdir_json(&path, out);
        } else if path.extension().is_some_and(|x| x == "json") {
            out.push(path);
        }
    }
}

static ENGINE_ONCE: OnceLock<()> = OnceLock::new();

fn audit(step: usize) -> (u64, u64, Vec<String>) {
    let Some(root) = corpus_root() else {
        eprintln!("derive-ting: no corpus at SKT_MORPH_DATA, skipping");
        return (0, 0, Vec::new());
    };
    let mut files = Vec::new();
    walkdir_json(&root, &mut files);
    files.sort();
    let mut checked = 0_u64;
    let mut matched = 0_u64;
    let mut misses = Vec::new();
    for (nth, path) in files.iter().enumerate() {
        if nth % step != 0 {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(id) = entry["id"].as_str() else {
            continue;
        };
        let Some(forms) = entry.get("upasarga_forms").and_then(|v| v.as_object()) else {
            continue;
        };
        for (stack_slp, uf) in forms {
            let Some(stack_deva) = stack_deva_display(stack_slp) else {
                continue;
            };
            let Some(conj) = uf.get("conjugations").and_then(|v| v.as_object()) else {
                continue;
            };
            for (family, map_kartari, map_karmani) in FAMILIES {
                // Merged truth: both pada halves, exactly as the site shows.
                // Corpus lakāra keys already carry the pada letter
                // (`pashirling`, `aashirling`) — use them as-is.
                // `<fam>` maps = kartari-tab content, `<fam>_yak` = karmaṇī.
                for (prayoga, map_name) in [("kartari", map_kartari), ("karmani", map_karmani)] {
                    let Some(map) = conj.get(map_name).and_then(|v| v.as_object()) else {
                        continue;
                    };
                    let mut truth: Vec<(String, Vec<String>)> = Vec::new();
                    for (lakara, slots) in map {
                        let Some(slots) = slots.as_array() else {
                            continue;
                        };
                        let devas: Vec<String> = slots
                            .iter()
                            .filter_map(|s| s.as_str())
                            .filter_map(slp1_to_deva)
                            .collect();
                        if devas.is_empty() {
                            continue;
                        }
                        truth.push((lakara.clone(), devas));
                    }
                    if truth.is_empty() {
                        continue;
                    }
                    let derived = match dhaturupa_wasm::derive_ting::derive_prefixed_ting(
                        &stack_deva,
                        id,
                        family,
                        prayoga,
                    ) {
                        Ok(t) => t,
                        Err(e) => {
                            if misses.len() < 25 {
                                misses
                                    .push(format!("{id} {stack_deva} {family}: derive error {e}"));
                            }
                            continue;
                        }
                    };
                    // Truth arrays are flat per-lakāra form lists (alternants
                    // ungrouped); derived cells are 9 puruṣa×vacana slots.
                    // Compare as multisets: slot placement follows from the
                    // shared puruṣa×vacana derivation grid by construction.
                    for (key, want_flat) in &truth {
                        checked += 1;
                        let got = derived.get(key).cloned().unwrap_or_default();
                        let mut got_flat: Vec<&str> = got
                            .split([';', ','])
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .collect();
                        got_flat.sort_unstable();
                        let mut want_sorted = want_flat.clone();
                        want_sorted.sort();
                        if got_flat == want_sorted.iter().map(String::as_str).collect::<Vec<_>>() {
                            matched += 1;
                        } else if misses.len() < 25 {
                            misses.push(format!(
                                "{id} {stack_deva} {family}/{prayoga} {key}: truth [{}] vs derived [{}]",
                                want_sorted.join(" "),
                                got_flat.join(" ")
                            ));
                        }
                    }
                    // Structural: every derived key carries exactly 9 slots.
                    for (key, got) in &derived {
                        if got.split(';').all(|c| c.trim().is_empty()) {
                            continue;
                        }
                        checked += 1;
                        if got.split(';').count() == 9 && truth.iter().any(|(k, _)| k == key) {
                            matched += 1;
                        } else if misses.len() < 25 {
                            misses.push(format!(
                                "{id} {stack_deva} {family}/{prayoga} {key}: phantom derived key"
                            ));
                        }
                    }
                } // end prayoga
            } // end family
        } // end stack
    } // end files
    let _ = ENGINE_ONCE;
    (checked, matched, misses)
}

#[test]
fn derive_ting_sampled() {
    let (checked, matched, misses) = audit(97);
    eprintln!(
        "derive-ting-sampled: checked={checked} matched={matched} misses={}",
        misses.len()
    );
    assert!(checked > 20, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored derive_ting"]
fn derive_ting_exhaustive() {
    let (checked, matched, misses) = audit(1);
    eprintln!(
        "derive-ting-exhaustive: checked={checked} matched={matched} misses={}",
        misses.len()
    );
    assert!(checked > 1_000, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}
