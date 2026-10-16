//! Tinanta/krdanta form tables from `dhatu/dhatuforms_*.txt` (display parity).
//!
//! These are the site's precomputed Devanagari forms — NOT prakriya prose.
//! - kartari files: `{baseindex: {lakara_key: "f1;f2;…;f9"}}` (9-slot, `;`-joined;
//!   a slot may hold `,`-variants, e.g. `भवतात्,भवताद्,भवतु`).
//! - krut files: `{baseindex: {krt: "base,m,f,n"}}` (`, `-separated; empty =
//!   avyaya/uninflected, e.g. `"भवन,,,भवनम्"` or `"भूत्वा,,,"`).
//!
//! SLP1 morphology (search/generation) lives in `slp1`/`grammar`;
//! full `subanta` 8×3 generation is the next layer (see README).

use std::collections::HashMap;

/// `{"01.0001": {"plat": "…", …}, …}` — values are raw joined strings.
pub type TingMap = HashMap<String, HashMap<String, String>>;
/// `{"01.0001": {"ल्युट्": "base,m,f,n", …}, …}`.
pub type KrutMap = HashMap<String, HashMap<String, String>>;

/// Ting families: one `(family, prayoga)` file each, except `nich_kartari`
/// which carries both `p*` and `a*` lakara keys (20 keys, causative
/// parasmaipada + atmanepada).
pub const TING_FAMILIES: [&str; 5] = ["shuddha", "nich", "san", "yang", "yangluk"];
/// Prayogas per ting family (`kartari` ≈ `p*` keys, `karmani` ≈ `a*` keys).
pub const TING_PRAYOGAS: [&str; 2] = ["kartari", "karmani"];
/// Krut families: one file each. `shuddha` lists the full ~63-krt set,
/// derived families list their subset (23–26 krts).
pub const KRUT_FAMILIES: [&str; 5] = ["shuddha", "nich", "san", "yang", "yangluk"];

/// Canonical ting path for `(family, prayoga)`, e.g. `shuddha/kartari`.
/// Returns `None` for unknown names.
#[must_use]
pub fn ting_filename(family: &str, prayoga: &str) -> Option<String> {
    if TING_FAMILIES.contains(&family) && TING_PRAYOGAS.contains(&prayoga) {
        Some(format!("data/ting_{family}_{prayoga}.json"))
    } else {
        None
    }
}

/// Canonical krut path for `family`. Returns `None` for unknown names.
#[must_use]
pub fn krut_filename(family: &str) -> Option<String> {
    if KRUT_FAMILIES.contains(&family) {
        Some(format!("data/krut_{family}.json"))
    } else {
        None
    }
}

/// Engine `conjugations` category for a dhatuforms ting family:
/// `shuddha -> ting`, the rest are identity (`nich -> nich`, …).
/// Prayoga (`kartari`/`karmani`) selects the `p*`/`a*` lakara-key subset —
/// except `nich_kartari`, which ships both.
#[must_use]
pub fn engine_ting_category(family: &str) -> Option<&'static str> {
    match family {
        "shuddha" => Some("ting"),
        "nich" => Some("nich"),
        "san" => Some("san"),
        "yang" => Some("yang"),
        "yangluk" => Some("yangluk"),
        _ => None,
    }
}

/// Engine `participles` family for a dhatuforms krut family
/// (matches `engine::krt::KRT_CATEGORIES`).
#[must_use]
pub fn engine_krut_family(family: &str) -> Option<&'static str> {
    match family {
        "shuddha" => Some("krut"),
        "nich" => Some("nich_krut"),
        "san" => Some("san_krut"),
        "yang" => Some("yang_krut"),
        "yangluk" => Some("yangluk_krut"),
        _ => None,
    }
}

