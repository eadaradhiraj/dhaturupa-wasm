//! Reverse indexes over Devanagari dhatu tables (portal search core).
//!
//! Keys are Devanagari display strings; every SLP1 engine call happens
//! inside the builders, so the portal (and these audits) never handle SLP1.
//! Three layers:
//!
//! - base: tinanta slots (`,`-variants) and krdanta cells (`/`-alternants),
//! - prefixed: runtime [`generate_prefixed`] over each dhatu's site stacks,
//! - subanta: runtime [`krt_table`] paradigms for declinable krt cells.
//!
//! [`search_union`] merges HK/SLP1-decoded query candidates the way the
//! portal search box will.
//!
//! [`Sampler`] makes audits tiered: sampled strides run in `cargo test`,
//! exhaustive passes are `#[ignore]`d for release/CI
//! (`cargo test --release -- --ignored`).
//!
//! [`generate_prefixed`]: crate::engine::upasarga::generate_prefixed
//! [`krt_table`]: crate::engine::subanta::krt_table

use crate::engine::subanta::krt_table;
use crate::engine::upasarga::{generate_prefixed, GenCtx};
use crate::forms::{engine_krut_family, parse_krt_rows, split_slots, split_variants};
use crate::slp1::split_alternants;
use crate::translit::{deva_stack_to_slp1, deva_to_slp1, slp1_to_deva};
use std::collections::HashMap;

/// Where a display form comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LocKind {
    /// `(family, prayoga, lakara, slot-pos)` in a ting file.
    Ting {
        family: String,
        prayoga: String,
        lakara: String,
        pos: usize,
    },
    /// `(family, krt, row, field)` cell in a krut file; `field` ∈ base/m/f/n.
    Krt {
        family: String,
        krt: String,
        row: usize,
        field: String,
    },
    /// Prefixed surface: site stack (Devanagari, e.g. `वि + अति`) + base loc.
    Prefixed { stack: String, inner: Box<LocKind> },
    /// Subanta cell of a krt paradigm.
    Subanta {
        family: String,
        krt: String,
        row: usize,
        field: String,
        vibhakti: String,
        vacana: String,
    },
}

/// One reverse-index row.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Posting {
    /// Canonical baseindex, e.g. `01.0001`.
    pub baseindex: String,
    /// Location of the form.
    pub loc: LocKind,
}

/// Krdanta cell fields in `base,m,f,n` order.
const KRUT_FIELDS: [&str; 4] = ["base", "m", "f", "n"];

/// Per-dhatu base forms collected for the prefixed stage.
pub type DhatuForms<S = std::collections::hash_map::RandomState> =
    HashMap<String, Vec<(LocKind, String)>, S>;

/// Coverage counters; `miss` must stay zero in every audit.
#[derive(Debug, Default)]
pub struct Stats {
    /// Base tinanta variants indexed.
    pub ting_forms: u64,
    /// Base krdanta alternants indexed.
    pub krt_forms: u64,
    /// Prefixed surfaces indexed.
    pub prefixed_forms: u64,
    /// Subanta cells indexed.
    pub subanta_forms: u64,
    /// Skipped: Devanagari→SLP1 input failed (separators, unknown chars).
    pub skip_deva: u64,
    /// Skipped: SLP1→Devanagari output failed (should not happen).
    pub skip_slp1: u64,
    /// Skipped: `krt_table` declined (indeclinable shape, unknown class).
    pub skip_table: u64,
    /// Skipped: dhatu has no base forms loaded (yang-only gaps, etc.).
    pub skip_no_base: u64,
    /// First few skip descriptions (capped).
    pub skip_samples: Vec<String>,
}

impl Stats {
    fn note(counter: &mut u64, samples: &mut Vec<String>, what: String) {
        *counter += 1;
        if samples.len() < 10 {
            samples.push(what);
        }
    }
}

/// Interned string pool: the few thousand distinct atoms (dhatu ids,
/// families, lakaras, krts, …) shared by millions of postings.
#[derive(Debug, Default)]
struct Pool {
    strings: Vec<std::sync::Arc<str>>,
    ids: HashMap<std::sync::Arc<str>, u16>,
}

impl Pool {
    fn id(&mut self, text: &str) -> u16 {
        if let Some(found) = self.ids.get(text) {
            return *found;
        }
        let next = self.strings.len();
        assert!(next < u16::MAX as usize, "string pool exhausted");
        let shared: std::sync::Arc<str> = text.into();
        let id = u16::try_from(next).expect("pool fits in u16");
        self.ids.insert(shared.clone(), id);
        self.strings.push(shared);
        id
    }

    fn get(&self, id: u16) -> &str {
        &self.strings[id as usize]
    }
}

/// Compact location code (all strings are pool ids).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocCode {
    tag: u8,
    a: u16,
    b: u16,
    c: u16,
    d: u16,
    e: u16,
    num: u32,
}

/// Compact posting: tags are `t`ing=`b't'`, `k`rt, `p`refixed, `s`ubanta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CompactEntry {
    dhatu: u16,
    loc: LocCode,
    inner: Option<LocCode>,
}

fn code_loc(pool: &mut Pool, loc: &LocKind) -> (LocCode, Option<LocCode>) {
    // Sequential interning (no closures: the Prefixed arm recurses).
    let coded = |pool: &mut Pool, tag: u8, parts: [&str; 5], num: u32| LocCode {
        tag,
        a: pool.id(parts[0]),
        b: pool.id(parts[1]),
        c: pool.id(parts[2]),
        d: pool.id(parts[3]),
        e: pool.id(parts[4]),
        num,
    };
    match loc {
        LocKind::Ting {
            family,
            prayoga,
            lakara,
            pos,
        } => (
            coded(
                pool,
                b't',
                [family, prayoga, lakara, "", ""],
                (*pos).try_into().unwrap_or(u32::MAX),
            ),
            None,
        ),
        LocKind::Krt {
            family,
            krt,
            row,
            field,
        } => (
            coded(
                pool,
                b'k',
                [family, krt, field, "", ""],
                (*row).try_into().unwrap_or(u32::MAX),
            ),
            None,
        ),
        LocKind::Prefixed { stack, inner } => {
            let (base, _) = code_loc(pool, inner);
            (coded(pool, b'p', [stack, "", "", "", ""], 0), Some(base))
        }
        LocKind::Subanta {
            family,
            krt,
            row,
            field,
            vibhakti,
            vacana,
        } => (
            coded(
                pool,
                b's',
                [family, krt, field, vibhakti, vacana],
                (*row).try_into().unwrap_or(u32::MAX),
            ),
            None,
        ),
    }
}

