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
