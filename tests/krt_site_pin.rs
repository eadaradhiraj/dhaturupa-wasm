//! Site-pinned grids for the कृदन्ते click-through.
//!
//! Every [`Pin`] is one click-through (कृत्, field, value, dhātu row)
//! captured from ashtadhyayi.com's own declension engine — the site's
//! `createSubantaForms` conventions: m/n pass the row base, f passes the f
//! form when it ends in आ/ई — on 2026-10-17, then confirmed cell-exact
//! against `krt_subanta` across the full corpus by the parity harness
//! (902 cases, 49,536 cells; the only remaining diffs are the documented
//! site-malformed अथुच् m / क्वरप् f cases).
//!
//! `expect` is the site's 24 cells in fixed order (prathamA…sambodhana ×
//! ekavacanam…bahuvacanam): each cell's forms — the leading `हे ` of the
//! सम्बोधन cells stripped — sorted and joined with `/`, cells joined with
//! `|`. These are hard pins: any engine/routing change that alters one of
//! these grids must be a deliberate, site-verified change.

use dhaturupa_wasm::wasm;
use serde_json::Value;
use std::collections::HashMap;

const VIB: [&str; 8] = [
    "prathamA",
    "dvitIyA",
    "tftIyA",
    "caturTI",
    "paJcamI",
    "zazWI",
    "saptamI",
    "sambodhana",
];
const VAC: [&str; 3] = ["ekavacanam", "dvivacanam", "bahuvacanam"];

struct Pin {
    krt: &'static str,
    field: &'static str,
    value: &'static str,
    baseindex: &'static str,
    expect: &'static str,
}