/// `form (Devanagari) -> postings`, with interned compact storage: the few
/// thousand distinct atoms are stored once, not once per posting. Forms live
/// in a sorted vector (binary search); builders stage into a hash map and
/// [`finish`](RevIndex::finish) merges it in.
#[derive(Debug, Default)]
pub struct RevIndex {
    staging: HashMap<String, Vec<CompactEntry>>,
    sorted: Vec<(Box<str>, Vec<CompactEntry>)>,
    pool: Pool,
}

impl RevIndex {
    fn decode(&self, code: &LocCode) -> LocKind {
        match code.tag {
            b't' => LocKind::Ting {
                family: self.pool.get(code.a).to_string(),
                prayoga: self.pool.get(code.b).to_string(),
                lakara: self.pool.get(code.c).to_string(),
                pos: code.num as usize,
            },
            b'k' => LocKind::Krt {
                family: self.pool.get(code.a).to_string(),
                krt: self.pool.get(code.b).to_string(),
                row: code.num as usize,
                field: self.pool.get(code.c).to_string(),
            },
            b'p' => LocKind::Prefixed {
                stack: self.pool.get(code.a).to_string(),
                // Prefixed wrappers only ever enclose base locations.
                inner: Box::new(self.decode_inner(code)),
            },
            _ => LocKind::Subanta {
                family: self.pool.get(code.a).to_string(),
                krt: self.pool.get(code.b).to_string(),
                row: code.num as usize,
                field: self.pool.get(code.c).to_string(),
                vibhakti: self.pool.get(code.d).to_string(),
                vacana: self.pool.get(code.e).to_string(),
            },
        }
    }

    fn decode_inner(&self, code: &LocCode) -> LocKind {
        match code.tag {
            b't' => LocKind::Ting {
                family: self.pool.get(code.a).to_string(),
                prayoga: self.pool.get(code.b).to_string(),
                lakara: self.pool.get(code.c).to_string(),
                pos: code.num as usize,
            },
            _ => LocKind::Krt {
                family: self.pool.get(code.a).to_string(),
                krt: self.pool.get(code.b).to_string(),
                row: code.num as usize,
                field: self.pool.get(code.c).to_string(),
            },
        }
    }

    fn materialize(&self, entry: &CompactEntry) -> Posting {
        Posting {
            baseindex: self.pool.get(entry.dhatu).to_string(),
            loc: match entry.inner {
                Some(inner) => LocKind::Prefixed {
                    stack: self.pool.get(entry.loc.a).to_string(),
                    inner: Box::new(self.decode(&inner)),
                },
                None => self.decode(&entry.loc),
            },
        }
    }

    /// Insert one posting under `form` (staged; call [`finish`](RevIndex::finish)
    /// before shipping or caching for query speed and memory).
    pub fn insert(&mut self, form: String, posting: &Posting) {
        let dhatu = self.pool.id(&posting.baseindex);
        let (loc, inner) = code_loc(&mut self.pool, &posting.loc);
        self.staging
            .entry(form)
            .or_default()
            .push(CompactEntry { dhatu, loc, inner });
    }

    /// Merge staged forms into the sorted query vector (linear merge).
    pub fn finish(&mut self) {
        if self.staging.is_empty() {
            return;
        }
        let mut staged: Vec<(Box<str>, Vec<CompactEntry>)> = std::mem::take(&mut self.staging)
            .into_iter()
            .map(|(form, entries)| (form.into_boxed_str(), entries))
            .collect();
        staged.sort_by(|left, right| left.0.cmp(&right.0));
        let mut merged = Vec::with_capacity(self.sorted.len() + staged.len());
        let mut sorted = std::mem::take(&mut self.sorted).into_iter().peekable();
        let mut incoming = staged.into_iter().peekable();
        loop {
            match (sorted.peek(), incoming.peek()) {
                (Some(current), Some(next)) if current.0 < next.0 => {
                    merged.push(sorted.next().expect("peeked"));
                }
                (Some(_), Some(_)) => {
                    let mut current = sorted.next().expect("peeked");
                    let next = incoming.next().expect("peeked");
                    if current.0 == next.0 {
                        current.1.extend(next.1);
                    } else {
                        merged.push(next);
                        merged.push(current);
                    }
                }
                (Some(_), None) => {
                    merged.push(sorted.next().expect("peeked"));
                }
                (None, Some(_)) => {
                    merged.push(incoming.next().expect("peeked"));
                }
                (None, None) => break,
            }
        }
        for slot in &mut merged {
            slot.1.shrink_to_fit();
        }
        merged.shrink_to_fit();
        self.sorted = merged;
    }

    fn lookup_sorted(&self, form: &str) -> Option<&Vec<CompactEntry>> {
        self.sorted
            .binary_search_by(|probe| probe.0.as_ref().cmp(form))
            .ok()
            .map(|pos| &self.sorted[pos].1)
    }

    /// Exact-match postings for `form` (possibly empty).
    #[must_use]
    pub fn get(&self, form: &str) -> Vec<Posting> {
        let mut out = Vec::new();
        if let Some(entries) = self.staging.get(form) {
            out.extend(entries.iter().map(|entry| self.materialize(entry)));
        }
        if let Some(entries) = self.lookup_sorted(form) {
            out.extend(entries.iter().map(|entry| self.materialize(entry)));
        }
        out
    }

