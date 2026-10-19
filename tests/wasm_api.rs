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
fn subanta_analyze_f5_trio_and_foreign_words() {
    // Spec trio semantics: `Wrampena` reads only as itself (an instrumental
    // reading of `Wrampa` would need natva → `WrampeRa`); `WrampeRa` reads
    // both as `Wrampa`'s instrumental singular and as itself.
    let payload = ok(&wasm::subanta_analyze("Wrampena"));
    assert_eq!(payload["word"], serde_json::json!("ठ्रम्पेन"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert_eq!(analyses.len(), 2, "m + n self-stem readings: {analyses:?}");
    assert!(analyses.iter().all(|a| {
        a["stem"] == "ठ्रम्पेन" && a["vibhakti"] == "sambodhana" && a["vacana"] == "ekavacanam"
    }));

    let payload = ok(&wasm::subanta_analyze("WrampeRa"));
    assert_eq!(payload["word"], serde_json::json!("ठ्रम्पेण"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "ठ्रम्प" && a["vibhakti"] == "tftIyA" && a["vacana"] == "ekavacanam"
    }));
    assert!(analyses
        .iter()
        .any(|a| a["stem"] == "ठ्रम्पेण" && a["vibhakti"] == "sambodhana"));
    // Generation: the word declines as a fresh stem (m + n a-stems).
    let tables = payload["tables"].as_array().expect("array");
    assert!(tables
        .iter()
        .any(|t| t["linga"] == "m" && t["class"] == "a-m"));
    assert!(tables
        .iter()
        .any(|t| t["linga"] == "n" && t["class"] == "a-n"));
    let text = serde_json::to_string(&payload).expect("serializes");
    assert!(!text.contains("Wramp"), "SLP1 never surfaces: {text}");

    // Portal input: Devanagari and HK land on the same word; the raw-SLP1
    // fallback covers shapes HK cannot spell (the trio above).
    assert_eq!(
        ok(&wasm::subanta_analyze("राम")).to_string(),
        ok(&wasm::subanta_analyze("rAma")).to_string()
    );
    assert_eq!(
        ok(&wasm::subanta_analyze("हरि")).to_string(),
        ok(&wasm::subanta_analyze("hari")).to_string()
    );
    assert!(!err(&wasm::subanta_analyze("no way!")).is_empty());
}

#[test]
fn subanta_analyze_nadi_i_stem_pin() {
    // ashtadhyayi.com शब्द `@lakSmI1` (लक्ष्मी, ईकारान्त स्त्रीलिङ्ग,
    // नदीसंज्ञकः): a lexical `-I` stem carries the visarga in
    // prathamA-ekavacana and — the stem has a trigger (ष्) — voices the
    // genitive plural (लक्ष्मीणाम्). The कृदन्त/उगित् `I-f` grid
    // (nominative = bare stem, pinned for शतृ f `भवन्ती`) stays offered as
    // the second candidate; the bare stem itself reads as a citation.
    let payload = ok(&wasm::subanta_analyze("लक्ष्मी"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "लक्ष्मी"
            && a["class"] == "I-f-nadi"
            && a["vibhakti"] == "citation"
            && a["vacana"] == ""
    }));
    assert!(analyses.iter().any(|a| {
        a["stem"] == "लक्ष्मी"
            && a["class"] == "I-f"
            && a["vibhakti"] == "prathamA"
            && a["vacana"] == "ekavacanam"
    }));
    let tables = payload["tables"].as_array().expect("array");
    assert_eq!(tables[0]["class"], serde_json::json!("I-f-nadi"));
    let grid: Vec<String> = tables[0]["cells"]
        .as_array()
        .expect("cells")
        .iter()
        .map(|c| c["forms"][0].as_str().expect("form").to_string())
        .collect();
    assert_eq!(
        grid.join("|"),
        "लक्ष्मीः|लक्ष्म्यौ|लक्ष्म्यः|लक्ष्मीम्|लक्ष्म्यौ|लक्ष्मीः|लक्ष्म्या|लक्ष्मीभ्याम्|लक्ष्मीभिः|लक्ष्म्यै|लक्ष्मीभ्याम्|लक्ष्मीभ्यः|लक्ष्म्याः|लक्ष्मीभ्याम्|लक्ष्मीभ्यः|लक्ष्म्याः|लक्ष्म्योः|लक्ष्मीणाम्|लक्ष्म्याम्|लक्ष्म्योः|लक्ष्मीषु|लक्ष्मि|लक्ष्म्यौ|लक्ष्म्यः"
    );
    assert_eq!(tables[1]["class"], serde_json::json!("I-f"));
    assert_eq!(
        tables[1]["cells"][0]["forms"][0],
        serde_json::json!("लक्ष्मी")
    );

    // The inflected input carries the नदी nominative reading next to the
    // shared accusative-plural cell (both are `लक्ष्मीः` on the site grid).
    let payload = ok(&wasm::subanta_analyze("लक्ष्मीः"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "लक्ष्मी" && a["class"] == "I-f-nadi" && a["vibhakti"] == "prathamA"
    }));
    assert!(analyses.iter().any(|a| {
        a["stem"] == "लक्ष्मी" && a["vibhakti"] == "dvitIyA" && a["vacana"] == "bahuvacanam"
    }));

    // `-U` mirrors (`वधू`): visarga in the nominative, dental नाम् (no
    // trigger in the stem).
    let payload = ok(&wasm::subanta_analyze("वधू"));
    let tables = payload["tables"].as_array().expect("array");
    assert_eq!(tables[0]["class"], serde_json::json!("U-f-nadi"));
    assert_eq!(tables[0]["cells"][0]["forms"][0], serde_json::json!("वधूः"));
    assert_eq!(
        tables[0]["cells"][17]["forms"][0],
        serde_json::json!("वधूनाम्")
    );
    // No SLP1 anywhere.
    assert!(!serde_json::to_string(&payload)
        .expect("serializes")
        .contains("vaDU"));
}

