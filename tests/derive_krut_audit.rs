//! Derived prefixed kṛdanta rows vs skt-morph-data ground truth.
//!
//! The portal derives prefixed kṛdanta rows live through Vidyuṭ (same
//! engine + tag mapping as the site) where no backfill exists — and the
//! backfill itself was built by this path's logic. This pins derivation
//! against the corpus `upasarga_forms.<stack>.participles` rows for every
//! (dhātu, stack, family, kṛt): same rows (as sets — row order is engine
//! order, display sorts deterministically), same fields, same alternants.
//!
//! m/f/n comparison uses the portal's avyaya rule: fields empty upstream
//! stay empty. Corpus: `SKT_MORPH_DATA` (default
//! `/home/edhiraj/Documents/projs/pypanini/skt-morph-data`); absent → skip
//! with notice, never false-green.
//!
//! Sampled tier runs in plain `cargo test`; exhaustive is `#[ignore]`d:
//! `cargo test --release -- --ignored derive_krut`.

use dhaturupa_wasm::forms::{self, KRUT_FAMILIES};
use dhaturupa_wasm::translit::slp1_to_deva;
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

struct Shared {
    upstream: HashMap<String, HashMap<String, HashMap<String, String>>>,
    notes: HashMap<String, String>,
    vidyut: HashMap<String, String>,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| {
        let mut upstream = HashMap::new();
        for family in KRUT_FAMILIES {
            let path = forms::krut_filename(family).expect("known family");
            let text = std::fs::read_to_string(&path).expect("krut file loads");
            let map = forms::parse_forms_map(&text).expect("krut file parses");
            for (bid, krts) in &map {
                for (krt, cells) in krts {
                    upstream
                        .entry(bid.clone())
                        .or_insert_with(HashMap::new)
                        .entry(family.to_string())
                        .or_insert_with(HashMap::new)
                        .insert(krt.clone(), cells.clone());
                }
            }
        }
        let dhatu_text = std::fs::read_to_string("data/dhatu_data.json").expect("dhatu data loads");
        let dhatu = dhaturupa_wasm::load_str(&dhatu_text).expect("dhatu data parses");
        let mut notes = HashMap::new();
        for entry in &dhatu.data {
            notes.insert(entry.baseindex.clone(), entry.dhatu_note.clone());
        }
        let vidyut_text =
            std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").expect("vidyut loads");
        let vidyut = dhaturupa_wasm::vidyut::load_tsv_str(&vidyut_text)
            .into_iter()
            .map(|row| (row.code, row.dhatu))
            .collect();
        Shared {
            upstream,
            notes,
            vidyut,
        }
    })
}

type UpstreamMap = HashMap<String, HashMap<String, HashMap<String, String>>>;

fn upstream_rows() -> &'static UpstreamMap {
    static SHARED: OnceLock<UpstreamMap> = OnceLock::new();
    SHARED.get_or_init(|| shared().upstream.clone())
}

use std::collections::HashMap;

