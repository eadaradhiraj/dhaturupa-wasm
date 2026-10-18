//! WASM portal glue: the String/JSON boundary. SLP1 never crosses it.
//!
//! Inputs accept Devanagari (generation endpoints also pass SLP1 through;
//! resolve HK through [`query_candidates`] first). Outputs are Devanagari
//! forms inside English envelopes (`{"ok":true,…}` / `{"ok":false,"error"}`).
//!
//! Table JSON comes from the portal's lazy fetches (per-family files or
//! per-dhatu blobs); parsed base indexes live in a WASM-side cache
//! ([`index_ensure_family`]), so keystroke searches never shuttle megabytes
//! across the JS boundary. Per-dhatu prefixed/subanta searches build, query,
//! and drop a small index inside one call.
//!
//! Every export is a plain `String → String` function, so the same tests
//! run on native and wasm targets.
//!
//! [`query_candidates`]: crate::wasm::query_candidates

use crate::forms::{parse_forms_map, parse_krt_rows, KRUT_FAMILIES, TING_PRAYOGAS};
use crate::hk::hk_to_slp1;
use crate::revindex::{
    index_krut_map, index_prefixed_dhatu, index_subanta_cell, index_ting_map, search_union,
    DhatuForms, LocKind, Posting, RevIndex, Stats, SubantaCell,
};
use crate::translit::{contains_deva, deva_to_slp1, slp1_to_deva};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use wasm_bindgen::prelude::*;

/// Cap on hits per query response (total count is always reported).
const MAX_HITS: usize = 100;

/// WASM-side base-index cache, keyed by dhatuforms family.
static FAMILY_INDEX: LazyLock<Mutex<HashMap<String, RevIndex>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// WASM-side prefix surface map for query splitting (`split_init`).
type SurfaceMap = HashMap<String, Vec<crate::engine::upasplit::SurfaceEntry>>;

static SURFACE_MAP: LazyLock<Mutex<Option<SurfaceMap>>> = LazyLock::new(|| Mutex::new(None));

fn ok_envelope(payload: serde_json::Value) -> String {
    let mut envelope = serde_json::Map::new();
    envelope.insert("ok".to_string(), serde_json::Value::Bool(true));
    if let serde_json::Value::Object(fields) = payload {
        for (key, value) in fields {
            envelope.insert(key, value);
        }
    }
    serde_json::Value::Object(envelope).to_string()
}

fn err_envelope(message: &str) -> String {
    serde_json::json!({"ok": false, "error": message}).to_string()
}

/// Devanagari query candidates with their reading schemes. Frontend rule:
/// Devanagari as-is; ASCII must be Harvard-Kyoto (raw SLP1 never surfaces —
/// the engine core stays SLP1-only internally, invisible to the portal).
fn candidates_with_schemes(query: &str) -> Vec<(String, &'static str)> {
    if contains_deva(query) {
        return vec![(query.to_string(), "Devanagari")];
    }
    let mut out = Vec::new();
    if let Some(decoded) = hk_to_slp1(query).and_then(|slp| slp1_to_deva(&slp)) {
        out.push((decoded, "Harvard-Kyoto"));
    }
    out
}

