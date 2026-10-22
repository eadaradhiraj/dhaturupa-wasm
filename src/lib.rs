//! `dhaturupa-wasm` — Rust clone of `ashtadhyayi.com/dhatu` list + info/upasarga.
//!
//! Scope: `dhatu/data.txt` only. Prakriya (`dhatuforms_*`, `vidyut_*`),
//! commentary (`sutraani/*`), kosha (`kosha/*` minus embedded `upasargas`)
//! are intentionally out of scope.
//!
//! Source: `https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/data.txt`
//! (mirrors: `/jsondata/dhatu/data.txt`,
//! `https://raw.githubusercontent.com/ashtadhyayi-com/data/master/dhatu/data.txt`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod blobs;
pub mod chandrika;
pub mod derive_krut;
pub mod derive_ting;
pub mod engine;
pub mod forms;
pub mod genctx;
pub mod grammar;
pub mod hk;
pub mod morphdata;
pub mod prefix_krut;
pub mod revindex;
pub mod slp1;
pub mod stamp;
pub mod translit;
pub mod vidyut;
pub mod wasm;

/// Which engine version this build was compiled from (build.rs hash over
/// `stamp::ENGINE_SOURCES`). The artifact writers stamp their manifests
/// with it; a served artifact whose manifest stamp differs is stale.
pub const ENGINE_STAMP: &str = env!("DHATURUPA_ENGINE_STAMP");

/// Top-level wrapper in `dhatu/data.txt`: `{"name":"dhatu","data":[...]}`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DhatuFile {
    /// Always `"dhatu"`.
    pub name: String,
    /// 2259 entries, file order = `baseindex` order (`01.0001`..`10.0509`).
    pub data: Vec<DhatuEntry>,
}

/// One dhatu row. Optional note fields default to empty.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DhatuEntry {
    /// Internal id, e.g. `"1010001"`.
    #[serde(default)]
    pub i: String,
    /// Canonical key, e.g. `"01.0001"`. URL: `/dhatu/01.0001`.
    #[serde(default)]
    pub baseindex: String,
    /// Display form, Devanagari, e.g. `"भू"`.
    #[serde(default)]
    pub dhatu: String,
    /// Aupadeshik form, `"-"` for ganasutras.
    #[serde(default)]
    pub aupadeshik: String,
    /// `"1"`..`"10"`.
    #[serde(default)]
    pub gana: String,
    /// `"P"`/`"A"`/`"U"`/`"-"`.
    #[serde(default)]
    pub pada: String,
    /// `"S"`/`"A"`/`"V"`/`"-"`.
    #[serde(default)]
    pub settva: String,
    /// `"S"`/`"A"`/`"D"`/`"-"`.
    #[serde(default)]
    pub karma: String,
    /// Sanskrit artha, e.g. `"सत्तायाम्"`.
    #[serde(default)]
    pub artha: String,
    /// English gloss, comma-separated.
    #[serde(default)]
    pub artha_english: String,
    /// Hindi gloss.
    #[serde(default)]
    pub artha_hindi: String,
    /// Comma-separated tags.
    #[serde(default)]
    pub tags: String,
    /// Kaumudi serial: `""` (non-kaumudi), digits, digits+`"P"` (pathabhed),
    /// `"सौत्रः"` (sautra), `"गणसूत्रम्"` (ganasutra marker in `sk` too).
    #[serde(default)]
    pub sk: String,
    /// Kaumudi sutra ref, e.g. `"2165"`.
    #[serde(default)]
    pub sk_sutra: String,
    /// Laghu-kaumudi sutra ref, e.g. `"372"`.
    #[serde(default)]
    pub lsk_sutra: String,
    /// Dhaturupnandini serial.
    #[serde(default)]
    pub dhaturoopnandini: String,
    /// Comma-separated antarganas, e.g. `"प्वादिः"`.
    #[serde(default)]
    pub antarganas: String,
    /// Embedded upasarga meanings (this is dhatu data, not kosha search).
    #[serde(default)]
    pub upasargas: Vec<UpasargaEntry>,
    // ---- optional notes (info tab) ----
    #[serde(default)]
    pub dhatu_note: String,
    #[serde(default)]
    pub artha_note: String,
    #[serde(default)]
    pub settva_note: String,
    #[serde(default)]
    pub karma_note: String,
    #[serde(default)]
    pub pada_note: String,
    #[serde(default)]
    pub pathabhed_note: String,
    #[serde(default)]
    pub dhaturoopnandini_note: String,
    #[serde(default)]
    pub ganasutra_note: String,
    #[serde(default)]
    pub no_forms_note: String,
    #[serde(default)]
    pub antargana_note_pvadi: Option<String>,
}

