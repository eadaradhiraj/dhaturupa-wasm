//! Native tests for the WASM boundary (`src/wasm.rs`): same functions,
//! JSON envelopes, Devanagari in/out. Real data files where cheap.

use dhaturupa_wasm::wasm;
use serde_json::Value;

// Both index-cache tests share the process-wide WASM cache.
static CACHE_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn ok(text: &str) -> Value {
    let payload: Value = serde_json::from_str(text).expect("envelope parses");
    assert_eq!(payload["ok"], Value::Bool(true), "expected ok: {text}");
    payload
}

fn err(text: &str) -> String {
    let payload: Value = serde_json::from_str(text).expect("envelope parses");
    assert_eq!(payload["ok"], Value::Bool(false), "expected err: {text}");
    payload["error"]
        .as_str()
        .expect("error message")
        .to_string()
}

#[test]
fn query_candidates_deva_and_hk() {
    let payload = ok(&wasm::query_candidates("भवति"));
    assert_eq!(payload["deva"], serde_json::json!(["भवति"]));
    let payload = ok(&wasm::query_candidates("bhavati"));
    let deva = payload["deva"].as_array().expect("array");
    assert!(deva.iter().any(|v| v == "भवति"));
    // Exactly one reading each; raw SLP1 never surfaces in the frontend.
    let payload = ok(&wasm::query_candidates("gacchantau"));
    assert_eq!(
        payload["readings"],
        serde_json::json!([{"text": "गच्छन्तौ", "scheme": "Harvard-Kyoto"}])
    );
    assert!(!err(&wasm::query_candidates("Bavati")).is_empty());
    assert!(!err(&wasm::query_candidates(";;;")).is_empty());
}

#[test]
fn to_devanagari_normalizes_hk_for_list_filter() {
    // Devanagari echoes; Harvard-Kyoto decodes; raw SLP1 never surfaces.
    let payload = ok(&wasm::to_devanagari("दा"));
    assert_eq!(payload["deva"], "दा");
    let payload = ok(&wasm::to_devanagari("dA"));
    assert_eq!(payload["deva"], "दा");
    let payload = ok(&wasm::to_devanagari("gacchantI"));
    assert_eq!(payload["deva"], "गच्छन्ती");
    assert!(!err(&wasm::to_devanagari("Bavati")).is_empty());
    assert!(!err(&wasm::to_devanagari(";;;")).is_empty());
}

#[test]
fn dhatu_card_minimal() {
    let file = serde_json::json!({"name": "dhatu", "data": [{
        "i": "1010001", "baseindex": "01.0001", "dhatu": "भू",
        "aupadeshik": "भू", "gana": "1", "pada": "P", "settva": "S",
        "karma": "A", "artha": "सत्तायाम्", "artha_english": "to be",
        "artha_hindi": "होना", "tags": "", "sk": "1", "sk_sutra": "",
        "lsk_sutra": "", "dhaturoopnandini": "", "antarganas": "",
        "upasargas": [{"name": "प्र", "artha_hindi": "प्रकट होना"}],
    }]});
    let payload = ok(&wasm::dhatu_card(&file.to_string(), "1.1"));
    assert_eq!(payload["dhatu"], serde_json::json!("भू"));
    assert_eq!(payload["upasargas"][0]["name"], serde_json::json!("प्र"));
    assert!(!err(&wasm::dhatu_card(&file.to_string(), "99.9999")).is_empty());
    assert!(!err(&wasm::dhatu_card("nope", "01.0001")).is_empty());
}

#[test]
fn ting_and_krut_tables_minimal() {
    let ting = serde_json::json!({"01.0001": {"plat": "भवति;भवतः"}});
    let payload = ok(&wasm::ting_table(&ting.to_string(), "01.0001"));
    assert_eq!(
        payload["tables"]["plat"],
        serde_json::json!([["भवति"], ["भवतः"]])
    );
    assert!(!err(&wasm::ting_table(&ting.to_string(), "99.9999")).is_empty());
    let krut = serde_json::json!({"01.0001": {"ण्वुल्": "भावक,भावकः,भाविका,भावकम्"}});
    let payload = ok(&wasm::krut_table_json(&krut.to_string(), "01.0001"));
    assert_eq!(payload["tables"]["ण्वुल्"][0]["m"], serde_json::json!("भावकः"));
    assert!(!err(&wasm::krut_table_json("nope", "01.0001")).is_empty());
}

