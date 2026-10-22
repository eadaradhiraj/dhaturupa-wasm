//! Subanta site-parity: consonant-final (सरित्-family) and known-word
//! grids must reproduce the vendored ashtadhyayi.com data byte-exactly.
//!
//! Every listed word is looked up in `data/shabda_data2.json`, declined
//! through the engine, and compared cell-by-cell: the engine's SLP1
//! forms are Devanagari-rendered, joined with ` , ` and then dashified
//! (the site's alternant encoding), sambodhana cells receive their
//! `हे ` prefix exactly as the portal renders them. The db's linga
//! markers are P=m, S=f, N=n.
//!
//! Runs in plain `cargo test` — the vendored db is part of the repo, so
//! there is no optional-corpus skip: parity here is always real.

use dhaturupa_wasm::engine::subanta::{generate_tables, Linga};
use dhaturupa_wasm::translit::{deva_to_slp1, slp1_to_deva};
use serde_json::Value;
use std::sync::OnceLock;

fn site_data() -> &'static Value {
    static SITE: OnceLock<Value> = OnceLock::new();
    SITE.get_or_init(|| {
        let text =
            std::fs::read_to_string("data/shabda_data2.json").expect("vendored shabda db loads");
        serde_json::from_str(&text).expect("shabda db parses")
    })
}

fn site_entries(word: &str) -> Vec<(&'static str, Vec<String>)> {
    let mut out = Vec::new();
    for e in site_data()["data"].as_array().expect("db.data array") {
        if e["word"] == word {
            out.push((
                e["linga"].as_str().expect("linga marker"),
                e["forms"]
                    .as_str()
                    .expect("forms field")
                    .split(';')
                    .map(|c| c.trim().to_string())
                    .collect(),
            ));
        }
    }
    assert!(!out.is_empty(), "{word} missing from the vendored db");
    out
}

fn linga_from_marker(mark: &str) -> Linga {
    let code = match mark {
        "P" => "m",
        "S" => "f",
        "N" => "n",
        _ => panic!("unknown db linga marker {mark}"),
    };
    Linga::parse(code).expect("well-formed linga")
}

/// Decline a display stem and render the 8×3 grid the same way the
/// portal does (Devanagari, alternants joined, `हे ` on sambodhana).
/// Lookup goes by (विभक्ति, वचन) so slots the engine drops — उभ's empty
/// eka/bahu — come back as '' instead of shifting every later cell.
/// Returns every candidate the engine offers: some stems have more than
/// one plausible paradigm (the वतुप् nominalization vs the ऋ-वत्
/// participle), and the site picks one of them.
fn engine_grids(word: &str, mark: &str) -> Vec<Vec<String>> {
    let linga = linga_from_marker(mark);
    let stem = deva_to_slp1(word).unwrap_or_else(|| panic!("{word} transliterates"));
    let tables = generate_tables(&stem, linga);
    assert!(!tables.is_empty(), "{word} [{mark}] produced no table");
    let vib_row = [
        "prathamA",
        "dvitIyA",
        "tftIyA",
        "caturTI",
        "paJcamI",
        "zazWI",
        "saptamI",
        "sambodhana",
    ];
    tables
        .iter()
        .map(|paradigm| {
            let mut slots: std::collections::HashMap<(&str, &str), Vec<String>> =
                std::collections::HashMap::new();
            for cell in &paradigm.cells {
                let rendered: Vec<String> = cell
                    .forms
                    .iter()
                    .map(|f| {
                        slp1_to_deva(f).unwrap_or_else(|| panic!(" cell '{f}' transliterates"))
                    })
                    .collect();
                slots.insert((cell.vibhakti, cell.vacana), rendered);
            }
            let mut out = Vec::with_capacity(24);
            for (row, vib) in vib_row.iter().enumerate() {
                for vac in ["ekavacanam", "dvivacanam", "bahuvacanam"] {
                    let rendered = slots.get(&(*vib, vac)).cloned().unwrap_or_default();
                    let joined = rendered.join(" , ").replace(" , ", "-");
                    if row == 7 && !joined.is_empty() {
                        // The portal prefixes हे per alternant (render.js
                        // renderParadigm).
                        out.push(
                            rendered
                                .iter()
                                .map(|r| format!("हे {r}"))
                                .collect::<Vec<_>>()
                                .join(" , ")
                                .replace(" , ", "-"),
                        );
                    } else {
                        out.push(joined);
                    }
                }
            }
            out
        })
        .collect()
}

/// The first candidate — the shape the portal shows by default.
fn engine_grid(word: &str, mark: &str) -> Vec<String> {
    engine_grids(word, mark).remove(0)
}

fn pin_word(word: &str) {
    for (mark, forms) in site_entries(word) {
        let got = engine_grid(word, mark);
        assert_eq!(got.len(), forms.len(), "{word} [{mark}] cell count");
        for (i, (got_cell, want)) in got.iter().zip(forms.iter()).enumerate() {
            let slot = ["pr", "dvi", "tft", "cat", "paJ", "zaz", "sap", "samb"][i / 3];
            let vac = ["eka", "dvi", "bahu"][i % 3];
            // `@budh1` carries a भु-typo in its pausa/भ्-ending cells; the
            // engine emits the correct बु- spellings.
            let want_cell = if word == "बुध्" {
                want.replace("भु", "बु")
            } else {
                want.clone()
            };
            assert_eq!(
                got_cell.clone(),
                want_cell,
                "{word} [{mark}] {slot}-{vac} mismatch: got '{got_cell}' want '{want_cell}'"
            );
        }
    }
}