#[test]
fn subanta_analyze_an_stem_families() {
    // The शब्दपाठः splits regular `-an` masculines three ways by weak stem
    // (राजन् @rAjan1 palatal ञ्, आत्मन् @Atman1 cluster-final, उक्षन्
    // @ukSan1 elided); the derived tables must match each family
    // cell-for-cell (grids read off the site's own cells).
    let grid = |stem: &str| -> String {
        let payload = ok(&wasm::subanta_table(stem, "m"));
        let tables = payload["tables"].as_array().expect("array");
        assert_eq!(tables[0]["class"], serde_json::json!("an-m"), "{stem}");
        tables[0]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["forms"][0].as_str().expect("form").to_string())
            .collect::<Vec<_>>()
            .join("|")
    };
    assert_eq!(
        grid("राजन्"),
        "राजा|राजानौ|राजानः|राजानम्|राजानौ|राज्ञः|राज्ञा|राजभ्याम्|राजभिः|राज्ञे|\
         राजभ्याम्|राजभ्यः|राज्ञः|राजभ्याम्|राजभ्यः|राज्ञः|राज्ञोः|राज्ञाम्|राज्ञि|\
         राज्ञोः|राजसु|राजन्|राजानौ|राजानः"
    );
    assert_eq!(
        grid("आत्मन्"),
        "आत्मा|आत्मानौ|आत्मानः|आत्मानम्|आत्मानौ|आत्मनः|आत्मना|आत्मभ्याम्|आत्मभिः|\
         आत्मने|आत्मभ्याम्|आत्मभ्यः|आत्मनः|आत्मभ्याम्|आत्मभ्यः|आत्मनः|आत्मनोः|आत्मनाम्|\
         आत्मनि|आत्मनोः|आत्मसु|आत्मन्|आत्मानौ|आत्मानः"
    );
    assert_eq!(
        grid("उक्षन्"),
        "उक्षा|उक्षाणौ|उक्षाणः|उक्षाणम्|उक्षाणौ|उक्ष्णः|उक्ष्णा|उक्षभ्याम्|उक्षभिः|\
         उक्ष्णे|उक्षभ्याम्|उक्षभ्यः|उक्ष्णः|उक्षभ्याम्|उक्षभ्यः|उक्ष्णः|उक्ष्णोः|उक्ष्णाम्|\
         उक्ष्णि|उक्ष्णोः|उक्षसु|उक्षन्|उक्षाणौ|उक्षाणः"
    );
    // कर्मन् exercises the nasal voicing: the cluster-final weak stem is
    // dental, the strong stem voices after र् (कर्माणौ), and the weak
    // endings voice (कर्मणा … कर्मणाम्).
    assert_eq!(
        grid("कर्मन्"),
        "कर्मा|कर्माणौ|कर्माणः|कर्माणम्|कर्माणौ|कर्मणः|कर्मणा|कर्मभ्याम्|कर्मभिः|\
         कर्मणे|कर्मभ्याम्|कर्मभ्यः|कर्मणः|कर्मभ्याम्|कर्मभ्यः|कर्मणः|कर्मणोः|कर्मणाम्|\
         कर्मणि|कर्मणोः|कर्मसु|कर्मन्|कर्माणौ|कर्माणः"
    );
    // ग्रावन् @grAvan1 — open `व्`, so the drop family (ग्राव्णः) even
    // though it ends in `-van`; the locative keeps both variants.
    assert_eq!(
        grid("ग्रावन्"),
        "ग्रावा|ग्रावाणौ|ग्रावाणः|ग्रावाणम्|ग्रावाणौ|ग्राव्णः|ग्राव्णा|ग्रावभ्याम्|ग्रावभिः|\
         ग्राव्णे|ग्रावभ्याम्|ग्रावभ्यः|ग्राव्णः|ग्रावभ्याम्|ग्रावभ्यः|ग्राव्णः|ग्राव्णोः|ग्राव्णाम्|\
         ग्राव्णि|ग्राव्णोः|ग्रावसु|ग्रावन्|ग्रावाणौ|ग्रावाणः"
    );
    // दुर्नामन् @durnAman1 — रुत्व-र् (`दुर्` < `दुस्`) never voices the
    // nasal it abuts: दुर्नामा/दुर्नामभ्याम् stay dental, while the च-वर्ग
    // strong and the elided weak voice exactly like उक्षन्.
    assert_eq!(
        grid("दुर्नामन्"),
        "दुर्नामा|दुर्नामानौ|दुर्नामानः|दुर्नामानम्|दुर्नामानौ|दुर्नाम्नः|दुर्नाम्ना|दुर्नामभ्याम्|\
         दुर्नामभिः|दुर्नाम्ने|दुर्नामभ्याम्|दुर्नामभ्यः|दुर्नाम्नः|दुर्नामभ्याम्|दुर्नामभ्यः|\
         दुर्नाम्नः|दुर्नाम्नोः|दुर्नाम्नाम्|दुर्नाम्नि|दुर्नाम्नोः|दुर्नामसु|\
         दुर्नामन्|दुर्नामानौ|दुर्नामानः"
    );
    // Locative singular: both site variants for the elided families
    // (राज्ञि-राजनि, उक्ष्णि-उक्षणि), one for `keep` (आत्मनि).
    let loc = |stem: &str| -> serde_json::Value {
        let payload = ok(&wasm::subanta_table(stem, "m"));
        payload["tables"][0]["cells"][18]["forms"].clone()
    };
    assert_eq!(loc("राजन्"), serde_json::json!(["राज्ञि", "राजनि"]));
    assert_eq!(loc("उक्षन्"), serde_json::json!(["उक्ष्णि", "उक्षणि"]));
    assert_eq!(loc("आत्मन्"), serde_json::json!(["आत्मनि"]));
    assert_eq!(loc("दुर्नामन्"), serde_json::json!(["दुर्नाम्नि", "दुर्नामनि"]));

    // Neuter `-an` (मनिन्) splits by the same rule: नामन् elides and shows
    // the site's alternant pairs, कर्मन्/धन्वन् keep.
    let ncells = |stem: &str| -> Vec<serde_json::Value> {
        let payload = ok(&wasm::subanta_table(stem, "n"));
        payload["tables"][0]["cells"]
            .as_array()
            .expect("cells")
            .clone()
    };
    let na = ncells("नामन्");
    assert_eq!(na[0]["forms"], serde_json::json!(["नाम"]));
    assert_eq!(na[1]["forms"], serde_json::json!(["नाम्नी", "नामनी"]));
    assert_eq!(na[6]["forms"], serde_json::json!(["नाम्ना"]));
    assert_eq!(na[17]["forms"], serde_json::json!(["नाम्नाम्"]));
    assert_eq!(na[18]["forms"], serde_json::json!(["नाम्नि", "नामनि"]));
    assert_eq!(na[22]["forms"], serde_json::json!(["नाम्नी", "नामनी"]));
    assert_eq!(na[21]["forms"], serde_json::json!(["नामन्", "नाम"]));
    let ka = ncells("कर्मन्");
    assert_eq!(ka[0]["forms"], serde_json::json!(["कर्म"]));
    assert_eq!(ka[1]["forms"], serde_json::json!(["कर्मणी"]));
    assert_eq!(ka[6]["forms"], serde_json::json!(["कर्मणा"]));
    assert_eq!(ka[18]["forms"], serde_json::json!(["कर्मणि"]));
    assert_eq!(ka[20]["forms"], serde_json::json!(["कर्मसु"]));
    let dh = ncells("धन्वन्");
    assert_eq!(dh[0]["forms"], serde_json::json!(["धन्व"]));
    assert_eq!(dh[6]["forms"], serde_json::json!(["धन्वना"]));
    // धीरन् (धीर + अन्) voices like कर्ण — the composed conjunct is
    // retroflex (`धीर्णा`), while दुर्नामन् (`दुर्` + नामन्) keeps र्न dental.
    let di = ncells("धीरन्");
    assert_eq!(di[0]["forms"], serde_json::json!(["धीर"]));
    assert_eq!(di[1]["forms"], serde_json::json!(["धीर्णी", "धीरणी"]));
    assert_eq!(di[6]["forms"], serde_json::json!(["धीर्णा"]));
    assert_eq!(di[18]["forms"], serde_json::json!(["धीर्णि", "धीरणि"]));

    // Reverse readings: every weak-family surface resolves to its stem.
    let analyzed = |form: &str| -> Vec<(String, String, String)> {
        let payload = ok(&wasm::subanta_analyze(form));
        payload["analyses"]
            .as_array()
            .expect("array")
            .iter()
            .map(|a| {
                (
                    a["stem"].as_str().expect("stem").to_string(),
                    a["class"].as_str().expect("class").to_string(),
                    format!(
                        "{}.{}",
                        a["vibhakti"].as_str().expect("vibhakti"),
                        a["vacana"].as_str().expect("vacana")
                    ),
                )
            })
            .collect()
    };
    let has = |form: &str, stem: &str, slot: &str| {
        let got = analyzed(form);
        assert!(
            got.iter().any(|(s, _, v)| s == stem && v == slot),
            "{form}: want {stem} {slot}, got {got:?}"
        );
    };
    has("उक्ष्णा", "उक्षन्", "tftIyA.ekavacanam");
    assert!(analyzed("उक्ष्णा")
        .iter()
        .any(|(s, c, v)| s == "उक्षन्" && c == "an-m" && v == "tftIyA.ekavacanam"));
    has("उक्ष्णः", "उक्षन्", "zazWI.ekavacanam");
    has("उक्ष्णाम्", "उक्षन्", "zazWI.bahuvacanam");
    has("मूर्ध्ना", "मूर्धन्", "tftIyA.ekavacanam");
    has("ग्राव्णः", "ग्रावन्", "zazWI.ekavacanam");
    has("ग्रावाणौ", "ग्रावन्", "prathamA.dvivacanam");
    has("ग्राव्णा", "ग्रावन्", "tftIyA.ekavacanam");
    has("दुर्नामा", "दुर्नामन्", "prathamA.ekavacanam");
    has("अणिम्नः", "अणिमन्", "zazWI.ekavacanam");
    has("दीर्घाह्नः", "दीर्घाहन्", "zazWI.ekavacanam");
    has("उक्षभिः", "उक्षन्", "tftIyA.bahuvacanam");
    has("कर्मभ्याम्", "कर्मन्", "tftIyA.dvivacanam");
    has("कर्मभिः", "कर्मन्", "tftIyA.bahuvacanam");
    has("कर्मणः", "कर्मन्", "zazWI.ekavacanam");
    has("आत्मनः", "आत्मन्", "zazWI.ekavacanam");
    // No SLP1 anywhere in the payload.
    assert!(!serde_json::to_string(&ok(&wasm::subanta_analyze("उक्ष्णा")))
        .expect("serializes")
        .contains("ukz"));
}

