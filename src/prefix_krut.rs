//! Kṛt-aware prefixing, shared by the search-index fold and the portal.
//!
//! Raw `generate_prefixed` is kṛt-blind. The corpus proves three
//! normalizations (see `tests/prefixed_krut_audit.rs`, which fails on any
//! over/under-generation):
//!
//! 1. **ल्यप् carries a baked stack.** Upstream ल्यप् cells ship pre-prefixed
//!    (usually `praBUya`, sometimes `aDi…`, or fused `prEDya`). Prefixing
//!    the selected stack on top double-prefixes (`aDi + praBUya →
//!    *aDipraBUya`; site and corpus show `aDiBUya`). So for ल्यप् only:
//!    strip the leading upasarga code (longest of the site's 22, plus fused
//!    `pr` before vowels), then prefix the selected stack.
//! 2. **Decline to prathamā-ekavacana.** Corpus m/f/n cells are declined
//!    nominatives, not stems (`boBuvan`, not `*boBuvat` — the site computes
//!    them via its own prathama-ekavacana step). So m/f/n surfaces decline
//!    through `krt_table`; the base column keeps the stem. Undeclinable
//!    surfaces fall back to the bare prefixed stem (never drop forms).
//! 3. **Twin suppression.** Where the engine yields both a `na`-form and
//!    its `Na`-twin, the corpus keeps only the `Na` one (2.19M corpus cells
//!    audited: twins are never kept). Drop the `na` form when its twin is
//!    present. (Neuter `t`/`d` alternation needs no special case: the
//!    declension already emits both finals.)
//!
//! Declined (subanta) surfaces bypass all of this: the declension already
//! emits complete finals, and stripping never applies to inflected forms.

use crate::engine::subanta::krt_table;
use crate::engine::upasarga::{generate_prefixed, GenCtx};
use std::collections::HashMap;

/// The site's 22 upasarga SLP1 codes (for baked-stack stripping),
/// longest first so `prati…` strips before `pra…`.
const UPASARGA_CODES: [&str; 22] = [
    "prati", "parA", "pari", "pra", "apa", "sam", "anu", "ava", "nis", "nir", "dus", "dur", "aBi",
    "aDi", "api", "ati", "vi", "AN", "ni", "su", "ud", "upa",
];