impl DhatuEntry {
    /// `true` when this row has no Kaumudi serial (`sk == ""`).
    #[must_use]
    pub fn is_non_kaumudi(&self) -> bool {
        self.sk.is_empty()
    }

    /// `true` for pathabheda rows (`sk` ends with `P`, e.g. `"41P"`).
    #[must_use]
    pub fn is_pathabheda(&self) -> bool {
        self.sk.ends_with('P')
    }

    /// `true` for sautra rows (`sk == "सौत्रः"`).
    #[must_use]
    pub fn is_sautra(&self) -> bool {
        self.sk == "सौत्रः"
    }

    /// `true` for ganasutra rows (`aupadeshik == "-"`).
    #[must_use]
    pub fn is_ganasutra(&self) -> bool {
        self.aupadeshik == "-"
    }

    /// Kaumudi number for display: digits only, `""` otherwise.
    /// Mirrors `Dhatu.renderDhatuList` (`sk.toNumber()==sk ? D(sk) : ""`).
    #[must_use]
    pub fn kaumudi_number(&self) -> &str {
        if self.sk.chars().all(|c| c.is_ascii_digit()) && !self.sk.is_empty() {
            &self.sk
        } else {
            ""
        }
    }
}

/// One upasarga meaning attached to a dhatu.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpasargaEntry {
    /// e.g. `"अधि"`, `"वि + अति"`.
    #[serde(default)]
    pub name: String,
    /// Hindi meaning.
    #[serde(default)]
    pub artha_hindi: String,
}

/// Parse the full `dhatu/data.txt` payload.
pub fn load_str(s: &str) -> Result<DhatuFile, serde_json::Error> {
    serde_json::from_str(s)
}

/// Parse from a file path (local-first; fetch via `curl`, see README).
pub fn load_file(path: &std::path::Path) -> Result<DhatuFile, LoadError> {
    let text = std::fs::read_to_string(path).map_err(LoadError::Io)?;
    serde_json::from_str(&text).map_err(LoadError::Json)
}

/// Load failure: IO or JSON.
#[derive(Debug)]
pub enum LoadError {
    /// File read failed.
    Io(std::io::Error),
    /// JSON parse failed.
    Json(serde_json::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Normalize a user-typed baseindex: `"1.1"` -> `"01.0001"`.
/// Mirrors `Dhatu.renderDhatuForms` (`padStart(2)+"."+padStart(4)`).
#[must_use]
pub fn normalize_baseindex(raw: &str) -> String {
    let raw = raw.trim();
    if let Some((a, b)) = raw.split_once('.') {
        let a = a.trim();
        let b = b.trim();
        if a.len() <= 2 && b.len() <= 4 && is_digits(a) && is_digits(b) {
            return format!("{a:0>2}.{b:0>4}");
        }
    }
    raw.to_string()
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Find by canonical `baseindex` (accepts unpadded input too).
#[must_use]
pub fn find_by_baseindex<'a>(file: &'a DhatuFile, raw: &str) -> Option<&'a DhatuEntry> {
    let norm = normalize_baseindex(raw);
    file.data.iter().find(|e| e.baseindex == norm)
}

/// Find by Kaumudi serial: `/dhatu/sk<num>`.
#[must_use]
pub fn find_by_sk<'a>(file: &'a DhatuFile, sk: &str) -> Option<&'a DhatuEntry> {
    file.data.iter().find(|e| e.sk == sk)
}

/// `DhatuConstants.ganaStrings`: index 1..=10.
#[must_use]
pub fn gana_name(code: &str) -> &str {
    match code {
        "1" => "भ्वादिः",
        "2" => "अदादिः",
        "3" => "जुहोत्यादिः",
        "4" => "दिवादिः",
        "5" => "स्वादिः",
        "6" => "तुदादिः",
        "7" => "रुधादिः",
        "8" => "तनादिः",
        "9" => "क्र्यादिः",
        "10" => "चुरादिः",
        _ => "",
    }
}