    /// Number of distinct key forms.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sorted.len()
            + self
                .staging
                .keys()
                .filter(|form| self.lookup_sorted(form).is_none())
                .count()
    }

    /// Total postings across all forms.
    #[must_use]
    pub fn postings(&self) -> usize {
        self.sorted.iter().map(|slot| slot.1.len()).sum::<usize>()
            + self.staging.values().map(Vec::len).sum::<usize>()
    }

    /// True when no form is indexed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sorted.is_empty() && self.staging.is_empty()
    }

    /// Every posting materialized (build-time merging only; expensive).
    #[must_use]
    pub fn all_postings(&self) -> Vec<(String, Posting)> {
        let mut out = Vec::new();
        for slot in &self.sorted {
            for entry in &slot.1 {
                out.push((slot.0.to_string(), self.materialize(entry)));
            }
        }
        for (form, entries) in &self.staging {
            for entry in entries {
                out.push((form.clone(), self.materialize(entry)));
            }
        }
        out
    }
}

/// Merge hits for several query candidates (HK ∪ SLP1 readings), sorted and
/// deduplicated.
#[must_use]
pub fn search_union(indexes: &[&RevIndex], candidates: &[String]) -> Vec<Posting> {
    let mut out = Vec::new();
    for idx in indexes {
        for cand in candidates {
            out.extend(idx.get(cand).iter().cloned());
        }
    }
    out.sort();
    out.dedup();
    out
}

fn loc_array(loc: &LocKind, out: &mut Vec<serde_json::Value>) {
    match loc {
        LocKind::Ting {
            family,
            prayoga,
            lakara,
            pos,
        } => {
            out.push("t".into());
            out.push(family.clone().into());
            out.push(prayoga.clone().into());
            out.push(lakara.clone().into());
            out.push((*pos).into());
        }
        LocKind::Krt {
            family,
            krt,
            row,
            field,
        } => {
            out.push("k".into());
            out.push(family.clone().into());
            out.push(krt.clone().into());
            out.push((*row).into());
            out.push(field.clone().into());
        }
        LocKind::Prefixed { stack, inner } => {
            out.push("p".into());
            out.push(stack.clone().into());
            loc_array(inner, out);
        }
        LocKind::Subanta {
            family,
            krt,
            row,
            field,
            vibhakti,
            vacana,
        } => {
            out.push("s".into());
            out.push(family.clone().into());
            out.push(krt.clone().into());
            out.push((*row).into());
            out.push(field.clone().into());
            out.push(vibhakti.clone().into());
            out.push(vacana.clone().into());
        }
    }
}

/// Compact shippable encoding of a base index: forms and postings sorted,
/// locations as positional arrays (`["t",family,prayoga,lakara,pos]`,
/// `["k",family,krt,row,field]`, `["p",stack,inner…]`,
/// `["s",family,krt,row,field,vibhakti,vacana]`). Finishes the index first.
#[must_use]
pub fn to_compact_value(index: &mut RevIndex) -> serde_json::Value {
    index.finish();
    let mut encoded = serde_json::Map::with_capacity(index.sorted.len());
    for slot in &index.sorted {
        let mut postings: Vec<Posting> = slot
            .1
            .iter()
            .map(|entry| index.materialize(entry))
            .collect();
        postings.sort();
        let rows: Vec<serde_json::Value> = postings
            .into_iter()
            .map(|posting| {
                let mut row = vec![serde_json::Value::String(posting.baseindex)];
                loc_array(&posting.loc, &mut row);
                serde_json::Value::Array(row)
            })
            .collect();
        encoded.insert(slot.0.to_string(), serde_json::Value::Array(rows));
    }
    serde_json::json!({"v": 1, "forms": encoded})
}

/// One compact row atom: display string or counter.
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Atom {
    Str(String),
    Num(u64),
}

fn atom_str(atoms: &[Atom], pos: usize) -> Option<&str> {
    match atoms.get(pos)? {
        Atom::Str(text) => Some(text),
        Atom::Num(_) => None,
    }
}

fn atom_usize(atoms: &[Atom], pos: usize) -> Option<usize> {
    match atoms.get(pos)? {
        Atom::Num(num) => (*num).try_into().ok(),
        Atom::Str(_) => None,
    }
}

fn loc_of_atoms(atoms: &[Atom]) -> Option<(LocKind, usize)> {
    let tag = atom_str(atoms, 0)?;
    match tag {
        "t" => Some((
            LocKind::Ting {
                family: atom_str(atoms, 1)?.to_string(),
                prayoga: atom_str(atoms, 2)?.to_string(),
                lakara: atom_str(atoms, 3)?.to_string(),
                pos: atom_usize(atoms, 4)?,
            },
            5,
        )),
        "k" => Some((
            LocKind::Krt {
                family: atom_str(atoms, 1)?.to_string(),
                krt: atom_str(atoms, 2)?.to_string(),
                row: atom_usize(atoms, 3)?,
                field: atom_str(atoms, 4)?.to_string(),
            },
            5,
        )),
        "p" => {
            let stack = atom_str(atoms, 1)?.to_string();
            let (inner, used) = loc_of_atoms(&atoms[2..])?;
            Some((
                LocKind::Prefixed {
                    stack,
                    inner: Box::new(inner),
                },
                2 + used,
            ))
        }
        "s" => Some((
            LocKind::Subanta {
                family: atom_str(atoms, 1)?.to_string(),
                krt: atom_str(atoms, 2)?.to_string(),
                row: atom_usize(atoms, 3)?,
                field: atom_str(atoms, 4)?.to_string(),
                vibhakti: atom_str(atoms, 5)?.to_string(),
                vacana: atom_str(atoms, 6)?.to_string(),
            },
            7,
        )),
        _ => None,
    }
}

use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor};

