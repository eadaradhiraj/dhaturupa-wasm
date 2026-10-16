//! Vendored SLP1 generation engine (from `panini-rt/morph/src`).
//!
//! `slp1`/`grammar` resolve to the crate-root ports (identical logic).
//! Everything else lives under `crate::engine::*` so the ashtadhyayi
//! metadata model (`crate::DhatuFile`) never collides with the corpus
//! model (`crate::engine::data::DhatuFile`).
//!
//! `api.rs` (WASM/JSON glue) is intentionally not vendored.

pub mod data;
pub mod index;
pub mod krt;
pub mod sandhi;
pub mod search;
pub mod subanta;
pub mod upasarga;
pub mod upasplit;

pub use data::{load_corpus, load_dir, DhatuFile as CorpusFile, LoadError as CorpusLoadError};
pub use index::{build_krt_index, build_tinanta_index};
pub use search::{hit_ids, search_exact};