/// Accept dhatuforms names (`shuddha`, `nich`, …) or engine names
/// (`krut`, `nich_krut`, …) for the `krttable` family argument.
#[must_use]
pub fn normalize_krut_engine_family(arg: &str) -> Option<&'static str> {
    match arg {
        "krut" | "shuddha" => Some("krut"),
        "nich_krut" | "nich" => Some("nich_krut"),
        "san_krut" | "san" => Some("san_krut"),
        "yang_krut" | "yang" => Some("yang_krut"),
        "yangluk_krut" | "yangluk" => Some("yangluk_krut"),
        _ => None,
    }
}

/// Parse a `dhatuforms_*.txt` payload into a nested map.
pub fn parse_forms_map(text: &str) -> Result<TingMap, serde_json::Error> {
    serde_json::from_str(text)
}

/// Split a ting slot-string on `;` (9 slots expected, variants kept intact).
#[must_use]
pub fn split_slots(joined: &str) -> Vec<&str> {
    if joined.is_empty() {
        return Vec::new();
    }
    joined.split(';').collect()
}

/// Split one slot's `,`-variants: `भवतात्,भवताद्,भवतु` -> 3.
#[must_use]
pub fn split_variants(slot: &str) -> Vec<&str> {
    if slot.is_empty() {
        return Vec::new();
    }
    slot.split(',').collect()
}

/// A parsed krdanta cell: base + citation m/f/n (each possibly empty,
/// each possibly holding `/`-alternants like `भवत्/भवद्`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrtCell {
    /// Raw cell, e.g. `"भावक,भावकः,भाविका,भावकम्"`.
    pub raw: String,
    /// `base` — always present positionally (may be empty string).
    pub base: String,
    /// Masculine citation (prathama eka), `""` when indeklinable/empty.
    pub m: String,
    /// Feminine citation.
    pub f: String,
    /// Neuter citation.
    pub n: String,
}

/// Parse `"base,m,f,n"` (trailing empties allowed: `"भूत्वा,,,"`).
#[must_use]
pub fn parse_krt_cell(raw: &str) -> KrtCell {
    parse_krt_rows(raw).into_iter().next().unwrap_or(KrtCell {
        raw: raw.to_string(),
        base: String::new(),
        m: String::new(),
        f: String::new(),
        n: String::new(),
    })
}

/// Split a krt cell on `;` into variant rows.
///
/// Most cells hold one row (`base,m,f,n`); some hold several (e.g. `क्वसु`
/// with three periphrastic rows, `क्तिन्` with spelling doublets).
/// Empty rows are dropped.
#[must_use]
pub fn split_krt_rows(cell: &str) -> Vec<&str> {
    if cell.is_empty() {
        return Vec::new();
    }
    cell.split(';').filter(|row| !row.is_empty()).collect()
}

/// Parse every `;`-row of a krt cell.
#[must_use]
pub fn parse_krt_rows(raw: &str) -> Vec<KrtCell> {
    split_krt_rows(raw)
        .into_iter()
        .map(|row| {
            let mut parts = row.split(',');
            let get = |o: Option<&str>| o.unwrap_or("").to_string();
            let (base, m, f, n) = (
                get(parts.next()),
                get(parts.next()),
                get(parts.next()),
                get(parts.next()),
            );
            KrtCell {
                raw: row.to_string(),
                base,
                m,
                f,
                n,
            }
        })
        .collect()
}

impl KrtCell {
    /// True when the krt is avyaya (no m/f/n citations).
    #[must_use]
    pub fn is_avyaya(&self) -> bool {
        self.m.is_empty() && self.f.is_empty() && self.n.is_empty()
    }

    /// Declinable citation forms, non-empty subset of `[m, f, n]`.
    #[must_use]
    pub fn declinable(&self) -> Vec<(&'static str, &str)> {
        let mut out = Vec::new();
        if !self.m.is_empty() {
            out.push(("m", self.m.as_str()));
        }
        if !self.f.is_empty() {
            out.push(("f", self.f.as_str()));
        }
        if !self.n.is_empty() {
            out.push(("n", self.n.as_str()));
        }
        out
    }
}

