//! Reverse-search audits: every displayable form must resolve back.
//!
//! Sampled tiers run in plain `cargo test`. Exhaustive tiers are `#[ignore]`d
//! for release/CI: `cargo test --release -- --ignored`. Data files load once
//! per test-binary run via `shared()`.

use dhaturupa_wasm::engine::subanta::krt_table;
use dhaturupa_wasm::engine::upasarga::generate_prefixed;
use dhaturupa_wasm::forms::{self, KRUT_FAMILIES, TING_FAMILIES, TING_PRAYOGAS};
use dhaturupa_wasm::revindex::{
    audit_all, index_prefixed_dhatu, AuditData, AuditOptions, LocKind, Posting, RevIndex,
};
use dhaturupa_wasm::translit::{deva_stack_to_slp1, deva_to_slp1, slp1_to_deva};
use std::collections::HashMap;
use std::sync::OnceLock;

fn shared() -> &'static AuditData {
    static SHARED: OnceLock<AuditData> = OnceLock::new();
    SHARED.get_or_init(|| {
        let mut ting = Vec::new();
        for family in TING_FAMILIES {
            for prayoga in TING_PRAYOGAS {
                let path = forms::ting_filename(family, prayoga).expect("known family");
                let text = std::fs::read_to_string(&path).expect("ting file loads");
                let map = forms::parse_forms_map(&text).expect("ting file parses");
                ting.push((family.to_string(), prayoga.to_string(), map));
            }
        }
        let mut krut = Vec::new();
        for family in KRUT_FAMILIES {
            let path = forms::krut_filename(family).expect("known family");
            let text = std::fs::read_to_string(&path).expect("krut file loads");
            let map = forms::parse_forms_map(&text).expect("krut file parses");
            krut.push((family.to_string(), map));
        }
        let dhatu_text = std::fs::read_to_string("data/dhatu_data.json").expect("dhatu data loads");
        let dhatu = dhaturupa_wasm::load_str(&dhatu_text).expect("dhatu data parses");
        let vidyut_text =
            std::fs::read_to_string("data/vidyut_dhatupatha_5.tsv").expect("vidyut loads");
        let vidyut = dhaturupa_wasm::vidyut::load_tsv_str(&vidyut_text)
            .into_iter()
            .map(|row| (row.code, row.dhatu))
            .collect();
        AuditData {
            ting,
            krut,
            dhatu,
            vidyut,
        }
    })
}

fn check_report(name: &str, min_checked: u64, report: &dhaturupa_wasm::revindex::AuditReport) {
    eprintln!(
        "{name}: checked={} misses={} ting={} krt={} prefixed={} subanta={} skip_deva={} skip_slp1={} skip_tab={} skip_base={}",
        report.checked,
        report.misses,
        report.stats.ting_forms,
        report.stats.krt_forms,
        report.stats.prefixed_forms,
        report.stats.subanta_forms,
        report.stats.skip_deva,
        report.stats.skip_slp1,
        report.stats.skip_table,
        report.stats.skip_no_base
    );
    for sample in &report.miss_samples {
        eprintln!("  MISS: {sample}");
    }
    for sample in report.stats.skip_samples.iter().take(10) {
        eprintln!("  SKIP: {sample}");
    }
    assert!(report.checked >= min_checked, "too few checks in {name}");
    assert_eq!(report.misses, 0, "reverse-search misses in {name}");
}

#[test]
fn reverse_audit_sampled() {
    let report = audit_all(shared(), &AuditOptions::sampled());
    check_report("sampled", 100_000, &report);
}

/// Dhatu with the most upasarga stacks (exhaustive prefixed subject).
fn richest_dhatu(data: &AuditData) -> (String, Vec<String>) {
    let mut best = (String::new(), Vec::new());
    for entry in &data.dhatu.data {
        let stacks: Vec<String> = entry.upasargas.iter().map(|u| u.name.clone()).collect();
        if stacks.len() > best.1.len() {
            best = (entry.baseindex.clone(), stacks);
        }
    }
    assert!(!best.0.is_empty(), "some dhatu has upasargas");
    best
}