/// Streaming decode of [`to_compact_value`] output: rows convert straight
/// into interned entries, so loading never holds a document-sized tree.
impl<'de> serde::Deserialize<'de> for RevIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FormsSeed<'a> {
            index: &'a mut RevIndex,
        }

        impl<'de> DeserializeSeed<'de> for FormsSeed<'_> {
            type Value = ();

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FormsVisitor<'a> {
                    index: &'a mut RevIndex,
                }

                impl<'de> Visitor<'de> for FormsVisitor<'_> {
                    type Value = ();

                    fn expecting(
                        &self,
                        formatter: &mut std::fmt::Formatter<'_>,
                    ) -> std::fmt::Result {
                        formatter.write_str("a form-to-rows map")
                    }

                    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                    where
                        A: MapAccess<'de>,
                    {
                        while let Some(form) = map.next_key::<String>()? {
                            let rows = map.next_value::<Vec<Vec<Atom>>>()?;
                            let mut entries = Vec::with_capacity(rows.len());
                            for row in &rows {
                                let baseindex = atom_str(row, 0).ok_or_else(|| {
                                    serde::de::Error::custom("row needs a baseindex")
                                })?;
                                let (loc, used) = loc_of_atoms(&row[1..])
                                    .ok_or_else(|| serde::de::Error::custom("bad location"))?;
                                if used != row.len() - 1 {
                                    return Err(serde::de::Error::custom("trailing atoms"));
                                }
                                let dhatu = self.index.pool.id(baseindex);
                                let (coded, inner) = code_loc(&mut self.index.pool, &loc);
                                entries.push(CompactEntry {
                                    dhatu,
                                    loc: coded,
                                    inner,
                                });
                            }
                            entries.shrink_to_fit();
                            self.index.sorted.push((form.into_boxed_str(), entries));
                        }
                        Ok(())
                    }
                }

                deserializer.deserialize_map(FormsVisitor { index: self.index })
            }
        }

        struct IndexVisitor;

        impl<'de> Visitor<'de> for IndexVisitor {
            type Value = RevIndex;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a versioned compact index")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut index = RevIndex::default();
                let mut version = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "v" => version = Some(map.next_value::<u64>()?),
                        "forms" => map.next_value_seed(FormsSeed { index: &mut index })?,
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                if version != Some(1) {
                    return Err(serde::de::Error::custom("bad artifact version"));
                }
                index.sorted.sort_by(|left, right| left.0.cmp(&right.0));
                Ok(index)
            }
        }

        deserializer.deserialize_map(IndexVisitor)
    }
}

/// Decode [`to_compact_value`] output from text (`None` on any mismatch).
#[must_use]
pub fn from_compact_str(text: &str) -> Option<RevIndex> {
    serde_json::from_str(text).ok()
}

/// Portal shard count for `build-index --shards` / `build-site`: one shard
/// answers any query, so list search holds only hot shards (LRU).
pub const PORTAL_SHARDS: u32 = 256;

/// FNV-1a 32-bit over UTF-8 bytes: shard assignment shared with the portal
/// JS (`TextEncoder` + same loop). Deterministic across builds and targets.
#[must_use]
pub const fn fnv1a32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

/// Shard index for a form (`shards` must be leniently > 0).
#[must_use]
pub fn shard_of(form: &str, shards: u32) -> u32 {
    fnv1a32(form.as_bytes()) % shards.max(1)
}

/// Partition an index into `shards` per-form buckets (hash sharding).
/// Each bucket is a standalone finished `RevIndex`; the union is the input.
#[must_use]
pub fn partition(mut index: RevIndex, shards: u32) -> Vec<RevIndex> {
    index.finish();
    let count = shards.max(1) as usize;
    let mut buckets = Vec::with_capacity(count);
    for _ in 0..count {
        buckets.push(RevIndex::default());
    }
    for slot in &index.sorted {
        let shard = shard_of(&slot.0, shards.max(1)) as usize;
        for entry in &slot.1 {
            buckets[shard].insert(slot.0.to_string(), &index.materialize(entry));
        }
    }
    for bucket in &mut buckets {
        bucket.finish();
    }
    buckets
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_vectors() {
        // Standard FNV-1a 32-bit vectors (must match the portal JS loop).
        assert_eq!(fnv1a32(b""), 0x811c_9dc5);
        assert_eq!(fnv1a32(b"a"), 0xe40c_292c);
        assert_eq!(fnv1a32(b"foobar"), 0xbf9c_f968);
    }

    #[test]
    fn portal_shard_count_pinned() {
        // `web/app.js` SHARDS must equal this.
        assert_eq!(PORTAL_SHARDS, 256);
    }

    #[test]
    fn partition_covers_union() {
        let mut index = RevIndex::default();
        index.insert(
            "भवति".to_string(),
            &Posting {
                baseindex: "01.0001".to_string(),
                loc: LocKind::Ting {
                    family: "shuddha".to_string(),
                    prayoga: "kartari".to_string(),
                    lakara: "plat".to_string(),
                    pos: 0,
                },
            },
        );
        index.insert(
            "भवतः".to_string(),
            &Posting {
                baseindex: "01.0001".to_string(),
                loc: LocKind::Ting {
                    family: "shuddha".to_string(),
                    prayoga: "kartari".to_string(),
                    lakara: "plat".to_string(),
                    pos: 1,
                },
            },
        );
        let buckets = partition(index, 64);
        assert_eq!(buckets.len(), 64);
        let total: usize = buckets.iter().map(RevIndex::postings).sum();
        assert_eq!(total, 2);
        let shard = shard_of("भवति", 64) as usize;
        assert_eq!(buckets[shard].len(), 1);
        assert_eq!(buckets[shard].get("भवति").len(), 1);
        assert!(buckets[shard].get("भवतः").is_empty());
    }
}

/// Deterministic audit sampler: `stride == 1` takes everything.
pub struct Sampler {
    stride: u64,
    count: u64,
}

impl Sampler {
    /// Take every `stride`-th item (`stride == 1` = exhaustive, `0` = 1).
    #[must_use]
    pub fn new(stride: u64) -> Self {
        Self {
            stride: stride.max(1),
            count: 0,
        }
    }