/// Devanagari query candidates, portal rule: Devanagari as-is, ASCII as the
/// union of HK-decoded and raw-SLP1-decoded readings.
fn candidates(query: &str) -> Vec<String> {
    candidates_with_schemes(query)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

/// Devanagari or SLP1 input → SLP1 (HK must go through [`query_candidates`]).
fn input_to_slp1(text: &str) -> Option<String> {
    if contains_deva(text) {
        deva_to_slp1(text)
    } else {
        Some(text.to_string())
    }
}

fn loc_json(posting: &Posting) -> serde_json::Value {
    let baseindex = serde_json::Value::String(posting.baseindex.clone());
    match &posting.loc {
        LocKind::Ting {
            family,
            prayoga,
            lakara,
            pos,
        } => serde_json::json!({
            "baseindex": baseindex,
            "kind": "ting",
            "family": family,
            "prayoga": prayoga,
            "lakara": lakara,
            "pos": pos,
        }),
        LocKind::Krt {
            family,
            krt,
            row,
            field,
        } => serde_json::json!({
            "baseindex": baseindex,
            "kind": "krt",
            "family": family,
            "krt": krt,
            "row": row,
            "field": field,
        }),
        LocKind::Prefixed { stack, inner } => {
            let inner_posting = Posting {
                baseindex: posting.baseindex.clone(),
                loc: (**inner).clone(),
            };
            serde_json::json!({
                "baseindex": baseindex,
                "kind": "prefixed",
                "stack": stack,
                "base": loc_json(&inner_posting),
            })
        }
        LocKind::Subanta {
            family,
            krt,
            row,
            field,
            vibhakti,
            vacana,
        } => {
            serde_json::json!({
                "baseindex": baseindex,
                "kind": "subanta",
                "family": family,
                "krt": krt,
                "row": row,
                "field": field,
                "vibhakti": vibhakti,
                "vacana": vacana,
            })
        }
    }
}

fn hits_json(indexes: &[&RevIndex], query: &str) -> String {
    let queries = candidates(query);
    if queries.is_empty() {
        return err_envelope("query is not Devanagari or Harvard-Kyoto");
    }
    let hits = search_union(indexes, &queries);
    let total = hits.len();
    let shown: Vec<serde_json::Value> = hits
        .into_iter()
        .take(MAX_HITS)
        .map(|hit| loc_json(&hit))
        .collect();
    ok_envelope(serde_json::json!({"query": query, "total": total, "hits": shown}))
}

/// Search-box readings for `query` (Devanagari candidates for table lookup),
/// each labeled with its scheme.
#[wasm_bindgen]
pub fn query_candidates(query: &str) -> String {
    let readings = candidates_with_schemes(query);
    if readings.is_empty() {
        return err_envelope("query is not Devanagari or Harvard-Kyoto");
    }
    let labeled: Vec<serde_json::Value> = readings
        .iter()
        .map(|(text, scheme)| serde_json::json!({"text": text, "scheme": scheme}))
        .collect();
    let deva: Vec<&str> = readings.iter().map(|(text, _)| text.as_str()).collect();
    ok_envelope(serde_json::json!({"query": query, "deva": deva, "readings": labeled}))
}

/// Devanagari reading of a portal query for client-side filtering: the list
/// boxes compare against Devanagari stems, so `dA` must normalize to `दा`
/// before matching. Same frontend rule as [`query_candidates`] — Devanagari
/// as-is, ASCII strictly Harvard-Kyoto (raw SLP1 never surfaces).
#[wasm_bindgen]
pub fn to_devanagari(query: &str) -> String {
    match candidates_with_schemes(query).into_iter().next() {
        Some((deva, _)) => ok_envelope(serde_json::json!({"query": query, "deva": deva})),
        None => err_envelope("query is not Devanagari or Harvard-Kyoto"),
    }
}

/// Dhatu info card + upasarga names from `dhatu/data.txt`-shaped JSON.
#[wasm_bindgen]
pub fn dhatu_card(dhatu_json: &str, baseindex: &str) -> String {
    let file: crate::DhatuFile = match serde_json::from_str(dhatu_json) {
        Ok(file) => file,
        Err(_) => return err_envelope("dhatu JSON does not parse"),
    };
    let Some(entry) = crate::find_by_baseindex(&file, baseindex) else {
        return err_envelope("unknown dhatu (try 01.0001, or 1.1)");
    };
    let info: Vec<serde_json::Value> = crate::info_rows(entry)
        .into_iter()
        .filter(|row| !row.value.is_empty() || !row.note.is_empty())
        .map(|row| serde_json::json!({"name": row.name, "value": row.value, "note": row.note}))
        .collect();
    let upasargas: Vec<serde_json::Value> = entry
        .upasargas
        .iter()
        .map(|item| serde_json::json!({"name": item.name, "artha_hindi": item.artha_hindi}))
        .collect();
    ok_envelope(serde_json::json!({
        "baseindex": entry.baseindex,
        "dhatu": entry.dhatu,
        "artha": entry.artha,
        "info": info,
        "upasargas": upasargas,
    }))
}

/// Tinanta tables for one dhatu from a ting-file JSON
/// (`{"01.0001":{"plat":"…;…",…},…}`); lakara values split into slots of variants.
#[wasm_bindgen]
pub fn ting_table(ting_json: &str, baseindex: &str) -> String {
    let Ok(map) = parse_forms_map(ting_json) else {
        return err_envelope("ting JSON does not parse");
    };
    let norm = crate::normalize_baseindex(baseindex);
    let Some(entry) = map.get(&norm) else {
        return err_envelope("no ting row for this dhatu in this table");
    };
    let mut lakaras: Vec<&String> = entry.keys().collect();
    lakaras.sort();
    let mut tables = serde_json::Map::new();
    for lakara in lakaras {
        let slots: Vec<Vec<&str>> = crate::forms::split_slots(&entry[lakara])
            .into_iter()
            .map(crate::forms::split_variants)
            .collect();
        tables.insert(lakara.clone(), serde_json::json!(slots));
    }
    ok_envelope(serde_json::json!({"baseindex": norm, "tables": tables}))
}

/// Krdanta rows for one dhatu from a krut-file JSON; each krt lists its
/// `;`-rows of `base,m,f,n` (empty string = absent for that field).
#[wasm_bindgen]
pub fn krut_table_json(krut_json: &str, baseindex: &str) -> String {
    let Ok(map) = parse_forms_map(krut_json) else {
        return err_envelope("krut JSON does not parse");
    };
    let norm = crate::normalize_baseindex(baseindex);
    let Some(entry) = map.get(&norm) else {
        return err_envelope("no krut row for this dhatu in this table");
    };
    let mut krts: Vec<&String> = entry.keys().collect();
    krts.sort();
    let mut tables = serde_json::Map::new();
    for krt in krts {
        let rows: Vec<serde_json::Value> = parse_krt_rows(&entry[krt])
            .into_iter()
            .map(|cell| {
                serde_json::json!({"base": cell.base, "m": cell.m, "f": cell.f, "n": cell.n})
            })
            .collect();
        tables.insert(krt.clone(), serde_json::Value::Array(rows));
    }
    ok_envelope(serde_json::json!({"baseindex": norm, "tables": tables}))
}

/// Class label for display. `Class::Word` labels are the raw SLP1 dictionary
/// word (`tad`, `yuzmad`, …) — the portal must never show SLP1, and the
/// Devanagari stem already names the word, so listed words get a neutral tag.
fn class_label(class: &crate::engine::subanta::Class) -> String {
    match class {
        crate::engine::subanta::Class::Word(_) => "known word".to_string(),
        other => other.label(),
    }
}

fn paradigm_json(table: &crate::engine::subanta::Paradigm) -> serde_json::Value {
    let cells: Vec<serde_json::Value> = table
        .cells
        .iter()
        .map(|slot| {
            let forms: Vec<serde_json::Value> = slot
                .forms
                .iter()
                .map(|form: &String| {
                    serde_json::Value::String(
                        slp1_to_deva(form.as_str()).unwrap_or_else(|| form.clone()),
                    )
                })
                .collect();
            serde_json::json!({
                "vibhakti": slot.vibhakti,
                "vacana": slot.vacana,
                "forms": forms,
            })
        })
        .collect();
    serde_json::json!({
        "stem": slp1_to_deva(&table.stem).unwrap_or_else(|| table.stem.clone()),
        "linga": table.linga.as_str(),
        "class": class_label(&table.class),
        "cells": cells,
    })
}

/// Full subanta declension for a stem (Devanagari or SLP1) + `m|f|n`.
#[wasm_bindgen]
pub fn subanta_table(stem: &str, linga: &str) -> String {
    let Some(stem_slp1) = input_to_slp1(stem) else {
        return err_envelope("stem is not Devanagari or SLP1");
    };
    let Some(parsed) = crate::engine::subanta::Linga::parse(linga) else {
        return err_envelope("linga must be m, f or n");
    };
    let tables = crate::engine::subanta::generate_tables(&stem_slp1, parsed);
    if tables.is_empty() {
        return err_envelope("not possible");
    }
    let rendered: Vec<serde_json::Value> = tables.iter().map(paradigm_json).collect();
    ok_envelope(serde_json::json!({"tables": rendered}))
}

/// Word-view input → SLP1. Portal rule for ASCII is Harvard-Kyoto; when HK
/// cannot decode the string (the spec's SLP1 trio `Wrampena`/`WrampeRa` is
/// not valid HK), raw SLP1 is accepted as a fallback. The string only
/// selects a reading — SLP1 still never reaches the DOM.
fn word_input_to_slp1(text: &str) -> Option<String> {
    if contains_deva(text) {
        return deva_to_slp1(text).filter(|s| crate::slp1::is_slp1_form(s));
    }
    if let Some(slp1) = hk_to_slp1(text) {
        return Some(slp1);
    }
    crate::slp1::is_slp1_form(text).then(|| text.to_string())
}

/// Word view: any word → readings (base word + slot) and declension
/// candidates (the word re-declined as a fresh stem, all three genders).
///
/// `Wrampena` resolves only to itself as a vocative (an instrumental
/// reading would need natva → `WrampeRa`); `WrampeRa` resolves both to
/// `Wrampa`'s instrumental singular and to itself.
#[wasm_bindgen]
pub fn subanta_analyze(word: &str) -> String {
    let Some(slp1) = word_input_to_slp1(word) else {
        return err_envelope("word is not Devanagari, Harvard-Kyoto, or SLP1");
    };
    let analyses: Vec<serde_json::Value> = crate::engine::subanta::analyze(&slp1, &HashMap::new())
        .into_iter()
        .map(|a| {
            let (vibhakti, vacana) = match a.slot.split_once('.') {
                Some((v, c)) => (v.to_string(), c.to_string()),
                None => (a.slot.clone(), String::new()),
            };
            serde_json::json!({
                "stem": slp1_to_deva(&a.stem).unwrap_or_else(|| a.stem.clone()),
                "linga": a.linga.as_str(),
                "class": class_label(&a.class),
                "vibhakti": vibhakti,
                "vacana": vacana,
            })
        })
        .collect();
    let mut tables = Vec::new();
    for linga in [
        crate::engine::subanta::Linga::M,
        crate::engine::subanta::Linga::F,
        crate::engine::subanta::Linga::N,
    ] {
        tables.extend(crate::engine::subanta::generate_tables(&slp1, linga));
    }
    let rendered: Vec<serde_json::Value> = tables.iter().map(paradigm_json).collect();
    ok_envelope(serde_json::json!({
        "word": slp1_to_deva(&slp1).unwrap_or_else(|| slp1.clone()),
        "analyses": analyses,
        "tables": rendered,
    }))
}

/// Krdanta click-through: declension of one krt cell value (Devanagari or
/// SLP1; `/`-alternants tried in order, first table wins).
/// Devanagari → SLP1 with non-Devanagari separators kept (`स्य-शतृ` →
/// `sya-Satf`). For kṛt display names: the class dispatch rides the stem
/// shape, and the only name-sensitive cases (`kvasu`, `tfc`) are plain
/// names, so separators are preserved instead of failing transliteration.
fn deva_to_slp1_keep_sep(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut buf = String::new();
    for ch in text.chars() {
        if contains_deva(&ch.to_string()) {
            buf.push(ch);
            continue;
        }
        if !buf.is_empty() {
            out.push_str(&crate::translit::deva_to_slp1(&buf)?);
            buf.clear();
        }
        out.push(ch);
    }
    if !buf.is_empty() {
        out.push_str(&crate::translit::deva_to_slp1(&buf)?);
    }
    Some(out)
}

#[wasm_bindgen]
pub fn krt_subanta(family: &str, krt: &str, field: &str, value: &str, baseindex: &str) -> String {
    use crate::engine::subanta::{krt_table, Linga};
    let Some(engine_family) = crate::forms::normalize_krut_engine_family(family) else {
        return err_envelope("unknown krut family (try shuddha|nich|san|yang|yangluk)");
    };
    if Linga::parse(field).is_none() {
        return err_envelope("field must be m, f or n");
    }
    // Kṛt display names may carry separators (`स्य-शतृ`, `भावकर्म-शानच्`).
    let krt_slp = deva_to_slp1_keep_sep(krt)
        .or_else(|| crate::hk::hk_to_slp1(krt))
        .unwrap_or_else(|| krt.to_string());
    let mut values: Vec<String> = Vec::new();
    for part in value.split('/') {
        let Some(single) = input_to_slp1(part) else {
            return err_envelope("value is not Devanagari or SLP1");
        };
        values.push(single);
    }
    if values.is_empty() {
        return err_envelope("value is not Devanagari or SLP1");
    }
    for single in &values {
        for alternant in crate::slp1::split_alternants(single) {
            if let Some(table) = krt_table(engine_family, &krt_slp, field, alternant, baseindex) {
                return ok_envelope(serde_json::json!({"tables": [paradigm_json(&table)]}));
            }
        }
    }
    err_envelope("not possible")
}

/// Batch prefixing for whole tables: `forms_json` is an array of Devanagari
/// (or SLP1) base forms; returns the surface arrays in order. Unusable
/// inputs yield empty arrays at their positions (never an error).
/// Uses the default (root-level) context; pass dhatu context explicitly via
/// [`prefixed_table_ctx`] when the dhatu and category are known.
#[wasm_bindgen]
pub fn prefixed_table(stack: &str, forms_json: &str) -> String {
    let stack_slp1 = if contains_deva(stack) {
        match crate::translit::deva_stack_to_slp1(stack) {
            Some(stack) => stack,
            None => return err_envelope("stack is not a Devanagari upasarga list"),
        }
    } else {
        stack.to_string()
    };
    let bases: Vec<String> = match serde_json::from_str(forms_json) {
        Ok(bases) => bases,
        Err(_) => return err_envelope("forms JSON must be an array of strings"),
    };
    let ctx = crate::engine::upasarga::GenCtx::default();
    let mut tables = Vec::with_capacity(bases.len());
    for base in &bases {
        let surfaces = match input_to_slp1(base) {
            Some(base_slp1) => {
                crate::engine::upasarga::generate_prefixed(&stack_slp1, &base_slp1, ctx)
                    .into_iter()
                    .map(|form: String| slp1_to_deva(&form).unwrap_or(form))
                    .collect()
            }
            None => Vec::new(),
        };
        tables.push(surfaces);
    }
    ok_envelope(serde_json::json!({"tables": tables}))
}

/// Upasarga-prefixed surfaces for a stack + base form (Devanagari or SLP1).
#[wasm_bindgen]
pub fn prefixed_forms(stack: &str, base: &str) -> String {
    let stack_slp1 = if contains_deva(stack) {
        match crate::translit::deva_stack_to_slp1(stack) {
            Some(stack) => stack,
            None => return err_envelope("stack is not a Devanagari upasarga list"),
        }
    } else {
        stack.to_string()
    };
    let Some(base_slp1) = input_to_slp1(base) else {
        return err_envelope("base is not Devanagari or SLP1");
    };
    let ctx = crate::engine::upasarga::GenCtx::default();
    let forms: Vec<serde_json::Value> =
        crate::engine::upasarga::generate_prefixed(&stack_slp1, &base_slp1, ctx)
            .into_iter()
            .map(|form: String| serde_json::Value::String(slp1_to_deva(&form).unwrap_or(form)))
            .collect();
    ok_envelope(serde_json::json!({"forms": forms}))
}

/// Engine category for a dhatuforms table (`ting`/`krut` + family), for
/// portal code that only knows display-side names. Returns the category
/// string (`ting`, `nich`, `san_krut`, …) or an error envelope.
#[wasm_bindgen]
pub fn engine_category(family: &str, table: &str) -> String {
    let category = match table {
        "ting" => crate::forms::engine_ting_category(family),
        "krut" => crate::forms::engine_krut_family(family),
        _ => None,
    };
    match category {
        Some(name) => ok_envelope(serde_json::json!({"category": name})),
        None => err_envelope(
            "unknown family/table (families shuddha|nich|san|yang|yangluk, tables ting|krut)",
        ),
    }
}

/// Batch prefixing with explicit dhatu context: `slp1_dhatu` is the vidyut
/// SLP1 dhatu (markers kept), `engine_category` comes from
/// [`engine_category`], and `dhatu_note` carries nitya-adi markings.
/// Same shape as [`prefixed_table`].
///
/// Fully engine-derived prefixed tiṅanta table for one (upasarga-stack,
/// dhātu, family, prayoga): `{tables: {lakara_key: "row;row;…"}}` in blob
/// cell format, Devanagari. Used by prefixed views where upstream ships no
/// base cell to prefix (e.g. वि + जि has no `a*` kartari keys upstream, yet
/// the site shows विजयते — derived, not prefixed). Pada resolution is the
/// engine's: each pada derived explicitly, missing paradigms have no key.
#[wasm_bindgen]
pub fn derive_prefixed_ting(stack: &str, baseindex: &str, family: &str, prayoga: &str) -> String {
    // Portal display names are ambiguous (`आ` means आङ्) — normalize
    // before deriving so live views agree with backfilled ones.
    let stack_norm = normalize_stack_display(stack);
    match crate::derive_ting::derive_prefixed_ting(&stack_norm, baseindex, family, prayoga) {
        Ok(tables) => ok_envelope(serde_json::json!({"tables": tables})),
        Err(message) => err_envelope(&message),
    }
}

/// Normalize a portal display stack for derivation: bare `आ` tokens mean
/// आङ् (`AN`). SLP1 codes pass through untouched.
fn normalize_stack_display(stack: &str) -> String {
    if !crate::translit::contains_deva(stack) {
        return stack.to_string();
    }
    let toks: Vec<String> = stack
        .split(['+', ';'])
        .map(|part| {
            let t = part.trim();
            if t == "आ" {
                "आङ्".to_string()
            } else {
                t.to_string()
            }
        })
        .collect();
    toks.join(" + ")
}

/// Fully engine-derived prefixed kṛdanta rows for one (upasarga-stack,
/// dhātu, family, kṛt): `{rows: [{base, m, f, n}]}` in Devanagari, one row
/// per distinct stem. `upstream_json` carries the dhātu's upstream rows
/// (`[{base, m, f, n}]` Devanagari strings) for the pre-feminized
/// supplement + vidyuṭ-missing fallback; `slp1`/`category`/`note` feed its
/// `GenCtx`. `pinned_cell` carries the corpus-pinned cell for this
/// (stack, family, kṛt) — the portal's `blob.krut_prefixed` piece, built by
/// [`crate::morphdata::backfill_prefixed_krut`] — and is served verbatim in
/// row order when non-empty (the site's data is the corpus; re-deriving
/// could only drift). Same normalization as [`derive_prefixed_ting`].
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn derive_prefixed_krut(
    stack: &str,
    baseindex: &str,
    family: &str,
    krt: &str,
    upstream_json: &str,
    slp1: &str,
    category: &str,
    note: &str,
    pinned_cell: &str,
) -> String {
    let stack_norm = normalize_stack_display(stack);
    let pinned = crate::prefix_krut::parse_pinned_cell(pinned_cell);
    let upstream: Vec<(String, String, String, String)> = if pinned.is_empty() {
        let Ok(upstream_rows) = serde_json::from_str::<Vec<serde_json::Value>>(upstream_json)
        else {
            return err_envelope("upstream JSON must be an array of {base, m, f, n}");
        };
        upstream_rows
            .iter()
            .map(|r| {
                let cell = |field: &str| {
                    r.get(field)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                (cell("base"), cell("m"), cell("f"), cell("n"))
            })
            .collect()
    } else {
        Vec::new()
    };
    match crate::derive_krut::derive_prefixed_krut(
        &stack_norm,
        baseindex,
        family,
        krt,
        &upstream,
        slp1,
        category,
        note,
        &pinned,
    ) {
        Ok(rows) => ok_envelope(serde_json::json!({"rows": rows})),
        Err(message) => err_envelope(&message),
    }
}

/// Kṛt-aware batch prefixing for one kṛt ROW: `rows_json` is the whole
/// upstream cell (`[{base, m, f, n}]` Devanagari strings, `/`-joined
/// alternants) and `row_index` selects the row this call renders. Returns
/// `{base, m, f, n}` alternatives for that row.
///
/// `pinned_cell` carries the corpus cell for this (stack, family, kṛt) when
/// the caller has it (portal blob piece, audit): it is partitioned across
/// the upstream rows ([`crate::prefix_krut::partition_pinned`], pure string
/// matching — no engine calls) and this row's slice is returned, so the
/// union of the per-row calls is the cell, losslessly. Without a pin the
/// engine path runs (ल्यप् baked-stack strip, prathamā-ekavacana
/// declension, twin-suppression — see `prefix_krut`) and is validated by
/// `tests/prefixed_krut_audit.rs`.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn prefixed_krut_row(
    stack: &str,
    krt: &str,
    rows_json: &str,
    row_index: usize,
    slp1_dhatu: &str,
    engine_category: &str,
    dhatu_note: &str,
    pinned_cell: &str,
) -> String {
    let stack_slp1 = if contains_deva(stack) {
        match crate::translit::deva_stack_to_slp1(stack) {
            Some(stack) => stack,
            None => return err_envelope("stack is not a Devanagari upasarga list"),
        }
    } else {
        stack.to_string()
    };
    let Ok(rows_value) = serde_json::from_str::<serde_json::Value>(rows_json) else {
        return err_envelope("rows JSON must be an array of {base, m, f, n}");
    };
    let upstream: Vec<(String, String, String, String)> = rows_value
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|r| {
                    let cell = |field: &str| {
                        r.get(field)
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string()
                    };
                    (cell("base"), cell("m"), cell("f"), cell("n"))
                })
                .collect()
        })
        .unwrap_or_default();
    let pinned = crate::prefix_krut::parse_pinned_cell(pinned_cell);
    if !pinned.is_empty() {
        let slices = crate::prefix_krut::partition_pinned(&pinned, &upstream);
        let assigned: Vec<&crate::prefix_krut::PinnedRow> = if slices.is_empty() {
            // No upstream rows to partition across: the whole cell goes out
            // through row 0 (so nothing is lost).
            if row_index == 0 {
                pinned.iter().collect()
            } else {
                Vec::new()
            }
        } else {
            slices
                .get(row_index)
                .map(|slice| slice.iter().collect())
                .unwrap_or_default()
        };
        let mut out = serde_json::Map::new();
        for (slot, field) in ["base", "m", "f", "n"].iter().enumerate() {
            // Verbatim (duplicates kept): the audit's pin contract is a
            // multiset union over rows.
            let forms: Vec<String> = assigned
                .iter()
                .flat_map(|row| row[slot].split('/').filter(|a| !a.is_empty()))
                .map(str::to_string)
                .collect();
            out.insert((*field).to_string(), serde_json::Value::from(forms));
        }
        return ok_envelope(serde_json::json!({"row": out}));
    }
    let Some(row) = upstream.get(row_index) else {
        return err_envelope("row index out of range");
    };
    let field_of = |row: &(String, String, String, String), field: &str| -> String {
        match field {
            "base" => row.0.clone(),
            "m" => row.1.clone(),
            "f" => row.2.clone(),
            _ => row.3.clone(),
        }
    };
    let ctx = crate::genctx::ctx_for(slp1_dhatu, engine_category, dhatu_note);
    let Some(engine_family) = crate::forms::normalize_krut_engine_family(engine_category) else {
        return err_envelope("unknown krut family (try shuddha|nich|san|yang|yangluk)");
    };
    let cells: Vec<(String, Vec<String>)> = ["base", "m", "f", "n"]
        .iter()
        .map(|field| {
            let alts: Vec<String> = field_of(row, field)
                .split('/')
                .filter(|a| !a.is_empty())
                .map(str::to_string)
                .collect();
            ((*field).to_string(), alts)
        })
        .collect();
    let prefixed =
        crate::prefix_krut::prefix_krut_row(&stack_slp1, krt, engine_family, &cells, ctx);
    let mut out = serde_json::Map::new();
    for field in ["base", "m", "f", "n"] {
        let deva: Vec<String> = prefixed
            .get(field)
            .map(|forms| {
                forms
                    .iter()
                    .map(|s| crate::translit::slp1_to_deva(s).unwrap_or_else(|| s.clone()))
                    .collect()
            })
            .unwrap_or_default();
        out.insert(field.to_string(), serde_json::Value::from(deva));
    }
    ok_envelope(serde_json::json!({"row": out}))
}

