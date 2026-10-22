//! Compile-time artifact stamp: hash the engine sources into
//! `DHATURUPA_ENGINE_STAMP` so the CLI and wasm builds both know which
//! engine version produced their artifacts. See `src/stamp.rs`.

#[path = "src/stamp.rs"]
mod stamp;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in stamp::ENGINE_SOURCES {
        // Watched source edits (and only those) re-run this script and
        // refresh the stamp — the rest of the tree builds silently.
        println!("cargo:rerun-if-changed={rel}");
    }
    match stamp::fingerprint(root) {
        Some(stamped) => {
            println!("cargo:rustc-env=DHATURUPA_ENGINE_STAMP={stamped}");
        }
        // A watched file missing at build time is a broken tree, not a
        // thing to ship quietly.
        None => panic!("artifact stamp: watched engine source missing — see ENGINE_SOURCES"),
    }
}
