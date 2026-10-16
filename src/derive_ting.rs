//! Engine-derived prefixed tiṅanta tables (Vidyuṭ).
//!
//! Upstream ships only the cells it ships: for 1127 of 2229 dhātus the
//! kartari tables carry no आत्मनेपदम् (`a*`) keys at all (e.g. जि 01.1096,
//! परस्मैपदी), yet with an upasarga the verb may take ātmanepada
//! (विपराभ्यां जे 1.3.19: वि + जि → विजयते). Cell-prefixing cannot produce
//! what has no base cell — so prefixed tiṅanta views derive the full table
//! from the engine instead, exactly as ashtadhyayi.com does live.
//!
//! Each pada is derived explicitly (परस्मैपदम् → `p*` keys, आत्मनेपदम् →
//! `a*` keys); a lakāra with no paradigm for a pada simply has no key —
//! which is precisely the site's behavior (वि + जि shows only `a*`
//! sections). Output reuses the portal's blob cell format so rendering
//! needs no new code path.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use vidyut_prakriya::args::{Dhatu, DhatuPada, Lakara, Prayoga, Purusha, Sanadi, Tinanta, Vacana};
use vidyut_prakriya::{Dhatupatha, Prakriya, Vyakarana};

/// Pinned data this module derives against (same pin as the shipped tables).
const DHATUPATHA_TSV: &str = include_str!("../data/vidyut_dhatupatha_5.tsv");

/// Portal lakāra keys in traditional order.
const LAKARAS: [(&str, Lakara); 10] = [
    ("lat", Lakara::Lat),
    ("lit", Lakara::Lit),
    ("lut", Lakara::Lut),
    ("lrut", Lakara::Lrt),
    ("lot", Lakara::Lot),
    ("lang", Lakara::Lan),
    ("vidhiling", Lakara::VidhiLin),
    ("ashirling", Lakara::AshirLin),
    ("lung", Lakara::Lun),
    ("lrung", Lakara::Lrn),
];

const PURUSHAS: [Purusha; 3] = [Purusha::Prathama, Purusha::Madhyama, Purusha::Uttama];
const VACANAS: [Vacana; 3] = [Vacana::Eka, Vacana::Dvi, Vacana::Bahu];

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

/// Derive the full prefixed tiṅanta table for one
/// (upasarga-stack, dhātu, family, prayoga): `{lakara_key: [slots]}` with
/// SLP1 surfaces per puruṣa×vacana slot. See [`derive_prefixed_ting`] for
/// the key convention.
pub fn derive_prefixed_ting_slp1(
    stack_deva: &str,
    baseindex: &str,
    family: &str,
    prayoga: &str,
) -> Result<HashMap<String, Vec<Vec<String>>>, String> {
    let stack_slp = crate::translit::deva_stack_to_slp1(stack_deva)
        .ok_or_else(|| "stack is not Devanagari upasargas".to_string())?;
    let guard = engine()?;
    let engine = guard
        .as_ref()
        .ok_or_else(|| "derive engine is unavailable".to_string())?;
    let mut dhatu: Dhatu = engine
        .dhatupatha
        .get(baseindex)
        .ok_or_else(|| format!("unknown dhatu {baseindex}"))?
        .clone();
    let prefixes: Vec<&str> = stack_slp.split(';').collect();
    dhatu = dhatu.with_prefixes(&prefixes);
    if let Some(sanadi) = sanadi_of(family) {
        dhatu = dhatu.with_sanadi(&[sanadi]);
    }
    let prayoga_arg = match prayoga {
        "karmani" => Prayoga::Karmani,
        _ => Prayoga::Kartari,
    };
    let mut tables: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for (key, lakara) in &LAKARAS {
        for (prefix, pada) in [("p", DhatuPada::Parasmai), ("a", DhatuPada::Atmane)] {
            let mut rows: Vec<Vec<String>> = Vec::with_capacity(9);
            let mut complete = true;
            for purusha in &PURUSHAS {
                for vacana in &VACANAS {
                    let args = Tinanta::builder()
                        .dhatu(dhatu.clone())
                        .lakara(*lakara)
                        .prayoga(prayoga_arg)
                        .purusha(*purusha)
                        .vacana(*vacana)
                        .pada(pada)
                        .build()
                        .map_err(|e| format!("args: {e:?}"))?;
                    let mut forms: Vec<String> = engine
                        .vp
                        .derive_tinantas(&args)
                        .iter()
                        .map(Prakriya::text)
                        .filter(|t| !t.is_empty())
                        .collect();
                    forms.sort();
                    forms.dedup();
                    if forms.is_empty() {
                        // No paradigm for this (lakāra, pada): no key, like
                        // the site (वि + जि has no `p*` sections at all).
                        complete = false;
                        break;
                    }
                    rows.push(forms);
                }
                if !complete {
                    break;
                }
            }
            if complete {
                tables.insert(format!("{prefix}{key}"), rows);
            }
        }
    }
    Ok(tables)
}

/// Devanagari version of [`derive_prefixed_ting_slp1`]: `{lakara_key:
/// "row;row;…"}` in blob cell format. SLP1 never leaves the module.
pub fn derive_prefixed_ting(
    stack_deva: &str,
    baseindex: &str,
    family: &str,
    prayoga: &str,
) -> Result<HashMap<String, String>, String> {
    let slp1 = derive_prefixed_ting_slp1(stack_deva, baseindex, family, prayoga)?;
    let mut out = HashMap::with_capacity(slp1.len());
    for (key, slots) in slp1 {
        let cells: Vec<String> = slots
            .iter()
            .map(|slot| {
                slot.iter()
                    .filter_map(|s| crate::translit::slp1_to_deva(s))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        out.insert(key, cells.join(";"));
    }
    Ok(out)
}