#[wasm_bindgen]
pub fn prefixed_table_ctx(
    stack: &str,
    forms_json: &str,
    slp1_dhatu: &str,
    engine_category: &str,
    dhatu_note: &str,
) -> String {
    let stack_slp1 = if contains_deva(stack) {
        match crate::translit::deva_stack_to_slp1(stack) {
            Some(stack) => stack,
            None => return err_envelope("stack is not a Devanagari upasarga list"),
        }
    } else {
        stack.to_string()
    };
    let Ok(bases) = serde_json::from_str::<Vec<String>>(forms_json) else {
        return err_envelope("forms JSON must be an array of strings");
    };
    let ctx = crate::genctx::ctx_for(slp1_dhatu, engine_category, dhatu_note);
    let mut tables = Vec::with_capacity(bases.len());
    for base in &bases {
        let surfaces = match input_to_slp1(base) {
            Some(base_slp1) => {
                crate::engine::upasarga::generate_prefixed(&stack_slp1, &base_slp1, ctx)
                    .into_iter()
                    .map(|form: String| slp1_to_deva(&form).unwrap_or(form))
                    .collect()
            }
            None => Vec::new(),
        };
        tables.push(surfaces);
    }
    ok_envelope(serde_json::json!({"tables": tables}))
}

/// Build (or rebuild) the cached base reverse index for one family from its
/// three table JSONs. The portal calls this once per family after fetching.
#[wasm_bindgen]
pub fn index_ensure_family(
    family: &str,
    ting_kartari_json: &str,
    ting_karmani_json: &str,
    krut_json: &str,
) -> String {
    if !KRUT_FAMILIES.contains(&family) {
        return err_envelope("unknown family (try shuddha|nich|san|yang|yangluk)");
    }
    let mut index = RevIndex::default();
    let mut stats = Stats::default();
    for prayoga in TING_PRAYOGAS {
        let text = if prayoga == "kartari" {
            ting_kartari_json
        } else {
            ting_karmani_json
        };
        let Ok(map) = parse_forms_map(text) else {
            return err_envelope("ting JSON does not parse");
        };
        index_ting_map(
            &mut index,
            &map,
            family,
            prayoga,
            None::<&mut DhatuForms>,
            &mut stats,
        );
    }
    let Ok(krut_map) = parse_forms_map(krut_json) else {
        return err_envelope("krut JSON does not parse");
    };
    index_krut_map(
        &mut index,
        &krut_map,
        family,
        None::<&mut DhatuForms>,
        &mut stats,
    );
    let forms = stats.ting_forms + stats.krt_forms;
    index.finish();
    match FAMILY_INDEX.lock() {
        Ok(mut cache) => {
            cache.insert(family.to_string(), index);
        }
        Err(_) => return err_envelope("index cache is unavailable"),
    }
    ok_envelope(serde_json::json!({"family": family, "forms": forms}))
}