#[test]
fn subanta_table_deva() {
    let payload = ok(&wasm::subanta_table("राम", "m"));
    let tables = payload["tables"].as_array().expect("array");
    assert!(!tables.is_empty());
    let text = serde_json::to_string(&payload).expect("serializes");
    assert!(text.contains("रामेण"));
    assert!(!text.contains("rAmeRa"));
    assert_eq!(
        err(&wasm::subanta_table("राम", "x")),
        "linga must be m, f or n"
    );
    assert_eq!(
        wasm::subanta_table("rAma", "m"),
        wasm::subanta_table("राम", "m")
    );
}

#[test]
fn krt_subanta_deva() {
    let payload = ok(&wasm::krt_subanta("shuddha", "शतृ", "f", "भवन्ती", ""));
    // Portal contract: array under `tables`, like `subanta_table`.
    assert_eq!(payload["tables"][0]["stem"], serde_json::json!("भवन्ती"));
    let text = serde_json::to_string(&payload).expect("serializes");
    assert!(text.contains("भवन्तीभ्याम्"));
    assert!(!err(&wasm::krt_subanta("nope", "शतृ", "f", "भवन्ती", "")).is_empty());
    assert!(!err(&wasm::krt_subanta("shuddha", "शतृ", "x", "भवन्ती", "")).is_empty());
}

#[test]
fn prefixed_forms_deva() {
    let payload = ok(&wasm::prefixed_forms("वि + अति", "भवति"));
    assert_eq!(payload["forms"], serde_json::json!(["व्यतिभवति"]));
    let payload = ok(&wasm::prefixed_forms("pra", "Bavati"));
    assert_eq!(payload["forms"][0], serde_json::json!("प्रभवति"));
}

