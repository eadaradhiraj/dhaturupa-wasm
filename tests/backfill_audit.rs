//! Prefixed-tiṅanta backfill audit: built blobs must carry the corpus
//! values byte-exactly for every listed (dhātu, stack).
//!
//! The blob builder backfills `ting_prefixed[stack][family][prayoga][lakara]`
//! from skt-morph-data (values) in engine slot grouping. This walks every
//! built blob in `dist/data/blobs` against the corpus and demands exact
//! cell equality — the portal's prefixed views render these cells directly,
//! so this is site parity by construction.
//!
//! Corpus: `SKT_MORPH_DATA` (default
//! `/home/edhiraj/Documents/projs/pypanini/skt-morph-data`). Missing corpus
//! or missing blobs dir → skip with notice, never false-green.
//!
//! Sampled tier runs in plain `cargo test`; exhaustive is `#[ignore]`d:
//! `cargo test --release -- --ignored backfill`.

use dhaturupa_wasm::translit::slp1_to_deva;
use std::sync::OnceLock;

fn corpus_root() -> Option<std::path::PathBuf> {
    let root = std::env::var("SKT_MORPH_DATA")
        .unwrap_or_else(|_| "/home/edhiraj/Documents/projs/pypanini/skt-morph-data".to_string());
    let path = std::path::PathBuf::from(root);
    path.is_dir().then_some(path)
}

/// Corpus SLP1 key for an offered display name: exact transliteration
/// first, then Ṅ-appended variants per token (portal `आ` vs corpus `AN` —
/// same rule as the build's backfill lookup, so audit and builder agree).
fn corpus_key_for<'a>(
    forms: &'a serde_json::Map<String, serde_json::Value>,
    stack_deva: &str,
) -> Option<&'a String> {
    // Same rule as the build: normalized first (bare `A` means आङ्),
    // then exact.
    let base = dhaturupa_wasm::translit::deva_stack_to_slp1(stack_deva)?;
    let normalized = dhaturupa_wasm::translit::normalize_stack_slp1(&base);
    if normalized != base {
        if let Some(key) = forms.keys().find(|k| k.as_str() == normalized) {
            return Some(key);
        }
    }
    forms.keys().find(|k| k.as_str() == base)
}

/// Corpus map names per (portal family, prayoga).
fn corpus_map(family: &str, prayoga: &str) -> Option<&'static str> {
    match (family, prayoga) {
        ("shuddha", "kartari") => Some("ting"),
        ("shuddha", "karmani") => Some("yak"),
        ("nich", "kartari") => Some("nich"),
        ("nich", "karmani") => Some("nich_yak"),
        ("san", "kartari") => Some("san"),
        ("san", "karmani") => Some("san_yak"),
        ("yang", "kartari") => Some("yang"),
        ("yang", "karmani") => Some("yang_yak"),
        ("yangluk", "kartari") => Some("yangluk"),
        ("yangluk", "karmani") => Some("yangluk_yak"),
        _ => None,
    }
}

/// Corpus krut-family per portal family.
fn corpus_krut_family(family: &str) -> Option<&'static str> {
    match family {
        "shuddha" => Some("krut"),
        "nich" => Some("nich_krut"),
        "san" => Some("san_krut"),
        "yang" => Some("yang_krut"),
        "yangluk" => Some("yangluk_krut"),
        _ => None,
    }
}

