//! `vidyut_dhatupatha` join: `baseindex -> SLP1 dhatu`.
//!
//! The dhatu list (`dhatu/data.txt`) is Devanagari metadata. The SLP1
//! morphology engine keys everything by SLP1 (`BU`, `eDa~`, …). The bridge
//! is `dhatu/vidyut/vidyut_dhatupatha_<VER>.tsv`:
//!
//! ```text
//! code<TAB>dhatu<TAB>artha
//! 01.0001<TAB>BU<TAB>sattAyAm
//! ```
//!
//! Source: `https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/vidyut/vidyut_dhatupatha_5.tsv`
//! (`VIDYUT_WASM_VERSION=5` in the site JS).

use std::collections::HashMap;

/// One TSV row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VidyutRow {
    /// e.g. `"01.0001"` — joins to [`crate::DhatuEntry::baseindex`].
    pub code: String,
    /// SLP1 dhatu, e.g. `"BU"`.
    pub dhatu: String,
    /// SLP1 artha, e.g. `"sattAyAm"`.
    pub artha: String,
}

/// Parse TSV text (header `code<TAB>dhatu<TAB>artha` + rows).
/// Malformed lines are skipped.
#[must_use]
pub fn load_tsv_str(text: &str) -> Vec<VidyutRow> {
    let mut rows = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let mut cols = line.split('\t');
        if let (Some(code), Some(dhatu), Some(artha)) = (cols.next(), cols.next(), cols.next()) {
            let (code, dhatu, artha) = (
                code.trim().to_string(),
                dhatu.trim().to_string(),
                artha.trim().to_string(),
            );
            if code.is_empty() || dhatu.is_empty() {
                continue;
            }
            rows.push(VidyutRow { code, dhatu, artha });
        }
    }
    rows
}

/// Build `code -> row index` map.
#[must_use]
pub fn code_map(rows: &[VidyutRow]) -> HashMap<&str, usize> {
    rows.iter()
        .enumerate()
        .map(|(i, r)| (r.code.as_str(), i))
        .collect()
}

/// Find SLP1 row for a baseindex (accepts unpadded `"1.1"` too).
#[must_use]
pub fn find<'a>(rows: &'a [VidyutRow], raw_code: &str) -> Option<&'a VidyutRow> {
    let norm = crate::normalize_baseindex(raw_code);
    rows.iter().find(|r| r.code == norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TSV: &str = "code\tdhatu\tartha\n01.0001\tBU\tsattAyAm\n01.0002\teDa~\tvfdDO\n";

    #[test]
    fn parses_rows() {
        let rows = load_tsv_str(TSV);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dhatu, "BU");
    }

    #[test]
    fn finds_unpadded() {
        let rows = load_tsv_str(TSV);
        assert!(find(&rows, "1.1").is_some());
        assert!(find(&rows, "99.9999").is_none());
    }
}