#[test]
fn prefixed_table_batch() {
    let payload: serde_json::Value =
        serde_json::from_str(&wasm::prefixed_table("प्र", r#"["भवति", "भव;ति"]"#))
            .expect("envelope parses");
    assert_eq!(payload["ok"], serde_json::json!(true));
    assert_eq!(payload["tables"][0], serde_json::json!(["प्रभवति"]));
    assert_eq!(payload["tables"][1], serde_json::json!([]));
    assert!(!payload.to_string().contains("praBavati"));
}

#[test]
fn engine_category_and_ctx_table() {
    let payload: serde_json::Value =
        serde_json::from_str(&wasm::engine_category("nich", "ting")).expect("parses");
    assert_eq!(payload["category"], serde_json::json!("nich"));
    let payload: serde_json::Value =
        serde_json::from_str(&wasm::engine_category("shuddha", "krut")).expect("parses");
    assert_eq!(payload["category"], serde_json::json!("krut"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&wasm::engine_category("nope", "ting"))
            .expect("parses")["ok"],
        serde_json::json!(false)
    );
    // Same stack+base, different context: `anu;AN` fuses to `avA` for
    // derived categories but stays `anvA` at root level.
    let forms = r#"["भवति"]"#;
    let root: serde_json::Value =
        serde_json::from_str(&wasm::prefixed_table_ctx("anu;AN", forms, "BU", "ting", ""))
            .expect("parses");
    let derived: serde_json::Value =
        serde_json::from_str(&wasm::prefixed_table_ctx("anu;AN", forms, "BU", "nich", ""))
            .expect("parses");
    assert_eq!(root["ok"], serde_json::json!(true));
    assert_eq!(derived["ok"], serde_json::json!(true));
    assert_ne!(root["tables"], derived["tables"]);
    assert!(root.to_string().contains("अन्वा") || root.to_string().contains("अन्व"));
    assert!(derived.to_string().contains("अवा"));
}

#[test]
fn krt_subanta_slash_alternants() {
    // Clicking a `/`-joined cell (e.g. शतृ n ददत्/ददद्) declines the first
    // workable alternant instead of erroring (portal onKrtClick path).
    let payload = ok(&wasm::krt_subanta("shuddha", "शतृ", "n", "ददत्/ददद्", ""));
    let text = serde_json::to_string(&payload).expect("serializes");
    assert!(text.contains("ददती"));
}

#[test]
fn family_index_roundtrip() {
    let _guard = CACHE_GUARD.lock().expect("guard");
    let kartari = serde_json::json!({"01.0001": {"plat": "भवति;भवतः"}}).to_string();
    let karmani = serde_json::json!({"01.0001": {"alat": "भवते"}}).to_string();
    let krut = serde_json::json!({"01.0001": {"ण्वुल्": "भावक,भावकः,,"}}).to_string();
    let payload = ok(&wasm::index_ensure_family(
        "shuddha", &kartari, &karmani, &krut,
    ));
    assert!(payload["forms"].as_u64().expect("count") >= 4);
    let payload = ok(&wasm::index_query_family("shuddha", "bhavati"));
    assert_eq!(payload["total"], serde_json::json!(1));
    assert_eq!(payload["hits"][0]["lakara"], serde_json::json!("plat"));
    let payload = ok(&wasm::index_query_family("shuddha", "भावकः"));
    assert_eq!(payload["hits"][0]["kind"], serde_json::json!("krt"));
    assert!(!err(&wasm::index_query_family("missing", "भवति")).is_empty());
    assert!(!err(&wasm::index_ensure_family(
        "nope", &kartari, &karmani, &krut
    ))
    .is_empty());
    let payload = ok(&wasm::index_drop_family("shuddha"));
    assert_eq!(payload["dropped"], serde_json::json!(true));
    assert!(!err(&wasm::index_query_family("shuddha", "भवति")).is_empty());
}

#[test]
fn index_query_all_merges_families() {
    let _guard = CACHE_GUARD.lock().expect("guard");
    let shuddha = serde_json::json!({"v": 1, "forms": {
        "भवति": [["01.0001", "t", "shuddha", "kartari", "plat", 0]],
    }})
    .to_string();
    let empty = serde_json::json!({"v": 1, "forms": {}}).to_string();
    ok(&wasm::index_ensure_all(
        &shuddha, &empty, &empty, &empty, &empty,
    ));
    let payload: serde_json::Value =
        serde_json::from_str(&wasm::index_query_all("bhavati")).expect("parses");
    assert_eq!(payload["total"], serde_json::json!(1));
    assert_eq!(
        payload["hits"][0]["baseindex"],
        serde_json::json!("01.0001")
    );
    for family in ["shuddha", "nich", "san", "yang", "yangluk"] {
        ok(&wasm::index_drop_family(family));
    }
    assert!(!wasm::index_query_all("bhavati").contains("\"ok\":true"));
}

#[test]
fn split_verify_unlisted_stack() {
    let _guard = CACHE_GUARD.lock().expect("guard");
    // Base index knows भवति; the splitter knows only प्र (unlisted for भू).
    let kartari = serde_json::json!({"01.0001": {"plat": "भवति;भवतः"}}).to_string();
    let karmani = serde_json::json!({}).to_string();
    let krut = serde_json::json!({}).to_string();
    ok(&wasm::index_ensure_family(
        "shuddha", &kartari, &karmani, &krut,
    ));
    ok(&wasm::split_init(r#"["प्र"]"#));
    let splits: serde_json::Value =
        serde_json::from_str(&wasm::split_query("प्रभवति", 64)).expect("parses");
    assert_eq!(splits["ok"], serde_json::json!(true));
    let pair = splits["splits"][0].clone();
    assert_eq!(pair["stack"], serde_json::json!("प्र"));
    assert_eq!(pair["base"], serde_json::json!("भवति"));
    let hits: serde_json::Value = serde_json::from_str(&wasm::verify_split(
        "प्रभवति",
        pair["stack"].as_str().expect("stack"),
        pair["base"].as_str().expect("base"),
    ))
    .expect("parses");
    assert_eq!(hits["total"], serde_json::json!(1));
    assert_eq!(hits["hits"][0]["kind"], serde_json::json!("prefixed"));
    assert_eq!(hits["hits"][0]["stack"], serde_json::json!("प्र"));
    ok(&wasm::index_drop_family("shuddha"));
}

#[test]
fn split_verify_declined_subanta_base() {
    let _guard = CACHE_GUARD.lock().expect("guard");
    // Declined prefixed kṛdanta forms (प्रददद्भिः = प्र + ददद्भिः) must
    // verify against the subanta set, not just tiṇanta/kṛdanta cells.
    let shard = serde_json::to_string(&serde_json::json!({
        "v": 1,
        "forms": {
            "ददद्भिः": [["03.0010", "s", "shuddha", "शतृ", 0, "m", "tftIyA", "bahuvachanam"]],
        },
    }))
    .expect("json");
    // Key-namespaced portal shard: base set first …
    ok(&wasm::index_load_key("b0", &shard));
    ok(&wasm::split_init(r#"["प्र"]"#));
    let splits: serde_json::Value =
        serde_json::from_str(&wasm::split_query("प्रददद्भिः", 1)).expect("parses");
    let pair = splits["splits"]
        .as_array()
        .expect("splits")
        .iter()
        .find(|s| s["base"] == "ददद्भिः")
        .expect("ददद्भिः split proposed")
        .clone();
    // … then the subanta set under its own key (the portal loads both).
    ok(&wasm::index_load_key("u0", &shard));
    let hits: serde_json::Value = serde_json::from_str(&wasm::verify_split(
        "प्रददद्भिः",
        pair["stack"].as_str().expect("stack"),
        pair["base"].as_str().expect("base"),
    ))
    .expect("parses");
    assert_eq!(hits["total"], serde_json::json!(1));
    assert_eq!(hits["hits"][0]["kind"], serde_json::json!("prefixed"));
    assert_eq!(
        hits["hits"][0]["base"]["kind"],
        serde_json::json!("subanta")
    );
    assert_eq!(
        hits["hits"][0]["base"]["vibhakti"],
        serde_json::json!("tftIyA")
    );
    ok(&wasm::index_drop_family("b0"));
    ok(&wasm::index_drop_family("u0"));
}

#[test]
fn index_artifact_roundtrip() {
    let _guard = CACHE_GUARD.lock().expect("guard");
    let kartari = serde_json::json!({"01.0001": {"plat": "भवति;भवतः"}}).to_string();
    let karmani = serde_json::json!({"01.0001": {"alat": "भवते"}}).to_string();
    let krut = serde_json::json!({"01.0001": {"ण्वुल्": "भावक,भावकः,,"}}).to_string();
    ok(&wasm::index_ensure_family(
        "shuddha", &kartari, &karmani, &krut,
    ));
    let before = wasm::index_query_family("shuddha", "bhavati");
    let exported = ok(&wasm::index_export_family("shuddha"));
    ok(&wasm::index_drop_family("shuddha"));
    ok(&wasm::index_load_family(
        "shuddha",
        &exported["index"].to_string(),
    ));
    let after = wasm::index_query_family("shuddha", "bhavati");
    assert_eq!(before, after);
    assert!(!err(&wasm::index_load_family("shuddha", r#"{"v":99}"#)).is_empty());
    assert!(!err(&wasm::index_load_family("shuddha", "nope")).is_empty());
    assert!(!err(&wasm::index_load_family("nope", "{}")).is_empty());
    ok(&wasm::index_drop_family("shuddha"));
}

#[test]
fn per_dhatu_search_roundtrip() {
    let kartari = serde_json::json!({"plat": "भवति;भवतः"}).to_string();
    let karmani = serde_json::json!({}).to_string();
    let krut = serde_json::json!({"शतृ": ",,भवन्ती,"}).to_string();
    let stacks = serde_json::json!(["प्र"]).to_string();
    let dhatu = serde_json::json!({
        "family": "shuddha", "baseindex": "01.0001", "slp1": "BU", "note": "",
    })
    .to_string();
    let payload = ok(&wasm::search_prefixed_dhatu(
        &kartari,
        &karmani,
        &krut,
        &dhatu,
        &stacks,
        "प्रभवति",
    ));
    assert_eq!(payload["total"], serde_json::json!(1));
    assert_eq!(payload["hits"][0]["kind"], serde_json::json!("prefixed"));
    let payload = ok(&wasm::search_subanta_dhatu(
        &krut,
        "shuddha",
        "01.0001",
        "भवन्तीभ्याम्",
    ));
    // भवन्तीभ्याम् surfaces in three slots (caturTI/tftIyA/paJcamI dvi).
    assert_eq!(payload["total"], serde_json::json!(3));
    assert_eq!(payload["hits"][0]["kind"], serde_json::json!("subanta"));
}