/// `DhatuConstants.padaStrings`.
#[must_use]
pub fn pada_name(code: &str) -> &str {
    match code {
        "P" => "परस्मैपदी",
        "A" => "आत्मनेपदी",
        "U" => "उभयपदी",
        _ => "",
    }
}

/// `DhatuConstants.settvaStrings`.
#[must_use]
pub fn settva_name(code: &str) -> &str {
    match code {
        "S" => "सेट्",
        "A" => "अनिट्",
        "V" => "वेट्",
        _ => "",
    }
}

/// `DhatuConstants.karmaStrings`.
#[must_use]
pub fn karma_name(code: &str) -> &str {
    match code {
        "S" => "सकर्मकः",
        "A" => "अकर्मकः",
        "D" => "द्विकर्मकः",
        _ => "",
    }
}

/// Minimal filter set for the list page (subset of `DhatuFiltersData`).
#[derive(Debug, Clone, Default)]
pub struct DhatuFilter {
    /// e.g. `["1"]` for bhvadi.
    pub ganas: Vec<String>,
    /// e.g. `["P"]`.
    pub padas: Vec<String>,
    /// e.g. `["S"]`.
    pub settvas: Vec<String>,
    /// e.g. `["A"]`.
    pub karmas: Vec<String>,
}

impl DhatuFilter {
    /// `true` when no constraint is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ganas.is_empty()
            && self.padas.is_empty()
            && self.settvas.is_empty()
            && self.karmas.is_empty()
    }

    /// Apply to a slice; ganasutra rows (`-`) never match pada/settva/karma
    /// filters (mirrors `!t.ganasutra_note` guard in JS).
    #[must_use]
    pub fn apply<'a>(&self, entries: &'a [DhatuEntry]) -> Vec<&'a DhatuEntry> {
        entries
            .iter()
            .filter(|e| {
                if !self.ganas.is_empty() && !self.ganas.contains(&e.gana) {
                    return false;
                }
                if !self.padas.is_empty() && (e.is_ganasutra() || !self.padas.contains(&e.pada)) {
                    return false;
                }
                if !self.settvas.is_empty()
                    && (e.is_ganasutra() || !self.settvas.contains(&e.settva))
                {
                    return false;
                }
                if !self.karmas.is_empty() && (e.is_ganasutra() || !self.karmas.contains(&e.karma))
                {
                    return false;
                }
                true
            })
            .collect()
    }
}

/// One info-tab row (mirrors `Dhatu.getInfoData`, links omitted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoRow {
    /// Hindi label, e.g. `"गणः"`.
    pub name: String,
    /// Value (Devanagari/English as in source).
    pub value: String,
    /// Optional note.
    pub note: String,
}

/// Build info-tab rows for a detail page.
#[must_use]
pub fn info_rows(e: &DhatuEntry) -> Vec<InfoRow> {
    let row = |name: &str, value: &str, note: &str| InfoRow {
        name: name.to_string(),
        value: value.to_string(),
        note: note.to_string(),
    };
    vec![
        row("धातुः", &e.dhatu, ""),
        row("औपदेशिकस्वरूपम्", &e.aupadeshik, ""),
        row("अर्थः", &e.artha, &e.artha_note),
        row("English Meaning", &e.artha_english, ""),
        row("हिन्दी अर्थ", &e.artha_hindi, ""),
        row("गणः", gana_name(&e.gana), ""),
        row(
            "अन्तर्गणः",
            &e.antarganas,
            e.antargana_note_pvadi.as_deref().unwrap_or(""),
        ),
        row("पदम्", pada_name(&e.pada), &e.pada_note),
        row("कर्मकत्वम्", karma_name(&e.karma), &e.karma_note),
        row("इडागमयोग्यता", settva_name(&e.settva), &e.settva_note),
        row("धातुरूपनन्दिनीक्रमाङ्कः", &e.dhaturoopnandini, ""),
        row("कौमुदीधातुक्रमाङ्कः", &e.sk, &e.pathabhed_note),
        row("कौमुदीसूत्रक्रमाङ्कः", &e.sk_sutra, ""),
        row("लघुकौमुदीसूत्रक्रमाङ्कः", &e.lsk_sutra, ""),
        row("धातुरूपनन्दिनीटिप्पणी", &e.dhaturoopnandini_note, ""),
        row("धातुविशेषः", &e.dhatu_note, ""),
    ]
}