#[test]
fn subanta_table_feminine_an_in_stems() {
    // Feminine नजन्त (`दामन्`/`सीमन्` elided, `महाजालिन्` इन्-keep): the
    // AnM/InM shapes with a feminine guard — grids read off the
    // site's own cells (@dAman2/@sIman1/@mahAjAlin1). The accusative
    // plural stays weak (`दाम्नः`), exactly like the masculine analog.
    let grid = |stem: &str, linga: &str, class: &str| -> String {
        let payload = ok(&wasm::subanta_table(stem, linga));
        let tables = payload["tables"].as_array().expect("array");
        assert_eq!(tables[0]["class"], serde_json::json!(class), "{stem}");
        tables[0]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["forms"][0].as_str().expect("form").to_string())
            .collect::<Vec<_>>()
            .join("|")
    };
    assert_eq!(
        grid("दामन्", "f", "an-f"),
        "दामा|दामानौ|दामानः|दामानम्|दामानौ|दाम्नः|दाम्ना|दामभ्याम्|दामभिः|दाम्ने|\
         दामभ्याम्|दामभ्यः|दाम्नः|दामभ्याम्|दामभ्यः|दाम्नः|दाम्नोः|दाम्नाम्|दाम्नि|\
         दाम्नोः|दामसु|दामन्|दामानौ|दामानः"
    );
    assert_eq!(
        grid("सीमन्", "f", "an-f"),
        "सीमा|सीमानौ|सीमानः|सीमानम्|सीमानौ|सीम्नः|सीम्ना|सीमभ्याम्|सीमभिः|सीम्ने|\
         सीमभ्याम्|सीमभ्यः|सीम्नः|सीमभ्याम्|सीमभ्यः|सीम्नः|सीम्नोः|सीम्नाम्|सीम्नि|\
         सीम्नोः|सीमसु|सीमन्|सीमानौ|सीमानः"
    );
    assert_eq!(
        grid("महाजालिन्", "f", "in-f"),
        "महाजाली|महाजालिनौ|महाजालिनः|महाजालिनम्|महाजालिनौ|महाजालिनः|महाजालिना|\
         महाजालिभ्याम्|महाजालिभिः|महाजालिने|महाजालिभ्याम्|महाजालिभ्यः|महाजालिनः|\
         महाजालिभ्याम्|महाजालिभ्यः|महाजालिनः|महाजालिनोः|महाजालिनाम्|महाजालिनि|\
         महाजालिनोः|महाजालिषु|महाजालिन्|महाजालिनौ|महाजालिनः"
    );
    // Locative singular: pair variants for the elided families
    // (दाम्नि-दामनि, सीम्नि-सीमनि), single for the इन्-keep.
    let floc = |stem: &str, linga: &str| -> serde_json::Value {
        let payload = ok(&wasm::subanta_table(stem, linga));
        payload["tables"][0]["cells"][18]["forms"].clone()
    };
    assert_eq!(floc("दामन्", "f"), serde_json::json!(["दाम्नि", "दामनि"]));
    assert_eq!(floc("सीमन्", "f"), serde_json::json!(["सीम्नि", "सीमनि"]));
    assert_eq!(floc("महाजालिन्", "f"), serde_json::json!(["महाजालिनि"]));
    // Reverse readings: feminine readings surface alongside the masculine.
    let analyzed = |form: &str| -> Vec<(String, String, String)> {
        let payload = ok(&wasm::subanta_analyze(form));
        payload["analyses"]
            .as_array()
            .expect("array")
            .iter()
            .map(|a| {
                (
                    a["stem"].as_str().expect("stem").to_string(),
                    a["class"].as_str().expect("class").to_string(),
                    format!(
                        "{}.{}",
                        a["vibhakti"].as_str().expect("vibhakti"),
                        a["vacana"].as_str().expect("vacana")
                    ),
                )
            })
            .collect()
    };
    let has_linga = |form: &str, stem: &str, linga: &str, slot: &str| {
        let got = analyzed(form);
        assert!(
            got.iter().any(|(s, _, v)| s == stem && v == slot),
            "{form}: want {stem} {slot}, got {got:?}"
        );
        let payload = ok(&wasm::subanta_analyze(form));
        assert!(
            payload["analyses"]
                .as_array()
                .expect("array")
                .iter()
                .any(|a| a["stem"] == stem && a["linga"] == linga && a["class"] != "A-f"),
            "{form}: want {stem} {linga} {slot}"
        );
    };
    has_linga("दाम्ना", "दामन्", "f", "tftIyA.ekavacanam");
    has_linga("दामानम्", "दामन्", "f", "dvitIyA.ekavacanam");
    has_linga("दामसु", "दामन्", "f", "saptamI.bahuvacanam");
    has_linga("महाजालिनौ", "महाजालिन्", "f", "prathamA.dvivacanam");
    has_linga("महाजालिना", "महाजालिन्", "f", "tftIyA.ekavacanam");
    // No SLP1 anywhere in the payload.
    assert!(!serde_json::to_string(&ok(&wasm::subanta_analyze("दाम्ना")))
        .expect("serializes")
        .contains("dAm"));
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