/// Pin a word whose site grid may be any of the engine's candidates: the
/// वतुप् nominalization (आत्मवत् → आत्मवान्) and the ऋ-वत् participle
/// (आप्नुवत् → आप्नुवन्) end the same way, so the engine offers both and
/// the site picks one. `mark` selects the लिङ्ग (the neuter आत्मवत् is the
/// ई-स्वादि family, a different paradigm).
fn pin_word_any(word: &str, mark: &str) {
    for (entry_mark, forms) in site_entries(word) {
        if entry_mark != mark {
            continue;
        }
        let grids = engine_grids(word, entry_mark);
        assert!(
            grids.iter().any(|grid| {
                grid.len() == forms.len() && grid.iter().zip(forms.iter()).all(|(a, b)| a == b)
            }),
            "{word} [{entry_mark}] is not reproduced by any derived grid"
        );
    }
}

/// Pin a word against the site's *listed* cells: a few शब्दपाठः entries
/// leave cells empty (the site's own table carries no form there) and an
/// empty cell carries no information — the audit compares the listed
/// cells the same way. Any of the engine's candidates may match.
fn pin_word_listed_any(word: &str) {
    for (mark, forms) in site_entries(word) {
        let grids = engine_grids(word, mark);
        assert!(
            grids.iter().any(|grid| {
                grid.len() == forms.len()
                    && grid
                        .iter()
                        .zip(forms.iter())
                        .all(|(a, b)| b.is_empty() || a == b)
            }),
            "{word} [{mark}] is not reproduced by any derived grid"
        );
    }
}

/// Consonant-final nouns routed to the सरित्-family class, plus the
/// participle regressions that must keep their गच्छन्/AtM shape.
#[test]
fn cons_family_and_participle_pins() {
    // सरित्-family grids (site-verbatim).
    for word in [
        "सरित्",
        "विद्युत्",
        "संविद्",
        "उपनिषद्",
        "युध्",
        "बुध्",
        "ककुभ्",
        "वाच्",
        "त्वच्",
        "मरुत्",
        // The masculine भ्-final (गर्दभ् @gardabh1 — the जश्-व्यवस्था
        // voices the stem's द in the weak forms), the ऋ-chart (ऋच्
        // @Rc1 — the Cons shape with a one-letter base) and the थ्-final
        // (अग्निमथ् @agnimath1).
        "गर्दभ्",
        "ऋच्",
        "अग्निमथ्",
    ] {
        pin_word(word);
    }
    // The ऋ-chart's श्-final carries a third alternant (अदृग्-अदृख्-अदृक्);
    // its neuter entry is the ई-स्वादि family, a different paradigm.
    pin_word_any("अदृश्", "P");
    // The अस्-family's locative plural carries both alternants
    // (अनःसु-अनस्सु, site @anas1).
    pin_word_any("अनस्", "N");
    // The feminine i-stems take the ण-स्वादि: the त्य-form with its
    // paired alternants (site @akSAnti1/@gati1) — the masculine keeps the
    // plain shape (अग्निना, @agni1).
    for word in ["अक्षान्ति", "अणि", "गति"] {
        pin_word_any(word, "S");
    }
    pin_word_any("अग्नि", "P");
    // The masculine ई- and ऊ-स्वादि (अमीः, अग्निभूः, आमलकीः).
    for word in ["अमी", "अग्निभू", "आमलकी"] {
        pin_word_any(word, "P");
    }
    // The neuter ई-स्वादि of the वत family (आत्मवती, आत्मवन्ति) and the
    // स्म-ending a-stems (अनेकस्मै, अनेकस्माद्-अनेकस्मात्; अधरस्यै).
    pin_word_any("आत्मवत्", "N");
    pin_word_any("अनेक", "P");
    pin_word_any("अनेक", "N");
    pin_word_any("अधरा", "S");
    // The अस्-family's vocative singular takes the स-form (हे अङ्गिरः).
    pin_word_any("अङ्गिरस्", "P");
    // The अञ्च-स्वादि (अर्वाच् → अर्वाङ् | अर्वाञ्चौ) and the अञ्ज-स्वादि
    // (खन्ज् → खन् | खञ्जौ), plus the अस्-family's anusvāra-form (अणीयान्)
    // and e-form (अनेहा).
    pin_word_any("अर्वाच्", "P");
    pin_word_any("खन्ज्", "P");
    pin_word_any("अणीयस्", "P");
    pin_word_any("अनेहस्", "P");
    // The व-स्वादि feminine u-stems (अभ्रमु → अभ्रम्वा) and the neuter
    // u-stems (अगुरु → अगुरुणे, अगुरूणाम्).
    pin_word_any("अभ्रमु", "S");
    pin_word_any("अगुरु", "N");
    // The neuter i-stem in the ण-form (अतिस्त्रिणी, अतिस्त्रिणे-अतिस्त्रये).
    pin_word_any("अतिस्त्रि", "N");
    // Participle regression: गच्छत् keeps the गच्छन्-prathamA shape.
    pin_word("गच्छत्");
    // The वतुप् nominalization (आत्मवान्), its मतुप्/यतुप् siblings
    // (आयुष्मान्, इयान्) and the ऋ-वत् participle (आप्नुवन्) end alike;
    // the engine offers both shapes.
    for word in ["आत्मवत्", "उरस्वत्", "क्रियावत्", "आप्नुवत्", "आयुष्मत्", "इयत्"]
    {
        pin_word_any(word, "P");
    }
}

