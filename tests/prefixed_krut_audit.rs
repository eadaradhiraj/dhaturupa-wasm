//! Prefixed-kṛdanta audit: pinned corpus cells + the kṛt-aware live path.
//!
//! Two contracts, both driven over every corpus row:
//!
//! 1. **Pin fidelity** — `morphdata::backfill_prefixed_krut` (the function
//!    the portal blobs ship) must reproduce the corpus cell for every
//!    (stack, family, kṛt), field and alternant. These cells are what the
//!    site renders for listed stacks.
//! 2. **Pin partitioning** — `prefixed_krut_row` with the pin hands each
//!    upstream row its slice of the cell (pure string matching, no engine
//!    calls); the per-row calls must union back to the cell exactly.
//!
//! Unlisted stacks stay on the engine path (ल्यप् baked-stack strip +
//! prathamā-ekavacana declension + twin suppression — see `prefix_krut`);
//! they have no corpus truth, so only the export's shape is gated here.
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

fn audit(step: usize) -> (u64, u64, u64, u64, u64, Vec<String>) {
    let Some(root) = corpus_root() else {
        eprintln!("prefixed-krut: no corpus at SKT_MORPH_DATA, skipping");
        return (0, 0, 0, 0, 0, Vec::new());
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
    let mut unreachable = 0_u64;
    let mut misses = Vec::new();
    let zz_all = std::env::var("ZZ_ALL").is_ok();
    let cap = if zz_all { 200_000 } else { 400 };
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
            // Display forms are ambiguous between corpus keys: bare `A` (आ)
            // and `AN` (आङ्) both render as `आ`, and the production lookup
            // prefers the `AN` twin whenever it exists (`normalize_stack_slp1`
            // — the site's own list never writes आङ्). Keys shadowed that way
            // are unreachable through the display path, so the pin contract
            // does not apply to them; count them and move on.
            let normalized = dhaturupa_wasm::translit::normalize_stack_slp1(stack_slp);
            if normalized != *stack_slp && forms.contains_key(&normalized) {
                unreachable += 1;
                continue;
            }
            let Some(stack_deva) = stack_deva_display(stack_slp) else {
                continue;
            };
            let Some(parts) = uf.get("participles").and_then(|v| v.as_object()) else {
                continue;
            };
            // Corpus pin tables for this (dhātu, stack): family → kṛt → cell,
            // built by the production backfill function (the portal ships the
            // same cells in `blob.krut_prefixed`).
            let backfill =
                dhaturupa_wasm::morphdata::backfill_prefixed_krut(&root, &stack_deva, id);
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
                    // Corpus pin for this (stack, family, kṛt): the same cell
                    // the portal ships in `blob.krut_prefixed`, built by the
                    // production backfill function.
                    let pinned_cell = backfill
                        .as_ref()
                        .and_then(|v| v.get(family))
                        .and_then(|f| f.get(krt.as_str()))
                        .and_then(|c| c.as_str())
                        .unwrap_or_default()
                        .to_string();
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
                    // The pin itself must be the corpus cell, per field and
                    // alternant (this is the data the portal renders; the
                    // built blobs are pinned separately by `backfill_audit`).
                    let pinned = dhaturupa_wasm::prefix_krut::parse_pinned_cell(&pinned_cell);
                    let mut pin_by_field: [Vec<String>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for row in &pinned {
                        for (slot, value) in row.iter().enumerate() {
                            for alt in value.split('/').filter(|a| !a.is_empty()) {
                                pin_by_field[slot].push(alt.to_string());
                            }
                        }
                    }
                    for alts in &mut pin_by_field {
                        alts.sort();
                    }
                    let mut pin_ok = true;
                    for (slot, field) in ["base", "m", "f", "n"].iter().enumerate() {
                        if want_by_field[slot] == pin_by_field[slot] {
                            continue;
                        }
                        pin_ok = false;
                        checked += 1;
                        miss_total += 1;
                        let label = format!("pin/{krt}/{field}");
                        *classes.entry(label.clone()).or_insert(0) += 1;
                        if misses.len() < cap
                            && (zz_all || !misses.iter().any(|m: &String| m.starts_with(&label)))
                        {
                            misses.push(format!(
                                "{label} {id} {stack_deva} {family}: truth [{}] vs pin [{}]",
                                want_by_field[slot].join(" "),
                                pin_by_field[slot].join(" ")
                            ));
                        }
                    }
                    if !pin_ok {
                        continue;
                    }
                    let mut got_by_field: [Vec<String>; 4] =
                        [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    // One export call per upstream row (the kṛt-aware path
                    // shared with the portal's per-row fallback), pin mode:
                    // the per-row slices must union back to the cell. With no
                    // upstream rows, row 0 carries the whole cell.
                    let rows_json = serde_json::to_string(
                        &rows
                            .iter()
                            .map(|row| {
                                serde_json::json!({
                                    "base": row.base,
                                    "m": row.m,
                                    "f": row.f,
                                    "n": row.n,
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                    .expect("json");
                    let calls: Vec<usize> = if rows.is_empty() {
                        if pinned.is_empty() {
                            Vec::new()
                        } else {
                            vec![0]
                        }
                    } else {
                        (0..rows.len()).collect()
                    };
                    for i in calls {
                        let res = ok_envelope(
                            &wasm::prefixed_krut_row(
                                &stack_deva,
                                krt,
                                &rows_json,
                                i,
                                dhatu_slp,
                                &category,
                                &dhatu.dhatu_note,
                                &pinned_cell,
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
                    // No-pin engine smoke: unlisted stacks carry no corpus
                    // truth, so only the export's shape is gated here — the
                    // engine path must stay callable and well-formed.
                    if !rows.is_empty() {
                        let res = ok_envelope(
                            &wasm::prefixed_krut_row(
                                &stack_deva,
                                krt,
                                &rows_json,
                                0,
                                dhatu_slp,
                                &category,
                                &dhatu.dhatu_note,
                                "",
                            ),
                            "prefixed_krut_row (engine)",
                        );
                        assert!(
                            ["base", "m", "f", "n"]
                                .iter()
                                .all(|field| res["row"][field].is_array()),
                            "engine envelope malformed: {id} {stack_deva} {family} {krt}"
                        );
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
                        if misses.len() < cap
                            && (zz_all || !misses.iter().any(|m: &String| m.starts_with(&label)))
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
    (checked, matched, no_truth, miss_total, unreachable, misses)
}

#[test]
fn prefixed_krut_sampled() {
    let (checked, matched, no_truth, miss_total, unreachable, misses) = audit(41);
    eprintln!(
        "prefixed-krut-sampled: checked={checked} matched={matched} no-truth={no_truth} miss_total={miss_total} unreachable={unreachable} misses={}",
        misses.len()
    );
    assert!(checked > 500, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored prefixed_krut"]
fn prefixed_krut_exhaustive() {
    let (checked, matched, no_truth, miss_total, unreachable, misses) = audit(1);
    eprintln!(
        "prefixed-krut-exhaustive: checked={checked} matched={matched} no-truth={no_truth} miss_total={miss_total} unreachable={unreachable} misses={}",
        misses.len()
    );
    assert!(checked > 10_000, "audit covered too little: {checked}");
    assert!(misses.is_empty(), "misses:\n{}", misses.join("\n"));
}

/// Temporary probe: classify divergences without gating.
fn documented_gap(_krt: &str, _field: &str, _want: &[String], _got: &[String]) -> bool {
    false
}
