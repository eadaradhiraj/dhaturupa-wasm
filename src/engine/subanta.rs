//! Subanta generation + reverse lookup (F4/F5).
//!
//! Input: stem (SLP1) + linga (`m`/`f`/`n`) -> full table (8 vibhakti x 3
//! vacana, sambodhana eighth). Invalid stem+linga -> literal `not possible`.
//!
//! Design: explicit paradigm recipes per class (vowel stems, `f`/`t`/`n`/
//! `s` consonant stems, pronouns, numerals) + suffix-strip reverse analysis
//! with regenerate-and-confirm (sound by construction; over-analysis is
//! ordered krt/lexicon-backed first, bare stems after). Case-sensitive SLP1
//! throughout (`e/E`, `n/N` distinct).

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Linga {
    M,
    F,
    N,
}

impl Linga {
    /// Parse `m`/`f`/`n` (exact, lowercase only).
    #[must_use]
    pub const fn parse(s: &str) -> Option<Self> {
        match s.as_bytes() {
            [b'm'] => Some(Self::M),
            [b'f'] => Some(Self::F),
            [b'n'] => Some(Self::N),
            _ => None,
        }
    }

    /// Display `m`/`f`/`n`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::M => "m",
            Self::F => "f",
            Self::N => "n",
        }
    }
}

impl fmt::Display for Linga {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Nominal class selecting the paradigm recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Class {
    /// `a`-stem masculine (`rAma`).
    AM,
    /// `a`-stem neuter (`vana`).
    AN,
    /// `A`-stem feminine (`ramA`).
    AAF,
    /// `i`-stem masculine/feminine (`kavi`/`mati`).
    IMF,
    /// `i`-stem neuter (`vAri`).
    IN,
    /// `I`-stem feminine (`nadI`, `BavantI`, `awavI`).
    IIF,
    /// `u`-stem masculine/feminine (`guru`/`Denu`).
    UMF,
    /// `u`-stem neuter (`maDu`).
    UN,
    /// `U`-stem feminine (`vaDU`).
    UUF,
    /// `f`-stem masculine (`kartf`); feminine approximated (`NOM/VOC.SG` =
    /// stem, rest masculine — true feminine agents use `-trI`/I-stems).
    RFM,
    /// `t`-stem masculine (`Bavat` weak; `Bavan` strong nom).
    AtM,
    /// `t`-stem neuter (`Bavat`/`Bavad`).
    AtN,
    /// `an`-stem masculine (`rAjan`).
    AnM,
    /// `in`-stem masculine (`balin`).
    InM,
    /// `as`-stem neuter (`manas`).
    AsN,
    /// `as`-stem masculine (`vedHas`).
    AsM,
    /// `vas`-stem masculine (`vidvas`, kvasu).
    VasM,
    /// `vant`-stem masculine (`BUtavat`, ktavatu; nom. sg. `-vAn`,
    /// unlike at-stems). Selected by kṛt, not by ending, so at-stems
    /// ending in `vat` (e.g. `sunvat`) stay `AtM`.
    VantM,
    /// Closed pronoun/numeral word (declines by its own table).
    Word(&'static str),
}

impl Class {
    /// Short label for UI display.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::AM => "a-m".to_string(),
            Self::AN => "a-n".to_string(),
            Self::AAF => "A-f".to_string(),
            Self::IMF => "i-m/f".to_string(),
            Self::IN => "i-n".to_string(),
            Self::IIF => "I-f".to_string(),
            Self::UMF => "u-m/f".to_string(),
            Self::UN => "u-n".to_string(),
            Self::UUF => "U-f".to_string(),
            Self::RFM => "f-m/f".to_string(),
            Self::AtM => "at-m".to_string(),
            Self::AtN => "at-n".to_string(),
            Self::AnM => "an-m".to_string(),
            Self::InM => "in-m".to_string(),
            Self::AsN => "as-n".to_string(),
            Self::AsM => "as-m".to_string(),
            Self::VasM => "vas-m".to_string(),
            Self::VantM => "vant-m".to_string(),
            Self::Word(w) => (*w).to_string(),
        }
    }
}

/// One table slot (vibhakti × vacana) with its exact surface form(s).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub vibhakti: &'static str,
    pub vacana: &'static str,
    pub forms: Vec<String>,
}

impl Slot {
    // In-progress M5 API (used by upcoming table builders).
    #[allow(dead_code)]
    fn one(vibhakti: &'static str, vacana: &'static str, form: String) -> Self {
        Self {
            vibhakti,
            vacana,
            forms: vec![form],
        }
    }
}

/// Full declension table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paradigm {
    pub stem: String,
    pub linga: Linga,
    pub class: Class,
    pub cells: Vec<Slot>,
}

impl Paradigm {
    /// Every surface form in the table.
    #[must_use]
    pub fn all_forms(&self) -> Vec<&str> {
        self.cells
            .iter()
            .flat_map(|c| c.forms.iter().map(String::as_str))
            .collect()
    }

    /// True when `form` occurs anywhere in the table.
    #[must_use]
    pub fn contains(&self, form: &str) -> bool {
        self.cells.iter().any(|c| c.forms.iter().any(|f| f == form))
    }
}

pub const NOT_POSSIBLE: &str = "not possible";

const EKA: &str = "ekavacanam";
const DVI: &str = "dvivacanam";
const BAHU: &str = "bahuvacanam";

