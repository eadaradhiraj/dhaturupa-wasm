//! Oracle reverse indexes built from JSON (SPEC §9 T1–T3, M1).
//!
//! Host-only (`std::collections::HashMap`). Occurrences borrow from the
//! loaded `files` slice so the index holds no duplicated id/category copies
//! beyond the form-string keys.

use crate::engine::data::{DhatuFile, KrtCell};
use crate::slp1::split_alternants;
use std::collections::{BTreeMap, HashMap};

/// Accessor for one krdanta cell field.
type KrtFieldGet = fn(&KrtCell) -> &String;

/// `(field, accessor)` pairs in `base,m,f,n` order.
const KRT_FIELDS: [(&str, KrtFieldGet); 4] = [
    ("base", |c| &c.base),
    ("m", |c| &c.m),
    ("f", |c| &c.f),
    ("n", |c| &c.n),
];

/// One tinanta occurrence: `(dhatu, category.lakara[pos])`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinantaOcc<'a> {
    /// Borrowed dhatu id, e.g. `01.0001`.
    pub dhatu_id: &'a str,
    /// Top-level category, e.g. `ting`, `yak`, `san`.
    pub category: &'a str,
    /// Lakara/pada slot, e.g. `plat`, `plang`.
    pub lakara: &'a str,
    /// Position inside the form array.
    pub pos: usize,
}

/// One krdanta cell occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KrtOcc<'a> {
    /// Borrowed dhatu id.
    pub dhatu_id: &'a str,
    /// Participle family: `krut`, `san_krut`, `nich_krut`, …
    pub family: &'a str,
    /// Krt name, e.g. `Satf`, `tavya`.
    pub krt: &'a str,
    /// Cell field: `base`, `m`, `f` or `n`.
    pub field: &'static str,
    /// Index of the cell inside the krt array.
    pub cell_idx: usize,
    /// Which alternant of a `/`-joined cell (`0` = first).
    pub alternant: usize,
}

/// Build `form -> occurrences` over all base `conjugations` (T1 oracle).
///
/// Skipped files (`skipped == true`) are excluded. Empty strings are ignored.
#[must_use]
pub fn build_tinanta_index<'a>(files: &'a [DhatuFile]) -> HashMap<String, Vec<TinantaOcc<'a>>> {
    let mut map: HashMap<String, Vec<TinantaOcc<'a>>> = HashMap::new();
    for file in files {
        if file.skipped {
            continue;
        }
        for (category, sub) in &file.conjugations {
            for (lakara, forms) in sub {
                for (pos, form) in forms.iter().enumerate() {
                    if form.is_empty() {
                        continue;
                    }
                    map.entry(form.clone()).or_default().push(TinantaOcc {
                        dhatu_id: file.id.as_str(),
                        category: category.as_str(),
                        lakara: lakara.as_str(),
                        pos,
                    });
                }
            }
        }
    }
    map
}

/// Build `form -> occurrences` over all `participles` cells (T2 oracle).
///
/// `/`-joined cells (`Bavat/Bavad`) index each alternant separately under
/// its own string, plus the raw cell is *not* indexed as a lookup form —
/// lookups are exact per alternant. Empty cells are ignored.
#[must_use]
pub fn build_krt_index<'a>(files: &'a [DhatuFile]) -> HashMap<String, Vec<KrtOcc<'a>>> {
    let mut map: HashMap<String, Vec<KrtOcc<'a>>> = HashMap::new();
    for file in files {
        if file.skipped {
            continue;
        }
        for (family, krts) in &file.participles {
            for (krt, cells) in krts {
                for (cell_idx, cell) in cells.iter().enumerate() {
                    for (field, get) in KRT_FIELDS {
                        let raw = get(cell);
                        if raw.is_empty() {
                            continue;
                        }
                        for (alternant, part) in split_alternants(raw).iter().enumerate() {
                            map.entry((*part).to_string()).or_default().push(KrtOcc {
                                dhatu_id: file.id.as_str(),
                                family: family.as_str(),
                                krt: krt.as_str(),
                                field,
                                cell_idx,
                                alternant,
                            });
                        }
                    }
                }
            }
        }
    }
    map
}

/// Independent scan for one tinanta form (cross-checks the index).
///
/// Returns sorted `(dhatu_id, category, lakara)` triples.
#[must_use]
pub fn scan_tinanta(files: &[DhatuFile], target: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for file in files {
        if file.skipped {
            continue;
        }
        for (category, sub) in &file.conjugations {
            for (lakara, forms) in sub {
                if forms.iter().any(|f| f.as_str() == target) {
                    out.push((file.id.clone(), category.clone(), lakara.clone()));
                }
            }
        }
    }
    out.sort();
    out
}