const PINS: &[Pin] = &[
    Pin {
        krt: "शतृ",
        field: "m",
        value: "भवत्",
        baseindex: "01.0001",
        // site @gacchat1
        expect: "भवन्|भवन्तौ|भवन्तः|भवन्तम्|भवन्तौ|भवतः|भवता|भवद्भ्याम्|भवद्भिः|भवते|भवद्भ्याम्|भवद्भ्यः|भवतः|भवद्भ्याम्|भवद्भ्यः|भवतः|भवतोः|भवताम्|भवति|भवतोः|भवत्सु|भवन्|भवन्तौ|भवन्तः",
    },
    Pin {
        krt: "शतृ",
        field: "m",
        value: "ददत्",
        baseindex: "03.0010",
        // site @dadat1
        expect: "ददत्/ददद्|ददतौ|ददतः|ददतम्|ददतौ|ददतः|ददता|ददद्भ्याम्|ददद्भिः|ददते|ददद्भ्याम्|ददद्भ्यः|ददतः|ददद्भ्याम्|ददद्भ्यः|ददतः|ददतोः|ददताम्|ददति|ददतोः|ददत्सु|ददत्/ददद्|ददतौ|ददतः",
    },
    Pin {
        krt: "शतृ",
        field: "n",
        value: "भवत्",
        baseindex: "01.0001",
        // site @pacat1
        expect: "भवत्/भवद्|भवन्ती|भवन्ति|भवत्/भवद्|भवन्ती|भवन्ति|भवता|भवद्भ्याम्|भवद्भिः|भवते|भवद्भ्याम्|भवद्भ्यः|भवतः|भवद्भ्याम्|भवद्भ्यः|भवतः|भवतोः|भवताम्|भवति|भवतोः|भवत्सु|भवत्/भवद्|भवन्ती|भवन्ति",
    },
    Pin {
        krt: "शतृ",
        field: "n",
        value: "ददत्",
        baseindex: "03.0010",
        // site @dadat2
        expect: "ददत्/ददद्|ददती|ददति/ददन्ति|ददत्/ददद्|ददती|ददति/ददन्ति|ददता|ददद्भ्याम्|ददद्भिः|ददते|ददद्भ्याम्|ददद्भ्यः|ददतः|ददद्भ्याम्|ददद्भ्यः|ददतः|ददतोः|ददताम्|ददति|ददतोः|ददत्सु|ददत्/ददद्|ददती|ददति/ददन्ति",
    },
    Pin {
        krt: "शतृ",
        field: "n",
        value: "तुदत्",
        baseindex: "06.0001",
        // site @tudat1
        expect: "तुदत्/तुदद्|तुदती/तुदन्ती|तुदन्ति|तुदत्/तुदद्|तुदती/तुदन्ती|तुदन्ति|तुदता|तुदद्भ्याम्|तुदद्भिः|तुदते|तुदद्भ्याम्|तुदद्भ्यः|तुदतः|तुदद्भ्याम्|तुदद्भ्यः|तुदतः|तुदतोः|तुदताम्|तुदति|तुदतोः|तुदत्सु|तुदत्/तुदद्|तुदती/तुदन्ती|तुदन्ति",
    },
    Pin {
        krt: "शतृ",
        field: "n",
        value: "क्रीणत्",
        baseindex: "09.0001",
        // site @viyat1
        expect: "क्रीणत्/क्रीणद्|क्रीणती|क्रीणन्ति|क्रीणत्/क्रीणद्|क्रीणती|क्रीणन्ति|क्रीणता|क्रीणद्भ्याम्|क्रीणद्भिः|क्रीणते|क्रीणद्भ्याम्|क्रीणद्भ्यः|क्रीणतः|क्रीणद्भ्याम्|क्रीणद्भ्यः|क्रीणतः|क्रीणतोः|क्रीणताम्|क्रीणति|क्रीणतोः|क्रीणत्सु|क्रीणत्/क्रीणद्|क्रीणती|क्रीणन्ति",
    },
    Pin {
        krt: "स्य-शतृ",
        field: "n",
        value: "दास्यत्",
        baseindex: "03.0010",
        // site @tudat1
        expect: "दास्यत्/दास्यद्|दास्यती/दास्यन्ती|दास्यन्ति|दास्यत्/दास्यद्|दास्यती/दास्यन्ती|दास्यन्ति|दास्यता|दास्यद्भ्याम्|दास्यद्भिः|दास्यते|दास्यद्भ्याम्|दास्यद्भ्यः|दास्यतः|दास्यद्भ्याम्|दास्यद्भ्यः|दास्यतः|दास्यतोः|दास्यताम्|दास्यति|दास्यतोः|दास्यत्सु|दास्यत्/दास्यद्|दास्यती/दास्यन्ती|दास्यन्ति",
    },
    Pin {
        krt: "शतृ",
        field: "f",
        value: "भवन्ती",
        baseindex: "01.0001",
        // site @devI1
        expect: "भवन्ती|भवन्त्यौ|भवन्त्यः|भवन्तीम्|भवन्त्यौ|भवन्तीः|भवन्त्या|भवन्तीभ्याम्|भवन्तीभिः|भवन्त्यै|भवन्तीभ्याम्|भवन्तीभ्यः|भवन्त्याः|भवन्तीभ्याम्|भवन्तीभ्यः|भवन्त्याः|भवन्त्योः|भवन्तीनाम्|भवन्त्याम्|भवन्त्योः|भवन्तीषु|भवन्ति|भवन्त्यौ|भवन्त्यः",
    },
    Pin {
        krt: "अच्",
        field: "f",
        value: "नदी",
        baseindex: "01.0056",
        // site @devI1
        expect: "नदी|नद्यौ|नद्यः|नदीम्|नद्यौ|नदीः|नद्या|नदीभ्याम्|नदीभिः|नद्यै|नदीभ्याम्|नदीभ्यः|नद्याः|नदीभ्याम्|नदीभ्यः|नद्याः|नद्योः|नदीनाम्|नद्याम्|नद्योः|नदीषु|नदि|नद्यौ|नद्यः",
    },
    Pin {
        krt: "अच्",
        field: "f",
        value: "अञ्चा",
        baseindex: "01.1000",
        // site @mAlA1
        expect: "अञ्चा|अञ्चे|अञ्चाः|अञ्चाम्|अञ्चे|अञ्चाः|अञ्चया|अञ्चाभ्याम्|अञ्चाभिः|अञ्चायै|अञ्चाभ्याम्|अञ्चाभ्यः|अञ्चायाः|अञ्चाभ्याम्|अञ्चाभ्यः|अञ्चायाः|अञ्चयोः|अञ्चानाम्|अञ्चायाम्|अञ्चयोः|अञ्चासु|अञ्चे|अञ्चे|अञ्चाः",
    },
    Pin {
        krt: "क्वसु",
        field: "m",
        value: "ददिवस्",
        baseindex: "03.0010",
        // site @sedivas1
        expect: "ददिवान्|ददिवांसौ|ददिवांसः|ददिवांसम्|ददिवांसौ|ददुषः|ददुषा|ददिवद्भ्याम्|ददिवद्भिः|ददुषे|ददिवद्भ्याम्|ददिवद्भ्यः|ददुषः|ददिवद्भ्याम्|ददिवद्भ्यः|ददुषः|ददुषोः|ददुषाम्|ददुषि|ददुषोः|ददिवत्सु|ददिवन्|ददिवांसौ|ददिवांसः",
    },
    Pin {
        krt: "क्वसु",
        field: "n",
        value: "ददिवस्",
        baseindex: "03.0010",
        // site @vidvas2
        expect: "ददिवत्/ददिवद्|दद्युषी|ददिवांसि|ददिवत्/ददिवद्|दद्युषी|ददिवांसि|दद्युषा|ददिवद्भ्याम्|ददिवद्भिः|दद्युषे|ददिवद्भ्याम्|ददिवद्भ्यः|दद्युषः|ददिवद्भ्याम्|ददिवद्भ्यः|दद्युषः|दद्युषोः|दद्युषाम्|दद्युषि|दद्युषोः|ददिवत्सु|ददिवत्/ददिवद्|दद्युषी|ददिवांसि",
    },
    Pin {
        krt: "तृच्",
        field: "n",
        value: "दातृ",
        baseindex: "03.0010",
        // site @dhAtR2
        expect: "दातृ|दातृणी|दातॄणि|दातृ|दातृणी|दातॄणि|दातृणा/दात्रा|दातृभ्याम्|दातृभिः|दातृणे/दात्रे|दातृभ्याम्|दातृभ्यः|दातुः/दातृणः|दातृभ्याम्|दातृभ्यः|दातुः/दातृणः|दातृणोः/दात्रोः|दातॄणाम्|दातरि/दातृणि|दातृणोः/दात्रोः|दातृषु|दातः/दातृ|दातृणी|दातॄणि",
    },
    Pin {
        krt: "क्तिन्",
        field: "f",
        value: "भूति",
        baseindex: "01.0001",
        // site @gati1
        expect: "भूतिः|भूती|भूतयः|भूतिम्|भूती|भूतीः|भूत्या|भूतिभ्याम्|भूतिभिः|भूतये/भूत्यै|भूतिभ्याम्|भूतिभ्यः|भूतेः/भूत्याः|भूतिभ्याम्|भूतिभ्यः|भूतेः/भूत्याः|भूत्योः|भूतीनाम्|भूतौ/भूत्याम्|भूत्योः|भूतिषु|भूते|भूती|भूतयः",
    },
    Pin {
        krt: "ङ्वनिप्",
        field: "n",
        value: "यज्वन्",
        baseindex: "01.1157",
        // site @sadman1
        expect: "यज्व|यज्वनी|यज्वानि|यज्व|यज्वनी|यज्वानि|यज्वना|यज्वभ्याम्|यज्वभिः|यज्वने|यज्वभ्याम्|यज्वभ्यः|यज्वनः|यज्वभ्याम्|यज्वभ्यः|यज्वनः|यज्वनोः|यज्वनाम्|यज्वनि|यज्वनोः|यज्वसु|यज्व/यज्वन्|यज्वनी|यज्वानि",
    },
    Pin {
        krt: "घिनुण्",
        field: "m",
        value: "दोहिन्",
        baseindex: "01.0839",
        // site @zazin1
        expect: "दोही|दोहिनौ|दोहिनः|दोहिनम्|दोहिनौ|दोहिनः|दोहिना|दोहिभ्याम्|दोहिभिः|दोहिने|दोहिभ्याम्|दोहिभ्यः|दोहिनः|दोहिभ्याम्|दोहिभ्यः|दोहिनः|दोहिनोः|दोहिनाम्|दोहिनि|दोहिनोः|दोहिषु|दोहिन्|दोहिनौ|दोहिनः",
    },
    Pin {
        krt: "घिनुण्",
        field: "n",
        value: "दोहिन्",
        baseindex: "01.0839",
        // site @kSamin2
        expect: "दोहि|दोहिनी|दोहीनि|दोहि|दोहिनी|दोहीनि|दोहिना|दोहिभ्याम्|दोहिभिः|दोहिने|दोहिभ्याम्|दोहिभ्यः|दोहिनः|दोहिभ्याम्|दोहिभ्यः|दोहिनः|दोहिनोः|दोहिनाम्|दोहिनि|दोहिनोः|दोहिषु|दोहि/दोहिन्|दोहिनी|दोहीनि",
    },
    Pin {
        krt: "उ",
        field: "n",
        value: "भिक्षु",
        baseindex: "01.0690",
        // site @kaTu4
        expect: "भिक्षु|भिक्षुणी|भिक्षूणि|भिक्षु|भिक्षुणी|भिक्षूणि|भिक्षुणा|भिक्षुभ्याम्|भिक्षुभिः|भिक्षवे/भिक्षुणे|भिक्षुभ्याम्|भिक्षुभ्यः|भिक्षुणः/भिक्षोः|भिक्षुभ्याम्|भिक्षुभ्यः|भिक्षुणः/भिक्षोः|भिक्षुणोः/भिक्ष्वोः|भिक्षूणाम्|भिक्षुणि/भिक्षौ|भिक्षुणोः/भिक्ष्वोः|भिक्षुषु|भिक्षु/भिक्षो|भिक्षुणी|भिक्षूणि",
    },
    Pin {
        krt: "अतृन्",
        field: "n",
        value: "जरत्",
        baseindex: "04.0025",
        // site @jagat1
        expect: "जरत्/जरद्|जरती|जरन्ति|जरत्/जरद्|जरती|जरन्ति|जरता|जरद्भ्याम्|जरद्भिः|जरते|जरद्भ्याम्|जरद्भ्यः|जरतः|जरद्भ्याम्|जरद्भ्यः|जरतः|जरतोः|जरताम्|जरति|जरतोः|जरत्सु|जरत्/जरद्|जरती|जरन्ति",
    },
    Pin {
        krt: "यत्",
        field: "n",
        value: "ग्रन्थ्य",
        baseindex: "10.0375",
        // site @phala1
        expect: "ग्रन्थ्यम्|ग्रन्थ्ये|ग्रन्थ्यानि|ग्रन्थ्यम्|ग्रन्थ्ये|ग्रन्थ्यानि|ग्रन्थ्येन|ग्रन्थ्याभ्याम्|ग्रन्थ्यैः|ग्रन्थ्याय|ग्रन्थ्याभ्याम्|ग्रन्थ्येभ्यः|ग्रन्थ्यात्/ग्रन्थ्याद्|ग्रन्थ्याभ्याम्|ग्रन्थ्येभ्यः|ग्रन्थ्यस्य|ग्रन्थ्ययोः|ग्रन्थ्यानाम्|ग्रन्थ्ये|ग्रन्थ्ययोः|ग्रन्थ्येषु|ग्रन्थ्य|ग्रन्थ्ये|ग्रन्थ्यानि",
    },
    Pin {
        krt: "ण्वुल्",
        field: "m",
        value: "भावक",
        baseindex: "01.0001",
        // site @deva1
        expect: "भावकः|भावकौ|भावकाः|भावकम्|भावकौ|भावकान्|भावकेन|भावकाभ्याम्|भावकैः|भावकाय|भावकाभ्याम्|भावकेभ्यः|भावकात्/भावकाद्|भावकाभ्याम्|भावकेभ्यः|भावकस्य|भावकयोः|भावकानाम्|भावके|भावकयोः|भावकेषु|भावक|भावकौ|भावकाः",
    },
];