/// Reverse-search the cached family index (Devanagari or HK query).
#[wasm_bindgen]
pub fn index_query_family(family: &str, query: &str) -> String {
    let Ok(cache) = FAMILY_INDEX.lock() else {
        return err_envelope("index cache is unavailable");
    };
    let Some(index) = cache.get(family) else {
        return err_envelope("family index is not built (call index_ensure_family first)");
    };
    hits_json(&[index], query)
}

/// Ensure all five family base indexes from their artifact JSONs, for
/// cross-family search. The portal calls this once (parallel fetches);
/// artifacts stay cached offline after.
#[wasm_bindgen]
pub fn index_ensure_all(
    shuddha_json: &str,
    nich_json: &str,
    san_json: &str,
    yang_json: &str,
    yangluk_json: &str,
) -> String {
    for (family, text) in [
        ("shuddha", shuddha_json),
        ("nich", nich_json),
        ("san", san_json),
        ("yang", yang_json),
        ("yangluk", yangluk_json),
    ] {
        let Some(index) = crate::revindex::from_compact_str(text) else {
            return err_envelope("index artifact version/shape mismatch");
        };
        match FAMILY_INDEX.lock() {
            Ok(mut cache) => {
                cache.insert(family.to_string(), index);
            }
            Err(_) => return err_envelope("index cache is unavailable"),
        }
    }
    ok_envelope(serde_json::json!({"families": ["shuddha", "nich", "san", "yang", "yangluk"]}))
}

