//! Rule-based upasarga prefix-split search (F3 search side, M2/M4 UI).
//!
//! Candidate generation is purely mechanical: [`build_surface_map`]
//! enumerates stack-surface realizations over a structural probe grid, and
//! [`split_candidates`] inverts exact surface-prefix matches back into
//! `(stack, base)` pairs. Soundness always comes from exact forward
//! regeneration with the true [`GenCtx`] (the caller verifies with
//! [`generate_prefixed`]); this module never consults oracle strings, only
//! the stack list and the engine.
//!
//! Documented coverage gaps (also logged in `DECISIONS.md`): stacks whose
//! surface depends on dhatu-lexical flags beyond `derived`/`z_initial`
//! (`nitya_adi` drops are found via the bare-base lookup instead;
//! `stambhi`/`sut_root` base transforms have no inverse here), and bases
//! whose third+ character changes a boundary verdict (probes cover 1–3
//! chars; measured recall in `tests/t7_upasarga_split.rs`).

use crate::engine::upasarga::{generate_prefixed, GenCtx};
use crate::slp1::split_prefix_stack;
use std::collections::{HashMap, HashSet};

/// SLP1 single-byte initials worth probing (ASCII letters).
fn initials() -> Vec<u8> {
    let mut v: Vec<u8> = (b'a'..=b'z').chain(b'A'..=b'Z').collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Second characters that can change a boundary verdict: vowels (guna/
/// vrddhi/yan/ayadi targets), `T` (`sT` → `zw`), `y`/`v` (ayadi +
/// natva-internal blocks), `n`/`r` (natva next-next blocks), `m`/`H`/`M`
/// (anusvara contexts), `s`/`d` (stutva/satva codas).
fn seconds() -> Vec<u8> {
    let mut v: Vec<u8> = b"aAiIuUeEoOTyvrnmHMsd".to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

/// Realize every surface for `stack` + `probe` under `ctx`.
fn realize(stack: &str, probe: &str, ctx: GenCtx) -> Vec<String> {
    if stack.is_empty() || probe.is_empty() {
        return Vec::new();
    }
    generate_prefixed(stack, probe, ctx)
}

/// One surface-map value: which stack and probe spelled this head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceEntry {
    pub stack: String,
    pub probe: String,
}

/// Build `surface_head -> [(stack, probe)]` over all stacks.
///
/// Probes: 1-char initials, 2-char (initial × seconds), and 3-char
/// `n`-initial + vowel + `{y, v}` (natva next-next blocks). Context grid:
/// `derived` × `z_initial` (the only flags that reshape prefix-region
/// surfaces for arbitrary bases).
#[must_use]
pub fn build_surface_map(stacks: &[String]) -> HashMap<String, Vec<SurfaceEntry>> {
    let inits = initials();
    let secs = seconds();
    let mut probes: Vec<String> = Vec::with_capacity(1600);
    for c in &inits {
        probes.push((*c as char).to_string());
    }
    for c in &inits {
        for d in &secs {
            probes.push(format!("{}{}", *c as char, *d as char));
        }
    }
    for v in b"aAiIuUeEoO" {
        for w in *b"yv" {
            probes.push(format!("n{}{}", *v as char, w as char));
        }
    }
    let mut map: HashMap<String, Vec<SurfaceEntry>> = HashMap::new();
    for stack in stacks {
        if split_prefix_stack(stack).is_empty() {
            continue;
        }
        for derived in [false, true] {
            for z_gate in [false, true] {
                let ctx = GenCtx {
                    derived,
                    z_initial: z_gate,
                    ..GenCtx::default()
                };
                for probe in &probes {
                    for surface in realize(stack, probe, ctx) {
                        map.entry(surface).or_default().push(SurfaceEntry {
                            stack: stack.clone(),
                            probe: probe.clone(),
                        });
                    }
                }
            }
        }
    }
    for entries in map.values_mut() {
        entries.sort_by(|a, b| a.stack.cmp(&b.stack).then(a.probe.cmp(&b.probe)));
        entries.dedup();
    }
    map
}

/// Split `form` into `(stack, base)` candidates via the surface map.
///
/// The query is tried as-is plus with each single `R` restored to dental
/// `n` (natva-inverse: surfaces like `nirBavARi` split via dental
/// `nirBavAni`; verification below re-voices exactly). For every split
/// point whose head is a known realization (from `probe`), proposes
/// `probe[0] + rest` (fusion consumed the initial) and `probe + rest`
/// (surface preserved it). `is_base` prunes to attested base forms. The
/// caller MUST verify each candidate by exact forward regeneration with
/// the true per-dhatu [`GenCtx`].
#[must_use]
pub fn split_candidates<S: ::std::hash::BuildHasher>(
    form: &str,
    map: &HashMap<String, Vec<SurfaceEntry>, S>,
    is_base: &dyn Fn(&str) -> bool,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut variants: Vec<String> = vec![form.to_string()];
    for (i, b) in form.bytes().enumerate() {
        if b == b'R' {
            let mut v = form.as_bytes().to_vec();
            v[i] = b'n';
            if let Ok(s) = String::from_utf8(v) {
                variants.push(s);
            }
        }
    }
    for query in &variants {
        for k in 1..=query.len() {
            if !query.is_char_boundary(k) {
                continue;
            }
            let Some(entries) = map.get(&query[..k]) else {
                continue;
            };
            let rest = &query[k..];
            for entry in entries {
                let mut first = entry.probe.chars();
                let init = first.next().unwrap_or('\0');
                for base in [format!("{init}{rest}"), format!("{}{rest}", entry.probe)] {
                    if base.is_empty() || !is_base(&base) {
                        continue;
                    }
                    let key = (entry.stack.clone(), base);
                    if seen.insert(key.clone()) {
                        out.push(key);
                    }
                }
            }
        }
    }
    out.sort_by(|(s_a, b_a), (s_b, b_b)| {
        let is_direct_a = format!("{s_a}{b_a}") == form;
        let is_direct_b = format!("{s_b}{b_b}") == form;
        let rank_a = match (is_direct_a, form.starts_with(s_a.as_str())) {
            (true, _) => 0,
            (false, true) => 1,
            _ => 2,
        };
        let rank_b = match (is_direct_b, form.starts_with(s_b.as_str())) {
            (true, _) => 0,
            (false, true) => 1,
            _ => 2,
        };
        rank_a
            .cmp(&rank_b)
            .then_with(|| s_a.cmp(s_b))
            .then_with(|| b_a.cmp(b_b))
    });
    out
}

/// Propose plausible opposite-pada counterparts for a verbal surface base in SLP1.
///
/// Under certain upasargas, verbal roots switch pada (e.g. 1.3.83 vyāṅparibhyo ramaḥ:
/// ram becomes parasmaipadī with vi/ā/pari, 1.3.17 ner viśaḥ: viś becomes ātmanepadī with ni).
/// The shard only stores the un-prefixed citation forms of the root; proposing
/// these counterparts allows the search splitter to locate the root's shard bucket.
#[must_use]
pub fn pada_counterparts(slp: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |stem: &str, endings: &[&str]| {
        if !stem.is_empty() {
            for end in endings {
                out.push(format!("{stem}{end}"));
            }
        }
    };

    // 1. lat / lrut (present / future)
    if let Some(stem) = slp.strip_suffix("anti") {
        add(stem, &["ante", "ate"]);
    } else if let Some(stem) = slp.strip_suffix("ati") {
        add(stem, &["ate", "ante"]);
    } else if let Some(stem) = slp.strip_suffix("ataH") {
        add(stem, &["ete", "Ate"]);
    } else if let Some(stem) = slp.strip_suffix("asi") {
        add(stem, &["ase", "se"]);
    } else if let Some(stem) = slp.strip_suffix("aTaH") {
        add(stem, &["eTe", "ATe"]);
    } else if let Some(stem) = slp.strip_suffix("aTa") {
        add(stem, &["aDve", "Dve"]);
    } else if let Some(stem) = slp.strip_suffix("Ami") {
        add(stem, &["e"]);
    } else if let Some(stem) = slp.strip_suffix("AvaH") {
        add(stem, &["Avahe"]);
    } else if let Some(stem) = slp.strip_suffix("AmaH") {
        add(stem, &["Amahe"]);
    } else if let Some(stem) = slp.strip_suffix("ti") {
        add(stem, &["te"]);
    } else if let Some(stem) = slp.strip_suffix("si") {
        add(stem, &["se"]);
    } else if let Some(stem) = slp.strip_suffix("mi") {
        add(stem, &["e"]);
    } else if let Some(stem) = slp.strip_suffix("ante") {
        add(stem, &["anti"]);
    } else if let Some(stem) = slp.strip_suffix("ate") {
        add(stem, &["ati", "anti"]);
    } else if let Some(stem) = slp.strip_suffix("ete") {
        add(stem, &["ataH"]);
    } else if let Some(stem) = slp.strip_suffix("Ate") {
        add(stem, &["ataH"]);
    } else if let Some(stem) = slp.strip_suffix("ase") {
        add(stem, &["asi"]);
    } else if let Some(stem) = slp.strip_suffix("se") {
        add(stem, &["si", "asi"]);
    } else if let Some(stem) = slp.strip_suffix("eTe") {
        add(stem, &["aTaH"]);
    } else if let Some(stem) = slp.strip_suffix("ATe") {
        add(stem, &["aTaH"]);
    } else if let Some(stem) = slp.strip_suffix("aDve") {
        add(stem, &["aTa"]);
    } else if let Some(stem) = slp.strip_suffix("Dve") {
        add(stem, &["aTa"]);
    } else if let Some(stem) = slp.strip_suffix("Avahe") {
        add(stem, &["AvaH"]);
    } else if let Some(stem) = slp.strip_suffix("Amahe") {
        add(stem, &["AmaH"]);
    } else if let Some(stem) = slp.strip_suffix("te") {
        add(stem, &["ti"]);
    } else if let Some(stem) = slp.strip_suffix("e") {
        add(stem, &["Ami", "mi"]);
    }

    // 2. lot (imperative)
    if let Some(stem) = slp.strip_suffix("tu") {
        add(stem, &["tAm"]);
    } else if let Some(stem) = slp.strip_suffix("ntu") {
        add(stem, &["ntAm", "atAm"]);
    } else if let Some(stem) = slp.strip_suffix("Ani") {
        add(stem, &["E"]);
    } else if let Some(stem) = slp.strip_suffix("ntAm") {
        add(stem, &["ntu"]);
    } else if let Some(stem) = slp.strip_suffix("sva") {
        add(stem, &["hi", "a"]);
    }

    // 3. lang (imperfect)
    if let Some(stem) = slp.strip_suffix("an") {
        add(stem, &["anta", "ata"]);
    } else if let Some(stem) = slp.strip_suffix("anta") {
        add(stem, &["an"]);
    }

    // 4. vidhiling (potential)
    if let Some(stem) = slp.strip_suffix("et") {
        add(stem, &["eta"]);
    } else if let Some(stem) = slp.strip_suffix("eyuH") {
        add(stem, &["eran"]);
    } else if let Some(stem) = slp.strip_suffix("eta") {
        add(stem, &["et"]);
    } else if let Some(stem) = slp.strip_suffix("eran") {
        add(stem, &["eyuH"]);
    }

    out
}