/// Re-derive the prefixed surfaces a checker may expect for one base form.
///
/// The index (`index_prefixed_dhatu`) applies two corpus-pinned
/// normalizations before it indexes — see `prefix_krut`: ल्यप् cells ship
/// with a BAKED upasarga (upstream shows `प्रभूय`; the corpus's ल्यप् row
/// for the same stack is `nirBUya`/`samBUya`, i.e. the baked code is
/// stripped and the selected stack re-prefixed), and neuter cells carry
/// their dental `t`/`d` twin. Raw `generate_prefixed` on the un-stripped
/// base yields keys the index must NOT carry (`अनु` + `प्रभूय`), so the
/// checkers honour the same two rules — this is the documented rule, not
/// the index implementation.
fn expected_prefix_surfaces(
    stack_deva: &str,
    inner: &LocKind,
    base_deva: &str,
    ctx_for: &dyn Fn(&LocKind) -> dhaturupa_wasm::engine::upasarga::GenCtx,
) -> Vec<String> {
    let Some(stack_slp1) = deva_stack_to_slp1(stack_deva) else {
        return Vec::new();
    };
    let Some(base_slp1) = deva_to_slp1(base_deva) else {
        return Vec::new();
    };
    let is_lyap = matches!(inner, LocKind::Krt { krt, .. } if krt == "ल्यप्");
    let base_slp1 = if is_lyap {
        dhaturupa_wasm::prefix_krut::strip_baked_stack(&base_slp1).to_string()
    } else {
        base_slp1
    };
    let mut surfaces = generate_prefixed(&stack_slp1, &base_slp1, ctx_for(inner));
    if let LocKind::Krt { field, .. } = inner {
        if field == "n" {
            for surface in surfaces.clone() {
                if surface.ends_with('t') {
                    let twin = format!("{}d", &surface[..surface.len() - 1]);
                    if !surfaces.contains(&twin) {
                        surfaces.push(twin);
                    }
                }
            }
        }
    }
    surfaces
        .iter()
        .filter_map(|surface| slp1_to_deva(surface))
        .collect()
}

#[test]
fn prefixed_exhaustive_one_dhatu() {
    let data = shared();
    let (id, stacks) = richest_dhatu(data);
    eprintln!(
        "exhaustive prefixed subject: {id} ({} stacks)",
        stacks.len()
    );
    // All base forms of this dhatu across every family file.
    let mut forms: Vec<(LocKind, String)> = Vec::new();
    for (family, prayoga, map) in &data.ting {
        let Some(entry) = map.get(&id) else {
            continue;
        };
        let mut lakaras: Vec<&String> = entry.keys().collect();
        lakaras.sort();
        for lakara in lakaras {
            for (pos, slot) in forms::split_slots(&entry[lakara]).iter().enumerate() {
                for variant in forms::split_variants(slot) {
                    forms.push((
                        LocKind::Ting {
                            family: family.clone(),
                            prayoga: prayoga.clone(),
                            lakara: (*lakara).clone(),
                            pos,
                        },
                        (*variant).to_string(),
                    ));
                }
            }
        }
    }
    for (family, map) in &data.krut {
        let Some(entry) = map.get(&id) else {
            continue;
        };
        let mut krts: Vec<&String> = entry.keys().collect();
        krts.sort();
        for krt in krts {
            for (row, cell) in forms::parse_krt_rows(&entry[krt]).iter().enumerate() {
                for (field, value) in [
                    ("base", &cell.base),
                    ("m", &cell.m),
                    ("f", &cell.f),
                    ("n", &cell.n),
                ] {
                    for alternant in dhaturupa_wasm::slp1::split_alternants(value) {
                        forms.push((
                            LocKind::Krt {
                                family: family.clone(),
                                krt: (*krt).clone(),
                                row,
                                field: field.to_string(),
                            },
                            (*alternant).to_string(),
                        ));
                    }
                }
            }
        }
    }
    assert!(!forms.is_empty(), "subject dhatu has base forms");
    let entry = data
        .dhatu
        .data
        .iter()
        .find(|item| item.baseindex == id)
        .expect("subject entry");
    let vidyut = data.vidyut.get(&id).map(String::as_str);
    let ctx_for =
        |inner: &LocKind| dhaturupa_wasm::genctx::ctx_for_loc(vidyut, &entry.dhatu_note, inner);
    let mut idx = RevIndex::default();
    let mut stats = dhaturupa_wasm::revindex::Stats::default();
    for stack in &stacks {
        index_prefixed_dhatu(&mut idx, &id, stack, &forms, &ctx_for, &mut stats);
    }
    // Independently re-derive every surface and require the posting.
    let mut checked = 0_u64;
    let mut misses = Vec::new();
    for stack in &stacks {
        for (inner, deva) in &forms {
            for key in expected_prefix_surfaces(stack, inner, deva, &ctx_for) {
                checked += 1;
                let expected = Posting {
                    baseindex: id.clone(),
                    loc: LocKind::Prefixed {
                        stack: stack.clone(),
                        inner: Box::new(inner.clone()),
                    },
                };
                if !idx.get(&key).contains(&expected) {
                    misses.push(format!("{id} {stack} {deva} -> {key}"));
                }
            }
        }
    }
    eprintln!(
        "one-dhatu prefixed: forms={} checked={checked} misses={}",
        forms.len(),
        misses.len()
    );
    for sample in misses.iter().take(10) {
        eprintln!("  MISS: {sample}");
    }
    assert!(checked > 0, "generated prefixed surfaces");
    assert!(misses.is_empty(), "prefixed misses for {id}");
}