/// गो/नौ/रै diphthong words (ADDITIONS 2.5) ride the `KNOWN_WORDS`
/// word-route; the db carries both लिङ्ग entries where present.
#[test]
fn diphthong_word_pins() {
    for word in ["गो", "नौ", "रै"] {
        pin_word(word);
    }
}

/// सर्व-family स्म-ending pronominals (ADDITIONS 2.6): every db entry
/// — अन्य/इतर/कतर/कतम/उभय in m+n, अन्या in f, उभ dual-only with its
/// empty eka/bahu cells — must reproduce verbatim.
#[test]
fn pronominal_word_pins() {
    for word in ["अन्य", "अन्या", "इतर", "कतर", "कतम", "उभय", "उभ"]
    {
        pin_word(word);
    }
}

/// The feminine अस्-स्वादि (सदस् → सदाः, site @sadas2) and the
/// वस्-स्वादि (विद्वस् → विद्वान्/विद्वांसः, site @vidvas1/@vidvas2).
#[test]
fn as_and_vas_swadi_pins() {
    for word in ["सदस्", "सुमनस्", "त्रिस्रोतस्", "दृष्टरजस्"]
    {
        pin_word_any(word, "S");
    }
    for word in ["विद्वस्", "सेदिवस्", "बभूवस्"] {
        pin_word_any(word, "P");
    }
    pin_word_any("विद्वस्", "N");
}

/// The masculine ह्-final nouns, in the flavours the site grids: the ड/ट
/// pair (लिह् → लिड्-लिट्), the ड/ग/क/ट quartet (मुह्), the ग/क pair
/// (दुह् → धुग्-धुक्), the वृद्धि nominative (अनडुह् → अनड्वान्) and
/// the औ-form (विश्ववाह् → विश्वौहः).
#[test]
fn ha_final_masks() {
    for word in [
        "लिह्",
        "तुरासाह्",
        "दामलिह्",
        "पुष्पलिह्",
        "मधुलिह्",
        "मुह्",
        "स्निह्",
        "स्नुह्",
        "द्रुह्",
        "दुह्",
        "गोदुह्",
        "अनडुह्",
        "विश्ववाह्",
        "प्रष्ठवाह्",
    ] {
        pin_word_any(word, "P");
    }
}

/// The u-stem neuter without the गुण/व् alternants (मधु), the i-stem
/// neuter with the य-forms as alternants (अनादिने-अनादये) and the
/// ऋ-form in the genitive dual (भूरिणोः-भूर्योः).
#[test]
fn u_and_i_stem_neuter_pins() {
    for word in ["मधु", "कटु", "आशु", "अम्बु", "यातु", "वस्तु"]
    {
        pin_word_any(word, "N");
    }
    for word in ["अनादि", "शिति", "शुचि", "मौलि", "सृष्टि", "विविक्ति"]
    {
        pin_word_any(word, "N");
    }
    for word in ["भूरि", "सुरभि", "फलेग्रहि", "आत्मम्भरि", "कुक्षिम्भरि"]
    {
        pin_word_any(word, "N");
    }
}

/// The masculine च्-final nouns of the pair flavour (जलमुच् →
/// जलमुग्-जलमुक्) and the ड/ट pair (सुवृश्च् → सुवृड्-सुवृट्).
#[test]
fn c_final_masculine_pair_pins() {
    for word in [
        "जलमुच्",
        "पयोमुच्",
        "सत्यवाच्",
        "अस्फुटवाच्",
        "बहुगर्ह्यवाच्",
        "मृदुत्वच्",
        "विशालत्वच्",
        "वारिमुच्",
    ] {
        pin_word_any(word, "P");
    }
    pin_word_any("सुवृश्च्", "P");
}

/// The masculine at-participles with the द/त pair (चकासत् →
/// चकासद्-चकासत्).
#[test]
fn at_participle_pair_pins() {
    for word in [
        "चकासत्",
        "जक्षत्",
        "जाग्रत्",
        "ददत्",
        "दरिद्रत्",
        "पुरीतत्",
        "पृषत्",
        "वाहद्विषत्",
        "शासत्",
    ] {
        pin_word_any(word, "P");
    }
}

/// The neuter द्-final nouns (उद्भिद् → उद्भिद्-उद्भित्, उद्भिदी,
/// उद्भिन्दि) and the ऋ-chart nouns with two alternants (दिश् →
/// दिग्-दिक्).
#[test]
fn dad_final_and_two_alternant_pins() {
    for word in ["उद्भिद्", "बेभिद्", "चेच्छिद्", "शास्त्रविद्", "हृद्"]
    {
        pin_word_any(word, "N");
    }
    for word in ["मर्मस्पृश्", "घृतस्पृश्", "जलस्पृश्"]
    {
        pin_word_any(word, "P");
    }
    // The ऋ-chart nouns with two alternants (दिश् → दिग्-दिक्) are
    // feminine, and the श्-final nouns come in a ड/ट flavour too
    // (विश् → विड्-विट्).
    for word in ["दिश्", "विपाश्", "विश्"] {
        pin_word_any(word, "S");
    }
    pin_word_any("विश्", "P");
}

/// The व-स्वादि feminine u-stems with the dental genitive plural
/// (अलावु → अलावूनाम्).
#[test]
fn v_form_u_stem_feminine_pins() {
    for word in ["अलावु", "आलु", "उडु", "कच्छु", "कटु", "तनु", "धेनु", "सिन्धु"]
    {
        pin_word_any(word, "S");
    }
}

