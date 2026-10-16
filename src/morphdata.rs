//! skt-morph-data ground-truth reader (build-side only, never shipped).
//!
//! The corpus (`SKT_MORPH_DATA`, default
//! `/home/edhiraj/Documents/projs/pypanini/skt-morph-data`) is the same data
//! the site's engine version produced: per dhātu,
//! `upasarga_forms.<stack>.conjugations` holds one map per
//! (family, pada-half) — `ting`+`yak` = shuddha, `nich`+`nich_yak`, …
//! `<fam>` maps are kartari-tab content, `<fam>_yak` karmaṇī-tab content.
//! Lakāra keys already carry the pada letter (`plat`, `alat`);
//! each is a flat SLP1 form array in puruṣa×vacana order.
//!
//! Used by blob building to backfill prefixed tiṅanta tables byte-exactly
//! (values from the corpus, slot grouping from the engine — flat arrays
//! carry no slot boundaries). Absent corpus → backfill skipped with a
//! warning; the portal then engine-derives unlisted stacks live.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// (portal family, corpus kartari map, corpus karmaṇī map).
pub const FAMILIES: [(&str, &str, &str); 5] = [
    ("shuddha", "ting", "yak"),
    ("nich", "nich", "nich_yak"),
    ("san", "san", "san_yak"),
    ("yang", "yang", "yang_yak"),
    ("yangluk", "yangluk", "yangluk_yak"),
];

/// Backfill accounting, reported by the build.
#[derive(Default)]
pub struct BackfillStats {
    pub lakaras: u64,
    pub from_corpus: u64,
    pub engine_fallback: u64,
    pub missing: u64,
}

pub fn corpus_root() -> Option<PathBuf> {
    let root = std::env::var("SKT_MORPH_DATA")
        .unwrap_or_else(|_| "/home/edhiraj/Documents/projs/pypanini/skt-morph-data".to_string());
    let path = PathBuf::from(root);
    path.is_dir().then_some(path)
}

/// Flat SLP1 form arrays per (family, prayoga, lakāra) for one listed stack.
pub type StackFlat = HashMap<String, HashMap<String, HashMap<String, Vec<String>>>>;

fn entry_path(root: &Path, baseindex: &str) -> PathBuf {
    let gana = baseindex.split('.').next().unwrap_or("01");
    root.join(gana).join(format!("{baseindex}.json"))
}

/// Resolve a transliterated stack to a corpus key: normalized form first
/// (bare `A` means आङ् — see `normalize_stack_slp1`), then exact.
/// Returns `None` when the corpus has neither.
fn resolve_stack_key(
    forms: &serde_json::Map<String, serde_json::Value>,
    stack_slp: &str,
) -> Option<String> {
    let normalized = crate::translit::normalize_stack_slp1(stack_slp);
    if normalized != stack_slp {
        if let Some(key) = forms.keys().find(|k| k.as_str() == normalized).cloned() {
            return Some(key);
        }
    }
    forms.keys().find(|k| k.as_str() == stack_slp).cloned()
}

