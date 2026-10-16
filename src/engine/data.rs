//! Data model for `skt-morph-data/**/*.json` (SPEC §4).
//!
//! Host-only loader (uses `std::fs` + `rayon`). Future WASM engine core
//! (`slp1`, generators) stays `alloc`-only; the index itself is never
//! checked in — tests rebuild it from JSON.

use rayon::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// One `{name, value}` dhatu metadata row.
#[derive(Debug, Clone, Deserialize)]
pub struct InfoEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: String,
}

/// One krdanta cell `{base,m,f,n}`. Empty string = absent for that linga.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct KrtCell {
    #[serde(default)]
    pub base: String,
    #[serde(default)]
    pub m: String,
    #[serde(default)]
    pub f: String,
    #[serde(default)]
    pub n: String,
}

impl KrtCell {
    /// Non-empty `(field, value)` pairs in `base,m,f,n` order.
    #[must_use]
    pub fn present_fields(&self) -> Vec<(&'static str, &str)> {
        let mut out = Vec::with_capacity(4);
        if !self.base.is_empty() {
            out.push(("base", self.base.as_str()));
        }
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

/// One upasarga lexicon row `{prefix, artha_hindi}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpasargaEntry {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub artha_hindi: String,
}

/// `upasarga: {dhatu, entries[]}`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Upasarga {
    #[serde(default)]
    pub dhatu: String,
    #[serde(default)]
    pub entries: Vec<UpasargaEntry>,
}

/// One `upasarga_forms.<stack>` value: precomputed prefixed forms.
/// Test oracle only (SPEC F3); never used as implementation.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpasargaFormData {
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub artha_hindi: String,
    #[serde(default)]
    pub conjugations: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub participles: BTreeMap<String, BTreeMap<String, Vec<KrtCell>>>,
}

/// Top-level dhatu JSON file.
#[derive(Debug, Clone, Deserialize)]
pub struct DhatuFile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub skipped: bool,
    #[serde(default)]
    pub skip_reason: String,
    #[serde(default)]
    pub info: Vec<InfoEntry>,
    #[serde(default)]
    pub conjugations: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    #[serde(default)]
    pub participles: BTreeMap<String, BTreeMap<String, Vec<KrtCell>>>,
    #[serde(default)]
    pub upasarga: Upasarga,
    #[serde(default)]
    pub upasarga_forms: BTreeMap<String, UpasargaFormData>,
}

impl DhatuFile {
    /// Raw `info[]` value for `name` (`""` when absent).
    #[must_use]
    pub fn info_value(&self, name: &str) -> &str {
        self.info
            .iter()
            .find(|e| e.name == name)
            .map_or("", |e| e.value.as_str())
    }

    /// Upadesha form (`OpadeSikasvarUpam`), e.g. `BU`.
    #[must_use]
    pub fn upadesha(&self) -> &str {
        self.info_value("OpadeSikasvarUpam")
    }

    /// Meaning (`arTaH`), e.g. `sattAyAm`.
    #[must_use]
    pub fn artha(&self) -> &str {
        self.info_value("arTaH")
    }

    /// Gana (`gaRaH`), e.g. `BvAdiH`.
    #[must_use]
    pub fn gana(&self) -> &str {
        self.info_value("gaRaH")
    }

    /// Padam (`padam`), e.g. `parasmEpadI`.
    #[must_use]
    pub fn padam(&self) -> &str {
        self.info_value("padam")
    }

    /// Iterate base `(category, lakara, form)` triples over `conjugations`.
    pub fn iter_conjugations(&self) -> impl Iterator<Item = (&str, &str, &str)> {
        self.conjugations.iter().flat_map(|(cat, sub)| {
            sub.iter()
                .flat_map(|(lak, arr)| arr.iter().map(|s| (cat.as_str(), lak.as_str(), s.as_str())))
        })
    }
}

/// Workspace-relative default data directory: `<crate>/../skt-morph-data`.
#[must_use]
pub fn default_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skt-morph-data")
}

/// Loader failure.
#[derive(Debug)]
pub enum LoadError {
    /// Directory walk / file read failure.
    Io { path: PathBuf, msg: String },
    /// JSON parse failure.
    Json { path: PathBuf, msg: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, msg } => write!(f, "io {}: {msg}", path.display()),
            Self::Json { path, msg } => write!(f, "json {}: {msg}", path.display()),
        }
    }
}

impl std::error::Error for LoadError {}

fn collect_json_files(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let mut stack = vec![dir.to_path_buf()];
    let mut out = Vec::new();
    while let Some(cur) = stack.pop() {
        let entries = std::fs::read_dir(&cur).map_err(|e| LoadError::Io {
            path: cur.clone(),
            msg: e.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| LoadError::Io {
                path: cur.clone(),
                msg: e.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "json") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn load_one(path: &Path) -> Result<DhatuFile, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io {
        path: path.to_path_buf(),
        msg: e.to_string(),
    })?;
    serde_json::from_slice(&bytes).map_err(|e| LoadError::Json {
        path: path.to_path_buf(),
        msg: e.to_string(),
    })
}

/// Load every `*.json` under `dir` (recursive, sorted by path).
///
/// Uses `rayon` for parallel JSON parsing; file discovery stays
/// single-threaded. Returns files in sorted-path order.
pub fn load_dir(dir: &Path) -> Result<Vec<DhatuFile>, LoadError> {
    let paths = collect_json_files(dir)?;
    paths
        .par_iter()
        .map(|p| load_one(p))
        .collect::<Result<Vec<DhatuFile>, LoadError>>()
}

/// Alias of [`load_dir`] kept for harness compatibility.
pub fn load_corpus(dir: &Path) -> Result<Vec<DhatuFile>, LoadError> {
    load_dir(dir)
}