    /// True when the current item should be checked.
    pub fn take(&mut self) -> bool {
        let take = self.count.is_multiple_of(self.stride);
        self.count += 1;
        take
    }
}

/// Index one ting map; optionally collect `(loc, deva-form)` per dhatu for
/// the prefixed stage.
pub fn index_ting_map<S: ::std::hash::BuildHasher>(
    idx: &mut RevIndex,
    map: &crate::forms::TingMap,
    family: &str,
    prayoga: &str,
    mut per_dhatu: Option<&mut DhatuForms<S>>,
    stats: &mut Stats,
) {
    let mut ids: Vec<&String> = map.keys().collect();
    ids.sort();
    for id in ids {
        let mut lakaras: Vec<&String> = map[id].keys().collect();
        lakaras.sort();
        for lakara in lakaras {
            for (pos, slot) in split_slots(&map[id][lakara]).iter().enumerate() {
                for variant in split_variants(slot) {
                    let loc = LocKind::Ting {
                        family: family.to_string(),
                        prayoga: prayoga.to_string(),
                        lakara: (*lakara).clone(),
                        pos,
                    };
                    idx.insert(
                        (*variant).to_string(),
                        &Posting {
                            baseindex: (*id).clone(),
                            loc: loc.clone(),
                        },
                    );
                    if let Some(collector) = per_dhatu.as_mut() {
                        collector
                            .entry((*id).clone())
                            .or_default()
                            .push((loc, (*variant).to_string()));
                    }
                    stats.ting_forms += 1;
                }
            }
        }
    }
}

/// Index one krut map (`;`-rows × `field` ∈ base/m/f/n × `/`-alternants);
/// optionally collects per-dhatu forms for the prefixed stage.
pub fn index_krut_map<S: ::std::hash::BuildHasher>(
    idx: &mut RevIndex,
    map: &crate::forms::KrutMap,
    family: &str,
    mut per_dhatu: Option<&mut DhatuForms<S>>,
    stats: &mut Stats,
) {
    let mut ids: Vec<&String> = map.keys().collect();
    ids.sort();
    for id in ids {
        let mut krts: Vec<&String> = map[id].keys().collect();
        krts.sort();
        for krt in krts {
            for (row, cell) in parse_krt_rows(&map[id][krt]).iter().enumerate() {
                let values = [&cell.base, &cell.m, &cell.f, &cell.n];
                for (field, value) in KRUT_FIELDS.iter().zip(values) {
                    for alternant in split_alternants(value) {
                        let loc = LocKind::Krt {
                            family: family.to_string(),
                            krt: (*krt).clone(),
                            row,
                            field: (*field).to_string(),
                        };
                        idx.insert(
                            (*alternant).to_string(),
                            &Posting {
                                baseindex: (*id).clone(),
                                loc: loc.clone(),
                            },
                        );
                        if let Some(collector) = per_dhatu.as_mut() {
                            collector
                                .entry((*id).clone())
                                .or_default()
                                .push((loc, (*alternant).to_string()));
                        }
                        stats.krt_forms += 1;
                    }
                }
            }
        }
    }
}

/// Index prefixed surfaces for one dhatu: every base form × `stack_deva`.
/// SLP1-only inside; failures are counted, never fatal. Context comes from
/// the caller per base form (family/category + lexical flags differ).
pub fn index_prefixed_dhatu(
    idx: &mut RevIndex,
    baseindex: &str,
    stack_deva: &str,
    forms: &[(LocKind, String)],
    ctx_for: &dyn Fn(&LocKind) -> GenCtx,
    stats: &mut Stats,
) {
    let Some(stack_slp1) = deva_stack_to_slp1(stack_deva) else {
        Stats::note(
            &mut stats.skip_deva,
            &mut stats.skip_samples,
            format!("prefix stack {baseindex} {stack_deva}"),
        );
        return;
    };
    for (inner, deva) in forms {
        let ctx = ctx_for(inner);
        // Kṛt-aware normalization shared with the portal (see `prefix_krut`):
        // ल्यप् cells ship with a baked stack (strip it, prefix the selected
        // one or the index carries wrong stacked forms and misses the right
        // ones), and neuter `t`-finals carry their `d` twin for recall.
        // (Declined inflections come from the separate declined fold, which
        // already emits complete finals — no declension here.)
        let (krt_name, field) = match inner {
            LocKind::Krt { krt, field, .. } => (krt.as_str(), field.as_str()),
            _ => ("", ""),
        };
        let Some(base_slp1) = deva_to_slp1(deva) else {
            Stats::note(
                &mut stats.skip_deva,
                &mut stats.skip_samples,
                format!("prefix base {baseindex} {stack_deva} {deva}"),
            );
            continue;
        };
        let base = if krt_name == "ल्यप्" {
            crate::prefix_krut::strip_baked_stack(&base_slp1).to_string()
        } else {
            base_slp1
        };
        let mut surfaces = generate_prefixed(&stack_slp1, &base, ctx);
        if field == "n" {
            // Neuter dental twin (see `prefix_krut` docs).
            let mut extra = Vec::new();
            for surface in &surfaces {
                if surface.ends_with('t') {
                    let twin = format!("{}d", &surface[..surface.len() - 1]);
                    if !surfaces.contains(&twin) && !extra.contains(&twin) {
                        extra.push(twin);
                    }
                }
            }
            surfaces.extend(extra);
        }
        for surface in surfaces {
            let surface: String = surface;
            let Some(key) = slp1_to_deva(&surface) else {
                Stats::note(
                    &mut stats.skip_slp1,
                    &mut stats.skip_samples,
                    format!("prefix surface {baseindex} {stack_deva} {surface}"),
                );
                continue;
            };
            idx.insert(
                key,
                &Posting {
                    baseindex: baseindex.to_string(),
                    loc: LocKind::Prefixed {
                        stack: stack_deva.to_string(),
                        inner: Box::new(inner.clone()),
                    },
                },
            );
            stats.prefixed_forms += 1;
        }
    }
}