/// The 24-cell signature of a `krt_subanta` JSON envelope: for each
/// vibhakti × vacana (fixed order), the cell's forms (leading `हे `
/// stripped), sorted, joined with `/`; cells joined with `|`.
fn signature(payload: &Value) -> String {
    let mut grid: HashMap<(String, String), String> = HashMap::new();
    if let Some(tables) = payload["tables"].as_array() {
        for tab in tables {
            let Some(cells) = tab["cells"].as_array() else {
                continue;
            };
            for cell in cells {
                let (Some(v), Some(c)) = (cell["vibhakti"].as_str(), cell["vacana"].as_str())
                else {
                    continue;
                };
                let mut forms: Vec<String> = Vec::new();
                if let Some(fs) = cell["forms"].as_array() {
                    for f in fs.iter().filter_map(|f| f.as_str()) {
                        let s = f.strip_prefix("हे ").unwrap_or(f).trim();
                        if !s.is_empty() {
                            forms.push(s.to_string());
                        }
                    }
                }
                forms.sort();
                grid.insert((v.to_string(), c.to_string()), forms.join("/"));
            }
        }
    }
    let mut parts = Vec::with_capacity(24);
    for v in VIB {
        for c in VAC {
            parts.push(
                grid.get(&(v.to_string(), c.to_string()))
                    .cloned()
                    .unwrap_or_default(),
            );
        }
    }
    parts.join("|")
}

#[test]
fn site_pinned_click_through_grids() {
    for (i, pin) in PINS.iter().enumerate() {
        let out = wasm::krt_subanta("shuddha", pin.krt, pin.field, pin.value, pin.baseindex);
        let payload: Value = serde_json::from_str(&out).expect("json envelope");
        assert_eq!(
            payload["ok"],
            Value::Bool(true),
            "case {i}: {} {} {} @{} => {out}",
            pin.krt,
            pin.field,
            pin.value,
            pin.baseindex
        );
        assert_eq!(
            signature(&payload),
            pin.expect,
            "case {i}: {} {} {} @{}",
            pin.krt,
            pin.field,
            pin.value,
            pin.baseindex
        );
    }
}