/// Reverse-search every cached family index and merge (Devanagari or HK).
#[wasm_bindgen]
pub fn index_query_all(query: &str) -> String {
    let Ok(cache) = FAMILY_INDEX.lock() else {
        return err_envelope("index cache is unavailable");
    };
    if cache.is_empty() {
        return err_envelope("no family index is built (call index_ensure_all first)");
    }
    let indexes: Vec<&RevIndex> = cache.values().collect();
    hits_json(&indexes, query)
}

/// Load any compact index under an arbitrary cache key (portal shard sets).
#[wasm_bindgen]
pub fn index_load_key(key: &str, index_json: &str) -> String {
    let Some(index) = crate::revindex::from_compact_str(index_json) else {
        return err_envelope("index artifact version/shape mismatch");
    };
    let forms = index.postings();
    match FAMILY_INDEX.lock() {
        Ok(mut cache) => {
            cache.insert(key.to_string(), index);
        }
        Err(_) => return err_envelope("index cache is unavailable"),
    }
    ok_envelope(serde_json::json!({"key": key, "forms": forms}))
}

/// FNV-1a shard of a form, mirroring `revindex::shard_of` (portal hashing).
#[wasm_bindgen]
pub fn index_shard(form: &str, shards: u32) -> String {
    ok_envelope(serde_json::json!({
        "shard": crate::revindex::shard_of(form, shards),
    }))
}