fn audit(step: usize) -> (u64, u64, u64, Vec<String>) {
    let Some(root) = corpus_root() else {
        eprintln!("derive-krut: no corpus at SKT_MORPH_DATA, skipping");
        return (0, 0, 0, Vec::new());
    };
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
    let miss_total = 0_u64;
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
            let Some(parts) = uf.get("participles").and_then(|v| v.as_object()) else {
                continue;
            };
            for (family, corpus_fam) in FAMILIES {
                let Some(want_fam) = parts.get(corpus_fam).and_then(|v| v.as_object()) else {
                    continue;
                };
                // Upstream rows for avyaya masking + supplement.
                let up_fam = upstream_rows().get(id).and_then(|m| m.get(family));
                let dhatu_slp = shared().vidyut.get(id).cloned().unwrap_or_default();
                let note = shared().notes.get(id).cloned().unwrap_or_default();
                let category = dhaturupa_wasm::forms::engine_krut_family(family).unwrap_or("krut");
                for (krt_slp, truth_rows) in want_fam {
                    let Some(krt_deva) = slp1_to_deva(krt_slp) else {
                        continue;
                    };
                    let Some(truth_rows) = truth_rows.as_array() else {
                        continue;
                    };
                    let up_cells = up_fam
                        .and_then(|m| m.get(&krt_deva))
                        .cloned()
                        .unwrap_or_default();
                    let upstream: Vec<(String, String, String, String)> =
                        forms::parse_krt_rows(&up_cells)
                            .iter()
                            .map(|c| (c.base.clone(), c.m.clone(), c.f.clone(), c.n.clone()))
                            .collect();
                    // Derive through the exact portal path.
                    let derived = match dhaturupa_wasm::derive_krut::derive_prefixed_krut(
                        &stack_deva,
                        id,
                        family,
                        &krt_deva,
                        &upstream,
                        &dhatu_slp,
                        category,
                        &note,
                    ) {
                        Ok(rows) => rows,
                        Err(e) => {
                            if misses.len() < 25 {
                                misses.push(format!(
                                    "{id} {stack_deva} {family} {krt_deva}: derive error {e}"
                                ));
                            }
                            continue;
                        }
                    };
                    // Avyaya mask from upstream emptiness (portal rule).
                    let filled = ["m", "f", "n"].map(|field| {
                        up_fam.and_then(|m| m.get(&krt_deva)).is_some_and(|cells| {
                            forms::parse_krt_rows(cells).iter().any(|cell| {
                                let v = match field {
                                    "m" => &cell.m,
                                    "f" => &cell.f,
                                    _ => &cell.n,
                                };
                                !v.trim().is_empty()
                            })
                        })
                    });
                    // Compare row multisets (order-insensitive).
                    let mut want_set: Vec<String> = Vec::new();
                    for row in truth_rows {
                        let mut parts_row = Vec::new();
                        for (field, filled_flag) in ["base", "m", "f", "n"]
                            .iter()
                            .zip([true, filled[0], filled[1], filled[2]])
                        {
                            if !filled_flag {
                                parts_row.push(String::new());
                                continue;
                            }
                            let mut alts: Vec<String> = row
                                .get(field)
                                .and_then(|v| v.as_str())
                                .map(|s| {
                                    s.split('/')
                                        .filter(|a| !a.is_empty())
                                        .filter_map(slp1_to_deva)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            alts.sort();
                            parts_row.push(alts.join("/"));
                        }
                        want_set.push(parts_row.join(","));
                    }
                    want_set.sort();
                    let mut got_set: Vec<String> = Vec::new();
                    for row in &derived {
                        let mut parts_row = Vec::new();
                        for (field, filled_flag) in ["base", "m", "f", "n"]
                            .iter()
                            .zip([true, filled[0], filled[1], filled[2]])
                        {
                            if !filled_flag {
                                parts_row.push(String::new());
                                continue;
                            }
                            let mut alts: Vec<String> = row
                                .get(*field)
                                .map(|s| {
                                    s.split('/')
                                        .filter(|a| !a.is_empty())
                                        .map(str::to_string)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            alts.sort();
                            parts_row.push(alts.join("/"));
                        }
                        got_set.push(parts_row.join(","));
                    }
                    got_set.sort();
                    checked += 1;
                    if got_set == want_set {
                        matched += 1;
                    } else {
                        // Row counts equal but content differs: report the
                        // first differing row so the class is visible.
                        let detail = if derived.len() == want_set.len() {
                            let mut first = String::new();
                            for (w, g) in want_set.iter().zip(got_set.iter()) {
                                if w != g {
                                    first = format!(" row[{w}] != [{g}]");
                                    break;
                                }
                            }
                            first
                        } else {
                            String::new()
                        };
                        misses.push(format!(
                            "{id} {stack_deva} {family} {krt_deva}: truth {} rows vs derived {} rows{detail}",
                            want_set.len(),
                            got_set.len(),
                        ));
                    }
                }
            }
        }
    }
    (checked, matched, miss_total, misses)
}

#[test]
fn derive_krut_sampled() {
    let (checked, matched, miss_total, misses) = audit(97);
    eprintln!(
        "derive-krut-sampled: checked={checked} matched={matched} miss_total={miss_total} misses={}",
        misses.len()
    );
    assert!(checked > 20, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored derive_krut"]
fn derive_krut_exhaustive() {
    let (checked, matched, miss_total, misses) = audit(1);
    eprintln!(
        "derive-krut-exhaustive: checked={checked} matched={matched} miss_total={miss_total} misses={}",
        misses.len()
    );
    assert!(checked > 1_000, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}