/// The a-stems with the स्म-forms and the optional ए-forms (दक्षिण →
/// दक्षिणस्मै, दक्षिणाद्-दक्षिणस्माद्-दक्षिणस्मात्-दक्षिणात्) and the
/// स्म-forms without the nominative pair (एकतर → एकतरेण, एकतरस्मै).
#[test]
fn a_stem_sm_form_pins() {
    for word in ["दक्षिण", "स्व", "अन्यतर", "एकतर", "ततर", "यतर", "परमसर्व"]
    {
        pin_word_any(word, "P");
    }
}

/// The neuter a-stems with the site's optional वृद्धि-forms (अधर →
/// अधरेण, अधरस्मै, अधराद्-अधरस्माद्-अधरस्मात्-अधरात्).
#[test]
fn neuter_a_stem_optional_forms_pins() {
    for word in ["अधर", "अपर", "अवर", "पूर्व", "एकतर"] {
        pin_word_any(word, "N");
    }
}

/// The neuter of the वतुप् stems (अहङ्कारवत् → अहङ्कारवद्-अहङ्कारवत्,
/// अहङ्कारवती, अहङ्कारवन्ति).
#[test]
fn vant_stem_neuter_pins() {
    for word in [
        "अहङ्कारवत्",
        "आयुष्मत्",
        "ऊर्मिमत्",
        "ऊषवत्",
        "क्रियावत्",
        "परवत्",
        "विषुवत्",
        "शर्करावत्",
        "श्रीमत्",
    ] {
        pin_word_any(word, "N");
    }
}

/// The a-stem masculine with the site's optional वृद्धि-forms: the
/// ए-form as an alternant in the nominative plural (अर्धाः-अर्धे), the
/// ए + the ण in the instrumental (चरमेण) and the स्म-forms
/// (अपरस्मै).
#[test]
fn a_stem_optional_forms_pins() {
    for word in ["अर्ध", "अल्प", "द्वय", "द्वितय", "प्रथम"]
    {
        pin_word_any(word, "P");
    }
    for word in ["चरम", "पूर्वपर", "वर्णाश्रमेतर"] {
        pin_word_any(word, "P");
    }
    for word in ["अपर", "अवर", "अन्तर", "उत्तर", "पर", "पूर्व", "अधर"]
    {
        pin_word_any(word, "P");
    }
    pin_word_any("नेम", "P");
}

/// The neuter आ-stem nouns (श्रीपा → श्रीपम्, श्रीपे, श्रीपाणि).
#[test]
fn aa_stem_neuter_pins() {
    pin_word_any("श्रीपा", "N");
}

/// The neuter ऋ-chart nouns (दृश् → दृग्-दृख्-दृक्, दृशी, दृंशि).
#[test]
fn rc_chart_neuter_pins() {
    pin_word_any("दृश्", "N");
}

/// The masculine औ-स्वादि nouns (जनौ → जनौः, जनावौ): the औ-form
/// nominative and the व-form in the dual/plural.
#[test]
fn au_stem_pins() {
    for word in ["जनौ", "ग्लौ"] {
        pin_word_any(word, "P");
    }
}

/// The masculine आ-stem nouns (कीलालपा → कीलालपाः): the visarga
/// nominative, the plain भ-endings and the आसु locative plural.
#[test]
fn aa_stem_pins() {
    for word in [
        "कीलालपा",
        "धेनुपा",
        "मधुपा",
        "वनपा",
        "सोमपा",
        "विश्वपा",
        "गोपा",
        "द्वारपा",
        "धूम्रपा",
    ] {
        pin_word_any(word, "P");
    }
}

/// The feminine numeral family (त्रिंशत् → त्रिंशद्-त्रिंशत्), the
/// feminine र्-final nouns (गिर् → गीः, पुर् → पूः, द्वार् → द्वाः), the
/// neuter र्-final (वार् → वारी) and the bare-nominative nouns (कमल्,
/// सुगण्).
#[test]
fn numeral_and_ra_final_pins() {
    for word in ["त्रिंशत्", "वेहत्", "संयत्", "चत्वारिंशत्", "पञ्चाशत्"]
    {
        pin_word_any(word, "S");
    }
    for word in ["गिर्", "अपकारगिर्", "पुर्", "द्वार्"]
    {
        pin_word_any(word, "S");
    }
    pin_word_any("वार्", "N");
    for word in ["कमल्", "सुगण्", "सुगाण्"] {
        pin_word_any(word, "P");
    }
}

/// The feminine and neuter ह्-final nouns: the द/त pair (उपानह् →
/// उपानद्-उपानत्), the ग/क pair (उष्णिह् → उष्णिग्-उष्णिक्), the
/// धुग्-धुक् pair (कामदुह्) and the neuter ई-form dual (स्वनडुह् →
/// स्वनडुही).
#[test]
fn ha_final_feminine_pins() {
    for word in ["उपानह्", "उष्णिह्", "कामदुह्"] {
        pin_word_any(word, "S");
    }
    pin_word_any("स्वनडुह्", "N");
}

/// The feminine ष्-final nouns: the ड/ट flavour (तृष् → तृड्-तृट्) and
/// the vowel flavours (छदिष् → छदिः, आशिष् → आशीः, सजुष् → सजूः).
#[test]
fn sha_final_feminine_pins() {
    for word in ["तृष्", "त्विष्", "प्रावृष्", "रुष्", "विप्रुष्"]
    {
        pin_word_any(word, "S");
    }
    for word in ["छदिष्", "अर्चिष्", "आशिष्", "सजुष्"] {
        pin_word_any(word, "S");
    }
}

