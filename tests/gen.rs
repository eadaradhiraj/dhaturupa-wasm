//! Generation vectors: subanta, krdanta click-through, upasarga prefixing (SLP1).

use dhaturupa_wasm::engine::subanta::{generate_tables, krt_table, Linga};
use dhaturupa_wasm::engine::upasarga::{generate_prefixed, GenCtx};

#[test]
fn subanta_rama_contains_instrumental() {
    let tables = generate_tables("rAma", Linga::M);
    assert!(!tables.is_empty());
    assert!(tables.iter().any(|t| t.contains("rAmeRa")));
}

#[test]
fn subanta_invalid_is_empty() {
    // Raw `generate_tables` has no lexical gate (that is `generate_tables_checked`
    // with a krt map in the full harness): truly malformed stems yield nothing.
    assert!(generate_tables("", Linga::M).is_empty());
    assert!(generate_tables("rAma123", Linga::M).is_empty());
    assert!(!generate_tables("BUti", Linga::F).is_empty());
}

#[test]
fn krt_click_through_bavanti() {
    let t = krt_table("krut", "Satf", "f", "BavantI").expect("BavantI f table");
    assert!(t.contains("BavantIByAm"));
}

#[test]
fn krt_visarga_citations_decline() {
    // `ktin` feminine `-iH` citation: visarga is citation spelling, the
    // declension stem is bare (`mutti`).
    let t = krt_table("krut", "ktin", "f", "muttiH").expect("muttiH f table");
    assert!(t.contains("muttyOH"));
    // `Nvul` masculine `-aH` citation still rides the a-stem.
    let u = krt_table("krut", "Rvul", "m", "BAvakaH").expect("BAvakaH m table");
    assert!(u.contains("BAvakam"));
}

#[test]
fn prefix_pra_bavati() {
    let out = generate_prefixed("pra", "Bavati", GenCtx::default());
    assert!(out.iter().any(|f| f == "praBavati"));
}

#[test]
fn prefix_stack_folds_with_sandhi() {
    let out = generate_prefixed("vi;ati", "Bavati", GenCtx::default());
    assert!(out.iter().any(|f| f == "vyatiBavati"));
}