/// Build the prefix surface map from Devanagari stack names
/// (`["प्र", "वि + अति", …]`, e.g. distinct `upasargas` across `dhatu_data`).
/// Called once by the portal; later calls rebuild (same input → same map).
#[wasm_bindgen]
pub fn split_init(stacks_json: &str) -> String {
    let names: Vec<String> = match serde_json::from_str(stacks_json) {
        Ok(names) => names,
        Err(_) => return err_envelope("stacks JSON must be an array of strings"),
    };
    let mut stacks = Vec::with_capacity(names.len());
    for name in &names {
        match crate::translit::deva_stack_to_slp1(name) {
            Some(stack) => stacks.push(stack),
            None => return err_envelope("a stack is not Devanagari upasargas"),
        }
    }
    let map = crate::engine::upasplit::build_surface_map(&stacks);
    match SURFACE_MAP.lock() {
        Ok(mut slot) => {
            *slot = Some(map);
        }
        Err(_) => return err_envelope("splitter is unavailable"),
    }
    ok_envelope(serde_json::json!({"stacks": names.len()}))
}

/// Split a query into `(stack, base)` candidates with their base shards
/// (`SHARDS` must match the built artifacts). Stacks and bases are
/// Devanagari display strings; bases are attested in the loaded cache.
#[wasm_bindgen]
pub fn split_query(query: &str, shards: u32) -> String {
    use crate::revindex::shard_of;
    let readings = candidates(query);
    if readings.is_empty() {
        return err_envelope("query is not Devanagari or Harvard-Kyoto");
    }
    let Ok(map) = SURFACE_MAP.lock() else {
        return err_envelope("splitter is unavailable");
    };
    let Some(surface_map) = map.as_ref() else {
        return err_envelope("splitter is not initialized (call split_init first)");
    };
    // Unpruned: every structural split is proposed; the portal ensures each
    // base shard on demand and `verify_split` soundness-filters. (Pruning
    // here would need the full index resident.)
    let no_prune = |_slp: &str| true;
    let mut splits = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for reading in &readings {
        let Some(query_slp) = (if contains_deva(reading) {
            crate::translit::deva_to_slp1(reading)
        } else {
            crate::hk::hk_to_slp1(reading)
        }) else {
            continue;
        };
        for (stack_slp, base_slp) in
            crate::engine::upasplit::split_candidates(&query_slp, surface_map, &no_prune)
        {
            let (Some(stack), Some(base)) = (
                display_stack(&stack_slp),
                crate::translit::slp1_to_deva(&base_slp),
            ) else {
                continue;
            };
            if !seen.insert((stack.clone(), base.clone())) {
                continue;
            }
            splits.push(serde_json::json!({
                "stack": stack,
                "base": base,
                "shard": shard_of(&base, shards),
            }));
        }
    }
    ok_envelope(serde_json::json!({"query": query, "splits": splits}))
}