/// Build the 24-slot table for a validated (stem, linga, class).
///
/// `stem` is the declension stem (weak `Bavat` for at-stems, full `rAjan`
/// for an-stems). Returns [`NOT_POSSIBLE`] for ending/class mismatches.
pub fn decline(stem: &str, linga: Linga, class: &Class) -> Result<Paradigm, &'static str> {
    if stem.is_empty() || !stem.bytes().all(crate::slp1::is_slp1_letter_byte) {
        return Err(NOT_POSSIBLE);
    }
    let cells = match class {
        Class::AM => {
            if linga != Linga::M || !stem.ends_with('a') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(p, "aH"), f(p, "O"), f(p, "AH")),
                (f(p, "am"), f(p, "O"), f(p, "An")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "Aya"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "At"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayOH"), f(p, "AnAm")),
                (f(p, "e"), f(p, "ayOH"), f(p, "ezu")),
                (stem.to_string(), f(p, "O"), f(p, "AH")),
            ])
        }
        Class::AN => {
            if linga != Linga::N || !stem.ends_with('a') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "Aya"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "At"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayOH"), f(p, "AnAm")),
                (f(p, "e"), f(p, "ayOH"), f(p, "ezu")),
                (stem.to_string(), f(p, "e"), f(p, "Ani")),
            ])
        }
        Class::AAF => {
            if linga != Linga::F || !stem.ends_with('A') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "e"), f(p, "AH")),
                (f(p, "Am"), f(p, "e"), f(p, "AH")),
                (f(p, "ayA"), f(p, "AByAm"), f(p, "ABiH")),
                (f(p, "AyE"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "AyAH"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "AyAH"), f(p, "ayOH"), f(p, "AnAm")),
                (f(p, "AyAm"), f(p, "ayOH"), f(p, "Asu")),
                (f(p, "e"), f(p, "e"), f(p, "AH")),
            ])
        }
        Class::IMF => {
            if linga == Linga::N || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(p, "iH"), f(p, "I"), f(p, "ayaH")),
                (f(p, "im"), f(p, "I"), f(p, "In")),
                (f(p, "inA"), f(p, "iByAm"), f(p, "iBiH")),
                (f(p, "aye"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "eH"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "eH"), f(p, "yOH"), f(p, "InAm")),
                (f(p, "O"), f(p, "yOH"), f(p, "izu")),
                (f(p, "e"), f(p, "I"), f(p, "ayaH")),
            ])
        }
        Class::IN => {
            if linga != Linga::N || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "iRI"), f(p, "IRi")),
                (stem.to_string(), f(p, "iRI"), f(p, "IRi")),
                (f(p, "iRA"), f(p, "iByAm"), f(p, "iBiH")),
                (f(p, "iRe"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "iRaH"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "iRaH"), f(p, "iRoH"), f(p, "IRAm")),
                (f(p, "iRi"), f(p, "iRoH"), f(p, "izu")),
                (f(p, "e"), f(p, "iRI"), f(p, "IRi")),
            ])
        }
        Class::IIF => {
            if linga != Linga::F || !stem.ends_with('I') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "yO"), f(p, "yaH")),
                (f(p, "Im"), f(p, "yO"), f(p, "IH")),
                (f(p, "yA"), f(p, "IByAm"), f(p, "IBiH")),
                (f(p, "yE"), f(p, "IByAm"), f(p, "IByaH")),
                (f(p, "yAH"), f(p, "IByAm"), f(p, "IByaH")),
                (f(p, "yAH"), f(p, "yOH"), f(p, "InAm")),
                (f(p, "yAm"), f(p, "yOH"), f(p, "Izu")),
                (f(p, "i"), f(p, "yO"), f(p, "yaH")),
            ])
        }
        Class::UMF => {
            if linga == Linga::N || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(p, "uH"), f(p, "U"), f(p, "avaH")),
                (f(p, "um"), f(p, "U"), f(p, "Un")),
                (f(p, "unA"), f(p, "uByAm"), f(p, "uBiH")),
                (f(p, "ave"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "oH"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "oH"), f(p, "vOH"), f(p, "UnAm")),
                (f(p, "O"), f(p, "vOH"), f(p, "uzu")),
                (f(p, "o"), f(p, "U"), f(p, "avaH")),
            ])
        }
        Class::UN => {
            if linga != Linga::N || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "unI"), f(p, "Uni")),
                (stem.to_string(), f(p, "unI"), f(p, "Uni")),
                (f(p, "unA"), f(p, "uByAm"), f(p, "uBiH")),
                (f(p, "une"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "unaH"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "unaH"), f(p, "unOH"), f(p, "UnAm")),
                (f(p, "uni"), f(p, "unOH"), f(p, "uzu")),
                (f(p, "o"), f(p, "unI"), f(p, "Uni")),
            ])
        }
        Class::UUF => {
            if linga != Linga::F || !stem.ends_with('U') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "vO"), f(p, "vaH")),
                (f(p, "Um"), f(p, "vO"), f(p, "UH")),
                (f(p, "vA"), f(p, "UByAm"), f(p, "UBiH")),
                (f(p, "vE"), f(p, "UByAm"), f(p, "UByaH")),
                (f(p, "vAH"), f(p, "UByAm"), f(p, "UByaH")),
                (f(p, "vAH"), f(p, "vOH"), f(p, "UnAm")),
                (f(p, "vAm"), f(p, "vOH"), f(p, "Uzu")),
                (f(p, "u"), f(p, "vO"), f(p, "vaH")),
            ])
        }
        Class::RFM => {
            if linga == Linga::N || !stem.ends_with('f') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let nom_sg = if linga == Linga::M {
                f(p, "A")
            } else {
                stem.to_string()
            };
            let voc_sg = if linga == Linga::M {
                f(p, "ar")
            } else {
                stem.to_string()
            };
            table(&[
                (nom_sg, f(p, "ArO"), f(p, "AraH")),
                (f(p, "Aram"), f(p, "ArO"), f(p, "Fn")),
                (f(p, "rA"), f(p, "fByAm"), f(p, "fBiH")),
                (f(p, "re"), f(p, "fByAm"), f(p, "fByaH")),
                (f(p, "ur"), f(p, "fByAm"), f(p, "fByaH")),
                (f(p, "ur"), f(p, "roH"), f(p, "FRAm")),
                (f(p, "ari"), f(p, "roH"), f(p, "fzu")),
                (voc_sg, f(p, "ArO"), f(p, "AraH")),
            ])
        }
        Class::AtM => {
            if linga != Linga::M || !stem.ends_with('t') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let q = &stem[..stem.len() - 2];
            let strong = format!("{q}ant");
            let d = format!("{p}d");
            table(&[
                (f(q, "an"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(stem, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(stem, "OH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "OH"), f(stem, "su")),
                (f(q, "an"), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::VantM => {
            // मतुप्/वतुप् masculine: like AtM, but prathamā-ekavacana
            // takes vṛddhi strong (`BUtavAn`, not `*BUtavan`); the
            // vocative keeps the short weak (`BUtavan`, as in `Bagavan`).
            // Weak stems end `vat` (normalized by `krt_class`).
            if linga != Linga::M || !stem.ends_with("vat") || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let q = &stem[..stem.len() - 3];
            let strong = format!("{q}vant");
            let d = format!("{p}d");
            table(&[
                (format!("{q}vAn"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(stem, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(stem, "OH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "OH"), f(stem, "su")),
                (format!("{q}van"), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::AtN => {
            if linga != Linga::N || !(stem.ends_with('t') || stem.ends_with('d')) {
                return Err(NOT_POSSIBLE);
            }
            let weak = normalize_dental(stem);
            let dvar = format!("{}d", &weak[..weak.len() - 1]);
            let d = format!("{}d", &weak[..weak.len() - 1]);
            // Nom/acc/voc singular carry both `t`/`d` finals (`Bavat/Bavad`).
            let nom: Vec<String> = vec![weak.clone(), dvar];
            let mut cells = table(&[
                (String::new(), f(&weak, "I"), f(&weak, "anti")),
                (String::new(), f(&weak, "I"), f(&weak, "anti")),
                (f(&weak, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(&weak, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&weak, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&weak, "aH"), f(&weak, "OH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "OH"), f(&weak, "su")),
                (String::new(), f(&weak, "I"), f(&weak, "anti")),
            ]);
            for cell in &mut cells {
                if cell.vacana == EKA
                    && (cell.vibhakti == "prathamA"
                        || cell.vibhakti == "dvitIyA"
                        || cell.vibhakti == "sambodhana")
                {
                    cell.forms.clone_from(&nom);
                }
            }
            cells
        }
        Class::AnM => {
            if linga != Linga::M || !stem.ends_with("an") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let an = format!("{p}An");
            let y = format!("{p}Y");
            let az = format!("{p}a");
            table(&[
                (f(p, "A"), f(&an, "O"), f(&an, "aH")),
                (f(&an, "am"), f(&an, "O"), f(&y, "aH")),
                (f(&y, "A"), f(&az, "ByAm"), f(&az, "BiH")),
                (f(&y, "e"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&y, "aH"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&y, "aH"), f(&y, "oH"), f(&y, "Am")),
                (f(&y, "i"), f(&y, "oH"), f(&az, "su")),
                (stem.to_string(), f(&an, "O"), f(&an, "aH")),
            ])
        }
        Class::InM => {
            if linga != Linga::M || !stem.ends_with("in") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let strong = format!("{p}in");
            let weak = format!("{p}i");
            table(&[
                (f(p, "I"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "A"), f(&weak, "ByAm"), f(&weak, "BiH")),
                (f(&strong, "e"), f(&weak, "ByAm"), f(&weak, "ByaH")),
                (f(&strong, "aH"), f(&weak, "ByAm"), f(&weak, "ByaH")),
                (f(&strong, "aH"), f(&strong, "OH"), f(&strong, "Am")),
                (f(&strong, "i"), f(&strong, "OH"), f(&weak, "zu")),
                (stem.to_string(), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::AsN => {
            if linga != Linga::N || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            table(&[
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "OH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "OH"), f(&w, "su")),
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
            ])
        }
        Class::AsM => {
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            table(&[
                (f(q, "AH"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "OH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "OH"), f(&w, "su")),
                (f(q, "AH"), f(&w, "O"), f(&w, "aH")),
            ])
        }
        Class::VasM => {
            if linga != Linga::M || !stem.ends_with("as") || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let vams = format!("{p}AMs");
            // samprasarana: `vas` -> `uz` (the `v` drops: `vidvas` -> `viduz`).
            let uz_base = &p[..p.len() - 1];
            let uz = format!("{uz_base}uz");
            let vad = format!("{p}ad");
            let van = format!("{p}An");
            table(&[
                (van.clone(), f(&vams, "O"), f(&vams, "aH")),
                (f(&vams, "am"), f(&vams, "O"), f(&uz, "aH")),
                (f(&uz, "A"), f(&vad, "ByAm"), f(&vad, "BiH")),
                (f(&uz, "e"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&uz, "oH"), f(&uz, "Am")),
                (f(&uz, "i"), f(&uz, "oH"), f(&vad, "su")),
                (format!("{p}an"), f(&vams, "O"), f(&vams, "aH")),
            ])
        }
        Class::Word(w) => return word_table(stem, linga, w),
    };
    // Word-level Natva over recipe outputs (`gurunA -> guruRA`,
    // `balinA` stays: no trigger). Hand-written pronoun/numeral tables
    // bypass this (already correct) via the early return above.
    // Forms spelled exactly like the citation stem stay unvoiced: lookup
    // is by surface spelling (SPEC F5 trio `Wrampena` must remain findable
    // as its own vocative; utterance voicing is a separate layer).
    let cells = natva_cells(cells, stem);
    Ok(Paradigm {
        stem: stem.to_string(),
        linga,
        class: class.clone(),
        cells,
    })
}

/// Apply [`crate::engine::sandhi::natva`] to every form in recipe-built cells
/// (no stack prefix, so `pre_len` is zero), except forms spelled exactly
/// like the citation stem (lookup identity).
fn natva_cells(cells: Vec<Slot>, stem: &str) -> Vec<Slot> {
    cells
        .into_iter()
        .map(|mut c| {
            c.forms = c
                .forms
                .into_iter()
                .map(|s| {
                    if s == stem {
                        s
                    } else {
                        crate::engine::sandhi::natva(
                            &s, 0, false, false, false, false, false, false, false,
                        )
                    }
                })
                .collect();
            c
        })
        .collect()
}

fn f(base: &str, ending: &str) -> String {
    format!("{base}{ending}")
}

/// Assemble 24 cells from 8 rows of (eka, dvi, bahu).
fn table(rows: &[(String, String, String)]) -> Vec<Slot> {
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
    const VAC: [&str; 3] = [EKA, DVI, BAHU];
    let mut out = Vec::with_capacity(24);
    for (i, (a, b, c)) in rows.iter().enumerate() {
        for (j, form) in [a, b, c].iter().enumerate() {
            out.push(Slot {
                vibhakti: VIB[i],
                vacana: VAC[j],
                forms: vec![(*form).clone()],
            });
        }
    }
    out
}

/// Normalize dental-final weak stems (`Bavad` -> `Bavat`).
fn normalize_dental(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix('d') {
        format!("{stripped}t")
    } else {
        s.to_string()
    }
}

/// Normalize visarga-final citation (`manaH` -> `manas`).
fn normalize_visarga(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix('H') {
        format!("{stripped}s")
    } else {
        s.to_string()
    }
}

/// Closed pronoun/numeral tables. `stem` must equal the word.
fn word_table(stem: &str, linga: Linga, word: &'static str) -> Result<Paradigm, &'static str> {
    if stem != word {
        return Err(NOT_POSSIBLE);
    }
    let rows: Vec<(Vec<String>, Vec<String>, Vec<String>)> = match (word, linga) {
        ("sarva", Linga::M) => rows1(&[
            ["sarvaH", "sarvO", "sarve"],
            ["sarvam", "sarvO", "sarvAn"],
            ["sarveRa", "sarvAByAm", "sarvEH"],
            ["sarvasmE", "sarvAByAm", "sarveByaH"],
            ["sarvasmAt", "sarvAByAm", "sarveByaH"],
            ["sarvasya", "sarvayOH", "sarvezAm"],
            ["sarvasmin", "sarvayOH", "sarvezu"],
            ["sarva", "sarvO", "sarve"],
        ]),
        ("sarva", Linga::N) => rows1(&[
            ["sarvam", "sarve", "sarvARi"],
            ["sarvam", "sarve", "sarvARi"],
            ["sarveRa", "sarvAByAm", "sarvEH"],
            ["sarvasmE", "sarvAByAm", "sarveByaH"],
            ["sarvasmAt", "sarvAByAm", "sarveByaH"],
            ["sarvasya", "sarvayOH", "sarvezAm"],
            ["sarvasmin", "sarvayOH", "sarvezu"],
            ["sarva", "sarve", "sarvARi"],
        ]),
        ("sarva", Linga::F) => rows1(&[
            ["sarvA", "sarve", "sarvAH"],
            ["sarvAm", "sarve", "sarvAH"],
            ["sarvayA", "sarvAByAm", "sarvABiH"],
            ["sarvasyE", "sarvAByAm", "sarvAByaH"],
            ["sarvasyAH", "sarvAByAm", "sarvAByaH"],
            ["sarvasyAH", "sarvayOH", "sarvAsAm"],
            ["sarvasyAm", "sarvayOH", "sarvAsu"],
            ["sarve", "sarve", "sarvAH"],
        ]),
        ("tad", Linga::M) => rows1(&[
            ["saH", "tO", "te"],
            ["tam", "tO", "tAn"],
            ["tena", "tAByAm", "tEH"],
            ["tasmE", "tAByAm", "teByaH"],
            ["tasmAt", "tAByAm", "teByaH"],
            ["tasya", "tayOH", "tezAm"],
            ["tasmin", "tayOH", "tezu"],
            ["saH", "tO", "te"],
        ]),
        ("tad", Linga::N) => {
            let rows = rows1(&[
                ["tat", "te", "tAni"],
                ["tat", "te", "tAni"],
                ["tena", "tAByAm", "tEH"],
                ["tasmE", "tAByAm", "teByaH"],
                ["tasmAt", "tAByAm", "teByaH"],
                ["tasya", "tayOH", "tezAm"],
                ["tasmin", "tayOH", "tezu"],
                ["tat", "te", "tAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "tat", "tad");
            return Ok(Paradigm {
                stem: stem.to_string(),
                linga,
                class: Class::Word(word),
                cells,
            });
        }
        ("tad", Linga::F) => rows1(&[
            ["sA", "te", "tAH"],
            ["tAm", "te", "tAH"],
            ["tayA", "tAByAm", "tABiH"],
            ["tasyE", "tAByAm", "tAByaH"],
            ["tasyAH", "tAByAm", "tAByaH"],
            ["tasyAH", "tayOH", "tAsAm"],
            ["tasyAm", "tayOH", "tAsu"],
            ["sA", "te", "tAH"],
        ]),
        ("etad", Linga::M) => rows1(&[
            ["ezaH", "etO", "ete"],
            ["etam", "etO", "etAn"],
            ["etena", "etAByAm", "etEH"],
            ["etasmE", "etAByAm", "eteByaH"],
            ["etasmAt", "etAByAm", "eteByaH"],
            ["etasya", "etayOH", "etezAm"],
            ["etasmin", "etayOH", "etezu"],
            ["ezaH", "etO", "ete"],
        ]),
        ("etad", Linga::N) => {
            let rows = rows1(&[
                ["etat", "ete", "etAni"],
                ["etat", "ete", "etAni"],
                ["etena", "etAByAm", "etEH"],
                ["etasmE", "etAByAm", "eteByaH"],
                ["etasmAt", "etAByAm", "eteByaH"],
                ["etasya", "etayOH", "etezAm"],
                ["etasmin", "etayOH", "etezu"],
                ["etat", "ete", "etAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "etat", "etad");
            return Ok(Paradigm {
                stem: stem.to_string(),
                linga,
                class: Class::Word(word),
                cells,
            });
        }
        ("etad", Linga::F) => rows1(&[
            ["ezA", "ete", "etAH"],
            ["etAm", "ete", "etAH"],
            ["etayA", "etAByAm", "etABiH"],
            ["etasyE", "etAByAm", "etAByaH"],
            ["etasyAH", "etAByAm", "etAByaH"],
            ["etasyAH", "etayOH", "etAsAm"],
            ["etasyAm", "etayOH", "etAsu"],
            ["ezA", "ete", "etAH"],
        ]),
        ("yad", Linga::M) => rows1(&[
            ["yaH", "yO", "ye"],
            ["yam", "yO", "yAn"],
            ["yena", "yAByAm", "yEH"],
            ["yasmE", "yAByAm", "yeByaH"],
            ["yasmAt", "yAByAm", "yeByaH"],
            ["yasya", "yayOH", "yezAm"],
            ["yasmin", "yayOH", "yezu"],
            ["yaH", "yO", "ye"],
        ]),
        ("yad", Linga::N) => {
            let rows = rows1(&[
                ["yat", "ye", "yAni"],
                ["yat", "ye", "yAni"],
                ["yena", "yAByAm", "yEH"],
                ["yasmE", "yAByAm", "yeByaH"],
                ["yasmAt", "yAByAm", "yeByaH"],
                ["yasya", "yayOH", "yezAm"],
                ["yasmin", "yayOH", "yezu"],
                ["yat", "ye", "yAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "yat", "yad");
            return Ok(Paradigm {
                stem: stem.to_string(),
                linga,
                class: Class::Word(word),
                cells,
            });
        }
        ("yad", Linga::F) => rows1(&[
            ["yA", "ye", "yAH"],
            ["yAm", "ye", "yAH"],
            ["yayA", "yAByAm", "yABiH"],
            ["yasyE", "yAByAm", "yAByaH"],
            ["yasyAH", "yAByAm", "yAByaH"],
            ["yasyAH", "yayOH", "yAsAm"],
            ["yasyAm", "yayOH", "yAsu"],
            ["yA", "ye", "yAH"],
        ]),
        ("kim", Linga::M) => rows1(&[
            ["kaH", "kO", "ke"],
            ["kam", "kO", "kAn"],
            ["kena", "kAByAm", "kEH"],
            ["kasmE", "kAByAm", "keByaH"],
            ["kasmAt", "kAByAm", "keByaH"],
            ["kasya", "kayOH", "kezAm"],
            ["kasmin", "kayOH", "kezu"],
            ["kaH", "kO", "ke"],
        ]),
        ("kim", Linga::N) => rows1(&[
            ["kim", "ke", "kAni"],
            ["kim", "ke", "kAni"],
            ["kena", "kAByAm", "kEH"],
            ["kasmE", "kAByAm", "keByaH"],
            ["kasmAt", "kAByAm", "keByaH"],
            ["kasya", "kayOH", "kezAm"],
            ["kasmin", "kayOH", "kezu"],
            ["kim", "ke", "kAni"],
        ]),
        ("kim", Linga::F) => rows1(&[
            ["kA", "ke", "kAH"],
            ["kAm", "ke", "kAH"],
            ["kayA", "kAByAm", "kABiH"],
            ["kasyE", "kAByAm", "kAByaH"],
            ["kasyAH", "kAByAm", "kAByaH"],
            ["kasyAH", "kayOH", "kAsAm"],
            ["kasyAm", "kayOH", "kAsu"],
            ["kA", "ke", "kAH"],
        ]),
        ("idam", Linga::M) => rows1(&[
            ["ayam", "imO", "ime"],
            ["imam", "imO", "imAn"],
            ["anena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAt", "AByAm", "eByaH"],
            ["asya", "anayOH", "ezAm"],
            ["asmin", "anayOH", "ezu"],
            ["ayam", "imO", "ime"],
        ]),
        ("idam", Linga::N) => rows1(&[
            ["idam", "ime", "imAni"],
            ["idam", "ime", "imAni"],
            ["anena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAt", "AByAm", "eByaH"],
            ["asya", "anayOH", "ezAm"],
            ["asmin", "anayOH", "ezu"],
            ["idam", "ime", "imAni"],
        ]),
        ("idam", Linga::F) => rows1(&[
            ["iyam", "ime", "imAH"],
            ["imAm", "ime", "imAH"],
            ["anayA", "AByAm", "ABiH"],
            ["asyE", "AByAm", "AByaH"],
            ["asyAH", "AByAm", "AByaH"],
            ["asyAH", "anayOH", "AsAm"],
            ["asyAm", "anayOH", "Asu"],
            ["iyam", "ime", "imAH"],
        ]),
        ("adas", Linga::M) => rows1(&[
            ["asO", "amU", "amI"],
            ["amum", "amU", "amUn"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAt", "amUByAm", "amIByaH"],
            ["amuzya", "amuyOH", "amIzAm"],
            ["amuzmin", "amuyOH", "amIzu"],
            ["asO", "amU", "amI"],
        ]),
        ("adas", Linga::N) => rows1(&[
            ["adaH", "amU", "amUni"],
            ["adaH", "amU", "amUni"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAt", "amUByAm", "amIByaH"],
            ["amuzya", "amuyOH", "amIzAm"],
            ["amuzmin", "amuyOH", "amIzu"],
            ["adaH", "amU", "amUni"],
        ]),
        ("adas", Linga::F) => rows1(&[
            ["asO", "amU", "amUH"],
            ["amUm", "amU", "amUH"],
            ["amuyA", "amUByAm", "amUBiH"],
            ["amuzyE", "amUByAm", "amUByaH"],
            ["amuzyAH", "amUByAm", "amUByaH"],
            ["amuzyAH", "amuyOH", "amUzAm"],
            ["amuzyAm", "amuyOH", "amUzu"],
            ["asO", "amU", "amUH"],
        ]),
        ("yuzmad", _) => {
            return Ok(yuzmad_table(linga));
        }
        ("asmad", _) => {
            return Ok(asmad_table(linga));
        }
        ("eka", Linga::M) => rows1(&[
            ["ekaH", "", ""],
            ["ekam", "", ""],
            ["ekena", "", ""],
            ["ekasmE", "", ""],
            ["ekasmAt", "", ""],
            ["ekasya", "", ""],
            ["ekasmin", "", ""],
            ["eka", "", ""],
        ]),
        ("eka", Linga::N) => rows1(&[
            ["ekam", "", ""],
            ["ekam", "", ""],
            ["ekena", "", ""],
            ["ekasmE", "", ""],
            ["ekasmAt", "", ""],
            ["ekasya", "", ""],
            ["ekasmin", "", ""],
            ["ekam", "", ""],
        ]),
        ("eka", Linga::F) => rows1(&[
            ["ekA", "", ""],
            ["ekAm", "", ""],
            ["ekayA", "", ""],
            ["ekasyE", "", ""],
            ["ekasyAH", "", ""],
            ["ekasyAH", "", ""],
            ["ekasyAm", "", ""],
            ["ekA", "", ""],
        ]),
        ("dvi", _) => {
            return Ok(dvi_table(linga));
        }
        ("Bavat", Linga::M) => {
            return decline("Bavat", Linga::M, &Class::AtM);
        }
        ("Bavat", Linga::N) => {
            return decline("Bavat", Linga::N, &Class::AtN);
        }
        ("Bavat", Linga::F) => {
            return decline("BavatI", Linga::F, &Class::IIF);
        }
        // tri- masculine. Certain: trayaH/trIn/triBiH/triByaH/trayARAm/
        // trizu/trayoH/traye/treH. Best-effort (regular i-stem): trI du,
        // trim, triRA, trO, tre.
        ("tri", Linga::M) => rows1(&[
            ["trayaH", "trI", "trayaH"],
            ["trim", "trI", "trIn"],
            ["triRA", "triByAm", "triBiH"],
            ["traye", "triByAm", "triByaH"],
            ["treH", "triByAm", "triByaH"],
            ["treH", "trayoH", "trayARAm"],
            ["trO", "trayoH", "trizu"],
            ["tre", "trI", "trayaH"],
        ]),
        // tri- feminine. Certain: tisraH/tisfRNAm/tisfzu/tisarO.
        ("tri", Linga::F) => rows1(&[
            ["tisraH", "tisarO", "tisraH"],
            ["tisram", "tisarO", "tisraH"],
            ["tisrA", "tisfByAm", "tisfBiH"],
            ["tisre", "tisfByAm", "tisfByaH"],
            ["tisruH", "tisfByAm", "tisfByaH"],
            ["tisruH", "tisrayoH", "tisfRNAm"],
            ["tisari", "tisrayoH", "tisfzu"],
            ["tisraH", "tisarO", "tisraH"],
        ]),
        // tri- neuter. Certain: trIRi/trIRAm/trizu.
        ("tri", Linga::N) => rows1(&[
            ["tri", "triRI", "trIRi"],
            ["tri", "triRI", "trIRi"],
            ["triRA", "triByAm", "triBiH"],
            ["triRe", "triByAm", "triByaH"],
            ["triRaH", "triByAm", "triByaH"],
            ["triRaH", "triRoH", "trIRAm"],
            ["triRi", "triRoH", "trizu"],
            ["tre", "triRI", "trIRi"],
        ]),
        // catur- masculine. Certain: catvAraH/caturaH/caturBiH/caturByaH/
        // caturRAm/caturzu/caturoH.
        ("catur", Linga::M) => rows1(&[
            ["catvAraH", "caturU", "catvAraH"],
            ["caturam", "caturU", "caturaH"],
            ["caturRA", "caturByAm", "caturBiH"],
            ["caturave", "caturByAm", "caturByaH"],
            ["catoH", "caturByAm", "caturByaH"],
            ["catoH", "caturoH", "caturRAm"],
            ["catau", "caturoH", "caturzu"],
            ["cato", "caturU", "catvAraH"],
        ]),
        // catur- feminine. Certain: catasraH/catasfRNAm/catasfzu/catasarO.
        ("catur", Linga::F) => rows1(&[
            ["catasraH", "catasarO", "catasraH"],
            ["catasram", "catasarO", "catasraH"],
            ["catasrA", "catasfByAm", "catasfBiH"],
            ["catasre", "catasfByAm", "catasfByaH"],
            ["catasruH", "catasfByAm", "catasfByaH"],
            ["catasruH", "catasrayoH", "catasfRNAm"],
            ["catasari", "catasrayoH", "catasfzu"],
            ["catasraH", "catasarO", "catasraH"],
        ]),
        // catur- neuter. Certain: catvAri/caturRAm/caturzu.
        ("catur", Linga::N) => rows1(&[
            ["catur", "caturuRI", "catvAri"],
            ["catur", "caturuRI", "catvAri"],
            ["caturRA", "caturByAm", "caturBiH"],
            ["caturRe", "caturByAm", "caturByaH"],
            ["caturRaH", "caturByAm", "caturByaH"],
            ["caturRaH", "caturRoH", "caturRAm"],
            ["caturi", "caturRoH", "caturzu"],
            ["caturo", "caturuRI", "catvAri"],
        ]),
        ("paJcan", _) => rows1(&[
            ["paJca", "paJca", "paJca"],
            ["paJca", "paJca", "paJca"],
            ["paJcaBiH", "paJcaBiH", "paJcaBiH"],
            ["paJcaByaH", "paJcaByaH", "paJcaByaH"],
            ["paJcaByaH", "paJcaByaH", "paJcaByaH"],
            ["paJcAnAm", "paJcAnAm", "paJcAnAm"],
            ["paJcasu", "paJcasu", "paJcasu"],
            ["paJca", "paJca", "paJca"],
        ]),
        ("zaz", _) => rows1(&[
            ["zat", "zat", "zat"],
            ["zat", "zat", "zat"],
            ["zadBiH", "zadBiH", "zadBiH"],
            ["zadByaH", "zadByaH", "zadByaH"],
            ["zadByaH", "zadByaH", "zadByaH"],
            ["zaRRAm", "zaRRAm", "zaRRAm"],
            ["zatsu", "zatsu", "zatsu"],
            ["zat", "zat", "zat"],
        ]),
        ("saptan", _) => rows1(&[
            ["sapta", "sapta", "sapta"],
            ["sapta", "sapta", "sapta"],
            ["saptaBiH", "saptaBiH", "saptaBiH"],
            ["saptaByaH", "saptaByaH", "saptaByaH"],
            ["saptaByaH", "saptaByaH", "saptaByaH"],
            ["saptAnAm", "saptAnAm", "saptAnAm"],
            ["saptasu", "saptasu", "saptasu"],
            ["sapta", "sapta", "sapta"],
        ]),
        ("azwan", _) => rows1(&[
            ["azWA", "azWA", "azWA"],
            ["azWA", "azWA", "azWA"],
            ["azWABiH", "azWABiH", "azWABiH"],
            ["azWAByaH", "azWAByaH", "azWAByaH"],
            ["azWAByaH", "azWAByaH", "azWAByaH"],
            ["azWAnAm", "azWAnAm", "azWAnAm"],
            ["azWAsu", "azWAsu", "azWAsu"],
            ["azWA", "azWA", "azWA"],
        ]),
        ("navan", _) => rows1(&[
            ["nava", "nava", "nava"],
            ["nava", "nava", "nava"],
            ["navaBiH", "navaBiH", "navaBiH"],
            ["navaByaH", "navaByaH", "navaByaH"],
            ["navaByaH", "navaByaH", "navaByaH"],
            ["navAnAm", "navAnAm", "navAnAm"],
            ["navasu", "navasu", "navasu"],
            ["nava", "nava", "nava"],
        ]),
        ("daSan", _) => rows1(&[
            ["daSa", "daSa", "daSa"],
            ["daSa", "daSa", "daSa"],
            ["daSaBiH", "daSaBiH", "daSaBiH"],
            ["daSaByaH", "daSaByaH", "daSaByaH"],
            ["daSaByaH", "daSaByaH", "daSaByaH"],
            ["daSAnAm", "daSAnAm", "daSAnAm"],
            ["daSasu", "daSasu", "daSasu"],
            ["daSa", "daSa", "daSa"],
        ]),
        _ => return Err(NOT_POSSIBLE),
    };
    Ok(Paradigm {
        stem: stem.to_string(),
        linga,
        class: Class::Word(word),
        cells: rows_to_slots(&rows),
    })
}

fn rows1(rows: &[[&str; 3]; 8]) -> Vec<(Vec<String>, Vec<String>, Vec<String>)> {
    rows.iter()
        .map(|[a, b, c]| {
            (
                vec![(*a).to_string()],
                vec![(*b).to_string()],
                vec![(*c).to_string()],
            )
        })
        .collect()
}

fn rows_to_slots(rows: &[(Vec<String>, Vec<String>, Vec<String>)]) -> Vec<Slot> {
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
    const VAC: [&str; 3] = [EKA, DVI, BAHU];
    let mut out = Vec::with_capacity(24);
    for (i, (a, b, c)) in rows.iter().enumerate() {
        for (j, forms) in [a, b, c].iter().enumerate() {
            let kept: Vec<String> = forms.iter().filter(|s| !s.is_empty()).cloned().collect();
            if kept.is_empty() {
                continue;
            }
            out.push(Slot {
                vibhakti: VIB[i],
                vacana: VAC[j],
                forms: kept,
            });
        }
    }
    out
}

fn yuzmad_table(_linga: Linga) -> Paradigm {
    let cells = vec![
        (EKA, vec!["tvam"]),
        (DVI, vec!["yuvAm"]),
        (BAHU, vec!["yUyam"]),
        (EKA, vec!["tvAm", "tvA"]),
        (DVI, vec!["yuvAm"]),
        (BAHU, vec!["yuzmAn"]),
        (EKA, vec!["tvayA"]),
        (DVI, vec!["yuvAByAm"]),
        (BAHU, vec!["yuzmABiH"]),
        (EKA, vec!["tuByam"]),
        (DVI, vec!["yuvAByAm"]),
        (BAHU, vec!["yuzmaByam"]),
        (EKA, vec!["tvat"]),
        (DVI, vec!["yuvAByAm"]),
        (BAHU, vec!["yuzmat"]),
        (EKA, vec!["tava"]),
        (DVI, vec!["yuvayOH"]),
        (BAHU, vec!["yuzmAkam"]),
        (EKA, vec!["tvayi"]),
        (DVI, vec!["yuvayOH"]),
        (BAHU, vec!["yuzmAsu"]),
        (EKA, vec!["tvam"]),
        (DVI, vec!["yuvAm"]),
        (BAHU, vec!["yUyam"]),
    ];
    Paradigm {
        stem: "yuzmad".to_string(),
        linga: Linga::M,
        class: Class::Word("yuzmad"),
        cells: vib_slots(&cells),
    }
}

fn asmad_table(_linga: Linga) -> Paradigm {
    let cells = vec![
        (EKA, vec!["aham"]),
        (DVI, vec!["AvAm"]),
        (BAHU, vec!["vayam"]),
        (EKA, vec!["mAm", "mA"]),
        (DVI, vec!["AvAm"]),
        (BAHU, vec!["asmAn"]),
        (EKA, vec!["mayA"]),
        (DVI, vec!["AvAByAm"]),
        (BAHU, vec!["asmABiH"]),
        (EKA, vec!["maHyam"]),
        (DVI, vec!["AvAByAm"]),
        (BAHU, vec!["asmaByam"]),
        (EKA, vec!["mat"]),
        (DVI, vec!["AvAByAm"]),
        (BAHU, vec!["asmat"]),
        (EKA, vec!["mama"]),
        (DVI, vec!["AvayOH"]),
        (BAHU, vec!["asmAkam"]),
        (EKA, vec!["mayi"]),
        (DVI, vec!["AvayOH"]),
        (BAHU, vec!["asmAsu"]),
        (EKA, vec!["aham"]),
        (DVI, vec!["AvAm"]),
        (BAHU, vec!["vayam"]),
    ];
    Paradigm {
        stem: "asmad".to_string(),
        linga: Linga::M,
        class: Class::Word("asmad"),
        cells: vib_slots(&cells),
    }
}

fn dvi_table(linga: Linga) -> Paradigm {
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
    let (nom, acc): (Vec<String>, Vec<String>) = match linga {
        Linga::M => (vec!["dvO".to_string()], vec!["dvO".to_string()]),
        _ => (vec!["dve".to_string()], vec!["dve".to_string()]),
    };
    let forms: [Vec<String>; 8] = [
        nom.clone(),
        acc,
        vec!["dvAByAm".to_string()],
        vec!["dvAByAm".to_string()],
        vec!["dvAByAm".to_string()],
        vec!["dvayOH".to_string()],
        vec!["dvayOH".to_string()],
        nom,
    ];
    let cells: Vec<Slot> = VIB
        .iter()
        .zip(forms.iter())
        .map(|(v, fs)| Slot {
            vibhakti: v,
            vacana: DVI,
            forms: fs.clone(),
        })
        .collect();
    Paradigm {
        stem: "dvi".to_string(),
        linga,
        class: Class::Word("dvi"),
        cells,
    }
}

/// Slots from an explicit 24-cell (vacana, forms) list in vibhakti ×
/// vacana order (prathamA.eka … sambodhana.bahu).
fn vib_slots(cells: &[(&'static str, Vec<&'static str>)]) -> Vec<Slot> {
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
    cells
        .iter()
        .enumerate()
        .map(|(i, (vac, forms))| Slot {
            vibhakti: VIB[i / 3],
            vacana: vac,
            forms: forms.iter().map(|s| (*s).to_string()).collect(),
        })
        .collect()
}

/// Add the `d`-final twin to `t`-final nom/acc/voc singular slots
/// (`tat/tad`; the `Bavat/Bavad` pair lives in [`Class::AtN`]).
fn td_slots(cells: &mut [Slot], t: &str, d: &str) {
    for cell in cells.iter_mut() {
        if cell.vacana == EKA
            && (cell.vibhakti == "prathamA"
                || cell.vibhakti == "dvitIyA"
                || cell.vibhakti == "sambodhana")
            && cell.forms.len() == 1
            && cell.forms[0] == t
        {
            cell.forms.push(d.to_string());
        }
    }
}

/// Known pronoun/numeral stems.
pub const KNOWN_WORDS: &[&str] = &[
    "sarva", "tad", "etad", "yad", "kim", "idam", "adas", "yuzmad", "asmad", "Bavat", "eka", "dvi",
    "tri", "catur", "paJcan", "zaz", "saptan", "azwan", "navan", "daSan",
];

/// Strict manual classes for (stem, linga). May be plural for `-an`.
#[must_use]
pub fn classes_for(stem: &str, linga: Linga) -> Vec<Class> {
    if let Some(word) = KNOWN_WORDS.iter().find(|w| **w == stem) {
        if word_table(stem, linga, word).is_ok() {
            return vec![Class::Word(word)];
        }
        return vec![];
    }
    // `Bavat` pronoun behaves like at/I paradigms (covered by endings too,
    // except feminine which needs the word route).
    let mut out = Vec::new();
    if stem.ends_with('a') && !stem.ends_with("an") {
        match linga {
            Linga::M => out.push(Class::AM),
            Linga::N => out.push(Class::AN),
            Linga::F => {}
        }
    } else if stem.ends_with("an") {
        // Manual `-an`: an-stems (`rAjan`). Participle `-an` (`Bavan`)
        // always resolves through the krt-exact path, so manual never
        // needs at-m here; `-in` is in-stem only.
        if linga == Linga::M {
            if stem.ends_with("in") {
                out.push(Class::InM);
            } else {
                out.push(Class::AnM);
            }
        }
    } else if stem.ends_with('A') {
        if linga == Linga::F {
            out.push(Class::AAF);
        }
    } else if stem.ends_with('i') {
        match linga {
            Linga::M | Linga::F => out.push(Class::IMF),
            Linga::N => out.push(Class::IN),
        }
    } else if stem.ends_with('I') {
        if linga == Linga::F {
            out.push(Class::IIF);
        }
    } else if stem.ends_with('u') {
        match linga {
            Linga::M | Linga::F => out.push(Class::UMF),
            Linga::N => out.push(Class::UN),
        }
    } else if stem.ends_with('U') {
        if linga == Linga::F {
            out.push(Class::UUF);
        }
    } else if stem.ends_with('f') {
        if linga != Linga::N {
            out.push(Class::RFM);
        }
    } else if stem.ends_with('t') || stem.ends_with('d') {
        match linga {
            Linga::M => out.push(Class::AtM),
            Linga::N => out.push(Class::AtN),
            Linga::F => {}
        }
    } else if stem.ends_with('s') || stem.ends_with('H') {
        match linga {
            Linga::M => out.push(Class::AsM),
            Linga::N => out.push(Class::AsN),
            Linga::F => {}
        }
    }
    out
}

/// All strict-plausible tables for manual (stem, linga) input.
///
/// Lexicon-unaware: krt stems in a wrong linga still produce heuristic
/// tables here. Prefer [`generate_tables_checked`] (UI/tests), which enforces
/// krt linga fields (`BUti + m -> not possible`) and routes krt `-an`
/// through the exact class.
#[must_use]
pub fn generate_tables(stem: &str, linga: Linga) -> Vec<Paradigm> {
    classes_for(stem, linga)
        .iter()
        .filter_map(|c| {
            let weak = weak_for(c, stem);
            decline(&weak, linga, c).ok()
        })
        .collect()
}

/// Lexicon-checked generation: krt-known stems decline only in their
/// attested lingas via the exact krt class; other stems use [`classes_for`].
/// Empty output means literal `not possible`.
#[must_use]
pub fn generate_tables_checked<S: ::std::hash::BuildHasher>(
    stem: &str,
    linga: Linga,
    krtmap: &HashMap<String, Vec<KrtRef>, S>,
) -> Vec<Paradigm> {
    // Citation stems often drop a final visarga (`BUti` for cell `BUtiH`):
    // consult the citation form too. Pure `base`-field refs (avyaya
    // indeclinables, see `build_krt_stem_map`) are NOT a lexicon gate:
    // they neither license a linga nor block heuristic generation.
    let hform = format!("{stem}H");
    if let Some(refs) = krtmap.get(stem).or_else(|| krtmap.get(&hform)) {
        if let Some(first) = refs.iter().find(|r| r.field == linga.as_str()) {
            // Re-derive the exact class from the krt entry.
            if let Some((class, weak)) = krt_class(&first.family, &first.krt, &first.field, stem) {
                if let Ok(tab) = decline(&weak, linga, &class) {
                    return vec![tab];
                }
            }
        }
        if refs.iter().any(|r| r.field != "base") {
            return vec![];
        }
    }
    generate_tables(stem, linga)
}

/// Exact single table for a krt click-through (dhatu + krt + field known).
/// `None` for indeclinables / unknown shapes (`not possible` in UI).
#[must_use]
pub fn krt_table(family: &str, krt: &str, field: &str, value: &str) -> Option<Paradigm> {
    let linga = Linga::parse(field)?;
    let (class, weak) = krt_class(family, krt, field, value)?;
    decline(&weak, linga, &class).ok()
}

/// Weak/declension stem for a class from a citation stem.
#[must_use]
pub fn weak_for(class: &Class, stem: &str) -> String {
    match class {
        // Masculine a-stem `-aH` citation (nom. sg.): decline the bare stem.
        Class::AM if stem.ends_with("aH") => stem[..stem.len() - 1].to_string(),
        Class::AtM if stem.ends_with("an") || stem.ends_with("An") => {
            format!("{}at", &stem[..stem.len() - 2])
        }
        Class::AtM if stem.ends_with("ant") || stem.ends_with("Ant") => {
            format!("{}at", &stem[..stem.len() - 3])
        }
        Class::AtM | Class::AtN if stem.ends_with('d') => normalize_dental(stem),
        Class::AN if stem.ends_with("am") => stem[..stem.len() - 1].to_string(),
        Class::VasM if stem.ends_with("An") => {
            format!("{}as", &stem[..stem.len() - 2])
        }
        _ => stem.to_string(),
    }
}

/// Exact krt class + weak stem from (family, krt, field, cell value).
/// Returns `None` for indeclinables / unknown shapes.
#[must_use]
pub fn krt_class(family: &str, krt: &str, field: &str, value: &str) -> Option<(Class, String)> {
    if value.is_empty() || value.contains('/') {
        return None;
    }
    // क्तवतु masculine is a vant-stem (nom. sg. `-vAn`: `BUtavAn`), not an
    // at-stem (`*BUtavan`). Selected by kṛt so at-stems ending in `vat`
    // (e.g. `sunvat`, शतृ of √सु) stay `AtM`. All citation shapes normalize
    // to the weak `vat` stem.
    if krt == "ktavatu" && field == "m" {
        let weak = if let Some(prefix) = value.strip_suffix("vAn") {
            format!("{prefix}vat")
        } else if let Some(prefix) = value.strip_suffix("vant") {
            format!("{prefix}vat")
        } else if value.ends_with("vat") {
            value.to_string()
        } else {
            String::new()
        };
        if weak.len() >= 4 {
            return Some((Class::VantM, weak));
        }
        // Unusual citation shapes fall through to the generic classes below.
    }
    // kvasu masculine `-vAn` is vas-stem, not at-stem.
    if krt == "kvasu" && field == "m" && value.ends_with("An") {
        let weak = format!("{}as", &value[..value.len() - 2]);
        return Some((Class::VasM, weak));
    }
    // tfC masculine `-A` rides an `f`-stem (`BavitA` -> `Bavitf`); its
    // neuter `-tf` is an indeclinable citation (not clickable).
    if krt == "tfc" && field == "m" && value.ends_with('A') && value.len() >= 2 {
        let weak = format!("{}f", &value[..value.len() - 1]);
        return Some((Class::RFM, weak));
    }
    if krt == "tfc" && field == "n" && value.ends_with("tf") {
        return None;
    }
    // Strip a final visarga for vowel stems (`BavaH` -> `Bava`,
    // `BUtiH` -> `BUti`); `as`-stems keep theirs (`manaH`, `vedHAH`).
    if value.ends_with("AH") && field == "m" {
        let weak = format!("{}s", &value[..value.len() - 2]);
        return Some((Class::AsM, weak));
    }
    let plain = value.strip_suffix('H').unwrap_or(value);
    let class = match (field, plain.chars().last()) {
        ("m", Some('a')) if !plain.ends_with("an") => Class::AM,
        ("n", Some('a')) => Class::AN,
        ("n", _) if plain.ends_with("am") => Class::AN,
        ("f", Some('A')) => Class::AAF,
        ("m" | "f", Some('i')) => Class::IMF,
        ("n", Some('i')) => Class::IN,
        ("f", Some('I')) => Class::IIF,
        ("m" | "f", Some('u')) => Class::UMF,
        ("n", Some('u')) => Class::UN,
        ("f", Some('U')) => Class::UUF,
        ("m" | "f", Some('f')) => Class::RFM,
        ("m", _) if plain.ends_with("an") || plain.ends_with("An") || plain.ends_with("ant") => {
            Class::AtM
        }
        ("m", _) if plain.ends_with('t') || plain.ends_with('d') => Class::AtM,
        ("n", _) if plain.ends_with('t') || plain.ends_with('d') => Class::AtN,
        ("m", _) if plain.ends_with("in") => Class::InM,
        ("n", _) if plain.ends_with('s') => Class::AsN,
        ("m", _) if plain.ends_with('s') => Class::AsM,
        _ => return None,
    };
    // `family` selects nothing beyond the shape here; unknown families with
    // standard shapes still decline (logged via tests measuring coverage).
    let _ = family;
    // Weak stems drop the citation visarga (see the `BavaH`/`BUtiH` note
    // above); only the early `as`-stem returns keep an `s`-final stem.
    let weak = weak_for(&class, plain);
    Some((class, weak))
}

/// Krt reverse-reference attached to an analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KrtRef {
    pub dhatu_id: String,
    pub family: String,
    pub krt: String,
    pub field: String,
}

/// Build alternant -> refs over participles m/f/n cells (T2 universe)
/// plus avyaya bases (cells with empty m/f/n, e.g. `lyap`/`ktvA`/`tumun`
/// indeclinables like `praSamayya`). Avyaya bases are stored with
/// `field == "base"`; declinable bare stems are NOT indexed (only their
/// m/f/n alternants), so the lexicon gate in [`generate_tables_checked`]
/// and ref-attach in [`analyze`] (both keyed on m/f/n fields) are
/// unaffected — `base` refs surface only through exact lookup (UI F2
/// Krdanta cards) and the avyaya-only suppression in [`analyze`].
#[must_use]
pub fn build_krt_stem_map(
    files: &[crate::engine::data::DhatuFile],
) -> HashMap<String, Vec<KrtRef>> {
    let mut map: HashMap<String, Vec<KrtRef>> = HashMap::new();
    for f in files {
        if f.skipped {
            continue;
        }
        for (family, krts) in &f.participles {
            for (krt, cells) in krts {
                for cell in cells {
                    for (field, raw) in [("m", &cell.m), ("f", &cell.f), ("n", &cell.n)] {
                        for part in crate::slp1::split_alternants(raw) {
                            let r = KrtRef {
                                dhatu_id: f.id.clone(),
                                family: family.clone(),
                                krt: krt.clone(),
                                field: field.to_string(),
                            };
                            map.entry(part.to_string()).or_default().push(r.clone());
                            // Declension-weak alternate key (e.g. AtM `gacCat`
                            // for citation `gacCan`): reverse lookups land on
                            // weak stems, so they need the same refs or the
                            // masculine reading stays unattributed. Same ref
                            // (field included), so the linga filter and the
                            // lexicon gate treat it exactly like the citation.
                            if let Some((_, weak)) = krt_class(family, krt, field, part) {
                                if weak != part {
                                    let slot = map.entry(weak).or_default();
                                    if !slot.contains(&r) {
                                        slot.push(r);
                                    }
                                }
                            }
                        }
                    }
                    // Avyaya cells only: indeclinable base with no m/f/n.
                    if cell.m.is_empty() && cell.f.is_empty() && cell.n.is_empty() {
                        for part in crate::slp1::split_alternants(&cell.base) {
                            map.entry(part.to_string()).or_default().push(KrtRef {
                                dhatu_id: f.id.clone(),
                                family: family.clone(),
                                krt: krt.clone(),
                                field: "base".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    map
}

/// One reverse-lookup analysis (F5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub stem: String,
    pub linga: Linga,
    pub class: Class,
    /// `vibhakti.vacana` slot of the matched form.
    pub slot: String,
    pub refs: Vec<KrtRef>,
}

/// Reverse lookup: declined SLP1 form -> verified (stem, linga, slot)
/// analyses, krt-backed first. Sound: every analysis regenerates a table
/// containing the form. Exact avyaya (indeclinable krt base) hits suppress
/// unattested generic guesses; the Krdanta refs themselves surface through
/// the krt map (UI F2), not through analyses.
#[must_use]
pub fn analyze<S: ::std::hash::BuildHasher>(
    form: &str,
    krtmap: &HashMap<String, Vec<KrtRef>, S>,
) -> Vec<Analysis> {
    if !crate::slp1::is_slp1_form(form) {
        return vec![];
    }
    let mut out: Vec<Analysis> = Vec::new();
    let mut seen: HashMap<(String, String, String), bool> = HashMap::new();
    let mut push = |stem: String, linga: Linga, class: Class| {
        let weak = weak_for(&class, &stem);
        let Ok(tab) = decline(&weak, linga, &class) else {
            return;
        };
        for cell in &tab.cells {
            if cell.forms.iter().any(|x| x == form) {
                let key = (
                    tab.stem.clone(),
                    linga.as_str().to_string(),
                    format!("{}.{}", cell.vibhakti, cell.vacana),
                );
                if seen.contains_key(&key) {
                    continue;
                }
                seen.insert(key, true);
                let mut refs = krtmap.get(&tab.stem).cloned().unwrap_or_default();
                refs.retain(|r| r.field == linga.as_str());
                out.push(Analysis {
                    stem: tab.stem.clone(),
                    linga,
                    class: class.clone(),
                    slot: format!("{}.{}", cell.vibhakti, cell.vacana),
                    refs,
                });
            }
        }
    };

    // Natva-inverse variants: the query with each single `R` restored to
    // dental `n`, plus the original. Strips match dental-normalized
    // variants; containment below always checks the ORIGINAL surface
    // (decline re-voices deterministically, so verification stays sound).
    // Covers natva-voiced endings (`IRAm` <- `InAm`) and voiced stem-final
    // `n` (`RAByAm` <- `nAByAm`), without any `R`-suffix strips.
    let mut variants: Vec<String> = vec![form.to_string()];
    for (i, b) in form.bytes().enumerate() {
        if b == b'R' {
            let mut v = form.as_bytes().to_vec();
            v[i] = b'n';
            if let Ok(s) = String::from_utf8(v) {
                variants.push(s);
            }
        }
    }
    // 1. Suffix strips (class, suffix, stem-tail to append).
    for probe in &variants {
        for (class, suffix, append) in STRIPS {
            if let Some(pre) = probe.strip_suffix(suffix) {
                if pre.is_empty() {
                    continue;
                }
                let stem = format!("{pre}{append}");
                for linga in [Linga::M, Linga::F, Linga::N] {
                    push(stem.clone(), linga, class.clone());
                }
            }
        }
    }
    // 2. Self-stem hypotheses (voc/prath bona fides like `Wrampena`).
    // Also run over natva-variants: a dental-normalized variant that is
    // itself a valid stem recovers forms whose `R` voices (the variant
    // table re-voices to the original surface on verification).
    for probe in &variants {
        for class in classes_for(probe, Linga::M)
            .into_iter()
            .chain(classes_for(probe, Linga::F))
            .chain(classes_for(probe, Linga::N))
        {
            let linga = match &class {
                Class::AN | Class::IN | Class::UN | Class::AtN | Class::AsN => Linga::N,
                Class::AAF | Class::IIF | Class::UUF => Linga::F,
                Class::Word(_) => continue,
                _ => Linga::M,
            };
            // Known words handled separately below with exact tables.
            push(probe.clone(), linga, class);
        }
    }
    // 3. Known pronoun/numeral words (exact tables + citation forms like
    // `tad`/`adas`, whose dictionary entry is not itself a declined cell).
    for word in KNOWN_WORDS {
        if form == *word {
            for linga in [Linga::M, Linga::F, Linga::N] {
                if decline(word, linga, &Class::Word(word)).is_ok() {
                    let key = (
                        (*word).to_string(),
                        linga.as_str().to_string(),
                        "citation".to_string(),
                    );
                    if seen.contains_key(&key) {
                        continue;
                    }
                    seen.insert(key, true);
                    out.push(Analysis {
                        stem: (*word).to_string(),
                        linga,
                        class: Class::Word(word),
                        slot: "citation".to_string(),
                        refs: vec![],
                    });
                }
            }
        }
        for linga in [Linga::M, Linga::F, Linga::N] {
            let Ok(tab) = decline(word, linga, &Class::Word(word)) else {
                continue;
            };
            for cell in &tab.cells {
                if cell.forms.iter().any(|x| x == form) {
                    let key = (
                        tab.stem.clone(),
                        linga.as_str().to_string(),
                        format!("{}.{}", cell.vibhakti, cell.vacana),
                    );
                    if seen.contains_key(&key) {
                        continue;
                    }
                    seen.insert(key, true);
                    out.push(Analysis {
                        stem: tab.stem.clone(),
                        linga,
                        class: Class::Word(word),
                        slot: format!("{}.{}", cell.vibhakti, cell.vacana),
                        refs: vec![],
                    });
                }
            }
        }
    }
    // krt/lexicon-backed first, then bare; stable otherwise.
    // Avyaya-known forms (exact indeclinable krt bases: map hits with ONLY
    // `base`-field refs, never declined stems) cannot decline, so drop the
    // unattested generic guesses; krt-backed and known-word readings stay.
    // Mixed keys (also an m/f/n alternant) keep everything.
    if let Some(refs) = krtmap.get(form) {
        if !refs.is_empty() && refs.iter().all(|r| r.field == "base") {
            out.retain(|a| !a.refs.is_empty() || matches!(a.class, Class::Word(_)));
        }
    }
    out.sort_by(|a, b| {
        (!a.refs.is_empty())
            .cmp(&(!b.refs.is_empty()))
            .reverse()
            .then_with(|| a.stem.cmp(&b.stem))
            .then_with(|| a.slot.cmp(&b.slot))
    });
    out
}

type StripRule = (Class, &'static str, &'static str);

/// (class, suffix, stem-tail): strip `suffix`, append tail, regenerate.
#[allow(clippy::too_many_lines)]
const STRIPS: &[StripRule] = &[
    // a-stems.
    (Class::AM, "AByAm", "a"),
    (Class::AM, "eByaH", "a"),
    (Class::AM, "asya", "a"),
    (Class::AM, "ARAm", "a"),
    (Class::AM, "AnAm", "a"),
    (Class::AM, "eRa", "a"),
    (Class::AM, "ena", "a"),
    (Class::AM, "Aya", "a"),
    (Class::AM, "yOH", "a"),
    (Class::AM, "aH", "a"),
    (Class::AM, "AH", "a"),
    (Class::AM, "EH", "a"),
    (Class::AM, "am", "a"),
    (Class::AM, "O", "a"),
    (Class::AM, "At", "a"),
    (Class::AM, "ezu", "a"),
    (Class::AM, "e", "a"),
    (Class::AN, "AByAm", "a"),
    (Class::AN, "eByaH", "a"),
    (Class::AN, "asya", "a"),
    (Class::AN, "AnAm", "a"),
    (Class::AN, "Ani", "a"),
    (Class::AN, "ena", "a"),
    (Class::AN, "Aya", "a"),
    (Class::AN, "yOH", "a"),
    (Class::AN, "At", "a"),
    (Class::AN, "am", "a"),
    (Class::AN, "EH", "a"),
    (Class::AN, "ezu", "a"),
    (Class::AN, "e", "a"),
    // A-stems.
    (Class::AAF, "AByAm", "A"),
    (Class::AAF, "AByaH", "A"),
    (Class::AAF, "ABiH", "A"),
    (Class::AAF, "ARAm", "A"),
    (Class::AAF, "AnAm", "A"),
    (Class::AAF, "AyAH", "A"),
    (Class::AAF, "AyAm", "A"),
    (Class::AAF, "ayA", "A"),
    (Class::AAF, "AyE", "A"),
    (Class::AAF, "ayOH", "A"),
    (Class::AAF, "yOH", "A"),
    (Class::AAF, "Am", "A"),
    (Class::AAF, "Asu", "A"),
    (Class::AAF, "AH", "A"),
    (Class::AAF, "e", "A"),
    // i-stems.
    (Class::IMF, "iByAm", "i"),
    (Class::IMF, "iByaH", "i"),
    (Class::IMF, "iBiH", "i"),
    (Class::IMF, "InAm", "i"),
    (Class::IMF, "ayaH", "i"),
    (Class::IMF, "aye", "i"),
    (Class::IMF, "inA", "i"),
    (Class::IMF, "yOH", "i"),
    (Class::IMF, "iH", "i"),
    (Class::IMF, "im", "i"),
    (Class::IMF, "In", "i"),
    (Class::IMF, "eH", "i"),
    (Class::IMF, "O", "i"),
    (Class::IMF, "e", "i"),
    (Class::IMF, "I", "i"),
    (Class::IMF, "izu", "i"),
    (Class::IN, "iByAm", "i"),
    (Class::IN, "iByaH", "i"),
    (Class::IN, "iBiH", "i"),
    (Class::IN, "IRAm", "i"),
    (Class::IN, "iRI", "i"),
    (Class::IN, "IRi", "i"),
    (Class::IN, "iRA", "i"),
    (Class::IN, "iRe", "i"),
    (Class::IN, "iRaH", "i"),
    (Class::IN, "iRoH", "i"),
    (Class::IN, "iRi", "i"),
    (Class::IN, "izu", "i"),
    (Class::IN, "e", "i"),
    // I-stems.
    (Class::IIF, "IByAm", "I"),
    (Class::IIF, "IByaH", "I"),
    (Class::IIF, "IBiH", "I"),
    (Class::IIF, "IRAm", "I"),
    (Class::IIF, "InAm", "I"),
    (Class::IIF, "Izu", "I"),
    (Class::IIF, "yAH", "I"),
    (Class::IIF, "yAm", "I"),
    (Class::IIF, "yA", "I"),
    (Class::IIF, "yE", "I"),
    (Class::IIF, "yO", "I"),
    (Class::IIF, "yOH", "I"),
    (Class::IIF, "yaH", "I"),
    (Class::IIF, "Im", "I"),
    (Class::IIF, "IH", "I"),
    (Class::IIF, "i", "I"),
    // u-stems.
    (Class::UMF, "uByAm", "u"),
    (Class::UMF, "uByaH", "u"),
    (Class::UMF, "uBiH", "u"),
    (Class::UMF, "URAm", "u"),
    (Class::UMF, "UnAm", "u"),
    (Class::UMF, "avaH", "u"),
    (Class::UMF, "ave", "u"),
    (Class::UMF, "unA", "u"),
    (Class::UMF, "vOH", "u"),
    (Class::UMF, "oH", "u"),
    (Class::UMF, "um", "u"),
    (Class::UMF, "Un", "u"),
    (Class::UMF, "O", "u"),
    (Class::UMF, "o", "u"),
    (Class::UMF, "U", "u"),
    (Class::UMF, "uH", "u"),
    (Class::UMF, "uzu", "u"),
    (Class::UN, "uByAm", "u"),
    (Class::UN, "uByaH", "u"),
    (Class::UN, "uBiH", "u"),
    (Class::UN, "URAm", "u"),
    (Class::UN, "UnAm", "u"),
    (Class::UN, "unI", "u"),
    (Class::UN, "Uni", "u"),
    (Class::UN, "unA", "u"),
    (Class::UN, "une", "u"),
    (Class::UN, "unaH", "u"),
    (Class::UN, "unOH", "u"),
    (Class::UN, "uni", "u"),
    (Class::UN, "uzu", "u"),
    (Class::UN, "o", "u"),
    // U-stems.
    (Class::UUF, "UByAm", "U"),
    (Class::UUF, "UByaH", "U"),
    (Class::UUF, "UBiH", "U"),
    (Class::UUF, "URAm", "U"),
    (Class::UUF, "UnAm", "U"),
    (Class::UUF, "Uzu", "U"),
    (Class::UUF, "vAH", "U"),
    (Class::UUF, "vAm", "U"),
    (Class::UUF, "vA", "U"),
    (Class::UUF, "vE", "U"),
    (Class::UUF, "vO", "U"),
    (Class::UUF, "vOH", "U"),
    (Class::UUF, "vaH", "U"),
    (Class::UUF, "Um", "U"),
    (Class::UUF, "UH", "U"),
    (Class::UUF, "u", "U"),
    // f-stems.
    (Class::RFM, "fByAm", "f"),
    (Class::RFM, "fByaH", "f"),
    (Class::RFM, "fBiH", "f"),
    (Class::RFM, "FRAm", "f"),
    (Class::RFM, "AraH", "f"),
    (Class::RFM, "ArO", "f"),
    (Class::RFM, "Aram", "f"),
    (Class::RFM, "rA", "f"),
    (Class::RFM, "re", "f"),
    (Class::RFM, "ur", "f"),
    (Class::RFM, "roH", "f"),
    (Class::RFM, "ari", "f"),
    (Class::RFM, "ar", "f"),
    (Class::RFM, "A", "f"),
    (Class::RFM, "Fn", "f"),
    (Class::RFM, "fzu", "f"),
    // t-stems.
    (Class::AtM, "ByAm", "at"),
    (Class::AtM, "BiH", "at"),
    (Class::AtM, "dByAm", "t"),
    (Class::AtM, "dBiH", "t"),
    (Class::AtM, "antO", "at"),
    (Class::AtM, "antaH", "at"),
    (Class::AtM, "antam", "at"),
    (Class::AtM, "an", "at"),
    (Class::AtM, "AntO", "At"),
    (Class::AtM, "AntaH", "At"),
    (Class::AtM, "Antam", "At"),
    (Class::AtM, "An", "At"),
    (Class::AtM, "atA", "at"),
    (Class::AtM, "ate", "at"),
    (Class::AtM, "ataH", "at"),
    (Class::AtM, "atOH", "at"),
    (Class::AtM, "atAm", "at"),
    (Class::AtM, "ati", "at"),
    (Class::AtM, "atsu", "at"),
    (Class::AtM, "AtA", "At"),
    (Class::AtM, "Ate", "At"),
    (Class::AtM, "AtaH", "At"),
    (Class::AtM, "AtOH", "At"),
    (Class::AtM, "AtAm", "At"),
    (Class::AtM, "Ati", "At"),
    (Class::AtM, "Atsu", "At"),
    (Class::AtN, "ByAm", "at"),
    (Class::AtN, "BiH", "at"),
    (Class::AtN, "dByAm", "t"),
    (Class::AtN, "dBiH", "t"),
    (Class::AtN, "anti", ""),
    (Class::AtN, "Anti", "At"),
    (Class::AtN, "atI", "at"),
    (Class::AtN, "AtI", "At"),
    (Class::AtN, "atA", "at"),
    (Class::AtN, "AtA", "At"),
    (Class::AtN, "ate", "at"),
    (Class::AtN, "Ate", "At"),
    (Class::AtN, "ataH", "at"),
    (Class::AtN, "AtaH", "At"),
    (Class::AtN, "atOH", "at"),
    (Class::AtN, "AtOH", "At"),
    (Class::AtN, "atAm", "at"),
    (Class::AtN, "AtAm", "At"),
    (Class::AtN, "ati", "at"),
    (Class::AtN, "Ati", "At"),
    (Class::AtN, "atsu", "at"),
    (Class::AtN, "Atsu", "At"),
    // n-stems.
    (Class::AnM, "AnO", "an"),
    (Class::AnM, "AnaH", "an"),
    (Class::AnM, "Anam", "an"),
    (Class::AnM, "AByAm", "an"),
    (Class::AnM, "ABiH", "an"),
    (Class::AnM, "YaH", "an"),
    (Class::AnM, "YA", "an"),
    (Class::AnM, "Ye", "an"),
    (Class::AnM, "YoH", "an"),
    (Class::AnM, "YAm", "an"),
    (Class::AnM, "Yi", "an"),
    (Class::AnM, "asu", "an"),
    (Class::InM, "inO", "in"),
    (Class::InM, "inaH", "in"),
    (Class::InM, "inam", "in"),
    (Class::InM, "iByAm", "in"),
    (Class::InM, "iBiH", "in"),
    (Class::InM, "inA", "in"),
    (Class::InM, "ine", "in"),
    (Class::InM, "inaH", "in"),
    (Class::InM, "inOH", "in"),
    (Class::InM, "inAm", "in"),
    (Class::InM, "ini", "in"),
    (Class::InM, "izu", "in"),
    (Class::InM, "I", "in"),
    // s-stems.
    (Class::AsN, "oByAm", "as"),
    (Class::AsN, "oByaH", "as"),
    (Class::AsN, "oBiH", "as"),
    (Class::AsN, "AMsi", "as"),
    (Class::AsN, "asI", "as"),
    (Class::AsN, "asA", "as"),
    (Class::AsN, "ase", "as"),
    (Class::AsN, "asaH", "as"),
    (Class::AsN, "asOH", "as"),
    (Class::AsN, "asAm", "as"),
    (Class::AsN, "asi", "as"),
    (Class::AsN, "assu", "as"),
    (Class::AsN, "aH", "as"),
    (Class::AsM, "oByAm", "as"),
    (Class::AsM, "oByaH", "as"),
    (Class::AsM, "oBiH", "as"),
    (Class::AsM, "asO", "as"),
    (Class::AsM, "asaH", "as"),
    (Class::AsM, "asam", "as"),
    (Class::AsM, "asA", "as"),
    (Class::AsM, "ase", "as"),
    (Class::AsM, "asOH", "as"),
    (Class::AsM, "asAm", "as"),
    (Class::AsM, "asi", "as"),
    (Class::AsM, "assu", "as"),
    (Class::AsM, "AH", "as"),
    (Class::AsM, "sA", "s"),
    (Class::AsM, "sAm", "s"),
    (Class::AsM, "saH", "s"),
    (Class::AsM, "ssu", "s"),
    (Class::VasM, "AMsO", "as"),
    (Class::VasM, "AMsaH", "as"),
    (Class::VasM, "AMsam", "as"),
    (Class::VasM, "An", "as"),
    (Class::VasM, "an", "as"),
    (Class::VasM, "uzA", "vas"),
    (Class::VasM, "uze", "vas"),
    (Class::VasM, "uzaH", "vas"),
    (Class::VasM, "uzoH", "vas"),
    (Class::VasM, "uzAm", "vas"),
    (Class::VasM, "uzi", "vas"),
    (Class::VasM, "vadByAm", "vas"),
    (Class::VasM, "vadBiH", "vas"),
    (Class::VasM, "vadsu", "vas"),
    (Class::VasM, "dByAm", "s"),
    (Class::VasM, "dsu", "s"),
];