fn blob_paths(step: usize) -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = std::fs::read_dir("dist/data/blobs")
        .map(|dir| {
            dir.filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out.into_iter()
        .enumerate()
        .filter(|(n, _)| n % step == 0)
        .map(|(_, p)| p)
        .collect()
}

static SEEN: OnceLock<()> = OnceLock::new();

fn audit(step: usize) -> (u64, u64, Vec<String>) {
    let Some(root) = corpus_root() else {
        eprintln!("backfill: no corpus at SKT_MORPH_DATA, skipping");
        return (0, 0, Vec::new());
    };
    let _ = SEEN;
    let mut checked = 0_u64;
    let mut matched = 0_u64;
    let mut misses = Vec::new();
    let mut dbg_krut_stacks = 0_u64;
    let mut dbg_krut_fams = 0_u64;
    let mut dbg_krut_names = 0_u64;
    let mut dbg_truth_hit = 0_u64;
    for path in blob_paths(step) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(blob) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(id) = blob["baseindex"].as_str() else {
            continue;
        };
        let gana = id.split('.').next().unwrap_or("01");
        let Ok(corpus_text) = std::fs::read_to_string(root.join(gana).join(format!("{id}.json")))
        else {
            continue;
        };
        let Ok(corpus) = serde_json::from_str::<serde_json::Value>(&corpus_text) else {
            continue;
        };
        let Some(forms) = corpus.get("upasarga_forms").and_then(|v| v.as_object()) else {
            continue;
        };
        // Backfilled stacks: union of ting + krut (either side may cover
        // a stack the other lacks).
        let mut stack_names: Vec<&String> = Vec::new();
        for key in ["ting_prefixed", "krut_prefixed"] {
            if let Some(map) = blob.get(key).and_then(|v| v.as_object()) {
                for name in map.keys() {
                    if !stack_names.contains(&name) {
                        stack_names.push(name);
                    }
                }
            }
        }
        for stack_deva in stack_names {
            // Same resolution rule as the build (exact, then Ṅ-variants).
            let Some(stack_slp) = corpus_key_for(forms, stack_deva) else {
                if misses.len() < 25 {
                    misses.push(format!("{id} {stack_deva}: no corpus stack"));
                }
                continue;
            };
            let Some(conj) = forms[stack_slp].get("conjugations") else {
                continue;
            };
            let Some(parts) = forms[stack_slp].get("participles") else {
                continue;
            };
            if let Some(ting_families) = blob
                .get("ting_prefixed")
                .and_then(|v| v.as_object())
                .and_then(|m| m.get(stack_deva))
                .and_then(|v| v.as_object())
            {
                for (family, prayogas) in ting_families {
                    let Some(prayogas) = prayogas.as_object() else {
                        continue;
                    };
                    for (prayoga, lakaras) in prayogas {
                        let Some(map_name) = corpus_map(family, prayoga) else {
                            continue;
                        };
                        let Some(lakaras) = lakaras.as_object() else {
                            continue;
                        };
                        for (lakara, cells) in lakaras {
                            let Some(cells) = cells.as_str() else {
                                continue;
                            };
                            let want: Vec<String> = conj[map_name][lakara]
                                .as_array()
                                .map(|slots| {
                                    slots
                                        .iter()
                                        .filter_map(|s| s.as_str())
                                        .filter_map(slp1_to_deva)
                                        .collect()
                                })
                                .unwrap_or_default();
                            if want.is_empty() {
                                continue;
                            }
                            checked += 1;
                            // Corpus arrays are flat; blob cells are 9 slots —
                            // compare flattened multisets (slot grouping is the
                            // engine's grid, pinned by the derive audit).
                            let mut got: Vec<&str> = cells
                                .split([';', ','])
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .collect();
                            got.sort_unstable();
                            let mut want_sorted = want.clone();
                            want_sorted.sort();
                            if got == want_sorted.iter().map(String::as_str).collect::<Vec<_>>() {
                                matched += 1;
                            } else if misses.len() < 25 {
                                misses.push(format!(
                                "{id} {stack_deva} {family}/{prayoga} {lakara}: corpus [{}] vs blob [{}]",
                                want_sorted.join(" "),
                                got.join(" ")
                            ));
                            }
                        }
                    }
                }
            }
            // Kṛdanta backfill: blob rows vs corpus participle rows, exact
            // per field (rows are explicit — no slot partition involved).
            dbg_krut_stacks += 1;
            let Some(krut_families) = blob
                .get("krut_prefixed")
                .and_then(|v| v.as_object())
                .and_then(|m| m.get(stack_deva))
                .and_then(|v| v.as_object())
            else {
                continue;
            };
            for (family, krts) in krut_families {
                let Some(corpus_fam) = corpus_krut_family(family) else {
                    continue;
                };
                dbg_krut_fams += 1;
                let Some(krts) = krts.as_object() else {
                    continue;
                };
                for (krt, rows) in krts {
                    let Some(rows) = rows.as_str() else {
                        continue;
                    };
                    // Corpus kṛt key by transliteration.
                    dbg_krut_names += 1;
                    let Some(truth_rows) = parts
                        .get(corpus_fam)
                        .and_then(|v| v.as_object())
                        .and_then(|m| {
                            m.iter()
                                .find(|(k, _)| {
                                    dhaturupa_wasm::translit::slp1_to_deva(k).as_deref()
                                        == Some(krt.as_str())
                                })
                                .map(|(_, v)| v)
                        })
                        .and_then(|v| v.as_array())
                    else {
                        continue;
                    };
                    dbg_truth_hit += 1;
                    if dbg_truth_hit < 4 {
                        eprintln!("TRACE {id} {stack_deva} {family} krt={krt} rows={rows:?}");
                    }
                    let blob_rows: Vec<&str> = rows.split(';').collect();
                    if blob_rows.len() != truth_rows.len() {
                        if misses.len() < 25 {
                            misses.push(format!(
                                "{id} {stack_deva} {family} {krt}: {} blob rows vs {} corpus rows",
                                blob_rows.len(),
                                truth_rows.len()
                            ));
                        }
                        continue;
                    }
                    for (blob_row, truth) in blob_rows.iter().zip(truth_rows.iter()) {
                        for (field, cell) in ["base", "m", "f", "n"].iter().zip(blob_row.split(','))
                        {
                            let Some(want) = truth.get(field).and_then(|v| v.as_str()) else {
                                continue;
                            };
                            let mut want_deva: Vec<String> = want
                                .split('/')
                                .filter(|s| !s.is_empty())
                                .filter_map(slp1_to_deva)
                                .collect();
                            want_deva.sort();
                            let mut got: Vec<&str> = cell
                                .split('/')
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .collect();
                            got.sort_unstable();
                            if got.is_empty() && want_deva.is_empty() {
                                continue;
                            }
                            checked += 1;
                            if got == want_deva.iter().map(String::as_str).collect::<Vec<_>>() {
                                matched += 1;
                            } else if misses.len() < 25 {
                                misses.push(format!(
                                    "{id} {stack_deva} {family} {krt} {field}: corpus [{}] vs blob [{}]",
                                    want_deva.join(" "),
                                    got.join(" ")
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("dbg_krut: stacks={dbg_krut_stacks} fams={dbg_krut_fams} names={dbg_krut_names} truth_hit={dbg_truth_hit}");
    (checked, matched, misses)
}

#[test]
fn backfill_sampled() {
    let (checked, matched, misses) = audit(29);
    eprintln!(
        "backfill-sampled: checked={checked} matched={matched} misses={}",
        misses.len()
    );
    assert!(checked > 500, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored backfill"]
fn backfill_exhaustive() {
    let (checked, matched, misses) = audit(1);
    eprintln!(
        "backfill-exhaustive: checked={checked} matched={matched} misses={}",
        misses.len()
    );
    assert!(checked > 10_000, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}