pub fn prefixed_ting_flat(root: &Path, baseindex: &str, stack_slp: &str) -> Option<StackFlat> {
    let text = std::fs::read_to_string(entry_path(root, baseindex)).ok()?;
    let entry: serde_json::Value = serde_json::from_str(&text).ok()?;
    let forms = entry.get("upasarga_forms")?.as_object()?;
    let key = resolve_stack_key(forms, stack_slp)?;
    let conj = entry
        .get("upasarga_forms")?
        .get(&key)?
        .get("conjugations")?;
    let mut out: HashMap<String, HashMap<String, HashMap<String, Vec<String>>>> = HashMap::new();
    for (family, map_kartari, map_karmani) in FAMILIES {
        let mut prayogas: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        for (prayoga, map_name) in [("kartari", map_kartari), ("karmani", map_karmani)] {
            let Some(map) = conj.get(map_name).and_then(|v| v.as_object()) else {
                continue;
            };
            let mut lakaras: HashMap<String, Vec<String>> = HashMap::new();
            for (lakara, slots) in map {
                let Some(slots) = slots.as_array() else {
                    continue;
                };
                let forms: Vec<String> = slots
                    .iter()
                    .filter_map(|s| s.as_str())
                    .map(str::to_string)
                    .collect();
                if forms.is_empty() {
                    continue;
                }
                lakaras.insert(lakara.clone(), forms);
            }
            if !lakaras.is_empty() {
                prayogas.insert(prayoga.to_string(), lakaras);
            }
        }
        if !prayogas.is_empty() {
            out.insert(family.to_string(), prayogas);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Backfilled prefixed tiṅanta tables for one listed (dhātu, stack):
/// `{family: {prayoga: {lakara: "cell;cell;…"}}}` in blob cell format.
///
/// Values come from the corpus (byte-exact site parity); the 9-slot
/// grouping comes from the engine (flat corpus arrays carry no slot
/// boundaries). Where the engine's slot inventory disagrees with the
/// corpus length, that lakāra falls back to fully engine-derived cells
/// (counted in stats) rather than mis-slotting forms.
/// Backfilled prefixed kṛdanta tables for one (dhātu, stack):
/// `{family: {krt: "base,m,f,n;base,…"}}` in blob krut format.
///
/// Corpus participle rows are already per-row `[{base, m, f, n}]` — no
/// slot partition needed, just transliteration. Values are byte-exact
/// site parity; the portal renders these directly for reachable stacks.
pub fn backfill_prefixed_krut(
    root: &Path,
    stack_deva: &str,
    baseindex: &str,
) -> Option<serde_json::Value> {
    let stack_slp = crate::translit::deva_stack_to_slp1(stack_deva)?;
    let text = std::fs::read_to_string(entry_path(root, baseindex)).ok()?;
    let entry: serde_json::Value = serde_json::from_str(&text).ok()?;
    let forms = entry.get("upasarga_forms")?.as_object()?;
    let key = resolve_stack_key(forms, &stack_slp)?;
    let parts = entry
        .get("upasarga_forms")?
        .get(&key)?
        .get("participles")?
        .as_object()?;
    // Portal family ← corpus krut-family.
    let mut families = serde_json::Map::new();
    for (family, corpus_fam) in [
        ("shuddha", "krut"),
        ("nich", "nich_krut"),
        ("san", "san_krut"),
        ("yang", "yang_krut"),
        ("yangluk", "yangluk_krut"),
    ] {
        let Some(krts) = parts.get(corpus_fam).and_then(|v| v.as_object()) else {
            continue;
        };
        let mut krt_map = serde_json::Map::new();
        for (krt_slp, rows) in krts {
            let Some(krt_deva) = crate::translit::slp1_to_deva(krt_slp) else {
                continue;
            };
            let Some(rows) = rows.as_array() else {
                continue;
            };
            let mut out_rows = Vec::new();
            for row in rows {
                let cell = |field: &str| {
                    row.get(field)
                        .and_then(|v| v.as_str())
                        .map(|s| {
                            s.split('/')
                                .filter_map(|alt| {
                                    if alt.is_empty() {
                                        None
                                    } else {
                                        crate::translit::slp1_to_deva(alt)
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("/")
                        })
                        .unwrap_or_default()
                };
                let built = [cell("base"), cell("m"), cell("f"), cell("n")].join(",");
                if built.chars().any(|c| c != ',') {
                    out_rows.push(built);
                }
            }
            if !out_rows.is_empty() {
                krt_map.insert(krt_deva, serde_json::Value::String(out_rows.join(";")));
            }
        }
        if !krt_map.is_empty() {
            families.insert(family.to_string(), serde_json::Value::Object(krt_map));
        }
    }
    if families.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(families))
    }
}
/// Merge corpus SLP1 forms with engine slot grouping into a Devanagari
/// blob cell. Four cases (see call site): counts agree (exact), engine
/// superset (filter to corpus-attested), corpus carries extra stem variants
/// (scale the engine's cell boundaries), otherwise engine cells.
fn merge_slots(
    slots: Option<Vec<Vec<String>>>,
    truth: &[String],
    stats: &mut BackfillStats,
) -> String {
    let Some(slots) = slots else {
        stats.engine_fallback += 1;
        return String::new();
    };
    let flat_count: usize = slots.iter().map(Vec::len).sum();
    // Multiset of truth for membership tests.
    let mut remaining: HashMap<String, usize> = HashMap::new();
    for form in truth {
        *remaining.entry(form.clone()).or_insert(0) += 1;
    }
    if flat_count == truth.len() {
        // Counts agree: corpus values in engine slot order.
        let mut at = 0_usize;
        let mut out = Vec::with_capacity(slots.len());
        for slot in &slots {
            let mut cells = Vec::with_capacity(slot.len());
            for _ in 0..slot.len() {
                let form = &truth[at];
                at += 1;
                cells.push(crate::translit::slp1_to_deva(form).unwrap_or_else(|| form.clone()));
            }
            out.push(cells.join(","));
        }
        stats.from_corpus += 1;
        return out.join(";");
    }
    // Engine superset (or subset): filter each slot to corpus-attested
    // forms. Exact iff every truth form found its slot.
    let mut kept_total = 0_usize;
    let mut out = Vec::with_capacity(slots.len());
    for slot in &slots {
        let mut cells = Vec::with_capacity(slot.len());
        for form in slot {
            if let Some(n) = remaining.get_mut(form) {
                if *n > 0 {
                    *n -= 1;
                    kept_total += 1;
                    cells.push(crate::translit::slp1_to_deva(form).unwrap_or_else(|| form.clone()));
                }
            }
        }
        out.push(cells.join(","));
    }
    if kept_total == truth.len() {
        stats.from_corpus += 1;
        return out.join(";");
    }
    // Same slot structure with more stem variants per construction: pypanini
    // lists every variant where vidyuṭ emits fewer (09.0032 √ṛ san:
    // arirīṣiṣ-/aririṣiṣ-/erṣiṣi- against vidyuṭ's two), laid out slot-major
    // with the variants of one slot consecutive. Scale the engine's cell
    // boundaries: cell i takes `slots[i].len() * truth.len() / flat_count`
    // consecutive corpus forms. Exact only when that division holds for
    // every cell — this reproduces the site's own grouping (compare the same
    // dhātu's cells in `data/ting_san_*.json`, which carry all variants of a
    // person-slot in one cell).
    if truth.len() > flat_count && flat_count > 0 {
        let mut shares = Vec::with_capacity(slots.len());
        let mut scaled = true;
        for slot in &slots {
            let size = slot.len() * truth.len();
            if !size.is_multiple_of(flat_count) {
                scaled = false;
                break;
            }
            shares.push(size / flat_count);
        }
        if scaled {
            let mut at = 0_usize;
            let mut out = Vec::with_capacity(slots.len());
            for share in shares {
                let cells: Vec<String> = truth[at..at + share]
                    .iter()
                    .map(|f| crate::translit::slp1_to_deva(f).unwrap_or_else(|| f.clone()))
                    .collect();
                at += share;
                out.push(cells.join(","));
            }
            stats.from_corpus += 1;
            return out.join(";");
        }
    }
    // Otherwise: full engine cells (counted fallback — every surface kept,
    // none invented; the audit reports the residual).
    stats.engine_fallback += 1;
    slots
        .iter()
        .map(|slot| {
            slot.iter()
                .filter_map(|s| crate::translit::slp1_to_deva(s))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join(";")
}

pub fn backfill_prefixed_ting(
    root: &Path,
    stack_deva: &str,
    baseindex: &str,
    stats: &mut BackfillStats,
) -> Option<serde_json::Value> {
    let stack_slp = crate::translit::deva_stack_to_slp1(stack_deva)?;
    let flat = prefixed_ting_flat(root, baseindex, &stack_slp)?;
    let mut families = serde_json::Map::new();
    for (family, prayogas) in &flat {
        let mut prayoga_map = serde_json::Map::new();
        for (prayoga, lakaras) in prayogas {
            // One SLP1 derivation per set feeds every lakāra below.
            let derived_slp1 = crate::derive_ting::derive_prefixed_ting_slp1(
                stack_deva, baseindex, family, prayoga,
            )
            .unwrap_or_default();
            let mut lakara_map = serde_json::Map::new();
            for (lakara, forms) in lakaras {
                stats.lakaras += 1;
                let mut cell = String::new();
                if forms.is_empty() {
                    stats.missing += 1;
                } else {
                    // Merge corpus values with engine slot grouping, in SLP1
                    // (no transliteration roundtrips). Four cases:
                    // - counts agree → corpus values in engine slots (exact);
                    // - engine ⊃ corpus → keep engine slots filtered to
                    //   corpus-attested forms (drops engine-only variants
                    //   like short augment `A` beside `AN`);
                    // - corpus ⊃ engine in whole-ratio steps → scale the
                    //   engine's slot boundaries (extra stem variants);
                    // - otherwise → full engine cells (counted fallback).
                    let slots: Option<Vec<Vec<String>>> = derived_slp1.get(lakara).cloned();
                    cell = merge_slots(slots, forms, stats);
                }
                if !cell.split(';').all(|c| c.trim().is_empty()) {
                    lakara_map.insert(lakara.clone(), serde_json::Value::String(cell));
                }
            }
            if !lakara_map.is_empty() {
                prayoga_map.insert(prayoga.clone(), serde_json::Value::Object(lakara_map));
            }
        }
        if !prayoga_map.is_empty() {
            families.insert(family.clone(), serde_json::Value::Object(prayoga_map));
        }
    }
    if families.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(families))
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_slots, BackfillStats};

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn merge_counts_agree_uses_corpus_in_engine_slots() {
        let mut stats = BackfillStats::default();
        let slots = vec![
            vec!["a".to_string(), "i".to_string()],
            vec!["u".to_string()],
        ];
        let cell = merge_slots(Some(slots), &strings(&["a", "i", "u"]), &mut stats);
        assert_eq!(cell, "अ,इ;उ");
        assert_eq!(stats.from_corpus, 1);
        assert_eq!(stats.engine_fallback, 0);
    }

    #[test]
    fn merge_engine_superset_filters_to_corpus_forms() {
        let mut stats = BackfillStats::default();
        let slots = vec![
            vec!["a".to_string(), "A".to_string()],
            vec!["i".to_string()],
        ];
        let cell = merge_slots(Some(slots), &strings(&["a", "i"]), &mut stats);
        assert_eq!(cell, "अ;इ");
        assert_eq!(stats.from_corpus, 1);
    }

    #[test]
    fn merge_extra_corpus_variants_scale_slot_boundaries() {
        let mut stats = BackfillStats::default();
        // Two engine cells of two forms; the corpus lists three variants
        // per cell (slot-major), as pypanini does for √ṛ's sannanta.
        let slots = vec![
            vec!["a".to_string(), "i".to_string()],
            vec!["u".to_string(), "f".to_string()],
        ];
        let cell = merge_slots(
            Some(slots),
            &strings(&["a", "i", "e", "u", "f", "o"]),
            &mut stats,
        );
        assert_eq!(cell, "अ,इ,ए;उ,ऋ,ओ");
        assert_eq!(stats.from_corpus, 1);
        assert_eq!(stats.engine_fallback, 0);
    }

    #[test]
    fn merge_non_integral_scaling_falls_back_to_engine_cells() {
        let mut stats = BackfillStats::default();
        let slots = vec![
            vec!["a".to_string()],
            vec!["i".to_string(), "u".to_string()],
        ];
        let cell = merge_slots(
            Some(slots),
            &strings(&["a", "i", "u", "f", "e"]),
            &mut stats,
        );
        assert_eq!(cell, "अ;इ,उ");
        assert_eq!(stats.engine_fallback, 1);
    }
}
