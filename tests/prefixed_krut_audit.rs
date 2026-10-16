//! Prefixed-kṛdanta audit: the kṛt-aware live path vs skt-morph-data.
//!
//! The portal renders backfilled corpus cells for reachable stacks
//! (pinned exact by `backfill_audit`); every other stack goes through
//! `prefixed_krut_cells` (ल्यप् baked-stack strip + neuter dental twin +
//! per-`dhātu` `GenCtx` — see `prefix_krut`). This drives that exact export
//! over every corpus (dhātu, stack, family, kṛt, field, alternant) and
//! demands cell-exact agreement: same rows, same fields, same alternants.
//!
//! Corpus: `SKT_MORPH_DATA` (default
//! `/home/edhiraj/Documents/projs/pypanini/skt-morph-data`); absent → skip
//! with notice, never false-green.
//!
//! Sampled tier runs in plain `cargo test`; exhaustive is `#[ignore]`d:
//! `cargo test --release -- --ignored prefixed_krut`.

use dhaturupa_wasm::forms::{self, KRUT_FAMILIES};
use dhaturupa_wasm::translit::slp1_to_deva;
use dhaturupa_wasm::wasm;
use std::collections::HashMap;
use std::sync::OnceLock;

/// (portal family, corpus krut-family).
const FAMILIES: [(&str, &str); 5] = [
    ("shuddha", "krut"),
    ("nich", "nich_krut"),
    ("san", "san_krut"),
    ("yang", "yang_krut"),
    ("yangluk", "yangluk_krut"),
];

fn corpus_root() -> Option<std::path::PathBuf> {
    let root = std::env::var("SKT_MORPH_DATA")
        .unwrap_or_else(|_| "/home/edhiraj/Documents/projs/pypanini/skt-morph-data".to_string());
    let path = std::path::PathBuf::from(root);
    path.is_dir().then_some(path)
}

struct Shared {
    dhatu: dhaturupa_wasm::DhatuFile,
    vidyut: HashMap<String, String>,
    krut: Vec<(String, dhaturupa_wasm::forms::KrutMap)>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let dhatu_text = std::fs::read_to_string("data/dhatu_data.json").expect("dhatu data loads");
        let dhatu = dhaturupa_wasm::load_str(&dhatu_text).expect("dhatu data parses");
        let vidyut_text =
            std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").expect("vidyut loads");
        let vidyut = dhaturupa_wasm::vidyut::load_tsv_str(&vidyut_text)
            .into_iter()
            .map(|row| (row.code, row.dhatu))
            .collect();
        let mut krut = Vec::new();
        for family in KRUT_FAMILIES {
            let path = forms::krut_filename(family).expect("known family");
            let text = std::fs::read_to_string(&path).expect("krut file loads");
            let map = forms::parse_forms_map(&text).expect("krut file parses");
            krut.push((family.to_string(), map));
        }
        Shared {
            dhatu,
            vidyut,
            krut,
        }
    })
}