/// Devanagari display for an engine stack (`vi;ati` → `वि + अति`).
fn display_stack(stack: &str) -> Option<String> {
    let mut parts = Vec::new();
    for token in crate::slp1::split_prefix_stack(stack) {
        parts.push(crate::translit::slp1_to_deva(token)?);
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join(" + "))
}

/// Verify one `(stack, base)` split against the loaded cache: exact forward
/// regeneration (category-only context) must reproduce the query.
#[wasm_bindgen]
pub fn verify_split(query: &str, stack: &str, base: &str) -> String {
    use crate::engine::upasarga::generate_prefixed;
    let Some(query_slp) = (if contains_deva(query) {
        crate::translit::deva_to_slp1(query)
    } else {
        crate::hk::hk_to_slp1(query)
    }) else {
        return err_envelope("query is not Devanagari or Harvard-Kyoto");
    };
    let Some(stack_slp) = crate::translit::deva_stack_to_slp1(stack) else {
        return err_envelope("stack is not Devanagari upasargas");
    };
    let Some(base_slp) = crate::translit::deva_to_slp1(base) else {
        return err_envelope("base is not Devanagari");
    };
    let Ok(cache) = FAMILY_INDEX.lock() else {
        return err_envelope("index cache is unavailable");
    };
    let mut hits = Vec::new();
    for index in cache.values() {
        for posting in index.get(base) {
            // Any attested surface works: tiṇanta/kṛdanta cells and declined
            // (subanta) kṛdanta forms — prefixing the surface directly is
            // sandhi-correct for lookup. Declined prefixed forms (e.g.
            // `प्रददद्भिः`) live only in the subanta set, so skipping these
            // made them unfindable.
            let (family, baseindex) = match &posting.loc {
                LocKind::Ting { family, .. }
                | LocKind::Krt { family, .. }
                | LocKind::Subanta { family, .. } => (family.clone(), posting.baseindex.clone()),
                LocKind::Prefixed { .. } => continue,
            };
            let Some(category) = crate::forms::engine_ting_category(&family) else {
                continue;
            };
            // Category-only context (no per-dhatu fetch on this path).
            let ctx = crate::engine::upasarga::GenCtx {
                derived: crate::engine::upasarga::is_derived_category(category),
                intensive_keep: category.starts_with("yang"),
                yangluk: category.starts_with("yangluk"),
                ..crate::engine::upasarga::GenCtx::default()
            };
            if generate_prefixed(&stack_slp, &base_slp, ctx).contains(&query_slp) {
                hits.push(Posting {
                    baseindex,
                    loc: LocKind::Prefixed {
                        stack: stack.to_string(),
                        inner: Box::new(posting.loc.clone()),
                    },
                });
            }
        }
    }
    hits.sort();
    hits.dedup();
    let shown: Vec<serde_json::Value> = hits.into_iter().map(|hit| loc_json(&hit)).collect();
    ok_envelope(serde_json::json!({"query": query, "total": shown.len(), "hits": shown}))
}

/// Drop a cached family index to free memory.
#[wasm_bindgen]
pub fn index_drop_family(family: &str) -> String {
    match FAMILY_INDEX.lock() {
        Ok(mut cache) => {
            cache.remove(family);
        }
        Err(_) => return err_envelope("index cache is unavailable"),
    }
    ok_envelope(serde_json::json!({"family": family, "dropped": true}))
}

/// Load a prebuilt compact index artifact (`build-index` output) into the
/// family cache. The portal fetches `data/indexes/<family>.index.json` once
/// instead of the raw table files + runtime build.
#[wasm_bindgen]
pub fn index_load_family(family: &str, index_json: &str) -> String {
    if !KRUT_FAMILIES.contains(&family) {
        return err_envelope("unknown family (try shuddha|nich|san|yang|yangluk)");
    }
    let Some(index) = crate::revindex::from_compact_str(index_json) else {
        return err_envelope("index artifact version/shape mismatch");
    };
    let forms = index.postings();
    match FAMILY_INDEX.lock() {
        Ok(mut cache) => {
            cache.insert(family.to_string(), index);
        }
        Err(_) => return err_envelope("index cache is unavailable"),
    }
    ok_envelope(serde_json::json!({"family": family, "forms": forms}))
}

