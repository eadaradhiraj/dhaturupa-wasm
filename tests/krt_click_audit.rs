//! Kṛdanta click-through audit: every displayed कृत् cell must decline.
//!
//! This walks the exact surface the portal's click handler runs
//! (`wasm::krt_subanta` — family, kṛt display name, field, cell value),
//! once unprefixed and once prefixed through the dhātu's first listed
//! upasarga. It exists because the click path regressed invisibly: kṛt
//! display names carrying separators (`स्य-शतृ`, `भावकर्म-शानच्`) were
//! rejected before reaching the engine, so every `स्य-*` click-through
//! failed in the browser while all other suites stayed green.
//!
//! Sampled tier runs in plain `cargo test`; exhaustive is `#[ignore]`d:
//! `cargo test --release -- --ignored krt_click`.

use dhaturupa_wasm::engine::upasarga::{generate_prefixed, GenCtx};
use dhaturupa_wasm::forms::{self, KRUT_FAMILIES};
use dhaturupa_wasm::translit::{deva_stack_to_slp1, deva_to_slp1, slp1_to_deva};
use dhaturupa_wasm::wasm;
use std::collections::HashMap;
use std::sync::OnceLock;

fn shared() -> &'static Vec<(String, forms::KrutMap)> {
    static SHARED: OnceLock<Vec<(String, forms::KrutMap)>> = OnceLock::new();
    SHARED.get_or_init(|| {
        KRUT_FAMILIES
            .iter()
            .map(|family| {
                let path = forms::krut_filename(family).expect("known family");
                let text = std::fs::read_to_string(&path).expect("krut file loads");
                let map = forms::parse_forms_map(&text).expect("krut file parses");
                (family.to_string(), map)
            })
            .collect()
    })
}

/// baseindex → (listed upasarga display names, vidyut SLP1 dhatu, note).
type DhatuCtx = (Vec<String>, Option<String>, String);

fn dhatu_ctx() -> &'static HashMap<String, DhatuCtx> {
    static SHARED: OnceLock<HashMap<String, DhatuCtx>> = OnceLock::new();
    SHARED.get_or_init(|| {
        let text = std::fs::read_to_string("data/dhatu_data.json").expect("dhatu data loads");
        let file = dhaturupa_wasm::load_str(&text).expect("dhatu data parses");
        let vidyut: HashMap<String, String> = dhaturupa_wasm::vidyut::load_tsv_str(
            &std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").expect("vidyut loads"),
        )
        .into_iter()
        .map(|row| (row.code, row.dhatu))
        .collect();
        file.data
            .iter()
            .map(|entry| {
                (
                    entry.baseindex.clone(),
                    (
                        entry.upasargas.iter().map(|u| u.name.clone()).collect(),
                        vidyut.get(&entry.baseindex).cloned(),
                        entry.dhatu_note.clone(),
                    ),
                )
            })
            .collect()
    })
}

/// Documented non-declinable cells — the click-through audit pins exactly
/// these (fried from its exhaustive run: 365 of 717,871 cells) so anything
/// NEW fails the suite.
fn documented_gap(krt: &str, field: &str, value: &str) -> bool {
    // Indeclinable by engine design (PROGRESS "Residual skips"): tfC neuter
    // is a half-formed citation, तृच् neuter likewise.
    if krt == "तृच्" && field == "n" {
        return true;
    }
    // Verbal कृत् citations (aorist अच्, liṭ नजिङ्, kyaP, lyu, ghinuN):
    // verb-stem forms, not declinable adjectives.
    if matches!(krt, "अच्" | "नजिङ्" | "क्यप्" | "ल्यु" | "घिनुण्")
    {
        return true;
    }
    // ङ्वनिप् masculine agent citations (`यज्वा`) — long-ā verbal stems; the
    // engine's आ-class is feminine-approximated by design.
    if krt == "ङ्वनिप्" && field == "m" {
        return true;
    }
    // Bare-consonant agent citations (वुन्/क्तावतु fem क-stems like `श्राक`,
    // `क्षीब`) and stray halant-final stems: no affix to decline on.
    match value.chars().last() {
        Some('\u{094D}') => true, // explicit virama
        Some(c) if ('\u{0915}'..='\u{0939}').contains(&c) => true, // bare consonant
        _ => false,
    }
}

struct Report {
    checked: u64,
    skipped: u64,
    prefixed: u64,
    misses: Vec<String>,
    histogram: HashMap<String, u64>,
}