/// Render one list row like `dhatulist-row-template` (text form).
#[must_use]
pub fn format_list_row(e: &DhatuEntry) -> String {
    format!(
        "{} {} | {} | {} {} {} {} | {} | {}",
        e.baseindex,
        e.dhatu,
        e.artha,
        gana_name(&e.gana),
        pada_name(&e.pada),
        settva_name(&e.settva),
        karma_name(&e.karma),
        e.artha_english,
        e.artha_hindi
    )
}

/// Render the detail header + info tab (text form).
#[must_use]
pub fn format_detail_info(e: &DhatuEntry) -> String {
    use std::fmt::Write as _;
    let mut out = format!("{} {} — {}\n", e.baseindex, e.dhatu, e.artha);
    for r in info_rows(e) {
        if r.value.is_empty() && r.note.is_empty() {
            continue;
        }
        if r.note.is_empty() {
            let _ = writeln!(out, "{}: {}", r.name, r.value);
        } else {
            let _ = writeln!(out, "{}: {} [{}]", r.name, r.value, r.note);
        }
    }
    out
}

/// Render the upasarga tab (text form).
#[must_use]
pub fn format_upasarga(e: &DhatuEntry) -> String {
    use std::fmt::Write as _;
    if e.upasargas.is_empty() {
        return format!("{} {} — no upasargas\n", e.baseindex, e.dhatu);
    }
    let mut out = format!(
        "{} {} — उपसर्गार्थाः ({}):\n",
        e.baseindex,
        e.dhatu,
        e.upasargas.len()
    );
    for u in &e.upasargas {
        let _ = writeln!(out, "{} — {}", u.name, u.artha_hindi);
    }
    out
}

/// Build `baseindex -> index` map for URL routing.
#[must_use]
pub fn baseindex_map(file: &DhatuFile) -> HashMap<&str, usize> {
    file.data
        .iter()
        .enumerate()
        .map(|(idx, e)| (e.baseindex.as_str(), idx))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{"name":"dhatu","data":[
      {"i":"1010001","baseindex":"01.0001","dhatu":"भू","aupadeshik":"भू","gana":"1","pada":"P","settva":"S","karma":"A","artha":"सत्तायाम्","artha_english":"to exist","artha_hindi":"होना","tags":"","sk":"1","sk_sutra":"2165","lsk_sutra":"372","dhaturoopnandini":"1","antarganas":"","upasargas":[{"name":"प्र","artha_hindi":"प्रकट होना"}]},
      {"i":"1010002","baseindex":"01.0002","dhatu":"एध्","aupadeshik":"एध्","gana":"1","pada":"A","settva":"S","karma":"A","artha":"वृद्धौ","artha_english":"to grow","artha_hindi":"बढ़ना","tags":"","sk":"2","sk_sutra":"","lsk_sutra":"","dhaturoopnandini":"","antarganas":"","upasargas":[]}
    ]}"#;

    #[test]
    fn loads_fixture() {
        let f = load_str(FIXTURE).expect("fixture parses");
        assert_eq!(f.data.len(), 2);
        assert_eq!(f.data[0].baseindex, "01.0001");
    }

    #[test]
    fn normalizes_baseindex() {
        assert_eq!(normalize_baseindex("1.1"), "01.0001");
        assert_eq!(normalize_baseindex("01.0001"), "01.0001");
        assert_eq!(normalize_baseindex("sk1"), "sk1");
    }

    #[test]
    fn finds_entries() {
        let f = load_str(FIXTURE).expect("fixture parses");
        assert!(find_by_baseindex(&f, "1.1").is_some());
        assert!(find_by_sk(&f, "2").is_some());
        assert!(find_by_baseindex(&f, "99.9999").is_none());
    }

    #[test]
    fn code_tables() {
        assert_eq!(gana_name("1"), "भ्वादिः");
        assert_eq!(pada_name("P"), "परस्मैपदी");
        assert_eq!(settva_name("S"), "सेट्");
        assert_eq!(karma_name("A"), "अकर्मकः");
        assert_eq!(gana_name("x"), "");
    }

    #[test]
    fn filters_gana_pada() {
        let f = load_str(FIXTURE).expect("fixture parses");
        let all = DhatuFilter::default().apply(&f.data);
        assert_eq!(all.len(), 2);
        let p = DhatuFilter {
            padas: vec!["A".to_string()],
            ..Default::default()
        }
        .apply(&f.data);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].baseindex, "01.0002");
    }
}