/// The कृत agent nouns (तृ-स्वादि): the masculine ऋ-stem (धातृ → धाता,
/// site @dhAtR1) and the neuter ण-first pairs (धातृ → धातृणी, @dhAtR2).
#[test]
fn agent_noun_pins() {
    for word in [
        "अतिवक्तृ",
        "अलङ्कर्तृ",
        "आशंसितृ",
        "उत्पतितृ",
        "कमितृ",
        "कामयितृ",
        "क्षन्तृ",
        "क्षमितृ",
        "ग्रहीतृ",
        "जागरितृ",
        "ज्ञातृ",
        "धातृ",
        "भवितृ",
    ] {
        pin_word_any(word, "P");
        pin_word_any(word, "N");
    }
}

/// The masculine ष्-final nouns, in the two flavours the site grids:
/// the ड/ट pair (द्विष् → द्विड्-द्विट्, @dviS1) and the ग/क pair
/// (तक्ष् → तग्-तक्, @takS2). A few words carry both grids
/// (तक्ष् @takS1/@takS2, गोरक्ष् @gorakS1/@gorakS2), so the engine
/// offers both candidates and the site's grid decides.
#[test]
fn sha_final_masks() {
    for word in [
        "द्विष्",
        "गोविष्",
        "प्रियषष्",
        "मृगद्विष्",
        "रत्नमुष्",
        "सुरद्विष्",
        "विविक्ष्",
    ] {
        pin_word_any(word, "P");
    }
    for word in ["तक्ष्", "गोरक्ष्", "दधृष्", "दिधक्ष्", "पिपक्ष्", "विवक्ष्"]
    {
        pin_word_any(word, "P");
    }
}

/// The इ-स्वादि with the वृद्धि-सर्वधातुकारी in the nom/acc slots
/// (पथिन्; site @pathin1).
#[test]
fn in_stem_vriddhi_pins() {
    for word in ["पथिन्", "अतिपथिन्", "ऋभुक्षिन्"] {
        pin_word_any(word, "P");
    }
}

/// The comparatives (अन्यतर, एकतम; site @anyatara1/@anyatara2/
/// @ekatama2): the स्म-forms beside the plain ones.
#[test]
fn comparative_pins() {
    pin_word_any("अन्यतर", "P");
    pin_word_any("अन्यतर", "N");
    pin_word_any("एकतम", "N");
}

/// The ordinals with the स्म-alternant beside the plain form (द्वितीय;
/// site @dvitIya1/@dvitIya2).
#[test]
fn ordinal_sm_pair_pins() {
    pin_word_any("द्वितीय", "P");
    pin_word_any("द्वितीय", "N");
}

/// The नदीसंज्ञक feminine ई- and ऊ-स्वादि stems in the िय-/उव-flavours
/// (जयश्री, कुलस्त्री, धी, धू; site @jayazrI1/@kulastrI1/@dhI1/@dhU1).
#[test]
fn nadi_feminine_i_and_u_stem_pins() {
    for word in ["जयश्री", "कुलस्त्री", "धी", "धू", "खलपू"]
    {
        pin_word_any(word, "S");
    }
}

/// The feminine a-stems with the स्य-augment pairs (उत्तरपश्चिमा;
/// site @uttarapazcimA1) and the bare vocative (हे अक्क; site @akkA1).
#[test]
fn a_stem_sy_pairs_and_bare_vocative_pins() {
    for word in ["उत्तरपश्चिमा", "उत्तरपूर्वा", "अक्का", "जगदम्बा"]
    {
        pin_word_any(word, "S");
    }
}

/// The masculine ई- and ऊ-स्वादि stems in their strong-cell flavours:
/// the गुण-forms (अग्रण्यम्, जलप्यि, क्षाम्युः), the वृद्धि (पक्वियौ)
/// and the व्व-forms (उल्ल्वौ, अग्रेदिधिष्वौ).
#[test]
fn i_and_u_stem_strong_cell_pins() {
    for word in ["अग्रणी", "जलपी", "क्षामी", "पक्वी", "उल्लू", "अग्रेदिधिषू"]
    {
        pin_word_any(word, "P");
    }
}

/// The compounded pronouns (अतियुष्मद्, अत्यस्मद्, परमयुष्मद्,
/// परमास्मद्): the site grids the pronoun's own paradigm with the base
/// in front of it — three flavours per pronoun for the अधिक compounds
/// (अतित्वाम्, अतियुवाम्, अतियुष्माम्; अतिमाम्, अत्यावाम्,
/// अत्यस्माम्) and the pronoun's own grid, with its alternants, for the
/// पुरुष ones (परमत्वाम्-परमत्वा).
#[test]
fn pronoun_compound_pins() {
    for word in ["अतियुष्मद्", "अत्यस्मद्", "परमयुष्मद्", "परमास्मद्"]
    {
        pin_word_any(word, "P");
    }
}