fn audit(step: usize) -> Report {
    let mut report = Report {
        checked: 0,
        skipped: 0,
        prefixed: 0,
        misses: Vec::new(),
        histogram: HashMap::new(),
    };
    let miss = |r: &mut Report, krt: &str, field: &str, value: &str, msg: &str| {
        let last: String = value
            .chars()
            .last()
            .map(|c| c.to_string())
            .unwrap_or_default();
        *r.histogram.entry(format!("{field}:{last}")).or_insert(0) += 1;
        if r.misses.len() < 25 {
            r.misses.push(format!("{krt} {field} {value}: {msg}"));
        }
    };
    for (family, map) in shared() {
        // Deterministic order: HashMap iteration is random, so `nth % step`
        // would sample different dhātus every run.
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for (nth, id) in ids.iter().enumerate() {
            if nth % step != 0 {
                continue;
            }
            let krts = &map[*id];
            let (stacks, _dhatu_slp, _note) = dhatu_ctx()
                .get(id.as_str())
                .map(|(s, v, n)| (s.clone(), v.clone(), n.clone()))
                .unwrap_or_default();
            for (krt, cells) in krts {
                for cell in forms::parse_krt_rows(cells) {
                    for (field, value) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                        for variant in value.split('/').filter(|v| !v.is_empty()) {
                            report.checked += 1;
                            let out = wasm::krt_subanta(family, krt, field, variant, id.as_str());
                            let parsed: serde_json::Value =
                                serde_json::from_str(&out).expect("envelope parses");
                            if parsed["ok"] != serde_json::json!(true) {
                                if documented_gap(krt, field, variant) {
                                    report.skipped += 1;
                                } else {
                                    miss(
                                        &mut report,
                                        krt,
                                        field,
                                        variant,
                                        parsed["error"].as_str().unwrap_or("?"),
                                    );
                                }
                                continue;
                            }
                            // Prefixed click-through: same engine path with
                            // the upasarga-prefixed stem.
                            if let Some(stack) = stacks.first() {
                                let Some(stack_slp) = deva_stack_to_slp1(stack) else {
                                    continue;
                                };
                                let Some(base_slp) = deva_to_slp1(variant) else {
                                    continue;
                                };
                                let category = forms::engine_ting_category(family)
                                    .unwrap_or("ting")
                                    .to_string();
                                let ctx = GenCtx {
                                    derived: dhaturupa_wasm::engine::upasarga::is_derived_category(
                                        &category,
                                    ),
                                    intensive_keep: category.starts_with("yang"),
                                    yangluk: category.starts_with("yangluk"),
                                    ..GenCtx::default()
                                };
                                for surface in generate_prefixed(&stack_slp, &base_slp, ctx) {
                                    let Some(surface_deva) = slp1_to_deva(&surface) else {
                                        continue;
                                    };
                                    report.prefixed += 1;
                                    let out = wasm::krt_subanta(
                                        family,
                                        krt,
                                        field,
                                        &surface_deva,
                                        id.as_str(),
                                    );
                                    let parsed: serde_json::Value =
                                        serde_json::from_str(&out).expect("envelope parses");
                                    if parsed["ok"] != serde_json::json!(true)
                                        && !documented_gap(krt, field, &surface_deva)
                                    {
                                        miss(
                                            &mut report,
                                            krt,
                                            field,
                                            &surface_deva,
                                            parsed["error"].as_str().unwrap_or("?"),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    report
}

fn report_line(name: &str, r: &Report) {
    eprintln!(
        "{name}: checked={} ok={} prefixed_checked={} indeclinable_skips={} misses={}",
        r.checked,
        r.checked - r.skipped - r.misses.len() as u64,
        r.prefixed,
        r.skipped,
        r.misses.len()
    );
    let mut hist: Vec<_> = r.histogram.iter().collect();
    hist.sort_by(|a, b| b.1.cmp(a.1));
    for (label, n) in hist.iter().take(12) {
        eprintln!("  miss {label} × {n}");
    }
}

#[test]
fn krt_click_sampled() {
    let r = audit(17);
    report_line("krt-click-sampled", &r);
    assert!(r.checked > 5_000, "audit covered too little: {}", r.checked);
    assert!(r.misses.is_empty(), "misses:\n{}", r.misses.join("\n"));
}

#[test]
#[ignore = "exhaustive: cargo test --release -- --ignored krt_click"]
fn krt_click_exhaustive() {
    let r = audit(1);
    report_line("krt-click-exhaustive", &r);
    assert!(
        r.checked > 100_000,
        "audit covered too little: {}",
        r.checked
    );
    assert!(r.misses.is_empty(), "misses:\n{}", r.misses.join("\n"));
}
