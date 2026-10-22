//! Artifact freshness guard: the compiled `dhaturupa_wasm::ENGINE_STAMP`
//! is a hash over `stamp::ENGINE_SOURCES`; every served artifact family
//! carries `manifest.json` naming the engine stamp its builder ran with.
//!
//! - `build_stamp_matches_current_engine_sources` reds if build.rs and
//!   the stamp module ever disagree (wiring broken, list not watched).
//! - `served_manifests_match_the_engine` reds when dist artifacts exist
//!   but were built by another engine version — that is the "rebuild the
//!   heavy artifacts by hand before calling the engine change done"
//!   reminder, enforced instead of remembered. A missing manifest is
//!   *unknown* freshness (skip with notice, never a false red).

use dhaturupa_wasm::stamp;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn project_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

#[test]
fn build_stamp_matches_current_engine_sources() {
    let fresh = stamp::fingerprint(project_root()).expect("engine sources all present");
    assert_eq!(
        dhaturupa_wasm::ENGINE_STAMP,
        fresh,
        "compiled stamp drifted from the sources — build.rs wiring is broken"
    );
    assert!(
        dhaturupa_wasm::ENGINE_STAMP.starts_with("v1:"),
        "stamp must carry its format version"
    );
}

/// Served artifact families this portal fetches (paths under the repo).
fn artifact_families() -> Vec<(&'static str, PathBuf)> {
    let root = project_root();
    vec![
        ("blobs", root.join("dist/data/blobs/manifest.json")),
        ("shards", root.join("dist/data/shards/manifest.json")),
    ]
}

#[test]
fn served_manifests_match_the_engine() {
    let mut checked = 0;
    for (family, manifest_path) in artifact_families() {
        let Ok(text) = std::fs::read_to_string(&manifest_path) else {
            eprintln!(
                "notice: no {family} manifest at {} — artifacts not rebuilt yet, nothing to guard",
                manifest_path.display()
            );
            continue;
        };
        checked += 1;
        let stamp_field = stamp_field_of(&text).unwrap_or_else(|| {
            panic!(
                "{family} manifest at {} has no stamp field — hand-edited or corrupt",
                manifest_path.display()
            )
        });
        assert!(
            !stamp::is_stale(&stamp_field, dhaturupa_wasm::ENGINE_STAMP),
            "{family} artifacts at {} are stale: built with {stamp_field}, engine is {} — \
             rebuild (build-blobs / build-index) before releasing",
            manifest_path.parent().unwrap_or(Path::new(".")).display(),
            dhaturupa_wasm::ENGINE_STAMP
        );
    }
    if checked > 0 {
        assert!(
            checked == artifact_families().len(),
            "saw {checked}/{0} manifests — half-stamped state is not shippable",
            artifact_families().len()
        );
    }
    eprintln!("artifact_stamp: {checked} manifest(s) checked fresh");
}

/// The `stamp` field from manifest text without extra parsing
/// machinery: find `"stamp": "…"` in the shape the writer emits.
fn stamp_field_of(text: &str) -> Option<String> {
    let key = "\"stamp\": \"";
    let at = text.find(key)? + key.len();
    let end = text[at..].find('"')? + at;
    Some(text[at..end].to_string())
}
