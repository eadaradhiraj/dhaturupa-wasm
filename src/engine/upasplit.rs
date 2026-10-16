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
    out.sort();
    out
}