/// Render a ting table (text form): one line per lakara.
#[must_use]
pub fn format_ting<S: std::hash::BuildHasher>(entry: &HashMap<String, String, S>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut keys: Vec<&String> = entry.keys().collect();
    keys.sort();
    for k in keys {
        let slots = split_slots(&entry[k]);
        let _ = writeln!(out, "{k} ({}): {}", slots.len(), entry[k]);
    }
    out
}

/// Render a krut table (text form): one line per krt (per row when several).
#[must_use]
pub fn format_krut<S: std::hash::BuildHasher>(entry: &HashMap<String, String, S>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let mut keys: Vec<&String> = entry.keys().collect();
    keys.sort();
    for k in keys {
        let rows = parse_krt_rows(&entry[k]);
        for (row, c) in rows.iter().enumerate() {
            let label = if rows.len() > 1 {
                format!("{k}[{row}]")
            } else {
                (*k).clone()
            };
            if c.is_avyaya() {
                let _ = writeln!(out, "{label}: {} (avyaya)", c.base);
            } else {
                let _ = writeln!(
                    out,
                    "{label}: base={} m={} f={} n={}",
                    c.base, c.m, c.f, c.n
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_ting_slots() {
        let s = "भवति;भवतः;भवन्ति";
        assert_eq!(split_slots(s).len(), 3);
        assert_eq!(split_variants("भवतात्,भवताद्,भवतु").len(), 3);
    }

    #[test]
    fn parses_krut_cells() {
        let c = parse_krt_cell("भावक,भावकः,भाविका,भावकम्");
        assert_eq!(c.base, "भावक");
        assert!(!c.is_avyaya());
        assert_eq!(c.declinable().len(), 3);
        let a = parse_krt_cell("भूत्वा,,,");
        assert!(a.is_avyaya());
        let l = parse_krt_cell("भवन,,,भवनम्");
        assert_eq!(l.declinable(), vec![("n", "भवनम्")]);
    }

    #[test]
    fn splits_krut_rows() {
        let rows = parse_krt_rows("स्पर्धि,,स्पर्धिः,;स्पर्द्धि,,स्पर्द्धिः,");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].base, "स्पर्धि");
        assert_eq!(rows[1].f, "स्पर्द्धिः");
        assert!(split_krt_rows("").is_empty());
        // Single-row cells keep working through the legacy entry point.
        assert_eq!(parse_krt_cell("अ,ब,,").m, "ब");
    }

    #[test]
    fn parses_map_shape() {
        let m: TingMap = parse_forms_map(r#"{"01.0001":{"plat":"a;b"}}"#).expect("parses");
        assert_eq!(m["01.0001"]["plat"], "a;b");
    }

    #[test]
    fn resolves_family_paths() {
        assert_eq!(
            ting_filename("shuddha", "kartari").as_deref(),
            Some("data/ting_shuddha_kartari.json")
        );
        assert_eq!(
            ting_filename("nich", "karmani").as_deref(),
            Some("data/ting_nich_karmani.json")
        );
        assert_eq!(krut_filename("san").as_deref(), Some("data/krut_san.json"));
        assert_eq!(ting_filename("x", "kartari"), None);
        assert_eq!(ting_filename("shuddha", "x"), None);
        assert_eq!(krut_filename("x"), None);
    }

    #[test]
    fn maps_engine_categories() {
        assert_eq!(engine_ting_category("shuddha"), Some("ting"));
        assert_eq!(engine_ting_category("yangluk"), Some("yangluk"));
        assert_eq!(engine_ting_category("x"), None);
        assert_eq!(engine_krut_family("shuddha"), Some("krut"));
        assert_eq!(engine_krut_family("nich"), Some("nich_krut"));
        assert_eq!(normalize_krut_engine_family("shuddha"), Some("krut"));
        assert_eq!(normalize_krut_engine_family("krut"), Some("krut"));
        assert_eq!(normalize_krut_engine_family("san_krut"), Some("san_krut"));
        assert_eq!(normalize_krut_engine_family("x"), None);
    }
}