/// Export a cached family index in compact artifact form (debugging and
/// roundtrip tests; the build pipeline uses `build-index` instead).
#[wasm_bindgen]
pub fn index_export_family(family: &str) -> String {
    let Ok(mut cache) = FAMILY_INDEX.lock() else {
        return err_envelope("index cache is unavailable");
    };
    let Some(index) = cache.get_mut(family) else {
        return err_envelope("family index is not built (call index_ensure_family first)");
    };
    ok_envelope(
        serde_json::json!({"family": family, "index": crate::revindex::to_compact_value(index)}),
    )
}

/// Reverse-search prefixed surfaces for one dhatu. Entry JSONs are the
/// per-dhatu table slices (`{lakara: "…;…"}`, `{krt: "base,m,f,n"}`);
/// `dhatu_json` is `{"family","baseindex","slp1","note"}` (vidyut dhatu +
/// note drive the generation context); `stacks_json` is `["वि + अति", …]`.
/// The small index is dropped after.
#[wasm_bindgen]
pub fn search_prefixed_dhatu(
    ting_kartari_entry_json: &str,
    ting_karmani_entry_json: &str,
    krut_entry_json: &str,
    dhatu_json: &str,
    stacks_json: &str,
    query: &str,
) -> String {
    let dhatu: HashMap<String, String> = match serde_json::from_str(dhatu_json) {
        Ok(dhatu) => dhatu,
        Err(_) => {
            return err_envelope("dhatu JSON must be {family,baseindex,slp1,note}");
        }
    };
    let (family, norm, slp1, note) = match (
        dhatu.get("family"),
        dhatu.get("baseindex"),
        dhatu.get("slp1"),
        dhatu.get("note"),
    ) {
        (Some(family), Some(baseindex), Some(slp1), Some(note)) => (
            family.clone(),
            crate::normalize_baseindex(baseindex),
            slp1.clone(),
            note.clone(),
        ),
        _ => return err_envelope("dhatu JSON must be {family,baseindex,slp1,note}"),
    };
    let mut collected: DhatuForms = HashMap::new();
    for (entry_json, prayoga) in [
        (ting_kartari_entry_json, "kartari"),
        (ting_karmani_entry_json, "karmani"),
    ] {
        if entry_json.trim().is_empty() {
            continue;
        }
        let ting_entry: HashMap<String, String> = match serde_json::from_str(entry_json) {
            Ok(entry) => entry,
            Err(_) => return err_envelope("ting entry JSON does not parse"),
        };
        let bucket = collected.entry(norm.clone()).or_default();
        let mut lakaras: Vec<&String> = ting_entry.keys().collect();
        lakaras.sort();
        for lakara in lakaras {
            for (pos, slot) in crate::forms::split_slots(&ting_entry[lakara])
                .iter()
                .enumerate()
            {
                for variant in crate::forms::split_variants(slot) {
                    bucket.push((
                        LocKind::Ting {
                            family: family.clone(),
                            prayoga: prayoga.to_string(),
                            lakara: (*lakara).clone(),
                            pos,
                        },
                        (*variant).to_string(),
                    ));
                }
            }
        }
    }
    let krut_entry: HashMap<String, String> = match serde_json::from_str(krut_entry_json) {
        Ok(entry) => entry,
        Err(_) => return err_envelope("krut entry JSON does not parse"),
    };
    let stacks: Vec<String> = match serde_json::from_str(stacks_json) {
        Ok(stacks) => stacks,
        Err(_) => return err_envelope("stacks JSON must be an array of strings"),
    };
    let bucket = collected.entry(norm.clone()).or_default();
    let mut krts: Vec<&String> = krut_entry.keys().collect();
    krts.sort();
    for krt in krts {
        for (row, cell) in parse_krt_rows(&krut_entry[krt]).iter().enumerate() {
            for (field, value) in [
                ("base", &cell.base),
                ("m", &cell.m),
                ("f", &cell.f),
                ("n", &cell.n),
            ] {
                for alternant in crate::slp1::split_alternants(value) {
                    bucket.push((
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
    let Some(dhatu_forms) = collected.get(&norm) else {
        return err_envelope("no base forms for this dhatu");
    };
    let ctx_for = |inner: &LocKind| crate::genctx::ctx_for_loc(Some(&slp1), &note, inner);
    let mut index = RevIndex::default();
    let mut stats = Stats::default();
    for stack in &stacks {
        index_prefixed_dhatu(&mut index, &norm, stack, dhatu_forms, &ctx_for, &mut stats);
    }
    hits_json(&[&index], query)
}

/// Reverse-search subanta surfaces for one dhatu's krut entry
/// (`{krt: "base,m,f,n"}`). The small index is dropped after.
#[wasm_bindgen]
pub fn search_subanta_dhatu(
    krut_entry_json: &str,
    family: &str,
    baseindex: &str,
    query: &str,
) -> String {
    let norm = crate::normalize_baseindex(baseindex);
    let krut_entry: HashMap<String, String> = match serde_json::from_str(krut_entry_json) {
        Ok(entry) => entry,
        Err(_) => return err_envelope("krut entry JSON does not parse"),
    };
    let mut index = RevIndex::default();
    let mut stats = Stats::default();
    let mut krts: Vec<&String> = krut_entry.keys().collect();
    krts.sort();
    for krt in krts {
        for (row, cell) in parse_krt_rows(&krut_entry[krt]).iter().enumerate() {
            for (field, value) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                if value.is_empty() {
                    continue;
                }
                index_subanta_cell(
                    &mut index,
                    &SubantaCell {
                        baseindex: &norm,
                        family,
                        krt_deva: krt,
                        row,
                        field,
                        value_deva: value,
                    },
                    &mut stats,
                );
            }
        }
    }
    hits_json(&[&index], query)
}