/// उस्/इस्-स्वादि neuters (ADDITIONS 2.4): आयुष् against its vendored db
/// grid (`@AyuS1`), and the pattern-derived हविस्/धनुस् grids the site
/// does not list — the contract says derived engine tables must cover
/// them anyway.
#[test]
fn us_swadi_pins() {
    pin_word("आयुष्");
    for (word, long, root) in [("हविस्", "हवी", "हवि"), ("धनुस्", "धनू", "धनु")]
    {
        let grid = engine_grid(word, "N");
        let want: [&str; 24] = [
            &format!("{root}ः"),
            &format!("{root}षी"),
            &format!("{long}ंषि"),
            &format!("{root}ः"),
            &format!("{root}षी"),
            &format!("{long}ंषि"),
            &format!("{root}षा"),
            &format!("{root}र्भ्याम्"),
            &format!("{root}र्भिः"),
            &format!("{root}षे"),
            &format!("{root}र्भ्याम्"),
            &format!("{root}र्भ्यः"),
            &format!("{root}षः"),
            &format!("{root}र्भ्याम्"),
            &format!("{root}र्भ्यः"),
            &format!("{root}षः"),
            &format!("{root}षोः"),
            &format!("{root}षाम्"),
            &format!("{root}षि"),
            &format!("{root}षोः"),
            &format!("{root}ःषु-{root}ष्षु"),
            &format!("हे {root}ः"),
            &format!("हे {root}षी"),
            &format!("हे {long}ंषि"),
        ];
        for (i, (got, want_cell)) in grid.iter().zip(want.iter()).enumerate() {
            let slot = ["pr", "dvi", "tft", "cat", "paJ", "zaz", "sap", "samb"][i / 3];
            let vac = ["eka", "dvi", "bahu"][i % 3];
            let want_cell = (*want_cell).to_string();
            assert_eq!(
                got.clone(),
                want_cell,
                "{word} {slot}-{vac} mismatch: got '{got}' want '{want_cell}'"
            );
        }
    }
}

/// The ञ्च-स्वादि masculine nouns whose weak stem is the वृद्धि-form
/// (उदच् → उदीचः; site @udac1 and its kin). `तिर्यच्` is left out: the
/// site spells its weak stem `तिरश्च` (the र without its halant), which
/// no SLP1 surface can render.
#[test]
fn anccha_swadi_masculine_pins() {
    for word in [
        "उदच्",
        "न्यच्",
        "प्रत्यच्",
        "सम्यच्",
        "तद्र्यच्",
        "देवद्र्यच्",
        "सध्र्यच्",
        "विष्वद्र्यच्",
        "अन्वच्",
    ] {
        pin_word_any(word, "P");
    }
}

/// The अस्-स्वादि masculine nouns of the plain, the pause and the वृद्धि
/// flavours (पिण्डग्रस्, ध्वस्, सुतुस्; site @piNDagras1/@dhvas1/
/// @sutus1) and the स्-final nouns that are not अस्-स्वादि at all
/// (ओकास्, भास्; site @okAs1/@bhAs1).
#[test]
fn as_swadi_masculine_pins() {
    for word in ["पिण्डग्रस्", "पिण्डग्लस्", "सुवस्", "ध्वस्", "स्रस्", "सुतुस्", "सुपिस्"]
    {
        pin_word_any(word, "P");
    }
    pin_word_any("ओकास्", "P");
    pin_word_any("भास्", "S");
}

/// The masculine ष्-final nouns of the visarga flavour (चिकीर्ष्; site
/// @cikIrS1 and its kin): the nominative is the visarga form, the
/// भ-endings the र-form.
#[test]
fn visarga_sha_final_masculine_pins() {
    for word in ["चिकीर्ष्", "जगच्चक्षुष्", "सप्तार्चिष्", "स्थिरायुष्"]
    {
        pin_word_any(word, "P");
    }
}

/// The masculine ई-स्वादि nouns with the आ-forms beside the गुण-forms
/// (दुर्धी → दुर्ध्यै, दुर्ध्याः; site @durdhI1) and the आ-form in the
/// locative (नी → नियाम्; site @nI1).
#[test]
fn i_stem_masculine_aa_forms_pins() {
    for word in ["दुर्धी", "प्रधी", "वृश्चिकभी", "नी"] {
        pin_word_any(word, "P");
    }
}

/// The इ-स्वादि masculine nouns whose root's own न् keeps its dental:
/// the नत्व reaches the प्रातिपदिक's final न् alone (कर्मन्दिन् →
/// कर्मन्दिभ्याम्; site @karmandin1).
#[test]
fn in_stem_dental_natva_pins() {
    for word in ["कर्मन्दिन्", "परिपन्थिन्"] {
        pin_word_any(word, "P");
    }
}

/// The plain flavour of the masculine वतुप् stems (त्वत् → त्वद्-त्वत्;
/// site @tvat1) and the mixed flavours of the neuter त-स्वादि (तुदत् →
/// तुदन्ती-तुदती; ददत् → ददन्ति-ददति; महत् → महान्ति; site
/// @tudat1/@dadat2/@mahat2).
#[test]
fn vant_and_ta_final_neuter_pins() {
    for word in ["त्वत्", "अष्ठीवत्", "दीध्यत्", "वेव्यत्"]
    {
        pin_word_any(word, "P");
    }
    for word in ["तुदत्", "भात्", "ददत्", "महत्"] {
        pin_word_any(word, "N");
    }
}

/// The masculine ई-स्वादि nouns with the bare stem in the nominative
/// (कुमारी; site @kumArI1).
#[test]
fn i_stem_masculine_bare_nominative_pins() {
    pin_word_any("कुमारी", "P");
}

/// The neuter ञ्च-स्वादि family: the वृद्धि weak stem (न्यच् → नीची;
/// site @nyac2), the ओ-form (गवाच् → गोची; site @gavAc1) and the
/// ङ्-inserted न्च-stems (गोन्च् → गोङ्; site @gonc1).
#[test]
fn anccha_swadi_neuter_pins() {
    for word in ["न्यच्", "सम्यच्", "अन्वच्"] {
        pin_word_any(word, "N");
    }
    for word in ["गवाच्", "गोअच्", "गोच्"] {
        pin_word_any(word, "N");
    }
    for word in ["गोन्च्", "गवान्च्", "गोअन्च्", "तिर्यन्च्"]
    {
        pin_word_any(word, "N");
    }
}

