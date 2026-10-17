//! Engine-derived prefixed kṛdanta rows (Vidyuṭ).
//!
//! Upstream kṛt cells prefixed by sandhi alone cannot match the site:
//! neuter `t`/`d` alternation, `ṇatva` scope, baked-stack replacement, and
//! declined nominatives (not stems) in m/f/n cells. The site derives
//! prefixed kṛdantas live through Vidyuṭ — so do we, with the pinned
//! engine, unioned with the upstream supplement:
//!
//! - base/m/n: vidyuṭ stems, declined by our subanta engine (which this
//!   audit cross-validates — VantM/AtM fixes included).
//! - f: vidyuṭ strī-pratyaya stems where changed, UNION upstream
//!   pre-feminized cells prefixed (at-stems like `ददती`, which no
//!   strī-formation derives from the masculine stem).
//! - kṛts vidyuṭ cannot derive (unādi gaps): upstream rows prefixed via
//!   the legacy cell path, whole.
//!
//! Display-name mapping follows the site's bundle exactly (only six
//! special tags; everything else derives under its own name):
//! `ग्स्नु→क्स्नु`, `ल्यप्→क्त्वा`, `स्य-शानच्→शानच्+lrut`,
//! `स्य-शतृ→शतृ+lrut`, `भावकर्म-शानच्→शानच्+karmani`,
//! `स्य-भावकर्म-शानच्→शानच्+lrut+karmani`.
//!
//! Output is rows (`{base, m, f, n}`, Devanagari alternants `/`-joined),
//! deduplicated. Avyaya masking (empty upstream cells stay empty) is the
//! portal's job — it owns the upstream emptiness facts.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use vidyut_prakriya::args::{BaseKrt, Krdanta, Lakara, Prayoga, Sanadi};
use vidyut_prakriya::{Dhatupatha, Prakriya, Vyakarana};

/// Pinned data this module derives against (same pin as the shipped tables).
const DHATUPATHA_TSV: &str = include_str!("../data/vidyut_dhatupatha_5.tsv");

/// Special display names: (vidyuṭ kṛt override, lakāra, prayoga).
/// Everything else derives under its own name with no extra params.
fn special_tag(krt_deva: &str) -> Option<(&'static str, Option<Lakara>, Option<Prayoga>)> {
    match krt_deva {
        "ग्स्नु" => Some(("क्स्नु", None, None)),
        "ल्यप्" => Some(("क्त्वा", None, None)),
        "स्य-शानच्" => Some(("शानच्", Some(Lakara::Lrt), None)),
        "स्य-शतृ" => Some(("शतृ", Some(Lakara::Lrt), None)),
        "भावकर्म-शानच्" => Some(("शानच्", None, Some(Prayoga::Karmani))),
        "स्य-भावकर्म-शानच्" => {
            Some(("शानच्", Some(Lakara::Lrt), Some(Prayoga::Karmani)))
        }
        _ => None,
    }
}

fn sanadi_of(family: &str) -> Option<Sanadi> {
    match family {
        "nich" => Some(Sanadi::Ric),
        "san" => Some(Sanadi::san),
        "yang" => Some(Sanadi::yaN),
        "yangluk" => Some(Sanadi::yaNluk),
        _ => None,
    }
}

struct Engine {
    vp: Vyakarana,
    dhatupatha: Dhatupatha,
}

static ENGINE: LazyLock<Mutex<Option<Engine>>> = LazyLock::new(|| Mutex::new(None));

fn engine() -> Result<std::sync::MutexGuard<'static, Option<Engine>>, String> {
    let mut slot = ENGINE
        .lock()
        .map_err(|_| "derive engine is unavailable".to_string())?;
    if slot.is_none() {
        let dhatupatha =
            Dhatupatha::from_text(DHATUPATHA_TSV).map_err(|e| format!("dhatupatha: {e:?}"))?;
        *slot = Some(Engine {
            vp: Vyakarana::builder().build(),
            dhatupatha,
        });
    }
    Ok(slot)
}

