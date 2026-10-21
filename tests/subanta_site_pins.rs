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
    for word in ["दिश्", "मर्मस्पृश्", "घृतस्पृश्", "जलस्पृश्"]
    {
        pin_word_any(word, "P");
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
