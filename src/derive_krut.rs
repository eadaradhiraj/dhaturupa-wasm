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
//! Display-name mapping follows the live site's rendering exactly. The
//! site's bundle maps six display names onto plain vidyuṭ kṛts (`क्स्नु`,
//! `क्त्वा`/प्र, `शानच्`, `शतृ`) and passes **no per-row lakāra/prayoga** to
//! the engine (`deriveKridantasUsingVidyut` reads `s.lakara` off the tag
//! map, never `s[e].lakara`), so its स्य-*/भावकर्म-* rows render the plain
//! varṭamāna forms (e.g. प्र+स्य-शतृ = प्रभवन्-family, प्र+भावकर्म-शानच् =
//! प्रैधमान — verified against the live site 01.0001/01.0002/01.0039).
//! We render the same: name-maps below, no lakāra/prayoga overrides. The
//! prefixed view therefore differs from the shipped dump on exactly the
//! स्य-*/भावकर्म-* rows — the dump's लृङ् cells are a different build's
//! data, and the base view (which the site fills from the dump) keeps them.
//!
//! Output is rows (`{base, m, f, n}`, Devanagari alternants `/`-joined),
//! deduplicated. Avyaya masking (empty upstream cells stay empty) is the
//! portal's job — it owns the upstream emptiness facts.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use vidyut_prakriya::args::{BaseKrt, Krdanta, Sanadi};
use vidyut_prakriya::{Dhatupatha, Prakriya, Vyakarana};

/// Pinned data this module derives against (same pin as the shipped tables).
const DHATUPATHA_TSV: &str = include_str!("../data/vidyut_dhatupatha_5.tsv");