/// One krt row-field value to decline into a subanta paradigm.
pub struct SubantaCell<'a> {
    /// Canonical baseindex.
    pub baseindex: &'a str,
    /// Dhatuforms family (`shuddha`, …).
    pub family: &'a str,
    /// Krt name in Devanagari (e.g. `शतृ`).
    pub krt_deva: &'a str,
    /// `;`-row index inside the cell.
    pub row: usize,
    /// Cell field (`m`, `f`, `n` or `base`).
    pub field: &'a str,
    /// Cell value in Devanagari (`/`-alternants split inside).
    pub value_deva: &'a str,
}

/// Index the full subanta paradigm(s) for one krt row-field value.
/// `family` is the dhatuforms name (`shuddha`, …); `field` ∈ m/f/n/base.
pub fn index_subanta_cell(idx: &mut RevIndex, cell: &SubantaCell<'_>, stats: &mut Stats) {
    for (loc, form) in subanta_pairs(
        cell.baseindex,
        cell.family,
        cell.krt_deva,
        cell.row,
        cell.field,
        cell.value_deva,
        stats,
    ) {
        idx.insert(
            form,
            &Posting {
                baseindex: cell.baseindex.to_string(),
                loc,
            },
        );
        stats.subanta_forms += 1;
    }
}
/// Decline one krt row-field value into `(Subanta loc, deva surface)` pairs.
/// Shared by plain subanta indexing and prefixed-subanta generation;
/// failures are counted in `stats`, never fatal.
pub fn subanta_pairs(
    baseindex: &str,
    family: &str,
    krt_deva: &str,
    row: usize,
    field: &str,
    value_deva: &str,
    stats: &mut Stats,
) -> Vec<(LocKind, String)> {
    let mut out = Vec::new();
    let Some(engine_family) = engine_krut_family(family) else {
        Stats::note(
            &mut stats.skip_table,
            &mut stats.skip_samples,
            format!("subanta family {baseindex} {family}"),
        );
        return out;
    };
    // Only `kvasu`/`tfc` branches read the krt name; dashed names
    // (`स्य-शतृ`) transliterate to `None` and safely fall back.
    let krt_slp = deva_to_slp1(krt_deva).unwrap_or_default();
    for alternant in split_alternants(value_deva) {
        let Some(value_slp) = deva_to_slp1(alternant) else {
            Stats::note(
                &mut stats.skip_deva,
                &mut stats.skip_samples,
                format!("subanta value {baseindex} {family} {krt_deva}[{row}].{field} {alternant}"),
            );
            continue;
        };
        let Some(paradigm) = krt_table(engine_family, &krt_slp, field, &value_slp) else {
            Stats::note(
                &mut stats.skip_table,
                &mut stats.skip_samples,
                format!("subanta table {baseindex} {family} {krt_deva}[{row}].{field} {alternant}"),
            );
            continue;
        };
        for slot in &paradigm.cells {
            for form in &slot.forms {
                let Some(key) = slp1_to_deva(form) else {
                    Stats::note(
                        &mut stats.skip_slp1,
                        &mut stats.skip_samples,
                        format!(
                            "subanta key {baseindex} {family} {krt_deva}[{row}].{field} {form}"
                        ),
                    );
                    continue;
                };
                out.push((
                    LocKind::Subanta {
                        family: family.to_string(),
                        krt: krt_deva.to_string(),
                        row,
                        field: field.to_string(),
                        vibhakti: slot.vibhakti.to_string(),
                        vacana: slot.vacana.to_string(),
                    },
                    key,
                ));
            }
        }
    }
    out
}

/// Loaded audit input: parsed family tables + dhatu metadata.
pub struct AuditData {
    /// `(family, prayoga, map)` for the 10 ting files.
    pub ting: Vec<(String, String, crate::forms::TingMap)>,
    /// `(family, map)` for the 5 krut files.
    pub krut: Vec<(String, crate::forms::KrutMap)>,
    /// `dhatu/data.txt` (provides per-dhatu upasarga stacks).
    pub dhatu: crate::DhatuFile,
    /// `baseindex -> SLP1 dhatu` join (`vidyut_dhatupatha_5.tsv`).
    pub vidyut: HashMap<String, String>,
}

/// Audit tuning: strides of 1 take everything.
pub struct AuditOptions {
    /// Check every Nth base ting variant.
    pub ting_stride: u64,
    /// Check every Nth base krt alternant.
    pub krt_stride: u64,
    /// Generate prefixed surfaces for every Nth base form.
    pub prefixed_stride: u64,
    /// Audit every Nth declinable krt cell for subanta.
    pub subanta_stride: u64,
}

impl AuditOptions {
    /// Fast tier for `cargo test`.
    #[must_use]
    pub const fn sampled() -> Self {
        Self {
            ting_stride: 37,
            krt_stride: 37,
            prefixed_stride: 13,
            subanta_stride: 37,
        }
    }

    /// Full tier for release/CI (`-- --ignored`).
    #[must_use]
    pub const fn full() -> Self {
        Self {
            ting_stride: 1,
            krt_stride: 1,
            prefixed_stride: 1,
            subanta_stride: 1,
        }
    }
}

/// Audit outcome; `misses` must be zero.
#[derive(Debug, Default)]
pub struct AuditReport {
    /// Location assertions evaluated.
    pub checked: u64,
    /// Lookups that did not contain the expected posting.
    pub misses: u64,
    /// First few miss descriptions (capped).
    pub miss_samples: Vec<String>,
    /// Builder coverage counters.
    pub stats: Stats,
}

impl AuditReport {
    fn miss(&mut self, what: String) {
        self.misses += 1;
        if self.miss_samples.len() < 10 {
            self.miss_samples.push(what);
        }
    }
}

