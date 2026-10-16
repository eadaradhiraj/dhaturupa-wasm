//! Exact search over the reverse index (F1/F2/F5 base layer).
//!
//! Query semantics: byte-exact SLP1 match only ([`RevIndex::get`]).

use crate::engine::data::DhatuFile;
use crate::engine::index::RevIndex;

/// Dhatu metadata for a result card (SPEC F1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DhatuMeta {
    pub id: String,
    pub upadesha: String,
    pub artha: String,
    pub gana: String,
    pub padam: String,
}

impl DhatuMeta {
    pub fn of(f: &DhatuFile) -> Self {
        Self {
            id: f.id.clone(),
            upadesha: f.upadesha().to_owned(),
            artha: f.artha().to_owned(),
            gana: f.gana().to_owned(),
            padam: f.padam().to_owned(),
        }
    }
}

/// One exact-match result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub meta: DhatuMeta,
    /// Location path inside the JSON entry.
    pub path: String,
    /// Prefix stack if the hit came from `upasarga_forms`.
    pub stack: String,
}

/// Exact search: every hit whose stored form equals `form` byte-for-byte.
#[must_use]
pub fn search_exact(corpus: &[DhatuFile], idx: &RevIndex, form: &str) -> Vec<SearchHit> {
    idx.get(form)
        .iter()
        .map(|h| SearchHit {
            meta: DhatuMeta::of(&corpus[h.file_idx as usize]),
            path: idx.path_of(h).to_owned(),
            stack: idx.stack_of(h).to_owned(),
        })
        .collect()
}

/// Sorted unique dhatu ids among hits (for T1 set-equality assertions).
#[must_use]
pub fn hit_ids(hits: &[SearchHit]) -> Vec<String> {
    let mut ids: Vec<String> = hits.iter().map(|h| h.meta.id.clone()).collect();
    ids.sort();
    ids.dedup();
    ids
}