/// Special display names: vidyuṭ kṛt override only. The site's live
/// derivation passes no per-row lakāra/prayoga (its bundle never applies
/// them), so स्य-शतृ/स्य-शानच् derive as plain शतृ/शानच् and the
/// भावकर्म-शानच् pair as plain शानच्. Everything else derives under its
/// own name with no extra params.
fn special_tag(krt_deva: &str) -> Option<&'static str> {
    match krt_deva {
        "ग्स्नु" => Some("क्स्नु"),
        "ल्यप्" => Some("क्त्वा"),
        "स्य-शानच्" | "भावकर्म-शानच्" | "स्य-भावकर्म-शानच्" => {
            Some("शानच्")
        }
        "स्य-शतृ" => Some("शतृ"),
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
    let name: &str = special_tag(krt_deva).unwrap_or(krt_deva);
    let slp1 = crate::translit::deva_to_slp1(name)?;
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

/// The row display for a prefixed कृदन्त cell. The site's prefixed rows
/// always print the नुम् nominative of शतृ (`अधिबोभुवन्`), even where the
/// click-through grid keeps the अभ्यस्त no-नुम् pair (`बोभुवत्/बोभुवद्`,
/// its @dadat1 ref): the display and the grid genuinely differ there.
fn display_nominal(
    krt_slp1: &str,
    field: &str,
    stem_slp1: &str,
    engine_family: &str,
    baseindex: &str,
) -> Vec<String> {
    // A क्तवतु stem that never took the vant suffix cannot feed the site's
    // @bhavat reference: it prints the bare stem for every linga
    // (01.0119 उल्लाघ, 01.0442 क्षीब).
    if krt_slp1 == "ktavatu" && !stem_slp1.ends_with('t') {
        return vec![stem_slp1.to_string()];
    }
    if field == "m" && krt_slp1 == "Satf" {
        // An empty baseindex selects the plain @gacchat1 (नुम्) paradigm.
        if let Some(forms) = prathama_ekavacana("krut", "Satf", "m", stem_slp1, "") {
            return forms;
        }
    }
    // No table for this linga: show nothing rather than leaking the raw
    // stem as a nominative (prefixed-only कृत्स mask by slot).
    let forms = prathama_ekavacana(engine_family, krt_slp1, field, stem_slp1, baseindex)
        .unwrap_or_default();
    // The site's n citation of an आ-final stem carries the final -m
    // (उपसर्याम्) while the click table's prathamā is the bare stem.
    if field == "n" && stem_slp1.ends_with('A') && forms.len() == 1 && forms[0] == stem_slp1 {
        return vec![format!("{stem_slp1}m")];
    }
    forms
}

fn prathama_ekavacana(
    family: &str,
    krt_slp1: &str,
    field: &str,
    stem_slp1: &str,
    baseindex: &str,
) -> Option<Vec<String>> {
    use crate::engine::subanta::krt_table;
    let table = krt_table(family, krt_slp1, field, stem_slp1, baseindex)?;
    let forms: Vec<String> = table
        .cells
        .iter()
        .filter(|slot| slot.vibhakti == "prathamA" && slot.vacana == "ekavacanam")
        .flat_map(|slot| slot.forms.clone())
        .collect();
    (!forms.is_empty()).then_some(forms)
}

/// The site's prefixed क्वसु strī from the strong stem: strip the final
/// `vas` and add `uṣī` with the stem-final vowel's sandhi — `yeti` →
/// येत्युषी, `vivI` → विव्युषी, `baBU` → बभूषी, `vavand` → ववन्दुषी.
fn kvasu_strong_f(stem_slp1: &str) -> Option<String> {
    let root = stem_slp1.strip_suffix("vas")?;
    if root.ends_with('i') || root.ends_with('I') {
        Some(format!("{}yuzI", &root[..root.len() - 1]))
    } else if root.ends_with('u') || root.ends_with('U') {
        Some(format!("{root}zI"))
    } else if root.ends_with('f') || root.ends_with('F') {
        // Vocalic ṛ + uṣī → r + uṣī (चकृ → चक्रुषी).
        Some(format!("{}ruzI", &root[..root.len() - 1]))
    } else {
        Some(format!("{root}uzI"))
    }
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
    // स्य-*/भावकर्म-* rows derive engine-only on the live site: the deriver
    // returns its own stems and any empty result REMOVES the row
    // (`0<(t=...).length&&(t=t.unique()...)` — no fallback), and the
    // corpus-pinned cells for them are a different build's लृङ् data.
    const SYA_KRTS: &[&str] = &["स्य-शतृ", "स्य-शानच्", "भावकर्म-शानच्", "स्य-भावकर्म-शानच्"];
    let sy_only = SYA_KRTS.contains(&krt_deva);
    // Corpus-pinned cell (the portal's `blob.krut_prefixed` piece for this
    // (stack, family, kṛt), or a client-supplied equivalent): served
    // verbatim, in row order. The corpus stack sections are the site's own
    // data — re-deriving them here could only drift from them. स्य-/भावकर्म-
    // rows bypass the pin: the live site always derives them engine-first,
    // and the mass ₹-indexed cells for them are stale (लृङ्).
    if !pinned.is_empty() && !sy_only {
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
    let stack_slp = if stack_deva.trim().is_empty() {
        // The site's base view derives through the same path with no
        // upasarga (`deriveKridantasUsingVidyut` with an empty list), so
        // the empty stack is a real input, not a missing one.
        String::new()
    } else {
        crate::translit::deva_stack_to_slp1(stack_deva)
            .ok_or_else(|| "stack is not Devanagari upasargas".to_string())?
    };
    // The site's stack composition (`forceUpasargas` + the ल्यप् rule), in
    // the site's order: the forced upasargas first, then ल्यप्'s own प्र
    // when nothing is selected at all.
    let mut stack_parts: Vec<String> = if stack_slp.is_empty() {
        Vec::new()
    } else {
        stack_slp
            .split(';')
            .map(|p| {
                if p == "A" {
                    "AN".to_string()
                } else {
                    p.to_string()
                }
            })
            .collect()
    };
    // 02.0041/02.0042 (the nitya-ādi pair) take अदि unless the stack
    // already ends in it; 02.0012 takes आ only when nothing is selected;
    // 10.0368 takes आ unless the stack already ends in it. (The site's
    // bundle also pushes अदि for every other dhātu, but neither the
    // published dump nor the corpus has that — the corpus-era data the
    // site ships is the oracle here.)
    if matches!(baseindex, "02.0041" | "02.0042") && stack_parts.last().is_none_or(|p| p != "aDi") {
        stack_parts.push("aDi".to_string());
    }
    if baseindex == "02.0012" && stack_parts.is_empty() {
        stack_parts.push("AN".to_string());
    }
    if baseindex == "10.0368" && stack_parts.last().is_none_or(|p| p != "AN") {
        stack_parts.push("AN".to_string());
    }
    // The site bakes प्र into ल्यप् when no upasarga is selected
    // (`"ल्यप्"==e&&0==i.length&&(i=["pra"])`), so the base view's ल्यप् row
    // is प्रभूय, not भूत्वा.
    if krt_deva == "ल्यप्" && stack_parts.is_empty() {
        stack_parts.push("pra".to_string());
    }
    let effective_stack_slp = stack_parts.join(";");
    let guard = engine()?;
    let engine = guard
        .as_ref()
        .ok_or_else(|| "derive engine is unavailable".to_string())?;
    let mut dhatu = engine
        .dhatupatha
        .get(baseindex)
        .ok_or_else(|| format!("unknown dhatu {baseindex}"))?
        .clone();
    let upasarga_prefixes: Vec<&str> = if effective_stack_slp.is_empty() {
        Vec::new()
    } else {
        effective_stack_slp.split(';').collect()
    };
    dhatu = dhatu.with_prefixes(&upasarga_prefixes);
    if let Some(sanadi) = sanadi_of(family) {
        dhatu = dhatu.with_sanadi(&[sanadi]);
    }
    // The site's live path passes no per-row lakāra/prayoga (see the
    // module header for the evidence), so kṛdantas derive plain.
    let base_krt = base_krt(krt_deva).ok_or_else(|| format!("unknown krt {krt_deva}"))?;
    let args = Krdanta::builder()
        .dhatu(dhatu)
        .krt(base_krt)
        .build()
        .map_err(|e| format!("args: {e:?}"))?;
    let mut stems: Vec<String> = engine
        .vp
        .derive_krdantas(&args)
        .iter()
        .map(Prakriya::text)
        .filter(|t| !t.is_empty())
        .collect();
    // The site keeps the engine's own order and only uniques it
    // (`t.map(...).unique()`), so the rows follow the engine — sorting
    // them would swap the site's two-row cells (10.0199 तृच्).
    stems.dedup();
    let engine_family = crate::forms::engine_krut_family(family).unwrap_or("krut");
    // The स्य-/भावकर्म- display names carry a hyphen the SLP1 converter
    // rejects; they decline under the base कृत् (शानच् → SAnac).
    let krt_slp1 = crate::translit::deva_to_slp1(krt_deva)
        .or_else(|| special_tag(krt_deva).and_then(crate::translit::deva_to_slp1))
        .unwrap_or_default();
    let ctx = crate::genctx::ctx_for(slp1_dhatu, engine_category, dhatu_note);
    // The site licenses BOTH prefix surfaces where `sam` meets a
    // stop/nasal (`saMpra` + `sampra`; the corpus keeps both rows), while
    // vidyuṭ emits only its own sandhi. Re-apply the stack's surface
    // variants to each derived stem's prefix (same-length swaps only).
    if !effective_stack_slp.is_empty() && !effective_stack_slp.contains('-') {
        let surfaces = crate::engine::upasarga::stack_surfaces(&effective_stack_slp, ctx);
        let mut extra: Vec<String> = Vec::new();
        for stem in &stems {
            for surface in &surfaces {
                if surface.len() < stem.len() && stem.starts_with(surface.as_str()) {
                    let rest = &stem[surface.len()..];
                    for other in &surfaces {
                        if other != surface && other.len() == surface.len() {
                            let cand = format!("{other}{rest}");
                            if !stems.contains(&cand) && !extra.contains(&cand) {
                                extra.push(cand);
                            }
                        }
                    }
                }
            }
        }
        stems.extend(extra);
    }

    // Vidyuṭ rows: one per stem.
    let build_row = |stem: &str| -> UpRow {
        let mut row: UpRow = HashMap::new();
        row.insert("base".to_string(), vec![stem.to_string()]);
        for field in ["m", "n"] {
            row.insert(
                field.to_string(),
                display_nominal(&krt_slp1, field, stem, engine_family, baseindex),
            );
        }
        let mut fem: Vec<String> = Vec::new();
        if sy_only {
            // The site's स्य-शतृ prefixed row carries the strī alternants
            // of the varṭamāna stem (`प्रभवन्ती` and `प्रभवती` — the
            // त्री-ant alternation of the त्-final शतृ stem). The engine's
            // own stryanta yields nothing for त्-final stems, so generate
            // the two alternates as values for the refS table (`भवन्ती`,
            // `भवती`). शानच्-type rows feminize through the तका (माला
            // ref): the stem already carries ण् → its आ-nom pads.
            if stem.ends_with('t') {
                let base = stem.strip_suffix('t').unwrap_or(stem);
                for cand in [format!("{base}ntI"), format!("{base}tI")] {
                    for form in decline_nominal(&krt_slp1, "f", &cand, engine_family, baseindex) {
                        if !fem.contains(&form) {
                            fem.push(form);
                        }
                    }
                }
            } else if stem.ends_with('n') {
                // शानच्-type stems (एधमान्…): the तका strī is आ-final
                // (एधमाना), the mAlA ref pads from the आ-nom.
                for form in decline_nominal(
                    &krt_slp1,
                    "f",
                    &format!("{stem}A"),
                    engine_family,
                    baseindex,
                ) {
                    if !fem.contains(&form) {
                        fem.push(form);
                    }
                }
            } else if let Some(a_stem) = stem.strip_suffix('a') {
                // शानच्-type a-stems (एधमान…): the strī is simply आ-final,
                // and the site keeps the stem's own surface (संभावयमाना) —
                // vidyuṭ's strī derivation normalizes it to the
                // parasavarṇa twin (सम्भावयमाना).
                for form in decline_nominal(
                    &krt_slp1,
                    "f",
                    &format!("{a_stem}A"),
                    engine_family,
                    baseindex,
                ) {
                    if !fem.contains(&form) {
                        fem.push(form);
                    }
                }
            }
        }
        // The स्य-* block above owns the f field outright; vidyuṭ's own
        // stryanta would add its parasavarṇa-normalized stem as an extra
        // alternant the site does not list.
        let mut fstems = if sy_only {
            Vec::new()
        } else {
            feminized_stems(&engine.vp, stem)
        };
        // The site's prefixed strī formation can differ from vidyuṭ's own
        // stryanta: शतृ shows both the strong …antī (अवादन्ती) and the
        // weak …atī (अदती, सर्स्रती) of the त्-final stem, and a क्वसु
        // strong stem with इट् (`…ivas`) feminizes from the strong stem
        // minus `vas` plus `uṣī` — `yeti-vas` → येत्युषी, जक्षिवस् →
        // जक्ष्युषी — while the an-iṭ stems (vavand-vas) keep the
        // weak-stem …uṣī.
        if krt_slp1 == "Satf" && stem.ends_with('t') {
            fstems.clear();
            let base = &stem[..stem.len() - 1];
            fstems.push(format!("{base}ntI"));
            fstems.push(format!("{base}tI"));
        } else if krt_slp1 == "kvasu" {
            if let Some(f) = kvasu_strong_f(stem) {
                fstems.push(f);
            }
        } else if krt_slp1 == "GinuR" && stem.ends_with('n') {
            // The site's prefixed घिनुण् strī is the dental stem + ई
            // (प्रसारिनी), not vidyuṭ's ṇatva form (प्रसारिणी).
            fstems.clear();
            fstems.push(format!("{stem}I"));
        } else if matches!(
            krt_slp1.as_str(),
            "ac" | "SAnac" | "sya-SAnac" | "BAvakarma-SAnac" | "sya-BAvakarma-SAnac"
        ) && stem.ends_with('a')
        {
            fstems.clear();
            // The site's prefixed अच्/शानच् strī keeps the stem's own
            // anusvāra surface (संप्रसरा, संक्रीडमाना); vidyuṭ's stryanta
            // normalizes it to the parasavarṇa twin (सम्प्रसरा).
            fstems.push(format!("{}A", stem.strip_suffix('a').unwrap_or(stem)));
        } else if matches!(
            krt_slp1.as_str(),
            "a" | "aN" | "Takan" | "Rvuc" | "ac" | "kyap"
        ) && stem.ends_with('A')
        {
            fstems.clear();
            // आ-final अ/अच् strī: the site keeps the stem's own surface
            // (उपार्च्छा with the च् retained), not vidyuṭ's normalized
            // upārchā.
            fstems.push(stem.to_string());
        } else if matches!(krt_slp1.as_str(), "Rvul" | "vuY" | "vun") && stem.ends_with("aka") {
            fstems.clear();
            // The site's -aka कृत्स take the vṛddhi strī -ikā
            // (व्याभाषक → व्याभाषिका); vidyuṭ offers no strī here.
            fstems.push(format!("{}ikA", &stem[..stem.len() - 3]));
        } else if krt_slp1 == "vun" {
            fstems.clear();
            // The शुद्ध/यङ्लुग् वुन् strī is the bare stem itself
            // (दाक, not दाका); the -aka forms took -ikā above.
            fstems.push(stem.to_string());
        } else if krt_slp1 == "ktin" {
            fstems.clear();
            // क्तिन्'s strī is the stem itself; vidyuṭ's stryanta
            // normalizes the prefix anusvāra (संप्रत्ति → सम्प्रत्ति), the
            // site keeps the stem's own surface.
            fstems.push(stem.to_string());
        } else if stem.ends_with('a') {
            fstems.clear();
            // The general a-stem strī is the stem's own surface + आ: the
            // site keeps the prefix sandhi it was given (संप्रत्ता), while
            // vidyuṭ's stryanta normalizes it to the parasavarṇa twin.
            fstems.push(format!("{}A", stem.strip_suffix('a').unwrap_or(stem)));
        }
        for fstem in fstems {
            for form in decline_nominal(&krt_slp1, "f", &fstem, engine_family, baseindex) {
                if !fem.contains(&form) {
                    fem.push(form);
                }
            }
        }
        if fem.is_empty() {
            // क्तिन्/उ/अ-type कृत्स have no separate strī stem: the derived
            // stem IS the feminine (निषेधिष्टिः, निषेधिषुः, निषेधिषा), so
            // decline it in the कृत्'s f reference paradigm.
            if let Some(forms) = prathama_ekavacana(engine_family, &krt_slp1, "f", stem, baseindex)
            {
                fem = forms;
            }
        }
        row.insert("f".to_string(), fem);
        row
    };
    let mut rows: Vec<UpRow> = stems.iter().map(|stem| build_row(stem)).collect();
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
    // स्य-*/भावकर्म-* rows skip the whole upstream supplement: the live
    // site's prefixed derivation never re-uses the corpus cells for them
    // (their लृङ् data is invisible there), and rows the engine cannot
    // derive are simply absent from the site's view.
    if !sy_only {
        // `ava` + short `a`: the live site derives with the root in place
        // (ava + √i + ana → avayana, avayām babhūva), not by prefixing the
        // already-derived base cell (which would give the dīrgha avāyana).
        // When a vidyut stem exists it is the site's surface; otherwise the
        // base is prefixed with the site's pararūpa (av + ayana).
        let last_is_ava = effective_stack_slp
            .rsplit(';')
            .next()
            .is_some_and(|t| t == "ava");
        let pararupa_stack = if last_is_ava {
            let mut parts: Vec<&str> = effective_stack_slp.split(';').collect();
            if let Some(last) = parts.last_mut() {
                *last = "av";
            }
            parts.join(";")
        } else {
            String::new()
        };
        let vidyut_real = rows.iter().any(|r| {
            r.get("base")
                .is_some_and(|b| b.iter().any(|s| !s.is_empty()))
        });
        for (base, m, f, n) in upstream {
            let base_alts = split_alts(base);
            let ava_a = last_is_ava
                && !pararupa_stack.is_empty()
                && !base_alts.is_empty()
                && base_alts.iter().all(|a| a.starts_with('a'));
            let chosen_stack = if ava_a {
                let normal_match = base_alts.iter().any(|alt| {
                    generate_prefixed(&effective_stack_slp, alt, ctx)
                        .iter()
                        .any(|v| surface_to_rows.contains_key(v))
                });
                if normal_match {
                    effective_stack_slp.clone()
                } else if vidyut_real {
                    // The site's live derivation already supplied the row
                    // (avayana); the dīrgha surface is a prefixing artifact.
                    continue;
                } else {
                    pararupa_stack.clone()
                }
            } else {
                effective_stack_slp.clone()
            };
            // Base variants in generation order; their count sets the group
            // count for this upstream row's fields.
            let mut base_groups: Vec<Vec<String>> = Vec::new();
            let mut n_groups = 0_usize;
            for alt in base_alts {
                // ल्यप् ships pre-prefixed (baked stack: `प्रभूय`, fused `प्रेड्य`);
                // strip the baked code before applying the selected stack, else
                // the supplement row double-prefixes and never matches vidyuṭ.
                let alt = if krt_deva == "ल्यप्" {
                    crate::prefix_krut::strip_baked_stack(&alt).to_string()
                } else {
                    alt
                };
                let variants = generate_prefixed(&chosen_stack, &alt, ctx);
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
                    let variants = generate_prefixed(&chosen_stack, &alt, ctx);
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
                    // A row without a base cannot be displayed or matched;
                    // empty-base rows come from alternating supplement groups
                    // whose other variant matched the vidyut row already.
                    if bases.is_empty() {
                        continue;
                    }
                    row.insert("base".to_string(), bases);
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
                    // f: union (upstream pre-feminized cells cover the
                    // engine's at-stem/ī-stem gaps, but the site's live
                    // derivation can carry a different f class — e.g.
                    // अच् सरा vs the base cell's सरी — so never drop the
                    // engine's own form).
                    if !f_groups[group].is_empty() {
                        let mut merged = row.get("f").cloned().unwrap_or_default();
                        for form in &f_groups[group] {
                            if !merged.contains(form) {
                                merged.push(form.clone());
                            }
                        }
                        row.insert("f".to_string(), merged);
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
    } // !sy_only (supplement skipped for स्य-/भावकर्म- rows)
      // The upstream supplement replaces `f` wholesale, dropping the
      // site-only alternants build_row added for शतृ/क्वसु. Re-apply them
      // to every row's own base stem.
    if !sy_only {
        for row in &mut rows {
            let bases = row.get("base").cloned().unwrap_or_default();
            for base in &bases {
                let mut cands: Vec<String> = Vec::new();
                if krt_slp1 == "Satf" && base.ends_with('t') {
                    let root = &base[..base.len() - 1];
                    cands.push(format!("{root}ntI"));
                    cands.push(format!("{root}tI"));
                } else if krt_slp1 == "kvasu" {
                    if let Some(f) = kvasu_strong_f(base) {
                        cands.push(f);
                    }
                } else if krt_slp1 == "GinuR" && base.ends_with('n') {
                    cands.push(format!("{base}I"));
                } else if krt_slp1 == "ktavatu" && base.ends_with('t') {
                    // क्तवतु strī from the row's own surface (उपार्छितवती,
                    // उपार्च्छितवती), not vidyuṭ's normalized stem.
                    cands.push(format!("{base}I"));
                }
                for cand in cands {
                    for form in decline_nominal(&krt_slp1, "f", &cand, engine_family, baseindex) {
                        let f = row.entry("f".to_string()).or_default();
                        if !f.contains(&form) {
                            f.push(form);
                        }
                    }
                }
            }
        }
    }
    // Natva-twin rows (दुर्निमित… / दुर्णिमित…) are separate rows in the
    // corpus, each carrying its own f alternant. Copy an f form across a
    // pair of rows whose bases differ only in n/Ṇ, mapping the same
    // positions in the f form.
    if !sy_only {
        let bases: Vec<String> = rows
            .iter()
            .map(|row| {
                row.get("base")
                    .and_then(|b| b.first())
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        let mut additions: Vec<(usize, String)> = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let Some(forms) = row.get("f") else {
                continue;
            };
            let base_i = bases[i].as_bytes();
            for (j, base_j) in bases.iter().enumerate() {
                if i == j || base_j.is_empty() || base_i.len() != base_j.len() {
                    continue;
                }
                let bj = base_j.as_bytes();
                let diffs: Vec<usize> = base_i
                    .iter()
                    .zip(bj)
                    .enumerate()
                    .filter(|(_, (a, b))| a != b)
                    .map(|(k, _)| k)
                    .collect();
                if diffs.is_empty()
                    || !diffs
                        .iter()
                        .all(|&k| matches!((base_i[k], bj[k]), (b'n', b'R') | (b'R', b'n')))
                {
                    continue;
                }
                for form in forms {
                    let bytes = form.as_bytes();
                    if diffs
                        .iter()
                        .any(|&k| k >= bytes.len() || !matches!(bytes[k], b'n' | b'R'))
                    {
                        continue;
                    }
                    let mut mapped = bytes.to_vec();
                    for &k in &diffs {
                        mapped[k] = bj[k];
                    }
                    let Ok(mapped) = String::from_utf8(mapped) else {
                        continue;
                    };
                    if !rows[j]
                        .get("f")
                        .is_some_and(|existing| existing.contains(&mapped))
                    {
                        additions.push((j, mapped));
                    }
                }
            }
        }
        for (j, form) in additions {
            rows[j].entry("f".to_string()).or_default().push(form);
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
    // The site's prefixed view derives live: a linga the कृत्'s reference
    // table declines is shown even when the base cell leaves that field
    // empty (01.1001 नङ् prefixed pu संयाच्ञाः, base cell याच्ञा,,,).
    for (i, field) in ["m", "f", "n"].iter().enumerate() {
        if !filled[i]
            && crate::engine::subanta::krt_has_slot(engine_family, &krt_slp1, field, baseindex)
        {
            filled[i] = true;
        }
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
    // स्य-/भावकर्म- rows are exempt: their सेध/सिध् grade variants are both
    // real rows in the corpus (निषिषिधिषत् AND निषिषेधिषत्), and the strī
    // alternants (…ती / …न्ती) twin across those rows too.
    for (i, field) in ["base", "m", "f", "n"].iter().enumerate() {
        let mut all: Vec<String> = Vec::new();
        for row in &rows {
            for form in row.get(*field).cloned().unwrap_or_default() {
                if !all.contains(&form) {
                    all.push(form);
                }
            }
        }
        let kept = if sy_only || *field == "f" {
            // Strī cells carry real formation alternants (वुन् …क AND
            // …का, घिनुण् dental प्रसारिनी), not a weak/strong pair.
            all.clone()
        } else {
            crate::prefix_krut::suppress_twins(&all)
        };
        // Never drop a form the upstream cell itself lists: the corpus keeps
        // both vowel grades where the dhātu really has them (01.0049 सन्
        // ण्वुल् `निषिषिधिषक` AND `निषिषेधिषक`). Only engine-only twins are
        // suppressed.
        let mut upstream_forms: Vec<String> = Vec::new();
        for (base, m, f, n) in upstream {
            let text = match i {
                0 => base,
                1 => m,
                2 => f,
                _ => n,
            };
            for form in split_alts(text) {
                for surface in generate_prefixed(&effective_stack_slp, &form, ctx) {
                    if !upstream_forms.contains(&surface) {
                        upstream_forms.push(surface);
                    }
                }
            }
        }
        for row in &mut rows {
            if let Some(forms) = row.get_mut(*field) {
                forms.retain(|form| kept.contains(form) || upstream_forms.contains(form));
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
    // The site's `aBi-AN` typo-stack keeps the hyphen only where the
    // displayed word is the hyphenated stem itself or a direct declension
    // of it: the m/n of a-final stems and the strī of क्तिन्/वुन्/अङ्.
    // Derived strī stems (दत्ता, दात्री, दायिका) and the transforming
    // m/n stems (दातृ, ददिवस्, दत्तवत्, ददत्) are rebuilt without it.
    let hyphen_stack = stack_deva.contains('-');
    for key in &order {
        let row = &merged[key];
        let a_final = key.ends_with('a') || key.ends_with('A');
        // कृत्स whose strī is the stem itself keep the hyphenated stem;
        // derived feminines (दत्ता, दात्री, दायिका, दत्तवती) rebuild it.
        // अच् is stem-final only in यङ्लुग् (दादा); the सन्/णिच्/यङ्
        // feminines lengthen the final vowel and lose the hyphen.
        let hyphen_f_kept = |form: &str| match krt_slp1.as_str() {
            "ktin" | "aN" | "u" | "a" => true,
            "vun" => !key.ends_with("aka"),
            "ac" | "kyap" => form.starts_with(key.as_str()),
            _ => false,
        };
        let m_kept = a_final || krt_slp1 == "u";
        let mut deva_row = HashMap::new();
        for field in ["base", "m", "f", "n"] {
            // The corpus's prefixed rows keep both the dental and the
            // retroflex rows (दुर्निमात्री AND दुर्णिमात्री), so no
            // na-twin suppression here.
            let mut forms: Vec<String> = row.get(field).cloned().unwrap_or_default();
            if hyphen_stack {
                match field {
                    // Derived strī stems drop the hyphen; क्तिन्/वुन्/अङ्
                    // keep the hyphenated base itself.
                    "f" => {
                        for form in &mut forms {
                            if !hyphen_f_kept(form) {
                                form.retain(|c| c != '-');
                            }
                        }
                    }
                    // Non-a-final m stems transform (दातृ → दाता), so the
                    // site's label is the hyphen-less surface.
                    "m" if !m_kept => {
                        for form in &mut forms {
                            form.retain(|c| c != '-');
                        }
                    }
                    // Non-a-final n rows carry BOTH the transformed form
                    // and the stem-final form (दत्तवत्/दत्तवद्).
                    "n" if !m_kept => {
                        let stripped: Vec<String> = forms
                            .iter()
                            .map(|form| form.replace('-', ""))
                            .filter(|form| !forms.contains(form))
                            .collect();
                        forms.extend(stripped);
                    }
                    _ => {}
                }
            }
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