/// Independent scan for one upasarga-prefixed tinanta form.
///
/// Returns sorted `(dhatu_id, prefix_stack, category, lakara)` tuples.
/// Linear scan over `upasarga_forms`; fast enough for spot checks
/// (≈10M strings in tens of ms in release).
#[must_use]
pub fn scan_upasarga(files: &[DhatuFile], target: &str) -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    for file in files {
        if file.skipped {
            continue;
        }
        for (stack, data) in &file.upasarga_forms {
            for (category, sub) in &data.conjugations {
                for (lakara, forms) in sub {
                    if forms.iter().any(|f| f.as_str() == target) {
                        out.push((
                            file.id.clone(),
                            stack.clone(),
                            category.clone(),
                            lakara.clone(),
                        ));
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// One exact-match location inside the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevHit {
    /// Index into the corpus slice passed to [`RevIndex::build`].
    pub file_idx: u32,
    /// Interned [`RevIndex::paths`] entry.
    pub path_idx: u32,
    /// Interned [`RevIndex::stacks`] entry (`""` for base forms).
    pub stack_idx: u32,
}

/// Unified reverse index over base + (optionally) prefixed forms.
///
/// Paths and stacks are interned dictionaries; hits hold `u32` indices so
/// the 3M+ occurrence table stays compact. Use [`RevIndex::path_of`] /
/// [`RevIndex::stack_of`] to resolve display strings.
#[derive(Debug, Default)]
pub struct RevIndex {
    map: HashMap<String, Vec<RevHit>>,
    paths: Vec<String>,
    stacks: Vec<String>,
    path_ids: HashMap<String, u32>,
    stack_ids: HashMap<String, u32>,
}

impl RevIndex {
    /// Build over `corpus`.
    ///
    /// Base `conjugations` + base `participles` alternants are always
    /// indexed; `upasarga_forms` conjugations + participles are included
    /// only when `include_upasarga` is true.
    #[must_use]
    pub fn build(corpus: &[DhatuFile], include_upasarga: bool) -> Self {
        let mut idx = Self::default();
        idx.stacks.push(String::new());
        idx.stack_ids.insert(String::new(), 0);
        for (file_idx, file) in corpus.iter().enumerate() {
            if file.skipped {
                continue;
            }
            let Ok(fi) = u32::try_from(file_idx) else {
                continue;
            };
            idx.index_base_file(file, fi);
            if include_upasarga {
                idx.index_upasarga_file(file, fi);
            }
        }
        idx
    }

    fn index_base_file(&mut self, file: &DhatuFile, fi: u32) {
        for (category, sub) in &file.conjugations {
            for (lakara, forms) in sub {
                let path = format!("conjugations.{category}.{lakara}");
                let pi = self.intern_path(&path);
                for form in forms {
                    if form.is_empty() {
                        continue;
                    }
                    self.map.entry(form.clone()).or_default().push(RevHit {
                        file_idx: fi,
                        path_idx: pi,
                        stack_idx: 0,
                    });
                }
            }
        }
        Self::index_krt_cells(self, &file.participles, fi, 0, &|family, krt, field| {
            format!("participles.{family}.{krt}.{field}")
        });
    }

    fn index_upasarga_file(&mut self, file: &DhatuFile, fi: u32) {
        for (stack, data) in &file.upasarga_forms {
            let si = self.intern_stack(stack);
            Self::index_conjugations(self, &data.conjugations, fi, si, &|cat, lak| {
                format!("upasarga_forms.{stack}.conjugations.{cat}.{lak}")
            });
            Self::index_krt_cells(self, &data.participles, fi, si, &|family, krt, field| {
                format!("upasarga_forms.{stack}.participles.{family}.{krt}.{field}")
            });
        }
    }

    fn index_conjugations(
        idx: &mut Self,
        conjugations: &BTreeMap<String, BTreeMap<String, Vec<String>>>,
        fi: u32,
        si: u32,
        path_of: &dyn Fn(&str, &str) -> String,
    ) {
        for (category, sub) in conjugations {
            for (lakara, forms) in sub {
                let pi = idx.intern_path(&path_of(category, lakara));
                for form in forms {
                    if form.is_empty() {
                        continue;
                    }
                    idx.map.entry(form.clone()).or_default().push(RevHit {
                        file_idx: fi,
                        path_idx: pi,
                        stack_idx: si,
                    });
                }
            }
        }
    }

    fn index_krt_cells(
        idx: &mut Self,
        participles: &BTreeMap<String, BTreeMap<String, Vec<KrtCell>>>,
        fi: u32,
        si: u32,
        path_of: &dyn Fn(&str, &str, &str) -> String,
    ) {
        for (family, krts) in participles {
            for (krt, cells) in krts {
                for cell in cells {
                    for (field, raw) in [
                        ("base", &cell.base),
                        ("m", &cell.m),
                        ("f", &cell.f),
                        ("n", &cell.n),
                    ] {
                        if raw.is_empty() {
                            continue;
                        }
                        let pi = idx.intern_path(&path_of(family, krt, field));
                        for part in split_alternants(raw) {
                            idx.map.entry(part.to_string()).or_default().push(RevHit {
                                file_idx: fi,
                                path_idx: pi,
                                stack_idx: si,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Intern a location path (at most tens of thousands of distinct values).
    fn intern_path(&mut self, path: &str) -> u32 {
        if let Some(id) = self.path_ids.get(path) {
            return *id;
        }
        // Counts are tiny (< 100k); saturation is unreachable in practice.
        let id = u32::try_from(self.paths.len()).unwrap_or(u32::MAX);
        self.paths.push(path.to_string());
        self.path_ids.insert(path.to_string(), id);
        id
    }

    /// Intern a prefix stack (at most hundreds of distinct values).
    fn intern_stack(&mut self, stack: &str) -> u32 {
        if let Some(id) = self.stack_ids.get(stack) {
            return *id;
        }
        let id = u32::try_from(self.stacks.len()).unwrap_or(u32::MAX);
        self.stacks.push(stack.to_string());
        self.stack_ids.insert(stack.to_string(), id);
        id
    }

    /// Exact-match hits for `form` (byte-exact SLP1), possibly empty.
    #[must_use]
    pub fn get(&self, form: &str) -> &[RevHit] {
        self.map.get(form).map_or(&[], Vec::as_slice)
    }

    /// Resolve a hit's location path.
    #[must_use]
    pub fn path_of(&self, hit: &RevHit) -> &str {
        &self.paths[hit.path_idx as usize]
    }

    /// Resolve a hit's prefix stack (`""` for base forms).
    #[must_use]
    pub fn stack_of(&self, hit: &RevHit) -> &str {
        &self.stacks[hit.stack_idx as usize]
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}
