//! Artifact freshness stamp: a hash over the engine sources that decide
//! what the built artifacts render (blobs, index shards, wasm engine).
//!
//! `build.rs` includes this file verbatim via `#[path]` and prints the
//! stamp into `DHATURUPA_ENGINE_STAMP`, so the CLI crate and the wasm
//! build both know the engine version they were built with; the
//! artifact writers (`build-blobs`, `build-index`) stamp their manifests
//! with the same value. The web shell compares its manifests against the
//! wasm export and warns when the served artifacts were built by a
//! different engine. A `manifest.json` *missing* is unknown freshness,
//! not staleness — the guard only fires on a disagreement.
//!
//! Deliberately excluded from the watch list: `lib.rs` (module wiring)
//! and `main.rs` (CLI harness) — they cannot change what an artifact
//! renders, and bumping the stamp when they do would nag every rebuild.
//! Includes this module must stay std-only: no `serde`, no crate deps.

/// Engine sources the artifacts' contents depend on, fingerprinted in
/// this exact order (stable, so the stamp is reproducible).
pub const ENGINE_SOURCES: &[&str] = &[
    "src/blobs.rs",
    "src/chandrika.rs",
    "src/derive_krut.rs",
    "src/derive_ting.rs",
    "src/engine/data.rs",
    "src/engine/index.rs",
    "src/engine/krt.rs",
    "src/engine/sandhi.rs",
    "src/engine/search.rs",
    "src/engine/subanta.rs",
    "src/engine/upasarga.rs",
    "src/engine/upasplit.rs",
    "src/forms.rs",
    "src/genctx.rs",
    "src/grammar.rs",
    "src/hk.rs",
    "src/morphdata.rs",
    "src/prefix_krut.rs",
    "src/revindex.rs",
    "src/slp1.rs",
    "src/translit.rs",
    "src/vidyut.rs",
    "src/wasm.rs",
];

/// FNV-1a fold of bytes into a running hash.
fn fold(mut state: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        state ^= u64::from(b);
        state = state.wrapping_mul(0x0000_0100_0000_01B3);
    }
    state
}

/// Stamp of the engine sources under `root`, form
/// `v1:<16-hex fnv1a>:<total bytes>`, or `None` when any watched file
/// is missing/unreadable — callers must treat that as a hard error; a
/// stamp of nothing proves nothing. The path pins file identity, the
/// bytes pin content: a rename alone still moves the stamp.
pub fn fingerprint(root: &std::path::Path) -> Option<String> {
    fingerprint_files(root, ENGINE_SOURCES.iter().copied())
}

/// The same ordered fold over any file list (unit tests drive the drift
/// case against a two-file tempdir instead of the full engine tree).
pub fn fingerprint_files(
    root: &std::path::Path,
    files: impl IntoIterator<Item = impl AsRef<str>>,
) -> Option<String> {
    let mut state = 0xcbf2_9ce4_8422_2325_u64;
    let mut total = 0_u64;
    for rel in files {
        let rel = rel.as_ref();
        let bytes = std::fs::read(root.join(rel)).ok()?;
        total += bytes.len() as u64;
        state = fold(state, rel.as_bytes());
        state = fold(state, &bytes);
    }
    Some(format!("v1:{state:016x}:{total}"))
}

/// A stored manifest is stale iff it names a different stamp. Blank
/// stored stamps never match (they would prove nothing).
#[allow(dead_code)] // the build-script compilation itself never calls this
pub fn is_stale(stored: &str, current: &str) -> bool {
    !stored.is_empty() && stored != current
}

use std::fmt::Write as _;

/// Hand-rolled manifest JSON (build.rs cannot use serde): flat string
/// fields, emitted in stable key order. Values must be pre-rendered.
#[allow(dead_code)] // the build-script compilation itself never calls these
pub fn manifest_json(kind: &str, fields: &[(&str, String)]) -> String {
    let mut out = String::from("{\n");
    let _ = write!(out, "  \"kind\": {}", json_str(kind));
    for (key, value) in fields {
        // json_str already emits the quotes
        let _ = write!(out, ",\n  {}: {}", json_str(key), value);
    }
    out.push_str("\n}\n");
    out
}

/// One manifest value: quote + escape as a JSON string.
#[allow(dead_code)] // same as `manifest_json`
pub fn manifest_str(value: &str) -> String {
    json_str(value)
}

#[allow(dead_code)]
fn json_str(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_changes_the_fingerprint() {
        let root = std::env::temp_dir().join("dhatu-stamp-drift");
        let sub = root.join("src");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("forms.rs"), b"echo one").unwrap();
        let files = ["src/forms.rs"];
        let before = fingerprint_files(&root, files).expect("two watched files");
        std::fs::write(sub.join("forms.rs"), b"echo one // engine change").unwrap();
        let after = fingerprint_files(&root, files).expect("still watched");
        assert!(before != after, "an engine source edit must move the stamp");
        assert!(is_stale(&after, &before), "old artifacts read as stale");
        assert!(!is_stale(&after, &after), "rebuilt artifacts read fresh");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_source_is_a_hard_error() {
        let root = std::env::temp_dir().join("dhatu-stamp-missing");
        std::fs::create_dir_all(&root).unwrap();
        let files = ["src/forms.rs"];
        assert!(
            fingerprint_files(&root, files).is_none(),
            "no fingerprint when a watched file is absent"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn watched_files_all_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for rel in ENGINE_SOURCES {
            assert!(
                root.join(rel).is_file(),
                "watched source '{rel}' is gone — update ENGINE_SOURCES"
            );
        }
    }

    #[test]
    fn manifest_json_shape() {
        let m = manifest_json(
            "dhaturupa-blobs",
            &[("stamp", manifest_str("v1:abc")), ("count", "7".into())],
        );
        assert_eq!(
            m,
            "{\n  \"kind\": \"dhaturupa-blobs\",\n  \"stamp\": \"v1:abc\",\n  \"count\": 7\n}\n"
        );
    }
}