/// Display Devanagari kṛt name → vidyuṭ `BaseKrt` (markers stripped).
/// Special tags resolve to their mapped kṛt first (site parity).
fn base_krt(krt_deva: &str) -> Option<BaseKrt> {
    let name = match special_tag(krt_deva) {
        Some((mapped, _, _)) => mapped.to_string(),
        None => krt_deva.to_string(),
    };
    let slp1 = crate::translit::deva_to_slp1(&name)?;
    BaseKrt::iter().find(|variant| variant.as_str().replace('~', "") == slp1)
}

/// Prathamā-ekavacana of a stem, or the stem itself when it does not
/// decline (never drop forms).
fn decline_nominal(
    krt_slp1: &str,
    field: &str,
    stem_slp1: &str,
    engine_family: &str,
    baseindex: &str,
) -> Vec<String> {
    use crate::engine::subanta::krt_table;
    let Some(table) = krt_table(engine_family, krt_slp1, field, stem_slp1, baseindex) else {
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

/// Strī-pratyaya SLP1 stems that DIFFER from the input stem (feminine
/// formation actually changed something).
fn feminized_stems(vp: &Vyakarana, stem_slp1: &str) -> Vec<String> {
    use vidyut_prakriya::args::Pratipadika;
    let Ok(slp1) = stem_slp1.to_string().try_into() else {
        return Vec::new();
    };
    let pratipadika = Pratipadika::basic(slp1);
    vp.derive_stryantas(&pratipadika)
        .iter()
        .map(Prakriya::text)
        .filter(|t| !t.is_empty() && t != stem_slp1)
        .collect()
}

/// Upstream row in SLP1 alternants per field.
type UpRow = HashMap<String, Vec<String>>;

/// Derived prefixed kṛdanta rows for one (upasarga-stack, dhātu, family,
/// kṛt): `[{base, m, f, n}]` with Devanagari `/`-joined cells.
/// `upstream` carries the dhātu's upstream rows (Deva `{base, m, f, n}`
/// strings, may be empty) for the supplement + vidyuṭ-missing fallback.
/// `slp1_dhatu`/`engine_category`/`dhatu_note` feed the supplement's `GenCtx`.
/// `pinned` carries the corpus cell for this (stack, family, kṛt) when the
/// caller has it (portal blob piece, audit); a non-empty pin is served
/// verbatim and the engine is not consulted.
/// SLP1 never leaves this module.
#[allow(clippy::too_many_arguments)]
pub fn derive_prefixed_krut(
    stack_deva: &str,
    baseindex: &str,
    family: &str,
    krt_deva: &str,
    upstream: &[(String, String, String, String)],
    slp1_dhatu: &str,
    engine_category: &str,
    dhatu_note: &str,
    pinned: &[crate::prefix_krut::PinnedRow],
) -> Result<Vec<HashMap<String, String>>, String> {
    use crate::engine::upasarga::generate_prefixed;
    // (kṛt, override-m, override-n). f is always repaired from the upstream
    // cell when present — the engine's strī derivation has at-stem/ī-stem
    // gaps (`भवन्ती`, `भाविका`, `बभूषी`). m/n are repaired only where the
    // engine's own declension diverges from the corpus: क्वसु m (`वाः` for
    // `वान्`) and n (`वः` for `वत्/वद्`), क्तवतु/शतृ n (missing voiced
    // alternant). शतृ m is NOT repaired: the yangluk upstream m cell is
    // `बोभुवत्/बोभुवद्` while the corpus wants `बोभुवन्`, so the engine's
    // own m wins there.
    const UPSTREAM_OVERRIDE_KRTS: &[(&str, bool, bool)] = &[
        ("क्वसु", true, true),
        ("क्तवतु", false, true),
        ("शतृ", false, true),
        ("क्तिन्", false, false),
        ("ण्वुल्", false, false),
        ("वुन्", false, false),
        ("ग्स्नु", false, false),
        ("उकञ्", false, false),
        ("अथुच्", true, false),
    ];
    // Corpus-pinned cell (the portal's `blob.krut_prefixed` piece for this
    // (stack, family, kṛt), or a client-supplied equivalent): served
    // verbatim, in row order. The corpus stack sections are the site's own
    // data — re-deriving them here could only drift from them.
    if !pinned.is_empty() {
        return Ok(pinned
            .iter()
            .map(|[base, m, f, n]| {
                let mut row = HashMap::new();
                row.insert("base".to_string(), base.clone());
                row.insert("m".to_string(), m.clone());
                row.insert("f".to_string(), f.clone());
                row.insert("n".to_string(), n.clone());
                row
            })
            .collect());
    }
    let (override_m, override_n) = UPSTREAM_OVERRIDE_KRTS
        .iter()
        .find(|(name, _, _)| *name == krt_deva)
        .map_or((false, false), |(_, m, n)| (*m, *n));
    let stack_slp = crate::translit::deva_stack_to_slp1(stack_deva)
        .ok_or_else(|| "stack is not Devanagari upasargas".to_string())?;
    let guard = engine()?;
    let engine = guard
        .as_ref()
        .ok_or_else(|| "derive engine is unavailable".to_string())?;
    let mut dhatu = engine
        .dhatupatha
        .get(baseindex)
        .ok_or_else(|| format!("unknown dhatu {baseindex}"))?
        .clone();
    let upasarga_prefixes: Vec<&str> = stack_slp.split(';').collect();
    dhatu = dhatu.with_prefixes(&upasarga_prefixes);
    if let Some(sanadi) = sanadi_of(family) {
        dhatu = dhatu.with_sanadi(&[sanadi]);
    }
    // Special-tag lakāra/prayoga (site parity).
    let (lakara, prayoga) = match special_tag(krt_deva) {
        Some((_, lakara, prayoga)) => (lakara, prayoga),
        None => (None, None),
    };
    let base_krt = base_krt(krt_deva).ok_or_else(|| format!("unknown krt {krt_deva}"))?;
    let mut builder = Krdanta::builder().dhatu(dhatu).krt(base_krt);
    if let Some(lakara) = lakara {
        builder = builder.lakara(lakara);
    }
    if let Some(prayoga) = prayoga {
        builder = builder.prayoga(prayoga);
    }
    let args = builder.build().map_err(|e| format!("args: {e:?}"))?;
    let mut stems: Vec<String> = engine
        .vp
        .derive_krdantas(&args)
        .iter()
        .map(Prakriya::text)
        .filter(|t| !t.is_empty())
        .collect();
    stems.sort();
    stems.dedup();
    let engine_family = crate::forms::engine_krut_family(family).unwrap_or("krut");
    let krt_slp1 = crate::translit::deva_to_slp1(krt_deva).unwrap_or_default();
    let ctx = crate::genctx::ctx_for(slp1_dhatu, engine_category, dhatu_note);

    // Vidyuṭ rows: one per stem. Feminine comes from strī-pratyaya
    // stems of THAT stem only (never shared across rows).
    let mut rows: Vec<UpRow> = Vec::new();
    for stem in &stems {
        let mut row: UpRow = HashMap::new();
        row.insert("base".to_string(), vec![stem.clone()]);
        for field in ["m", "n"] {
            row.insert(
                field.to_string(),
                decline_nominal(&krt_slp1, field, stem, engine_family, baseindex),
            );
        }
        let mut fem: Vec<String> = Vec::new();
        for fstem in feminized_stems(&engine.vp, stem) {
            for form in decline_nominal(&krt_slp1, "f", &fstem, engine_family, baseindex) {
                if !fem.contains(&form) {
                    fem.push(form);
                }
            }
        }
        row.insert("f".to_string(), fem);
        rows.push(row);
    }
    // Vidyuṭ receives the stack via `Dhatu::with_prefixes`, so each stem
    // is ALREADY prefixed (`AN+BAvaka -> ABAvaka`, `sam+cokzuditum ->
    // saMcokzuditum`/`saYcokzuditum`): no second prefixing pass here.

    let mut surface_to_rows: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, row) in rows.iter().enumerate() {
        for base in row.get("base").cloned().unwrap_or_default() {
            surface_to_rows.entry(base).or_default().push(idx);
        }
    }
    // Upstream cells are BARE (unprefixed) stems/declensions, while vidyuṭ
    // rows carry the stack baked in — so the supplement prefixes every
    // upstream alternant through our sandhi rules and matches the surface.
    // Where the prefix licenses several surfaces (`sam` before stops/nasals
    // gives the anusvāra form AND the parasavarṇa form, which the corpus
    // lists as separate rows), `generate_prefixed` returns them in a stable
    // order and the same index position of every field belongs to the same
    // variant, so each vidyuṭ row is repaired from ITS OWN variant.
    let mut handled: HashSet<usize> = HashSet::new();
    let split_alts = |text: &str| -> Vec<String> {
        text.split('/')
            .filter(|a| !a.is_empty())
            .map(|a| crate::translit::deva_to_slp1(a).unwrap_or_else(|| a.to_string()))
            .collect()
    };
    for (base, m, f, n) in upstream {
        // Base variants in generation order; their count sets the group
        // count for this upstream row's fields.
        let mut base_groups: Vec<Vec<String>> = Vec::new();
        let mut n_groups = 0_usize;
        for alt in split_alts(base) {
            // ल्यप् ships pre-prefixed (baked stack: `प्रभूय`, fused `प्रेड्य`);
            // strip the baked code before applying the selected stack, else
            // the supplement row double-prefixes and never matches vidyuṭ.
            let alt = if krt_deva == "ल्यप्" {
                crate::prefix_krut::strip_baked_stack(&alt).to_string()
            } else {
                alt
            };
            let variants = generate_prefixed(&stack_slp, &alt, ctx);
            n_groups = n_groups.max(variants.len());
            base_groups.push(variants);
        }
        if n_groups == 0 {
            continue;
        }
        // Per field: group-aligned prefixed surfaces (a single-variant
        // alternant belongs to every group).
        let field_groups = |text: &str| -> Vec<Vec<String>> {
            let mut groups: Vec<Vec<String>> = vec![Vec::new(); n_groups];
            for alt in split_alts(text) {
                let variants = generate_prefixed(&stack_slp, &alt, ctx);
                if variants.len() == 1 {
                    for group in &mut groups {
                        group.push(variants[0].clone());
                    }
                } else {
                    for (idx, surface) in variants.into_iter().enumerate() {
                        if let Some(group) = groups.get_mut(idx) {
                            group.push(surface);
                        }
                    }
                }
            }
            for group in &mut groups {
                group.sort();
                group.dedup();
            }
            groups
        };
        let m_groups = field_groups(m);
        let f_groups = field_groups(f);
        let n_forms_groups = field_groups(n);
        for group in 0..n_groups {
            let mut row_indices: Vec<usize> = Vec::new();
            for variants in &base_groups {
                if let Some(surface) = variants.get(group) {
                    if let Some(indices) = surface_to_rows.get(surface) {
                        row_indices.extend(indices);
                    }
                }
            }
            row_indices.sort_unstable();
            row_indices.dedup();
            if row_indices.is_empty() {
                // No vidyuṭ row for this variant (vidyuṭ cannot derive the
                // kṛt, or omitted the variant): render the upstream cell
                // prefixed whole.
                let mut row: UpRow = HashMap::new();
                let mut bases: Vec<String> = base_groups
                    .iter()
                    .filter_map(|variants| variants.get(group).cloned())
                    .collect();
                bases.sort();
                bases.dedup();
                if !bases.is_empty() {
                    row.insert("base".to_string(), bases);
                }
                if !f_groups[group].is_empty() {
                    row.insert("f".to_string(), f_groups[group].clone());
                }
                if !m_groups[group].is_empty() {
                    row.insert("m".to_string(), m_groups[group].clone());
                }
                if !n_forms_groups[group].is_empty() {
                    row.insert("n".to_string(), n_forms_groups[group].clone());
                }
                rows.push(row);
                continue;
            }
            for idx in row_indices {
                if !handled.insert(idx) {
                    continue;
                }
                let Some(row) = rows.get_mut(idx) else {
                    continue;
                };
                // f: always (upstream pre-feminized cells cover the
                // engine's at-stem/ī-stem gaps).
                if !f_groups[group].is_empty() {
                    row.insert("f".to_string(), f_groups[group].clone());
                }
                // m/n: only for kṛts whose engine declension diverges
                // from the corpus (see UPSTREAM_OVERRIDE_KRTS).
                if override_m && !m_groups[group].is_empty() {
                    row.insert("m".to_string(), m_groups[group].clone());
                }
                if override_n && !n_forms_groups[group].is_empty() {
                    row.insert("n".to_string(), n_forms_groups[group].clone());
                }
            }
        }
    }
    // Avyaya mask (portal rule): a field that NO upstream cell fills stays
    // empty in every derived row — the audit masks truth the same way, and
    // vidyuṭ's stem fallbacks (ल्युट्/तुमुन्-type) would otherwise leak
    // nominative-looking forms into empty cells.
    let mut filled = [false; 3];
    for (_, m, f, n) in upstream {
        for (i, text) in [m, f, n].iter().enumerate() {
            if text.split('/').any(|a| !a.is_empty()) {
                filled[i] = true;
            }
        }
    }
    for (i, field) in ["m", "f", "n"].iter().enumerate() {
        if !filled[i] {
            for row in &mut rows {
                row.insert(field.to_string(), Vec::new());
            }
        }
    }
    // Unvoiced twin suppression: upstream ships kṛdanta cells as
    // unvoiced/voiced pairs while the corpus keeps only the voiced member.
    // The pairs can arrive split across rows (vidyuṭ row + upstream
    // supplement row), so scan over every form of a field across all rows.
    for field in ["base", "m", "f", "n"] {
        let mut all: Vec<String> = Vec::new();
        for row in &rows {
            for form in row.get(field).cloned().unwrap_or_default() {
                if !all.contains(&form) {
                    all.push(form);
                }
            }
        }
        let kept = crate::prefix_krut::suppress_twins(&all);
        for row in &mut rows {
            if let Some(forms) = row.get_mut(field) {
                forms.retain(|form| kept.contains(form));
            }
        }
    }
    // Rows emptied by the suppression drop out entirely.
    rows.retain(|row| {
        ["base", "m", "f", "n"].iter().any(|f| {
            row.get(*f)
                .is_some_and(|forms| forms.iter().any(|s| !s.is_empty()))
        })
    });
    // Merge rows by base stem. Rows are keyed by EACH of their base
    // surfaces (an upstream row legitimately carries several alternants
    // inside one cell) so a supplement row lands in every vidyuṭ row whose
    // base it shares — keying by the first surface only collapsed whole
    // sandhi variants into one row, which the corpus lists separately.
    let mut merged: HashMap<String, UpRow> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for row in &rows {
        let bases = row.get("base").cloned().unwrap_or_default();
        let keys: Vec<String> = if bases.is_empty() {
            vec![String::new()]
        } else {
            bases.clone()
        };
        for key in &keys {
            if !order.contains(key) {
                order.push(key.clone());
            }
            let slot = merged.entry(key.clone()).or_default();
            for field in ["base", "m", "f", "n"] {
                let entry = slot.entry(field.to_string()).or_default();
                if let Some(forms) = row.get(field) {
                    for form in forms {
                        if !entry.contains(form) {
                            entry.push(form.clone());
                        }
                    }
                }
            }
        }
    }
    // Drop keys that ended up carrying only empty fields (stray alternants
    // from a supplement row whose base matched nothing).
    order.retain(|key| {
        merged.get(key).is_some_and(|row| {
            ["base", "m", "f", "n"].iter().any(|f| {
                row.get(*f)
                    .is_some_and(|forms| forms.iter().any(|s| !s.is_empty()))
            })
        })
    });
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for key in &order {
        let row = &merged[key];
        let mut deva_row = HashMap::new();
        for field in ["base", "m", "f", "n"] {
            let forms: Vec<String> = row.get(field).cloned().unwrap_or_default();
            let forms = crate::prefix_krut::suppress_na_twins(&forms);
            let mut devas: Vec<String> = forms
                .iter()
                .filter_map(|s| crate::translit::slp1_to_deva(s))
                .collect();
            devas.sort();
            devas.dedup();
            deva_row.insert(field.to_string(), devas.join("/"));
        }
        let fingerprint = ["base", "m", "f", "n"]
            .iter()
            .map(|f| deva_row.get(*f).cloned().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("|");
        if seen.contains(&fingerprint) {
            continue;
        }
        seen.push(fingerprint);
        out.push(deva_row);
    }
    Ok(out)
}