#[test]
fn subanta_analyze_new_paradigm_readings() {
    // The classes added for ADDITIONS 2.x must read back, not just decline:
    // the word view shows "readings" (base word + case) for the form the
    // user typed, and a paradigm whose surfaces cannot be inverted is
    // half-delivered.
    //
    // उस्/इस्-स्वादि neuters (2.4): every inflected form of the आयुष्
    // family reads back to the one citation, and the citation itself reads
    // as a base word (the सि is root+विसर्ग, not a cell of the paradigm).
    for (form, vibhakti, vacana) in [
        ("आयुषा", "tftIyA", "ekavacanam"),
        ("आयुषः", "paJcamI", "ekavacanam"),
        ("आयुषे", "caturTI", "ekavacanam"),
        ("आयुषि", "saptamI", "ekavacanam"),
        ("आयुषी", "prathamA", "dvivacanam"),
        ("आयुर्भ्याम्", "tftIyA", "dvivacanam"),
    ] {
        let payload = ok(&wasm::subanta_analyze(form));
        let analyses = payload["analyses"].as_array().expect("array");
        assert!(
            analyses.iter().any(|a| {
                a["stem"] == "आयुष्"
                    && a["class"] == "us-n"
                    && a["vibhakti"] == vibhakti
                    && a["vacana"] == vacana
            }),
            "{form} should read back to आयुष् us-n {vibhakti}.{vacana}"
        );
    }
    let payload = ok(&wasm::subanta_analyze("आयुष्"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "आयुष्" && a["class"] == "us-n" && a["vibhakti"] == "citation"
    }));
    // The i-root family keeps its स् spelling (हविस्, not हविज्).
    let payload = ok(&wasm::subanta_analyze("हविषा"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "हविस्" && a["class"] == "us-n" && a["vibhakti"] == "tftIyA"
    }));

    // Consonant-final nouns (2.1–2.3): the भ-endings read back to the
    // सरित्-family stem, both in its pause spelling and its voiced one.
    let payload = ok(&wasm::subanta_analyze("सरिद्भ्याम्"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "सरित्"
            && a["class"] == "cons"
            && a["vibhakti"] == "tftIyA"
            && a["vacana"] == "dvivacanam"
    }));
    let payload = ok(&wasm::subanta_analyze("सरित्सु"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "सरित्" && a["class"] == "cons" && a["vibhakti"] == "saptamI"
    }));

    // Word-route words (2.5/2.6) read back through their exact tables.
    let payload = ok(&wasm::subanta_analyze("अन्यस्मात्"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "अन्य" && a["class"] == "known word" && a["vibhakti"] == "paJcamI"
    }));
    let payload = ok(&wasm::subanta_analyze("गोभ्याम्"));
    let analyses = payload["analyses"].as_array().expect("array");
    assert!(analyses.iter().any(|a| {
        a["stem"] == "गो" && a["class"] == "known word" && a["vibhakti"] == "tftIyA"
    }));
}