/// Wanted dhatus: baseindexes having at least one upasarga stack.
fn wanted_dhatus(dhatu: &crate::DhatuFile) -> HashMap<String, Vec<String>> {
    let mut out = HashMap::new();
    for entry in &dhatu.data {
        if entry.upasargas.is_empty() {
            continue;
        }
        let stacks: Vec<String> = entry.upasargas.iter().map(|u| u.name.clone()).collect();
        out.insert(entry.baseindex.clone(), stacks);
    }
    out
}

/// Run the full reverse-search audit over loaded data.
///
/// Every checked form is looked up through [`RevIndex::get`] (the same call
/// the portal will use) and must contain its expected posting. Builders and
/// checkers share only the engine primitives and the [`Posting`] shape —
/// checkers re-derive every expected key independently.
#[must_use]
pub fn audit_all(data: &AuditData, opts: &AuditOptions) -> AuditReport {
    let mut report = AuditReport::default();
    let wanted = wanted_dhatus(&data.dhatu);
    // Per-dhatu base forms for wanted dhatus only (bounds memory).
    let mut per_dhatu: DhatuForms = HashMap::new();

    // Phase A: base tinanta.
    for (family, prayoga, map) in &data.ting {
        let mut idx = RevIndex::default();
        let mut sampler = Sampler::new(opts.ting_stride);
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for id in ids {
            let mut lakaras: Vec<&String> = map[id].keys().collect();
            lakaras.sort();
            for lakara in lakaras {
                for (pos, slot) in split_slots(&map[id][lakara]).iter().enumerate() {
                    for variant in split_variants(slot) {
                        let loc = LocKind::Ting {
                            family: family.clone(),
                            prayoga: prayoga.clone(),
                            lakara: (*lakara).clone(),
                            pos,
                        };
                        idx.insert(
                            (*variant).to_string(),
                            &Posting {
                                baseindex: (*id).clone(),
                                loc: loc.clone(),
                            },
                        );
                        report.stats.ting_forms += 1;
                        if wanted.contains_key(id.as_str()) {
                            per_dhatu
                                .entry((*id).clone())
                                .or_default()
                                .push((loc.clone(), (*variant).to_string()));
                        }
                        if sampler.take() {
                            report.checked += 1;
                            let expected = Posting {
                                baseindex: (*id).clone(),
                                loc,
                            };
                            if !idx.get(variant).contains(&expected) {
                                report.miss(format!(
                                    "ting {id} {family}/{prayoga} {lakara}[{pos}] {variant}"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Phase B: base krdanta.
    for (family, map) in &data.krut {
        let mut idx = RevIndex::default();
        let mut sampler = Sampler::new(opts.krt_stride);
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for id in ids {
            let mut krts: Vec<&String> = map[id].keys().collect();
            krts.sort();
            for krt in krts {
                for (row, cell) in parse_krt_rows(&map[id][krt]).iter().enumerate() {
                    let values = [&cell.base, &cell.m, &cell.f, &cell.n];
                    for (field, value) in KRUT_FIELDS.iter().zip(values) {
                        for alternant in split_alternants(value) {
                            let loc = LocKind::Krt {
                                family: family.clone(),
                                krt: (*krt).clone(),
                                row,
                                field: (*field).to_string(),
                            };
                            idx.insert(
                                (*alternant).to_string(),
                                &Posting {
                                    baseindex: (*id).clone(),
                                    loc: loc.clone(),
                                },
                            );
                            report.stats.krt_forms += 1;
                            if wanted.contains_key(id.as_str()) {
                                per_dhatu
                                    .entry((*id).clone())
                                    .or_default()
                                    .push((loc.clone(), (*alternant).to_string()));
                            }
                            if sampler.take() {
                                report.checked += 1;
                                let expected = Posting {
                                    baseindex: (*id).clone(),
                                    loc,
                                };
                                if !idx.get(alternant).contains(&expected) {
                                    report.miss(format!(
                                        "krut {id} {family} {krt}[{row}].{field} {alternant}"
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Phase C: prefixed surfaces (sampled generation, independent re-check).
    {
        let mut idx = RevIndex::default();
        let mut sampler = Sampler::new(opts.prefixed_stride);
        let notes: HashMap<&str, &str> = data
            .dhatu
            .data
            .iter()
            .map(|entry| (entry.baseindex.as_str(), entry.dhatu_note.as_str()))
            .collect();
        let mut ids: Vec<&String> = wanted.keys().collect();
        ids.sort();
        for id in ids {
            let Some(forms) = per_dhatu.get(id) else {
                report.stats.skip_no_base += 1;
                continue;
            };
            let Some(stacks) = wanted.get(id) else {
                continue;
            };
            let vidyut = data.vidyut.get(id.as_str()).map(String::as_str);
            let note = notes.get(id.as_str()).copied().unwrap_or("");
            let ctx_for = |inner: &LocKind| crate::genctx::ctx_for_loc(vidyut, note, inner);
            for stack_deva in stacks {
                for (inner, deva) in forms {
                    if !sampler.take() {
                        continue;
                    }
                    // Independent re-derivation through the raw engine, with
                    // the two corpus-pinned normalizations the index applies
                    // before storing (ल्यप् baked-stack strip, neuter dental
                    // twin — see `prefix_krut` and `prefixed_krut_audit`).
                    let Some(stack_slp1) = deva_stack_to_slp1(stack_deva) else {
                        continue;
                    };
                    let Some(raw_slp1) = deva_to_slp1(deva) else {
                        continue;
                    };
                    let base_slp1 = match inner {
                        LocKind::Krt { krt, .. } if krt == "ल्यप्" => {
                            crate::prefix_krut::strip_baked_stack(&raw_slp1).to_string()
                        }
                        _ => raw_slp1,
                    };
                    let mut surfaces = generate_prefixed(&stack_slp1, &base_slp1, ctx_for(inner));
                    if let LocKind::Krt { field, .. } = inner {
                        if field == "n" {
                            let twins: Vec<String> = surfaces
                                .iter()
                                .filter(|s| s.ends_with('t'))
                                .map(|s| format!("{}d", &s[..s.len() - 1]))
                                .filter(|t| !surfaces.contains(t))
                                .collect();
                            surfaces.extend(twins);
                        }
                    }
                    let keys: Vec<String> = surfaces
                        .iter()
                        .filter_map(|surface| slp1_to_deva(surface))
                        .collect();
                    index_prefixed_dhatu(
                        &mut idx,
                        id,
                        stack_deva,
                        std::slice::from_ref(&(inner.clone(), deva.clone())),
                        &ctx_for,
                        &mut report.stats,
                    );
                    for key in &keys {
                        report.checked += 1;
                        let expected = Posting {
                            baseindex: (*id).clone(),
                            loc: LocKind::Prefixed {
                                stack: stack_deva.clone(),
                                inner: Box::new(inner.clone()),
                            },
                        };
                        if !idx.get(key).contains(&expected) {
                            report.miss(format!("prefixed {id} {stack_deva} {deva} -> {key}"));
                        }
                    }
                }
            }
        }
    }

    // Phase D: subanta paradigms (sampled cells, all cells verified).
    for (family, map) in &data.krut {
        let Some(engine_family) = engine_krut_family(family) else {
            continue;
        };
        let mut idx = RevIndex::default();
        let mut sampler = Sampler::new(opts.subanta_stride);
        let mut ids: Vec<&String> = map.keys().collect();
        ids.sort();
        for id in ids {
            let mut krts: Vec<&String> = map[id].keys().collect();
            krts.sort();
            for krt in krts {
                for (row, cell) in parse_krt_rows(&map[id][krt]).iter().enumerate() {
                    for (field, value) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                        if value.is_empty() || !sampler.take() {
                            continue;
                        }
                        index_subanta_cell(
                            &mut idx,
                            &SubantaCell {
                                baseindex: id,
                                family,
                                krt_deva: krt,
                                row,
                                field,
                                value_deva: value,
                            },
                            &mut report.stats,
                        );
                        // Independent re-derivation: every paradigm cell must resolve.
                        let krt_slp = deva_to_slp1(krt).unwrap_or_default();
                        for alternant in split_alternants(value) {
                            let Some(value_slp) = deva_to_slp1(alternant) else {
                                continue;
                            };
                            let Some(paradigm) =
                                krt_table(engine_family, &krt_slp, field, &value_slp)
                            else {
                                continue;
                            };
                            for slot in &paradigm.cells {
                                for form in &slot.forms {
                                    let Some(key) = slp1_to_deva(form) else {
                                        continue;
                                    };
                                    report.checked += 1;
                                    let expected = Posting {
                                        baseindex: (*id).clone(),
                                        loc: LocKind::Subanta {
                                            family: family.clone(),
                                            krt: (*krt).clone(),
                                            row,
                                            field: field.to_string(),
                                            vibhakti: slot.vibhakti.to_string(),
                                            vacana: slot.vacana.to_string(),
                                        },
                                    };
                                    if !idx.get(&key).contains(&expected) {
                                        report.miss(format!(
                                            "subanta {id} {family} {krt}[{row}].{field} {key}"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Phase E: prefixed declined surfaces (wanted dhatus, sampled cells).
    // Every paradigm surface × every stack must resolve with its stack.
    {
        let mut idx = RevIndex::default();
        let mut sampler = Sampler::new(opts.subanta_stride);
        let notes: HashMap<&str, &str> = data
            .dhatu
            .data
            .iter()
            .map(|entry| (entry.baseindex.as_str(), entry.dhatu_note.as_str()))
            .collect();
        let mut ids: Vec<&String> = wanted.keys().collect();
        ids.sort();
        for id in ids {
            let Some(stacks) = wanted.get(id.as_str()) else {
                continue;
            };
            let vidyut = data.vidyut.get(id.as_str()).map(String::as_str);
            let note = notes.get(id.as_str()).copied().unwrap_or("");
            let ctx_for = |inner: &LocKind| crate::genctx::ctx_for_loc(vidyut, note, inner);
            for (family, map) in &data.krut {
                let Some(entry) = map.get(id) else {
                    continue;
                };
                let mut krts: Vec<&String> = entry.keys().collect();
                krts.sort();
                for krt in krts {
                    for (row, cell) in parse_krt_rows(&entry[krt]).iter().enumerate() {
                        for (field, value) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                            if value.is_empty() || !sampler.take() {
                                continue;
                            }
                            let pairs = subanta_pairs(
                                id,
                                family,
                                krt,
                                row,
                                field,
                                value,
                                &mut report.stats,
                            );
                            if pairs.is_empty() {
                                continue;
                            }
                            for stack in stacks {
                                index_prefixed_dhatu(
                                    &mut idx,
                                    id,
                                    stack,
                                    &pairs,
                                    &ctx_for,
                                    &mut report.stats,
                                );
                            }
                            // Independent re-derivation per stack.
                            let Some(stack_slps) = stacks
                                .iter()
                                .map(|stack| deva_stack_to_slp1(stack))
                                .collect::<Option<Vec<_>>>()
                            else {
                                continue;
                            };
                            for (inner, deva) in &pairs {
                                let Some(base_slp1) = deva_to_slp1(deva) else {
                                    continue;
                                };
                                for (stack, stack_slp1) in stacks.iter().zip(stack_slps.iter()) {
                                    for surface in
                                        generate_prefixed(stack_slp1, &base_slp1, ctx_for(inner))
                                    {
                                        let surface: String = surface;
                                        let Some(key) = slp1_to_deva(&surface) else {
                                            continue;
                                        };
                                        report.checked += 1;
                                        let expected = Posting {
                                            baseindex: (*id).clone(),
                                            loc: LocKind::Prefixed {
                                                stack: stack.clone(),
                                                inner: Box::new(inner.clone()),
                                            },
                                        };
                                        if !idx.get(&key).contains(&expected) {
                                            report.miss(format!(
                                                "prefixed-subanta {id} {stack} {deva} -> {key}"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    report
}