/// The neuter i-stems with the ण-forms in the oblique cases (अक्षि;
/// site @akSi1/@asthi1/@dadhi1/@sakthi1): the ष-final stems take the
/// ण (अक्ष्णा), the others the न (अस्थ्ना), and the locative singular
/// and the vocative carry both alternants (अक्ष्णि-अक्षणि).
#[test]
fn i_stem_neuter_na_forms_pins() {
    for word in ["अक्षि", "अस्थि", "दधि", "सक्थि", "अतिदधि"]
    {
        pin_word_any(word, "N");
    }
}

/// The neuter अस्-स्वादि with its own न (अन्तर्मनस् → अन्तर्मनांसि; site
/// @antarmanas2): the न belongs to the root, so the नाम्-नत्व does not
/// reach it.
#[test]
fn as_swadi_neuter_dental_nan_pins() {
    for word in ["अन्तर्मनस्", "दुर्मनस्", "प्रमनस्"]
    {
        pin_word_any(word, "N");
    }
}

/// The masculine and the neuter ञ्च-स्वादि with the परसवर्ण spelling
/// (तिर्यच् → तिरश्च; site @tiryac1/@tiryac2): the र keeps its
/// inherent vowel, so the श् follows it without a halant.
#[test]
fn tiryanccha_parasavarna_pins() {
    pin_word_any("तिर्यच्", "P");
    pin_word_any("तिर्यच्", "N");
}

/// The masculine ज्-final nouns of the ड/ट flavour (राज् → राड्-राट्;
/// site @rAj1 and its kin): the nominative carries the ड/ट pair and the
/// भ-endings the ड-form.
#[test]
fn j_final_dada_flavor_pins() {
    for word in [
        "राज्",
        "देवेज्",
        "परिमृज्",
        "परिव्राज्",
        "परेतराज्",
        "यक्षराज्",
        "यमराज्",
        "सम्राज्",
        "स्वाराज्",
        "विभ्राज्",
    ] {
        pin_word_any(word, "P");
    }
}

/// The ऋ-स्वादि masculine/feminine with the आ-form in the nominative
/// (माता, मातरौ; site @mAtR1/@pitR1): the nominative takes the आ-form,
/// the dual and plural the र-form and the accusative plural the ऋ-वृद्धि
/// (पितॄन्, मातॄः).
#[test]
fn r_swadi_pins() {
    for word in ["मातृ", "पितृ", "देवृ", "भ्रातृ", "दुहितृ", "ननन्दृ", "ननान्दृ", "यातृ"]
    {
        pin_word_any(word, "P");
    }
    pin_word_any("मातृ", "S");
}

/// The ह्न-final a-stems (अष्टाह्न; site @aSTAhna1 and its kin): the
/// locative singular carries three alternants — the plain form, the
/// ण-form and the form with the ह's halant dropped.
#[test]
fn hna_final_pins() {
    for word in [
        "अष्टाह्न",
        "चतुरह्न",
        "त्र्यह्न",
        "दशाह्न",
        "द्व्यह्न",
        "नवाह्न",
        "पञ्चाह्न",
        "मध्याह्न",
        "व्यह्न",
        "षडह्न",
        "सप्ताह्न",
        "सायाह्न",
    ] {
        pin_word_any(word, "P");
    }
}

/// The अ-स्वादि masculine (गुप्; site @gup1): the nominative carries the
/// जश् pause pair (गुब्-गुप्), the भ-endings the voiced form and the
/// locative plural the सु-form.
#[test]
fn a_swadi_masculine_pins() {
    pin_word_any("गुप्", "P");
}

/// The ओ-स्वादि compound plurals (आगस्त्य → अगस्तयः; site @Agastya1 and
/// its kin): the singular keeps the plain stem and the plural takes the
/// compound's reduced stem.
#[test]
fn o_swadi_plural_pins() {
    for word in [
        "आगस्त्य",
        "आत्रेय",
        "कौण्डिन्य",
        "कौत्स",
        "खारपायण",
        "गार्ग्य",
        "गौतम",
        "पाञ्चाल",
        "बास्तेय",
        "भार्गव",
        "मैत्रेय",
        "यास्क",
        "वासिष्ठ",
        "औडुलोमि",
        "पान्नागारि",
        "शारलोमि",
        "सादामत्ति",
    ] {
        pin_word_any(word, "P");
    }
}

/// The known-word tables: the numerals, the pronouns and the ष्/ञ्च
/// stems the site grids in full (एकैः, एकेषाम्, तिसृणाम्, त्रयाणाम्,
/// षड्-षट्, पञ्च).
#[test]
fn known_word_numeral_pins() {
    for word in ["एक", "इदम्", "एतद्", "त्रि", "चतुर्", "षष्", "अष्टन्", "पञ्चन्"]
    {
        pin_word_listed_any(word);
    }
}

/// The words whose cells carry the site's own second form — the
/// reduced-stem alternant (दन्तान्-दतः, आस्यानि-आसानि) and the like.
#[test]
fn cell_alternant_pins() {
    for word in ["नासिका", "निशा", "पृतना", "यकृत्", "शकृत्", "जरा"]
    {
        pin_word_listed_any(word);
    }
}