fn ok_envelope(text: &str, context: &str) -> serde_json::Value {
    let parsed: serde_json::Value = serde_json::from_str(text).expect("envelope parses");
    assert_eq!(
        parsed["ok"],
        serde_json::json!(true),
        "{context}: {}",
        parsed["error"].as_str().unwrap_or("?")
    );
    parsed
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

fn audit(step: usize) -> (u64, u64, u64, u64, Vec<String>) {
    let Some(root) = corpus_root() else {
        eprintln!("prefixed-krut: no corpus at SKT_MORPH_DATA, skipping");
        return (0, 0, 0, 0, Vec::new());
    };
    let shared = shared();
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for gana in ["01", "02", "03", "04", "05", "06", "07", "08", "09", "10"] {
        let dir = root.join(gana);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.filter_map(std::result::Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|x| x == "json") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut checked = 0_u64;
    let mut matched = 0_u64;
    let mut no_truth = 0_u64;
    let mut miss_total = 0_u64;
    let mut misses = Vec::new();
    let mut classes: HashMap<String, u64> = HashMap::new();
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
        let Some(dhatu) = shared.dhatu.data.iter().find(|e| e.baseindex == id) else {
            continue;
        };
        let Some(dhatu_slp) = shared.vidyut.get(id) else {
            continue;
        };
        let Some(forms) = entry.get("upasarga_forms").and_then(|v| v.as_object()) else {
            continue;
        };
        for (stack_slp, uf) in forms {
            let Some(stack_deva) = stack_deva_display(stack_slp) else {
                continue;
            };
            let Some(parts) = uf.get("participles").and_then(|v| v.as_object()) else {
                continue;
            };
            for (family, corpus_fam) in FAMILIES {
                let Some(upstream) = shared
                    .krut
                    .iter()
                    .find(|(f, _)| f == family)
                    .and_then(|(_, m)| m.get(id))
                else {
                    continue;
                };
                let category = ok_envelope(&wasm::engine_category(family, "krut"), "category")
                    ["category"]
                    .as_str()
                    .expect("category")
                    .to_string();
                let Some(want_fam) = parts.get(corpus_fam).and_then(|v| v.as_object()) else {
                    continue;
                };
                for (krt, cells) in upstream {
                    let Some(krt_rows) = want_fam
                        .iter()
                        .find(|(k, _)| slp1_to_deva(k).as_deref() == Some(krt.as_str()))
                        .map(|(_, v)| v)
                    else {
                        no_truth += 1;
                        continue;
                    };
                    let Some(truth_rows) = krt_rows.as_array() else {
                        continue;
                    };
                    let rows = forms::parse_krt_rows(cells);
                    // ROW SHAPE: the corpus enumerates one row per sandhi
                    // variant (`samBUya` and `saMBUya` are two rows for
                    // `अभि + सम्`); an upstream row lists its alternants
                    // inside the cells. Both display the same column content,
                    // so the contract is per-field, summed over rows — never
                    // a 1:1 row zip.
                    let mut want_by_field: [Vec<String>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for truth in truth_rows {
                        for (slot, field) in ["base", "m", "f", "n"].iter().enumerate() {
                            let Some(want) = truth.get(*field).and_then(|v| v.as_str()) else {
                                continue;
                            };
                            for form in want.split('/').filter(|s| !s.is_empty()) {
                                if let Some(deva) = slp1_to_deva(form) {
                                    want_by_field[slot].push(deva);
                                }
                            }
                        }
                    }
                    for alts in &mut want_by_field {
                        alts.sort();
                    }
                    let mut got_by_field: [Vec<String>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for row in &rows {
                        // One export call per row (kṛt-aware path shared
                        // with the portal); compare each field's union.
                        let res = ok_envelope(
                            &wasm::prefixed_krut_row(
                                &stack_deva,
                                krt,
                                &serde_json::to_string(&serde_json::json!({
                                    "base": row.base,
                                    "m": row.m,
                                    "f": row.f,
                                    "n": row.n,
                                }))
                                .expect("json"),
                                dhatu_slp,
                                &category,
                                &dhatu.dhatu_note,
                            ),
                            "prefixed_krut_row",
                        );
                        for (slot, field) in ["base", "m", "f", "n"].iter().enumerate() {
                            for form in res["row"][field]
                                .as_array()
                                .map(|ts| {
                                    ts.iter()
                                        .filter_map(|s| s.as_str())
                                        .map(str::to_string)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default()
                            {
                                got_by_field[slot].push(form);
                            }
                        }
                    }
                    for alts in &mut got_by_field {
                        alts.sort();
                    }
                    for (slot, field) in ["base", "m", "f", "n"].iter().enumerate() {
                        if want_by_field[slot].is_empty() && got_by_field[slot].is_empty() {
                            continue;
                        }
                        if want_by_field[slot] == got_by_field[slot] {
                            matched += 1;
                            checked += 1;
                            continue;
                        }
                        checked += 1;
                        miss_total += 1;
                        let label = format!("{krt}/{field}");
                        *classes.entry(label.clone()).or_insert(0) += 1;
                        let want_deva = &want_by_field[slot];
                        let got_all = &got_by_field[slot];
                        if documented_gap(krt, field, want_deva, got_all) {
                            continue;
                        }
                        let zz_all = std::env::var("ZZ_ALL").is_ok();
                        let cap = if zz_all { 200_000 } else { 400 };
                        if misses.len() < cap
                            && (zz_all
                                || !misses.iter().any(|m: &String| m.starts_with(&label)))
                        {
                            misses.push(format!(
                                "{label} {id} {stack_deva} {family}: truth [{}] vs export [{}]",
                                want_deva.join(" "),
                                got_all.join(" ")
                            ));
                        }
                    }
                }
            }
        }
    }
    let mut top: Vec<_> = classes.into_iter().collect();
    top.sort_by_key(|a| std::cmp::Reverse(a.1));
    eprintln!("miss classes (top 15):");
    for (k, n) in top.iter().take(15) {
        eprintln!("  {k}: {n}");
    }
    (checked, matched, no_truth, miss_total, misses)
}

#[test]
fn prefixed_krut_sampled() {
    let (checked, matched, no_truth, miss_total, misses) = audit(41);
    eprintln!(
        "prefixed-krut-sampled: checked={checked} matched={matched} no-truth={no_truth} miss_total={miss_total} misses={}",
        misses.len()
    );
    assert!(checked > 500, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored prefixed_krut"]
fn prefixed_krut_exhaustive() {
    let (checked, matched, no_truth, miss_total, misses) = audit(1);
    eprintln!(
        "prefixed-krut-exhaustive: checked={checked} matched={matched} no-truth={no_truth} miss_total={miss_total} misses={}",
        misses.len()
    );
    assert!(checked > 10_000, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

/// Temporary probe: classify divergences without gating.
fn documented_gap(_krt: &str, _field: &str, _want: &[String], _got: &[String]) -> bool {
    false
}