/// Strip a baked upasarga code from the start of an SLP1 base.
/// Returns the stripped base, or the original when nothing strips cleanly
/// (no code matches, or the remainder is empty).
///
/// Baked `pra` fuses with vowel-initial roots (`pra` + `EDya` → `prEDya`),
/// so after the code list, a bare `pr` before a vowel also strips.
pub fn strip_baked_stack(base_slp1: &str) -> &str {
    for code in UPASARGA_CODES {
        if let Some(rest) = base_slp1.strip_prefix(code) {
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    if let Some(rest) = base_slp1.strip_prefix("pr") {
        if rest.starts_with(['A', 'I', 'U', 'e', 'E', 'o', 'O', 'f', 'F', 'x', 'X']) {
            return rest;
        }
    }
    base_slp1
}

/// Drop every form that has a `Na`-twin in the set (same length, differing
/// only in `n`/`R` positions — SLP1 `R` is retroflex ṇa). The corpus never
/// keeps both.
pub(crate) fn suppress_na_twins(forms: &[String]) -> Vec<String> {
    forms
        .iter()
        .filter(|form| {
            !forms.iter().any(|other| {
                other.as_str() != form.as_str()
                    && other.len() == form.len()
                    && other
                        .chars()
                        .zip(form.chars())
                        .all(|(o, f)| o == f || (f == 'n' && o == 'R'))
            })
        })
        .cloned()
        .collect()
}

/// Both twin suppressions in one pass, for use on any form set.
///
/// The corpus never keeps both members of a pair, whichever row or field
/// each member arrived in — so callers apply this to the whole set (all
/// rows of one (dhātu, stack, family, kṛt)) rather than per row.
pub(crate) fn suppress_twins(forms: &[String]) -> Vec<String> {
    let na_kept = suppress_na_twins(forms);
    suppress_unvoiced_twins(&na_kept)
}

/// Vṛddhi/guṇa (voicing) twin pairs where the corpus keeps only the voiced
/// member: `vanaya`/`vAnaya`, `van`/`vAn`, `dava`/`dAva`, `ru`/`arO`. Same
/// length, exactly one slot differs, and in the other form that slot is the
/// strong grade of this form's slot. Drop the weak form — upstream ships
/// both rows of such pairs as separate cells, but the site derives only the
/// voiced row, so keeping both double-renders every field.
pub(crate) fn suppress_unvoiced_twins(forms: &[String]) -> Vec<String> {
    /// Weak SLP1 vowel slot → the strong slots it can strengthen to
    /// (`a`→`A` vṛddhi, `a`→`e`/`o` guṇa + vṛddhi; `f`/`F` vocalic ṛ/ṝ).
    fn strong_slots(weak: u8) -> &'static [u8] {
        match weak {
            b'a' => b"Aeo",
            b'i' => b"eo",
            b'I' => b"Eo",
            b'u' | b'U' => b"oO",
            b'f' | b'F' => b"ar",
            _ => b"",
        }
    }
    forms
        .iter()
        .filter(|form| {
            let bytes = form.as_bytes();
            !forms.iter().any(|other| {
                other.as_str() != form.as_str()
                    && other.len() == form.len()
                    && other.as_bytes().iter().zip(bytes).all(|(o, f)| {
                        o == f || (!strong_slots(*f).is_empty() && strong_slots(*f).contains(o))
                    })
            })
        })
        .cloned()
        .collect()
}

/// Prathamā-ekavacana of a prefixed stem, or the stem itself when it does
/// not decline (verbal citations, indeclinables — never drop forms).
fn decline_nominal(
    krt_slp1: &str,
    field: &str,
    stem_slp1: &str,
    engine_family: &str,
) -> Vec<String> {
    let Some(table) = krt_table(engine_family, krt_slp1, field, stem_slp1) else {
        return vec![stem_slp1.to_string()];
    };
    let mut out: Vec<String> = table
        .cells
        .iter()
        .filter(|slot| slot.vibhakti == "prathamA" && slot.vacana == "ekavacanam")
        .flat_map(|slot| slot.forms.clone())
        .collect();
    if out.is_empty() {
        out.push(stem_slp1.to_string());
    }
    out
}

/// Devanagari → SLP1 with non-Devanagari separators kept (`स्य-शतृ` →
/// `sya-Satf`). The declension dispatch rides the stem shape; only
/// `kvasu`/`tfc`/`lyap` are name-sensitive, and those are plain names.
fn deva_to_slp1_keep_sep(text: &str) -> Option<String> {
    let mut out = String::new();
    let mut buf = String::new();
    for ch in text.chars() {
        if crate::translit::contains_deva(&ch.to_string()) {
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

/// Prefixed SLP1 surfaces for one kṛt cell: `values_deva` are the cell's
/// Devanagari alternants, `krt_deva`/`field` select the ल्यप्/declension
/// paths, `engine_family` feeds the declension.
/// Returns the deduplicated union in first-seen order.
pub fn prefix_krut_cell(
    stack_slp1: &str,
    krt_deva: &str,
    field: &str,
    engine_family: &str,
    values_deva: &[String],
    ctx: GenCtx,
) -> Vec<String> {
    prefix_krut_row(
        stack_slp1,
        krt_deva,
        engine_family,
        &[(field.to_string(), values_deva.to_vec())],
        ctx,
    )
    .remove(field)
    .unwrap_or_default()
}

/// Prefixed SLP1 surfaces for one kṛt ROW: `cells` are
/// `(field, alternants)` pairs (`base`/`m`/`f`/`n`). Fields share
/// stem-recovery: a voiced stem attested by any declined nominative is
/// available to every field. Returns `{field: [surfaces]}`.
pub fn prefix_krut_row(
    stack_slp1: &str,
    krt_deva: &str,
    engine_family: &str,
    cells: &[(String, Vec<String>)],
    ctx: GenCtx,
) -> HashMap<String, Vec<String>> {
    let krt_slp1 = deva_to_slp1_keep_sep(krt_deva).unwrap_or_default();
    // Pass 1: prefix (+ decline m/f/n) each cell independently.
    let mut per_field: HashMap<String, Vec<String>> = HashMap::new();
    for (field, values) in cells {
        let mut out: Vec<String> = Vec::new();
        for value in values {
            let Some(base_slp1) = crate::translit::deva_to_slp1(value) else {
                continue;
            };
            let base = if krt_deva == "ल्यप्" {
                strip_baked_stack(&base_slp1).to_string()
            } else {
                base_slp1
            };
            for surface in generate_prefixed(stack_slp1, &base, ctx) {
                // m/f/n cells decline to prathamā-ekavacana (site parity);
                // the base column keeps prefixed stems.
                let forms: Vec<String> = if field == "base" {
                    vec![surface]
                } else {
                    decline_nominal(&krt_slp1, field, &surface, engine_family)
                };
                for form in forms {
                    if !out.contains(&form) {
                        out.push(form);
                    }
                }
            }
        }
        per_field.insert(field.clone(), out);
    }
    // Pass 2 (removed): stem-recovery from declined nominatives was tried
    // and REVERTED — it repaired base/m under-voicing but manufactured
    // halant and double-vowel artifacts faster than shaping rules could
    // contain them. Under-voiced cells are covered by backfill (exact);
    // the live path keeps primary surfaces only.
    // Pass 3: twin-suppression per field.
    for forms in per_field.values_mut() {
        let kept = suppress_na_twins(forms);
        *forms = kept;
    }
    per_field
}