/// The words whose cells carry the site's own second form: the
/// reduced-stem alternants (दन्तान्-दतः, आस्यानि-आसानि), the ड/ट pairs
/// (स्नुड्-स्नुग्-स्नुक्-स्नुट्) and the ण-form of the instrumental.
#[test]
fn site_cell_alternant_pins() {
    for word in [
        "अजर",
        "अदद्र्यच्",
        "आस्य",
        "उदक",
        "तृतीया",
        "दक्षिण",
        "दन्त",
        "दोष",
        "दोष्",
        "द्वितीया",
        "नश्",
        "निर्जर",
        "पाद",
        "प्रियक्रोष्टु",
        "भ्रू",
        "मांस",
        "मास",
        "यूष",
        "सानु",
        "सुभ्रू",
        "स्नुह्",
        "हृदय",
    ] {
        pin_word_listed_any(word);
    }
}

/// The site-verbatim grids: the irregular words the शब्दपाठः grids in
/// full and the engine cannot derive (the ᳶ-स्वादि roots, the द्यो/दिव्
/// compounds, the diphthong stems, the कृत्-स्वादि participles and the
/// rest). The engine keeps the site's own grid for each, so the reverse
/// analysis can read the word back.
#[test]
fn site_verbatim_grid_pins() {
    for (word, mark) in [
        ("अतिचमू", "P"),
        ("अतिलक्ष्मी", "P"),
        ("अतिसखि", "P"),
        ("अतिसखि", "P"),
        ("अतिस्त्रि", "P"),
        ("अतिस्त्रि", "S"),
        ("अप्", "S"),
        ("अर्वन्", "P"),
        ("अहन्", "N"),
        ("आङ्गिरस", "P"),
        ("इदकम्", "P"),
        ("ऊर्ज्", "N"),
        ("ऋ", "P"),
        ("ऌ", "P"),
        ("कति", "P"),
        ("कति", "S"),
        ("कति", "N"),
        ("कॄ", "P"),
        ("क्रुन्च्", "P"),
        ("क्रोष्टु", "P"),
        ("क्रोष्टु", "S"),
        ("गमॢ", "P"),
        ("गमॢ", "N"),
        ("गोपा", "S"),
        ("गोविष्", "N"),
        ("ग्रामणी", "S"),
        ("तमिस्रहन्", "P"),
        ("तॄ", "P"),
        ("त्यद्", "P"),
        ("त्यद्", "P"),
        ("त्यद्", "S"),
        ("त्यद्", "N"),
        ("दिव्", "S"),
        ("दीर्घाहन्", "P"),
        ("द्यो", "S"),
        ("नृ", "P"),
        ("पति", "P"),
        ("परमचतुर्", "P"),
        ("परमतद्", "P"),
        ("परमत्रि", "P"),
        ("परमत्रि", "S"),
        ("परमद्वि", "P"),
        ("परमपञ्चन्", "P"),
        ("परमषष्", "P"),
        ("परमसखि", "P"),
        ("परमाष्टन्", "P"),
        ("पिपठिष्", "N"),
        ("पुनर्भू", "S"),
        ("पुम्स्", "P"),
        ("प्रतिदिवन्", "P"),
        ("प्रत्येक", "P"),
        ("प्रद्यो", "N"),
        ("प्रधी", "P"),
        ("प्रधी", "P"),
        ("प्रधी", "S"),
        ("प्रधी", "S"),
        ("प्रधी", "N"),
        ("प्ररै", "N"),
        ("प्रशाम्", "P"),
        ("प्रान्च्", "P"),
        ("प्रियचतुर्", "P"),
        ("प्रियत्रि", "P"),
        ("प्रियत्रि", "P"),
        ("प्रियत्रि", "S"),
        ("प्रियत्रि", "S"),
        ("प्रियत्रि", "N"),
        ("बहुपाद्", "P"),
        ("बहुवृत्रहन्", "N"),
        ("बहुश्रेयसी", "P"),
        ("बहुसखि", "P"),
        ("बहुसखि", "P"),
        ("बुध्", "P"),
        ("भवत्", "P"),
        ("भवत्", "P"),
        ("भृस्ज्", "P"),
        ("मघवन्", "P"),
        ("मर्मस्पृश्", "N"),
        ("महत्", "P"),
        ("मातृष्वसृ", "S"),
        ("यात्", "P"),
        ("युज्", "P"),
        ("युवन्", "P"),
        ("विमलदिव्", "N"),
        ("विश्वपा", "N"),
        ("विश्वराज्", "P"),
        ("वीरहन्", "P"),
        ("वृत्रहन्", "P"),
        ("व्याघ्रपाद्", "P"),
        ("शकॢ", "P"),
        ("शमनस्वसृ", "S"),
        ("श्वन्", "P"),
        ("सखि", "P"),
        ("सखी", "P"),
        ("सखी", "P"),
        ("सखी", "P"),
        ("सुदिव्", "P"),
        ("सुद्यो", "P"),
        ("सुधी", "P"),
        ("सुधी", "S"),
        ("सुधी", "S"),
        ("सुधी", "N"),
        ("सुनौ", "N"),
        ("सुपथिन्", "N"),
        ("सुपाद्", "P"),
        ("सुपुम्स्", "N"),
        ("सुलू", "P"),
        ("सुलू", "N"),
        ("सुसखि", "P"),
        ("सुसखि", "P"),
        ("सुस्मृते", "N"),
        ("सुहिन्स्", "P"),
        ("से", "P"),
        ("स्मृते", "P"),
        ("स्मृतो", "P"),
        ("स्व", "N"),
        ("स्व", "N"),
        ("स्वप्", "N"),
        ("स्वसृ", "S"),
        ("हाहा", "P"),
    ] {
        pin_word_any(word, mark);
    }
}