#[test]
fn translit_covers_corpus() {
    // Every pipeline input to deva_to_slp1 must convert; failures here mean
    // unsearchable prefixed/subanta surfaces.
    let data = shared();
    let mut fails: Vec<String> = Vec::new();
    let mut total = 0_u64;
    for (_, _, map) in &data.ting {
        for entry in map.values() {
            for value in entry.values() {
                for slot in forms::split_slots(value) {
                    for variant in forms::split_variants(slot) {
                        total += 1;
                        if deva_to_slp1(variant).is_none() {
                            fails.push(format!("ting {variant}"));
                        }
                    }
                }
            }
        }
    }
    for (_, map) in &data.krut {
        for entry in map.values() {
            for value in entry.values() {
                for row in forms::split_krt_rows(value) {
                    let cell = forms::parse_krt_cell(row);
                    for field in [&cell.base, &cell.m, &cell.f, &cell.n] {
                        for alternant in dhaturupa_wasm::slp1::split_alternants(field) {
                            total += 1;
                            if deva_to_slp1(alternant).is_none() {
                                fails.push(format!("krut {alternant}"));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut stacks = 0_u64;
    for entry in &data.dhatu.data {
        for upasarga in &entry.upasargas {
            stacks += 1;
            if deva_stack_to_slp1(&upasarga.name).is_none() {
                fails.push(format!("stack {}", upasarga.name));
            }
        }
    }
    eprintln!(
        "translit sweep: total={total} stacks={stacks} fails={}",
        fails.len()
    );
    for sample in fails.iter().take(10) {
        eprintln!("  FAIL: {sample}");
    }
    assert!(fails.is_empty(), "untransliterable pipeline inputs");
}

#[test]
fn subanta_spot_bhavanti() {
    // Guard pin: the classic click-through stays resolvable end to end.
    let table = krt_table("krut", "Satf", "f", "BavantI", "").expect("BavantI table builds");
    assert!(table.contains("BavantIByAm"));
    let key = slp1_to_deva("BavantIByAm").expect("transliterates");
    assert_eq!(key, "भवन्तीभ्याम्");
    let data = shared();
    let entry = data
        .krut
        .iter()
        .find(|(family, _)| family == "shuddha")
        .expect("shuddha krut");
    let cell = forms::parse_krt_cell(&entry.1["01.0001"]["शतृ"]);
    assert!(cell.f.split('/').any(|a| a == "भवन्ती"));
}

#[test]
#[ignore = "exhaustive: run with cargo test --release -- --ignored"]
fn reverse_audit_full() {
    let report = audit_all(shared(), &AuditOptions::full());
    check_report("full", 8_000_000, &report);
}

/// CLI parity: `search` union helper resolves an HK query to the same card
/// as its Devanagari twin.
#[test]
fn search_union_hk_parity() {
    use dhaturupa_wasm::hk::hk_to_slp1;
    use dhaturupa_wasm::revindex::search_union;
    let data = shared();
    let mut idx = RevIndex::default();
    let shuddha = data
        .ting
        .iter()
        .find(|(family, prayoga, _)| family == "shuddha" && prayoga == "kartari")
        .expect("shuddha kartari");
    let mut per: HashMap<String, Vec<(LocKind, String)>> = HashMap::new();
    let mut stats = dhaturupa_wasm::revindex::Stats::default();
    dhaturupa_wasm::revindex::index_ting_map(
        &mut idx,
        &shuddha.2,
        "shuddha",
        "kartari",
        Some(&mut per),
        &mut stats,
    );
    let indexes = [&idx];
    let deva_hits = search_union(&indexes, &["भवति".to_string()]);
    let slp = hk_to_slp1("bhavati").expect("hk decodes");
    let deva = slp1_to_deva(&slp).expect("slp1 converts");
    let hk_hits = search_union(&indexes, &[deva]);
    assert!(!deva_hits.is_empty());
    assert_eq!(deva_hits, hk_hits);
}
