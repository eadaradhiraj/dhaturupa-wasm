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

/// Style of the masculine `t`-stem (`gacCan` / `dadat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtMStyle {
    /// नुम् strong stem in nominative/accusative (`gacCan`); शतृ default.
    Num,
    /// अभ्यस्त (reduplicated): weak throughout; nom/voc. sg. `dadat`/`dadad`
    /// (no नुम् — P. 7.1.78 *नाभ्यस्ताच्छतुः*).
    Abhyasta,
}

/// Style of the neuter `t`-stem (शतृ neuter and वत्-stems).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtNStyle {
    /// Strong stem in dual and plural (`BavantI`, `Bavanti`) — @pacat1.
    Num,
    /// Weak dual (`viyatI`), strong plural (`viyanti`) — @viyat1/@jagat1
    /// (also every वत्-stem: `FalavatI`, `Falavanti` — @phalavat2).
    WeakDual,
    /// Both stems in dual (`tudantI`/`tudatI`), strong plural — @tudat1.
    BothDual,
    /// अभ्यस्त: weak dual (`dadatI`), both stems in plural
    /// (`dadanti`/`dadati`) — @dadat2.
    Abhyasta,
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
    /// `I`-stem feminine कृदन्त/उगित् flavour (`BavantI`, `awavI`, and
    /// ashtadhyayi.com's कृदन्त `@devI1` grids): prathamA-ekavacana is the
    /// bare stem.
    IIF,
    /// `I`-stem feminine lexical नदीसंज्ञक flavour (`nadI`, `devI`,
    /// `lakzmI`): prathamA-ekavacana takes the visarga (`nadIH`); every
    /// other cell equals [`Class::IIF`] (site शब्द `@lakSmI1`).
    IIFN,
    /// `u`-stem masculine/feminine (`guru`).
    UMF,
    /// `u`-stem feminine with ङित् endings (`Denu`): `DenvA`, `Denave`/
    /// `DenvE` (the ashtadhyayi.com उ-f paradigm).
    UF,
    /// `u`-stem neuter (`maDu`).
    UN,
    /// `U`-stem feminine कृदन्त/उगित् flavour (`vaDU`): prathamA-ekavacana
    /// is the bare stem.
    UUF,
    /// `U`-stem feminine lexical नदीसंज्ञक flavour (a `vaDU` शब्द):
    /// prathamA-ekavacana takes the visarga (`vaDUH`); every other cell
    /// equals [`Class::UUF`].
    UUFN,
    /// `f`-stem masculine (`kartf`); feminine approximated (`NOM/VOC.SG` =
    /// stem, rest masculine — true feminine agents use `-trI`/I-stems).
    RFM,
    /// `f`-stem neuter (`dAtf`): `dAtfRI`, `dAtfRA`/`dAtRA`, `dAtfRaH`/
    /// `dAtuH` (the ashtadhyayi.com @dhAtR2 paradigm).
    RFN,
    /// ङित् `i`-stem feminine (`gati`): `gatyA`, `gataye`/`gatyE`,
    /// `gateH`/`gatyAH` (the ashtadhyayi.com @gati1 paradigm).
    GatiF,
    /// `t`-stem masculine (`Bavat` weak; `Bavan` strong nom).
    AtM(AtMStyle),
    /// `t`-stem neuter (`Bavat`/`Bavad`, `viyat`/`tudat` styles).
    AtN(AtNStyle),
    /// `an`-stem masculine (`rAjan`).
    AnM,
    /// `in`-stem masculine (`balin`).
    InM,
    /// `in`-stem neuter (`kzamin`): `kzami`, `kzamiRI`, `kzami` vocative
    /// (the ashtadhyayi.com @kSamin2 paradigm).
    InN,
    /// `an`-stem feminine (`dAman`/`sIman`): the [`Class::AnM`] shape,
    /// cell-for-cell identical (weak accusative plural included).
    AnF,
    /// `in`-stem feminine (`mahAjAlin`): identical to [`Class::InM`].
    InF,
    /// `as`-stem neuter (`manas`).
    AsN,
    /// `as`-stem masculine (`vedHas`).
    AsM,
    /// `vas`-stem masculine (`vidvas`, kvasu).
    VasM,
    /// `vas`-stem neuter (क्वसु): weak `dadyuz`/`babUz` with यण् and a
    /// `-vat` nominative (`dadivat`); the ashtadhyayi.com @vidvas2 paradigm.
    VasN,
    /// `vant`-stem masculine (`BUtavat`, ktavatu; nom. sg. `-vAn`,
    /// unlike at-stems). Selected by kṛt, not by ending, so at-stems
    /// ending in `vat` (e.g. `sunvat`) stay `AtM`.
    VantM,
    /// `j`-stem masculine/feminine (नजिङ्): pausa `svapnag`/`svapnak`,
    /// `svapnagByAm`, `svapnakzu`.
    Jvs,
    /// `j`-stem neuter (नजिङ्): `svapnajI`, `svapnaYji` (the @svapnaj2
    /// paradigm).
    JvsN,
    /// Full consonant-final noun paradigm (सरित्, विद्युत्, संविद्,
    /// उपनिषद्, युध्/बुध्, ककुभ्, वाच्/त्वच्): प्रथमा- and सम्बोधन-सि are
    /// pausa pairs (सरिद्-सरित्, ककुब्-ककुप्, वाग्-वाक्; site
    /// @sarit1/@kakubh1/@vAc1), भ्-endings take the voiced final, सु the
    /// voiceless one. Strong cells keep the input final. Distinct from
    /// the participle [`Class::AtM`] only in the prathamā-eka slot.
    Cons,
    /// उस्/इस्-स्वादि neuter (`आयुष्` `@AyuS1`; हविस्/धनुस् derive): the
    /// stem minus its ष्/स् is the root (आयु, हवि, धनु) — सि is the bare
    /// root + visarga (आयुः), द्वि the root + षि (आयुषी), बहु the
    /// lengthened root + ंषि (आयूंषि, हवींषि), the भ्-endings take the
    /// weak र्-discard (आयुर्भ्याम्), and loc-pl pairs आयुःषु-आयुष्षु.
    UsN,
    /// `van`-stem masculine (ङ्वनिप्, like `rAjan` with न्-drop):
    /// `yajvA`, `yajvAnO`, `yajvasu` (the @yajvan1 paradigm).
    VanM,
    /// `man`-stem neuter (ङ्वनिप्): `sadma`, `sadmanI`, `sadmAni`,
    /// `sadmasu` (the @sadman1 paradigm).
    ManN,
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
            Self::IIFN => "I-f-nadi".to_string(),
            Self::UMF => "u-m/f".to_string(),
            Self::UF => "u-f".to_string(),
            Self::UN => "u-n".to_string(),
            Self::UUF => "U-f".to_string(),
            Self::UUFN => "U-f-nadi".to_string(),
            Self::RFM => "f-m/f".to_string(),
            Self::RFN => "f-n".to_string(),
            Self::GatiF => "i-f".to_string(),
            Self::AtM(AtMStyle::Num) => "at-m".to_string(),
            Self::AtM(AtMStyle::Abhyasta) => "at-m-abhyasta".to_string(),
            Self::AtN(AtNStyle::Num) => "at-n".to_string(),
            Self::AtN(AtNStyle::WeakDual) => "at-n-viyat".to_string(),
            Self::AtN(AtNStyle::BothDual) => "at-n-tudat".to_string(),
            Self::AtN(AtNStyle::Abhyasta) => "at-n-abhyasta".to_string(),
            Self::AnM => "an-m".to_string(),
            Self::AnF => "an-f".to_string(),
            Self::InM => "in-m".to_string(),
            Self::InF => "in-f".to_string(),
            Self::InN => "in-n".to_string(),
            Self::AsN => "as-n".to_string(),
            Self::AsM => "as-m".to_string(),
            Self::Cons => "cons".to_string(),
            Self::UsN => "us-n".to_string(),
            Self::VasM => "vas-m".to_string(),
            Self::VasN => "vas-n".to_string(),
            Self::VantM => "vant-m".to_string(),
            Self::Jvs => "j-m/f".to_string(),
            Self::JvsN => "j-n".to_string(),
            Self::VanM => "van-m".to_string(),
            Self::ManN => "man-n".to_string(),
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
            let mut cells = table(&[
                (f(p, "aH"), f(p, "O"), f(p, "AH")),
                (f(p, "am"), f(p, "O"), f(p, "An")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "Aya"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "At"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayoH"), f(p, "AnAm")),
                (f(p, "e"), f(p, "ayoH"), f(p, "ezu")),
                (stem.to_string(), f(p, "O"), f(p, "AH")),
            ]);
            // Panchami eka: the site shows the vṛddhi `-Ad` before `-At`.
            put(&mut cells, 4, 0, vec![f(p, "Ad"), f(p, "At")]);
            cells
        }
        Class::AN => {
            if linga != Linga::N || !stem.ends_with('a') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "Aya"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "At"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayoH"), f(p, "AnAm")),
                (f(p, "e"), f(p, "ayoH"), f(p, "ezu")),
                (stem.to_string(), f(p, "e"), f(p, "Ani")),
            ]);
            put(&mut cells, 4, 0, vec![f(p, "Ad"), f(p, "At")]);
            cells
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
                (f(p, "AyAH"), f(p, "ayoH"), f(p, "AnAm")),
                (f(p, "AyAm"), f(p, "ayoH"), f(p, "Asu")),
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
                (f(p, "eH"), f(p, "yoH"), f(p, "InAm")),
                (f(p, "O"), f(p, "yoH"), f(p, "izu")),
                (f(p, "e"), f(p, "I"), f(p, "ayaH")),
            ])
        }
        Class::IN => {
            // `i`-stem neuter (`vAri`; site @vAri1): नुम् augment before the
            // `नि/नी` affixes and before vowel endings, written dental here —
            // the word-level natva pass (इगन्त neuter: all cells) voices it
            // where the trigger permits (`vArinI` -> `vAriRI`, `dadhinI`
            // stays).
            if linga != Linga::N || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(p, "inI"), f(p, "Ini")),
                (stem.to_string(), f(p, "inI"), f(p, "Ini")),
                (f(p, "inA"), f(p, "iByAm"), f(p, "iBiH")),
                (f(p, "ine"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "inaH"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "inaH"), f(p, "inoH"), f(p, "InAm")),
                (f(p, "ini"), f(p, "inoH"), f(p, "izu")),
                (f(p, "e"), f(p, "inI"), f(p, "Ini")),
            ]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "e")]);
            cells
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
                (f(p, "yAH"), f(p, "yoH"), f(p, "InAm")),
                (f(p, "yAm"), f(p, "yoH"), f(p, "Izu")),
                (f(p, "i"), f(p, "yO"), f(p, "yaH")),
            ])
        }
        Class::IIFN => {
            // नदीसंज्ञक lexical `-I` stem: the `I-f` grid with the visarga
            // in prathamA-ekavacana (site शब्द `@lakSmI1`).
            if linga != Linga::F || !stem.ends_with('I') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(stem, "H"), f(p, "yO"), f(p, "yaH")),
                (f(p, "Im"), f(p, "yO"), f(p, "IH")),
                (f(p, "yA"), f(p, "IByAm"), f(p, "IBiH")),
                (f(p, "yE"), f(p, "IByAm"), f(p, "IByaH")),
                (f(p, "yAH"), f(p, "IByAm"), f(p, "IByaH")),
                (f(p, "yAH"), f(p, "yoH"), f(p, "InAm")),
                (f(p, "yAm"), f(p, "yoH"), f(p, "Izu")),
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
                (f(p, "oH"), f(p, "voH"), f(p, "UnAm")),
                (f(p, "O"), f(p, "voH"), f(p, "uzu")),
                (f(p, "o"), f(p, "U"), f(p, "avaH")),
            ])
        }
        Class::UF => {
            // `u`-stem feminine ङित् (`Denu`): व्-form in instr, गुण/व्
            // optional pairs in 4-7 eka (ashtadhyayi.com @dhenu1).
            if linga != Linga::F || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "uH"), f(p, "U"), f(p, "avaH")),
                (f(p, "um"), f(p, "U"), f(p, "UH")),
                (f(p, "vA"), f(p, "uByAm"), f(p, "uBiH")),
                (f(p, "ave"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "oH"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "oH"), f(p, "voH"), f(p, "UnAm")),
                (f(p, "O"), f(p, "voH"), f(p, "uzu")),
                (f(p, "o"), f(p, "U"), f(p, "avaH")),
            ]);
            put(&mut cells, 3, 0, vec![f(p, "ave"), f(p, "vE")]);
            put(&mut cells, 4, 0, vec![f(p, "oH"), f(p, "vAH")]);
            put(&mut cells, 5, 0, vec![f(p, "oH"), f(p, "vAH")]);
            put(&mut cells, 6, 0, vec![f(p, "O"), f(p, "vAm")]);
            cells
        }
        Class::Cons => {
            // Full consonant-final noun (सरित्/विद्युत्/संविद्/उपनिषद्/
            // युध्/ककुभ्/वाच्; site @sarit1/@kakubh1/@vAc1). One shape for
            // the finals {t, d, D, B, c}: प्रथमा- and सम्बोधन-सि carry the
            // pausa pairs (सरिद्-सरित्, युद्-युत्, ककुब्-ककुप्, वाग्-वाक्),
            // the भ्-endings voice the final (सरिद्भ्याम्, ककुब्भ्याम्,
            // वाग्भ्याम्), and सु devoices it (सरित्सु, ककुप्सु; वाक्षु for
            // च्). Strong cells keep the input final (सरितौ/संविदौ/युधौ/
            // ककुभौ). Feminine reaches every final; masculine reaches
            // {t, d, D, B} (मरुत्, बुध्, गर्दभ्) — the site's masculine
            // च्-final words (अर्वाच्, अन्वच्) are the ञ्च-स्वादि family
            // (अर्वाञ्चः), a different paradigm, so च् stays feminine-only.
            // Participle अ-उपधा spellings stay with AtM.
            // The ऋ-chart (ऋक्/ऋग्/ऋच्, site @Rc1) is the same shape with
            // a one-letter base, so those two-letter stems are admitted.
            let r_chart = matches!(stem, "fc" | "fk" | "fg");
            if !matches!(linga, Linga::M | Linga::F) || (stem.len() < 3 && !r_chart) {
                return Err(NOT_POSSIBLE);
            }
            if stem.ends_with("at") || stem.ends_with("ant") {
                return Err(NOT_POSSIBLE);
            }
            let Some(c) = stem.chars().next_back() else {
                return Err(NOT_POSSIBLE);
            };
            let (voiced, voiceless, su_suffix) = match c {
                't' | 'd' | 'D' => ("d", "t", "su"),
                'B' => ("b", "p", "su"),
                'c' => ("g", "k", "zu"),
                _ => return Err(NOT_POSSIBLE),
            };
            if linga == Linga::M && c == 'c' {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The भ्-final family voices its internal द/ब/ग/ज in the weak
            // forms (the जश्-व्यवस्था): गर्दभ् → गर्धब्-गर्धप्,
            // गर्धब्भ्याम्, गर्धप्सु — the strong cells keep गर्दभः.
            let weak_base = if c == 'B' {
                jash_voice(base)
            } else {
                base.to_string()
            };
            let voiced_stem = f(&weak_base, voiced);
            let pausa_stem = f(&weak_base, voiceless);
            let dvi_form = f(stem, "O");
            let bahu_form = f(stem, "aH");
            let mut cells = table(&[
                (voiced_stem.clone(), dvi_form.clone(), bahu_form.clone()),
                (f(stem, "am"), dvi_form.clone(), bahu_form.clone()),
                (
                    f(stem, "A"),
                    f(&voiced_stem, "ByAm"),
                    f(&voiced_stem, "BiH"),
                ),
                (
                    f(stem, "e"),
                    f(&voiced_stem, "ByAm"),
                    f(&voiced_stem, "ByaH"),
                ),
                (
                    f(stem, "aH"),
                    f(&voiced_stem, "ByAm"),
                    f(&voiced_stem, "ByaH"),
                ),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(&pausa_stem, su_suffix)),
                (pausa_stem.clone(), dvi_form, bahu_form),
            ]);
            // The प्रथमा- and सम्बोधन-सि pausa pairs (सरिद्-सरित्).
            put(&mut cells, 0, 0, vec![voiced_stem, pausa_stem.clone()]);
            put(&mut cells, 7, 0, vec![f(&weak_base, voiced), pausa_stem]);
            cells
        }
        Class::UsN => {
            if linga != Linga::N || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let Some(c) = stem.chars().next_back() else {
                return Err(NOT_POSSIBLE);
            };
            if !matches!(c, 's' | 'z') {
                return Err(NOT_POSSIBLE);
            }
            let root = &stem[..stem.len() - 1];
            // बहु-vacana lengthens the root's final vowel (आयु → आयूंषि).
            let long_root = match root.chars().next_back() {
                Some('a') => f(&root[..root.len() - 1], "A"),
                Some('i') => f(&root[..root.len() - 1], "I"),
                Some('u') => f(&root[..root.len() - 1], "U"),
                Some('f') => f(&root[..root.len() - 1], "F"),
                _ => root.to_string(),
            };
            let mut cells = table(&[
                (f(root, "H"), f(root, "zI"), f(&long_root, "Mzi")),
                (f(root, "H"), f(root, "zI"), f(&long_root, "Mzi")),
                (
                    f(root, "zA"),
                    f(&f(root, "r"), "ByAm"),
                    f(&f(root, "r"), "BiH"),
                ),
                (
                    f(root, "ze"),
                    f(&f(root, "r"), "ByAm"),
                    f(&f(root, "r"), "ByaH"),
                ),
                (
                    f(root, "zaH"),
                    f(&f(root, "r"), "ByAm"),
                    f(&f(root, "r"), "ByaH"),
                ),
                (f(root, "zaH"), f(root, "zoH"), f(root, "zAm")),
                (f(root, "zi"), f(root, "zoH"), f(root, "Hzu")),
                (f(root, "H"), f(root, "zI"), f(&long_root, "Mzi")),
            ]);
            // Loc-pl alternant pair (आयुःषु-आयुष्षु).
            put(&mut cells, 6, 2, vec![f(root, "Hzu"), f(root, "zzu")]);
            cells
        }
        Class::UN => {
            if linga != Linga::N || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(p, "unI"), f(p, "Uni")),
                (stem.to_string(), f(p, "unI"), f(p, "Uni")),
                (f(p, "unA"), f(p, "uByAm"), f(p, "uBiH")),
                (f(p, "une"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "unaH"), f(p, "uByAm"), f(p, "uByaH")),
                (f(p, "unaH"), f(p, "unoH"), f(p, "UnAm")),
                (f(p, "uni"), f(p, "unoH"), f(p, "uzu")),
                (f(p, "o"), f(p, "unI"), f(p, "Uni")),
            ]);
            // गुण/व् optional pairs (ashtadhyayi.com @kaTu4).
            put(&mut cells, 3, 0, vec![f(p, "une"), f(p, "ave")]);
            put(&mut cells, 4, 0, vec![f(p, "unaH"), f(p, "oH")]);
            put(&mut cells, 5, 0, vec![f(p, "unaH"), f(p, "oH")]);
            put(&mut cells, 5, 1, vec![f(p, "unoH"), f(p, "voH")]);
            put(&mut cells, 6, 0, vec![f(p, "uni"), f(p, "O")]);
            put(&mut cells, 6, 1, vec![f(p, "unoH"), f(p, "voH")]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "o")]);
            cells
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
                (f(p, "vAH"), f(p, "voH"), f(p, "UnAm")),
                (f(p, "vAm"), f(p, "voH"), f(p, "Uzu")),
                (f(p, "u"), f(p, "vO"), f(p, "vaH")),
            ])
        }
        Class::UUFN => {
            // नदीसंज्ञक lexical `-U` stem: the `U-f` grid with the visarga
            // in prathamA-ekavacana.
            if linga != Linga::F || !stem.ends_with('U') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(stem, "H"), f(p, "vO"), f(p, "vaH")),
                (f(p, "Um"), f(p, "vO"), f(p, "UH")),
                (f(p, "vA"), f(p, "UByAm"), f(p, "UBiH")),
                (f(p, "vE"), f(p, "UByAm"), f(p, "UByaH")),
                (f(p, "vAH"), f(p, "UByAm"), f(p, "UByaH")),
                (f(p, "vAH"), f(p, "voH"), f(p, "UnAm")),
                (f(p, "vAm"), f(p, "voH"), f(p, "Uzu")),
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
                f(p, "aH")
            } else {
                stem.to_string()
            };
            table(&[
                (nom_sg, f(p, "ArO"), f(p, "AraH")),
                (f(p, "Aram"), f(p, "ArO"), f(p, "Fn")),
                (f(p, "rA"), f(p, "fByAm"), f(p, "fBiH")),
                (f(p, "re"), f(p, "fByAm"), f(p, "fByaH")),
                (f(p, "uH"), f(p, "fByAm"), f(p, "fByaH")),
                (f(p, "uH"), f(p, "roH"), f(p, "FRAm")),
                (f(p, "ari"), f(p, "roH"), f(p, "fzu")),
                (voc_sg, f(p, "ArO"), f(p, "AraH")),
            ])
        }
        Class::RFN => {
            // ऋ-stem neuter (`dAtf`): ण-first optional pairs (site @dhAtR2).
            if linga != Linga::N || !stem.ends_with('f') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(p, "fRI"), f(p, "FRi")),
                (stem.to_string(), f(p, "fRI"), f(p, "FRi")),
                (f(p, "fRA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(p, "fRe"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "fRaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "fRaH"), f(p, "fRoH"), f(p, "FRAm")),
                (f(p, "fRi"), f(p, "fRoH"), f(p, "fzu")),
                (stem.to_string(), f(p, "fRI"), f(p, "FRi")),
            ]);
            put(&mut cells, 2, 0, vec![f(p, "fRA"), f(p, "rA")]);
            put(&mut cells, 3, 0, vec![f(p, "fRe"), f(p, "re")]);
            put(&mut cells, 4, 0, vec![f(p, "fRaH"), f(p, "uH")]);
            put(&mut cells, 5, 0, vec![f(p, "fRaH"), f(p, "uH")]);
            put(&mut cells, 5, 1, vec![f(p, "fRoH"), f(p, "roH")]);
            put(&mut cells, 6, 0, vec![f(p, "fRi"), f(p, "ari")]);
            put(&mut cells, 6, 1, vec![f(p, "fRoH"), f(p, "roH")]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "aH")]);
            cells
        }
        Class::GatiF => {
            // ङित् `i`-stem feminine (क्तिन्; site @gati1).
            if linga != Linga::F || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "iH"), f(p, "I"), f(p, "ayaH")),
                (f(p, "im"), f(p, "I"), f(p, "IH")),
                (f(p, "yA"), f(p, "iByAm"), f(p, "iBiH")),
                (f(p, "aye"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "eH"), f(p, "iByAm"), f(p, "iByaH")),
                (f(p, "eH"), f(p, "yoH"), f(p, "InAm")),
                (f(p, "O"), f(p, "yoH"), f(p, "izu")),
                (f(p, "e"), f(p, "I"), f(p, "ayaH")),
            ]);
            put(&mut cells, 3, 0, vec![f(p, "aye"), f(p, "yE")]);
            put(&mut cells, 4, 0, vec![f(p, "eH"), f(p, "yAH")]);
            put(&mut cells, 5, 0, vec![f(p, "eH"), f(p, "yAH")]);
            put(&mut cells, 6, 0, vec![f(p, "O"), f(p, "yAm")]);
            cells
        }
        Class::AtM(style) => {
            if linga != Linga::M || !(stem.ends_with('t') || stem.ends_with('d')) || stem.len() < 3
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_dental(stem);
            let d = format!("{}d", &w[..w.len() - 1]);
            let strong = format!("{}nt", &w[..w.len() - 1]);
            let (nom, dvi, bahu, acc_sg, acc_bahu) = match style {
                AtMStyle::Num => (
                    // Site @gacchat1/@bhavat2 strong: `त्` -> `न्`
                    // (`BA t` -> `BAn`, `gacCat` -> `gacCan`).
                    format!("{}n", &w[..w.len() - 1]),
                    f(&strong, "O"),
                    f(&strong, "aH"),
                    f(&strong, "am"),
                    f(&w, "aH"),
                ),
                AtMStyle::Abhyasta => {
                    (w.clone(), f(&w, "O"), f(&w, "aH"), f(&w, "am"), f(&w, "aH"))
                }
            };
            let mut cells = table(&[
                (nom.clone(), dvi.clone(), bahu.clone()),
                (acc_sg, dvi.clone(), acc_bahu),
                (f(&w, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(&w, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (nom, dvi, bahu),
            ]);
            if *style == AtMStyle::Abhyasta {
                // अभ्यस्त: dental-first singular `dadad`/`dadat` (no नुम्).
                put(&mut cells, 0, 0, vec![d.clone(), w.clone()]);
                put(&mut cells, 7, 0, vec![d, w]);
            }
            cells
        }
        Class::VantM => {
            // मतुप्/वतुप्/यतुप् masculine: like AtM, but prathamā-ekavacana
            // takes the vṛddhi strong (`BUtavAn`, `AyuSmAn`, `IyAn`, not
            // `*BUtavan`); the vocative keeps the short weak (`BUtavan`, as
            // in `Bagavan`). The suffix letter is the तुप्'s own (व/म/य),
            // and the weak stems end `vat`/`mat`/`yat` (normalized by
            // `krt_class`).
            let Some(l) = ["v", "m", "y"]
                .iter()
                .find(|l| stem.ends_with(&format!("{l}at")))
            else {
                return Err(NOT_POSSIBLE);
            };
            if linga != Linga::M || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let q = &stem[..stem.len() - 3];
            let strong = format!("{q}{l}ant");
            let d = format!("{p}d");
            table(&[
                (format!("{q}{l}An"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(stem, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (format!("{q}{l}an"), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::AtN(style) => {
            if linga != Linga::N || !(stem.ends_with('t') || stem.ends_with('d')) {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_dental(stem);
            let d = format!("{}d", &w[..w.len() - 1]);
            let strong = format!("{}nt", &w[..w.len() - 1]);
            // Dual/plural stems per style (site @pacat1/@viyat1/@tudat1/@dadat2).
            let dual = match style {
                AtNStyle::Num => vec![f(&strong, "I")],
                AtNStyle::WeakDual | AtNStyle::Abhyasta => vec![f(&w, "I")],
                AtNStyle::BothDual => vec![f(&strong, "I"), f(&w, "I")],
            };
            let plural = match style {
                AtNStyle::Abhyasta => vec![f(&strong, "i"), f(&w, "i")],
                _ => vec![f(&strong, "i")],
            };
            let mut cells = table(&[
                (String::new(), String::new(), String::new()),
                (String::new(), String::new(), String::new()),
                (f(&w, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(&w, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (String::new(), String::new(), String::new()),
            ]);
            // Nom/acc/voc: dental-then-dental-stop singular (द् first), then
            // the style's dual/plural stems.
            for row in [0usize, 1, 7] {
                put(&mut cells, row, 0, vec![d.clone(), w.clone()]);
                put(&mut cells, row, 1, dual.clone());
                put(&mut cells, row, 2, plural.clone());
            }
            cells
        }
        Class::AnM => {
            if linga != Linga::M || !stem.ends_with("an") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            // The शब्दपाठः sorts regular `-an` masculines three ways by the
            // weak stem (राजन्→राज्ञः, आत्मन्→आत्मनः, उक्षन्→उक्ष्णः); the
            // spelling picks the family — see `an_weak`.
            let weak = match an_weak(p) {
                AnWeak::Y => format!("{p}Y"),
                AnWeak::Keep => stem.to_string(),
                AnWeak::Drop => format!("{p}n"),
            };
            let an = format!("{p}An");
            let az = format!("{p}a");
            let mut cells = table(&[
                (f(p, "A"), f(&an, "O"), f(&an, "aH")),
                (f(&an, "am"), f(&an, "O"), f(&weak, "aH")),
                (f(&weak, "A"), f(&az, "ByAm"), f(&az, "BiH")),
                (f(&weak, "e"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&weak, "aH"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(&az, "su")),
                (stem.to_string(), f(&an, "O"), f(&an, "aH")),
            ]);
            // Locative singular keeps the site's second variant for the
            // elided families (`राज्ञि-राजनि`, `उक्ष्णि-उक्षणि`); `keep`
            // collapses (आत्मनि).
            if weak != stem {
                put(&mut cells, 6, 0, vec![f(&weak, "i"), f(stem, "i")]);
            }
            cells
        }
        Class::AnF => {
            // Feminine `-an` (`dAman`/`sIman`): identical to [`Class::AnM`]
            // cell-for-cell, including the weak accusative plural
            // (`dAmnaH`, not `dAmAnaH`); drop/keep picks match
            // [`Class::AnM`] via `an_weak`.
            if linga != Linga::F || !stem.ends_with("an") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let weak = match an_weak(p) {
                AnWeak::Y => format!("{p}Y"),
                AnWeak::Keep => stem.to_string(),
                AnWeak::Drop => format!("{p}n"),
            };
            let an = format!("{p}An");
            let az = format!("{p}a");
            let mut cells = table(&[
                (f(p, "A"), f(&an, "O"), f(&an, "aH")),
                (f(&an, "am"), f(&an, "O"), f(&weak, "aH")),
                (f(&weak, "A"), f(&az, "ByAm"), f(&az, "BiH")),
                (f(&weak, "e"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&weak, "aH"), f(&az, "ByAm"), f(&az, "ByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(&az, "su")),
                (stem.to_string(), f(&an, "O"), f(&an, "aH")),
            ]);
            // Locative singular keeps the site's second variant for the
            // elided families (`दाम्नि-दामनि`); `keep` collapses (आत्मनि).
            if weak != stem {
                put(&mut cells, 6, 0, vec![f(&weak, "i"), f(stem, "i")]);
            }
            cells
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
                (f(&strong, "aH"), f(&strong, "oH"), f(&strong, "Am")),
                (f(&strong, "i"), f(&strong, "oH"), f(&weak, "zu")),
                (stem.to_string(), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::InF => {
            // Feminine `-in` (`mahAjAlin`): identical to [`Class::InM`].
            if linga != Linga::F || !stem.ends_with("in") {
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
                (f(&strong, "aH"), f(&strong, "oH"), f(&strong, "Am")),
                (f(&strong, "i"), f(&strong, "oH"), f(&weak, "zu")),
                (stem.to_string(), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::InN => {
            // `in`-stem neuter (`kzamin`; site @kSamin2): nominative and
            // vocative drop the `n` (`kzami`); the remaining slots attach to
            // the full stem with a dental `n`, voiced by the word-level natva
            // pass (value ends `n`: all cells) where unblocked
            // (`BraminAm` -> `BramiRAm`, `kAminAm` stays dental).
            if linga != Linga::N || !stem.ends_with("in") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let eka = format!("{p}i");
            let dual = format!("{stem}I");
            let plural = format!("{p}Ini");
            let mut cells = table(&[
                (eka.clone(), dual.clone(), plural.clone()),
                (eka.clone(), dual.clone(), plural.clone()),
                (f(stem, "A"), f(&eka, "ByAm"), f(&eka, "BiH")),
                (f(stem, "e"), f(&eka, "ByAm"), f(&eka, "ByaH")),
                (f(stem, "aH"), f(&eka, "ByAm"), f(&eka, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(&eka, "zu")),
                (eka.clone(), dual, plural),
            ]);
            put(&mut cells, 7, 0, vec![stem.to_string(), eka]);
            cells
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
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
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
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(q, "AH"), f(&w, "O"), f(&w, "aH")),
            ])
        }
        Class::VasM => {
            if linga != Linga::M || !stem.ends_with("as") || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let vams = format!("{p}AMs");
            let uz = vas_weak(p, false);
            let vad = format!("{p}ad");
            let vat = format!("{p}at");
            let van = format!("{p}An");
            table(&[
                (van.clone(), f(&vams, "O"), f(&vams, "aH")),
                (f(&vams, "am"), f(&vams, "O"), f(&uz, "aH")),
                (f(&uz, "A"), f(&vad, "ByAm"), f(&vad, "BiH")),
                (f(&uz, "e"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&uz, "oH"), f(&uz, "Am")),
                (f(&uz, "i"), f(&uz, "oH"), f(&vat, "su")),
                (format!("{p}an"), f(&vams, "O"), f(&vams, "aH")),
            ])
        }
        Class::VasN => {
            // क्वसु neuter (site @vidvas2): `-vat` nominative, यण् weak dual,
            // `-vAMs` plural.
            if linga != Linga::N || !stem.ends_with("as") || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let core = &stem[..stem.len() - 2];
            let vat = format!("{core}at");
            let vad = format!("{core}ad");
            let strong = format!("{core}AMs");
            let uz = vas_weak(core, true);
            let mut cells = table(&[
                (vat.clone(), f(&uz, "I"), f(&strong, "i")),
                (vat.clone(), f(&uz, "I"), f(&strong, "i")),
                (f(&uz, "A"), f(&vad, "ByAm"), f(&vad, "BiH")),
                (f(&uz, "e"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&vad, "ByAm"), f(&vad, "ByaH")),
                (f(&uz, "aH"), f(&uz, "oH"), f(&uz, "Am")),
                (f(&uz, "i"), f(&uz, "oH"), f(&vat, "su")),
                (vat.clone(), f(&uz, "I"), f(&strong, "i")),
            ]);
            for row in [0usize, 1, 7] {
                put(&mut cells, row, 0, vec![vad.clone(), vat.clone()]);
            }
            cells
        }
        Class::Jvs => {
            // `j`-stem m/f with पausa `g`/`k` (नजिङ्; site @svapnaj1/@ruj1).
            if !(linga == Linga::M || linga == Linga::F) || !stem.ends_with('j') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "g"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(p, "gByAm"), f(p, "gBiH")),
                (f(stem, "e"), f(p, "gByAm"), f(p, "gByaH")),
                (f(stem, "aH"), f(p, "gByAm"), f(p, "gByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(p, "kzu")),
                (f(p, "g"), f(stem, "O"), f(stem, "aH")),
            ]);
            for row in [0usize, 7] {
                put(&mut cells, row, 0, vec![f(p, "g"), f(p, "k")]);
            }
            cells
        }
        Class::JvsN => {
            // `j`-stem neuter (नजिङ्; site @svapnaj2): `svapnajI`, `svapnaYji`.
            if linga != Linga::N || !stem.ends_with('j') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "g"), f(stem, "I"), f(p, "Yji")),
                (f(p, "g"), f(stem, "I"), f(p, "Yji")),
                (f(stem, "A"), f(p, "gByAm"), f(p, "gBiH")),
                (f(stem, "e"), f(p, "gByAm"), f(p, "gByaH")),
                (f(stem, "aH"), f(p, "gByAm"), f(p, "gByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(p, "kzu")),
                (f(p, "g"), f(stem, "I"), f(p, "Yji")),
            ]);
            for row in [0usize, 1, 7] {
                put(&mut cells, row, 0, vec![f(p, "g"), f(p, "k")]);
            }
            cells
        }
        Class::VanM => {
            // `van`-stem masculine (ङ्वनिप्; site @yajvan1).
            if linga != Linga::M || !stem.ends_with("an") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 2];
            let strong = format!("{p}An");
            let short = format!("{p}a");
            table(&[
                (f(p, "A"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&short, "ByAm"), f(&short, "BiH")),
                (f(stem, "e"), f(&short, "ByAm"), f(&short, "ByaH")),
                (f(stem, "aH"), f(&short, "ByAm"), f(&short, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(&short, "su")),
                (stem.to_string(), f(&strong, "O"), f(&strong, "aH")),
            ])
        }
        Class::ManN => {
            // `an`-stem neuter (ङ्वनिप्/मनिन्; site @sadman1/@nAman1):
            // nominative = stem minus `n` (`sadma`), अनी-plural on the bare
            // `an`-less stem (`sadmAni`), instr/dat/abl dual/plural on the
            // short stem (`sadmaByAm`). The weak stem keeps its `n` for a
            // cluster-final `p` (कर्मन् → कर्मणा, धन्वन्) and elides for an
            // open one (नामन् → नाम्ना), where the site lists the
            // elided/unelided alternants side by side (`नाम्नी-नामनी`,
            // `नाम्नि-नामनि`).
            if linga != Linga::N || !stem.ends_with("an") {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let q = &stem[..stem.len() - 2];
            let weak = match an_weak(q) {
                AnWeak::Keep => stem.to_string(),
                _ => format!("{q}n"),
            };
            let plural = f(q, "Ani");
            let dual = f(p, "nI");
            let mut cells = table(&[
                (p.to_string(), dual.clone(), plural.clone()),
                (p.to_string(), dual.clone(), plural.clone()),
                (f(&weak, "A"), f(p, "ByAm"), f(p, "BiH")),
                (f(&weak, "e"), f(p, "ByAm"), f(p, "ByaH")),
                (f(&weak, "aH"), f(p, "ByAm"), f(p, "ByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(p, "su")),
                (p.to_string(), dual, plural),
            ]);
            if weak != stem {
                let du = vec![f(&weak, "I"), f(p, "nI")];
                put(&mut cells, 0, 1, du.clone());
                put(&mut cells, 1, 1, du.clone());
                put(&mut cells, 7, 1, du);
                put(&mut cells, 6, 0, vec![f(&weak, "i"), f(stem, "i")]);
            }
            put(&mut cells, 7, 0, vec![stem.to_string(), p.to_string()]);
            cells
        }
        Class::Word(w) => return word_table(stem, linga, w),
    };
    // Word-level Natva over recipe outputs (`gurunA -> guruRA`,
    // `balinA` stays: no trigger), gated per cell the way the site only
    // runs the नत्व step when a न-bearing sup् pratyaya or an न्-final
    // prātipadika is involved. Hand-written pronoun/numeral tables bypass
    // this (already correct) via the early return above.
    let cells = natva_cells(cells, stem, linga, class);
    Ok(Paradigm {
        stem: stem.to_string(),
        linga,
        class: class.clone(),
        cells,
    })
}

/// Word-level Natva over recipe outputs, mirroring the site's per-cell
/// gating. The site runs the नत्व step for a cell only when the sup्
/// pratyaya in play contains a न (इन instr-sg, नाम् gen-pl, neuter plural
/// आनि/नि; the नुम्-neuters' dual नी rides the n-final branch below) or
/// when the prātipadika itself ends in न् — so:
///
/// * `n`-final stems (`rAjan`, `kzamin`, `sadman`): every cell.
/// * इगन्त neuters with the नुम् augment (@kaTu4/@vAri1): every cell.
/// * otherwise instr-sg, gen-pl and neuter plural only — the a-stem
///   neuter dual (`granTye` = ग्रन्थ्ये) carries no न of its own, so the
///   step must not run there (it would voice the stem's न् instead).
/// * अदन्त/अत् stems never voice their strong नुम् (`gacCanti` stays
///   dental; @pacat1/@viyat1/@tudat1/@dadat2).
/// * Forms spelled exactly like the citation stem stay unvoiced: lookup is
///   by surface spelling (SPEC F5 trio `Wrampena` must remain findable as
///   its own vocative; utterance voicing is a separate layer).
fn natva_cells(cells: Vec<Slot>, stem: &str, linga: Linga, class: &Class) -> Vec<Slot> {
    // न्-final prātipadikas (`rAjan`/`kzamin`/`sadman`) and इगन्त neuters
    // with the नुम् augment (@kaTu4/@vAri1) run the step for every cell.
    let all = stem.ends_with('n') || matches!(class, Class::IN | Class::UN);
    // A stem that spells the `र्न` conjunct itself (`durnAman`) keeps its
    // abutting nasal dental: the site treats that `र्` as रुत्व-र्
    // (`दुर्` < `दुस्`). Stems spelling `र`+`न्` apart (`dhIran`) voice the
    // composed conjunct instead (`धीर्णा`, कर्ण-like).
    let ru_tva = stem.contains("rn");
    // Otherwise only cells whose sup् affix itself contains a न: इन
    // instr-sg (a/i/u-stems) and नाम् gen-pl (a/ā/i/ī/u/ū-stems). अदन्त/अत्
    // stems must not voice their strong नुम् (`gacCanti`, `kzuBnatA` stay
    // dental: @pacat1/@viyat1/@tudat1/@dadat2 never run the step).
    let instr_in = matches!(class, Class::AM | Class::AN | Class::IMF | Class::UMF);
    let genpl_nam = matches!(
        class,
        Class::AM
            | Class::AN
            | Class::AAF
            | Class::IIF
            | Class::IIFN
            | Class::IMF
            | Class::UMF
            | Class::UF
            | Class::UUF
            | Class::UUFN
            | Class::GatiF
    );
    let neuter_n_affix = linga == Linga::N && !matches!(class, Class::AtN(_));
    cells
        .into_iter()
        .enumerate()
        .map(|(idx, mut c)| {
            let (row, col) = (idx / 3, idx % 3);
            let allow = all
                || (col == 0 && row == 2 && instr_in)
                || (col == 2 && row == 5 && genpl_nam)
                || (neuter_n_affix && matches!(row, 0 | 1 | 7) && col == 2);
            c.forms = c
                .forms
                .into_iter()
                .map(|s| {
                    if !allow || s == stem {
                        s
                    } else {
                        site_natva(&s, ru_tva)
                    }
                })
                .collect();
            c
        })
        .collect()
}

/// Site-faithful णत्व for one surface form (SLP1).
///
/// The site's `रषाभ्याम्_or_अटकुप्_or_पदान्तस्य` step targets the *last*
/// letter `न्` of the word and looks left for the nearest `र्`/`ष्`/`ऋ`/`ॠ`
/// trigger; every letter strictly between must be in अट् + कवर्ग + पवर्ग +
/// ह्/य्/व् + anusvāra (ल्, न्, dentals, sibilants and the rest block), and
/// a word-final `न्` never voices (8.4.37 पदान्तस्य).
///
/// Two spelling details the site keeps: a nasal in a **ह्न conjunct** never
/// voices — दीर्घाह्नः/तमिस्रह्नः stay dental even though ह् is otherwise
/// transparent (बर्हिणः, ब्रह्मणः, ग्राहिणः voice through it) — and a nasal
/// that abuts the **रुत्व-र्** of a stem spelled with `र्न` stays dental
/// (दुर्नामा, दुर्नामभ्याम्), while an original `र्` voices the composed
/// conjunct (धीरन् → धीर्णा; कर्ण-like). `ru_tva` carries that stem fact.
fn site_natva(s: &str, ru_tva: bool) -> String {
    let Some(pos) = s.rfind('n') else {
        return s.to_string();
    };
    if pos == s.len() - 1 {
        return s.to_string();
    }
    if pos > 0 && s.as_bytes()[pos - 1] == b'h' {
        return s.to_string();
    }
    let mut trigger = false;
    for (i, b) in s.bytes().take(pos).rev().enumerate() {
        match b as char {
            // रुत्व-र् (`दुर्` < `दुस्`, only when the stem itself spells the
            // `र्न` conjunct: दुर्नामन्) never voices the nasal it abuts;
            // an original र् does (धीरन् → धीर्णा, बर्हिन् → बर्हिणा).
            'r' if i == 0 && ru_tva => {}
            'f' | 'F' | 'z' | 'r' => {
                trigger = true;
                break;
            }
            'a' | 'A' | 'i' | 'I' | 'u' | 'U' | 'x' | 'X' | 'e' | 'E' | 'o' | 'O' | 'y' | 'v'
            | 'h' | 'k' | 'K' | 'g' | 'G' | 'N' | 'p' | 'P' | 'b' | 'B' | 'm' | 'M' => {}
            _ => break,
        }
    }
    if !trigger {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..pos]);
    out.push('R');
    out.push_str(&s[pos + 1..]);
    out
}

fn f(base: &str, ending: &str) -> String {
    format!("{base}{ending}")
}

/// The जश्-व्यवस्था, in the narrow form the भ्-final family needs: the
/// stem's own द/ब/ग/ज voices before the भ्-ending — गर्दभ् → गर्धब्,
/// गर्धप्, गर्धब्भ्याम्, गर्धप्सु. Only the last such consonant is
/// voiced: the earlier ones sit before a soft consonant (ग in गर्दभ्
/// keeps ग) and the SLP1 spelling cannot tell a silent inherent vowel
/// from a real one.
fn jash_voice(base: &str) -> String {
    let chars: Vec<char> = base.chars().collect();
    let Some(last) = chars
        .iter()
        .rposition(|c| matches!(c, 'd' | 'b' | 'g' | 'j'))
    else {
        return base.to_string();
    };
    let mut out = String::with_capacity(base.len());
    for (i, ch) in chars.iter().enumerate() {
        if i == last {
            out.push(match ch {
                'd' => 'D',
                'b' => 'B',
                'g' => 'G',
                _ => 'J',
            });
        } else {
            out.push(*ch);
        }
    }
    out
}

/// Overwrite one cell's forms (row = 0-7 vibhakti, col = 0-2 vacana).
fn put(cells: &mut [Slot], row: usize, col: usize, forms: Vec<String>) {
    if let Some(cell) = cells.get_mut(row * 3 + col) {
        cell.forms = forms;
    }
}

/// क्वसु/वस् weak stem (site's वसोः सम्प्रसारणम् + सन्धि).
///
/// The site replaces the affix `वस्` with `उअस्` (samprasāraṇa), merges
/// `उअ` to `उ`, drops the इट् `इ` in the masculine paradigm (@sedivas1;
/// @vidvas2 keeps it for यण्), then resolves the junction of the stem-final
/// vowel with `उ` by internal sandhi and applies षत्व:
/// `babU`+उ = `babUz`, `dadiv` (m) -> `daduz` / (n) -> `dadyuz`,
/// `cakf` -> `cakruz`, `yayA`+उ = `yayoz`, `cikrI`+उ = `cikryuz`.
fn vas_weak(core: &str, neuter: bool) -> String {
    let q = core.strip_suffix('v').unwrap_or(core);
    let q = match q.strip_suffix('i') {
        Some(stripped) if !neuter => stripped,
        _ => q,
    };
    let (base, uz) = match q.chars().last() {
        Some('a' | 'A') => (&q[..q.len() - 1], "oz"),
        Some('i' | 'I') => (&q[..q.len() - 1], "yuz"),
        Some('u' | 'U') => (&q[..q.len() - 1], "Uz"),
        Some('f' | 'F') => (&q[..q.len() - 1], "ruz"),
        Some('x' | 'X') => (&q[..q.len() - 1], "luz"),
        Some('e') => (&q[..q.len() - 1], "ayuz"),
        Some('E') => (&q[..q.len() - 1], "Ayuz"),
        Some('o') => (&q[..q.len() - 1], "avuz"),
        Some('O') => (&q[..q.len() - 1], "Avuz"),
        _ => (q, "uz"),
    };
    format!("{base}{uz}")
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

/// Weak-stem family of a regular masculine `-an` stem (`p` = stem minus
/// `an`). The शब्दपाठः splits the regular paradigms three ways and the
/// spelling decides:
///
/// * `Y` (ञ्) — `p` ends in a palatal (`c`/`j`): राजन्→राज्ञः, मज्जन्,
///   प्रियपञ्चन् (the ञ् also survives the मज्ज-cluster);
/// * `Drop` — `p`'s last consonant is ट्/ध्/ष् (`w`/`D`/`z`) or "open"
///   (attached to a vowel, not a virama): उक्षन्→उक्ष्णः, मूर्धन्,
///   प्रियाष्टन्, ग्रावन्, प्रेमन्, अणिमन् (the अ elides);
/// * `Keep` — a cluster-final `p` (आत्मन्→आत्मनः, कर्मन्, अध्वन्, अश्मन्).
///
/// Irregular families are not spelling-derivable and stay with their
/// शब्दपाठः grids: the `वन्` type (अर्वन्→अर्वतः, मघवन्), special grades
/// (युवन्→यूनः, श्वन्→शुनः, प्रतिदिवन्→प्रतिदीव्नः), the हन्→घ्न paradigm
/// (वृत्रहन्→वृत्रघ्नः) and the numerals (अष्टन्/दशन्/…).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AnWeak {
    Y,
    Keep,
    Drop,
}

fn an_weak(p: &str) -> AnWeak {
    let mut chars = p.chars().rev();
    let Some(last) = chars.next() else {
        return AnWeak::Keep;
    };
    match last {
        'c' | 'j' => AnWeak::Y,
        'w' | 'D' | 'z' => AnWeak::Drop,
        // SLP1 consonants carry their inherent vowel, so a preceding vowel
        // character means "open"; anything else (or a vowel-final `p`,
        // which is malformed here) falls through to `keep`.
        _ => match chars.next() {
            Some(prev) if "aAiIuUfFxXeEoO".contains(prev) => AnWeak::Drop,
            _ => AnWeak::Keep,
        },
    }
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
            ["sarvasya", "sarvayoH", "sarvezAm"],
            ["sarvasmin", "sarvayoH", "sarvezu"],
            ["sarva", "sarvO", "sarve"],
        ]),
        ("sarva", Linga::N) => rows1(&[
            ["sarvam", "sarve", "sarvARi"],
            ["sarvam", "sarve", "sarvARi"],
            ["sarveRa", "sarvAByAm", "sarvEH"],
            ["sarvasmE", "sarvAByAm", "sarveByaH"],
            ["sarvasmAt", "sarvAByAm", "sarveByaH"],
            ["sarvasya", "sarvayoH", "sarvezAm"],
            ["sarvasmin", "sarvayoH", "sarvezu"],
            ["sarva", "sarve", "sarvARi"],
        ]),
        ("sarva", Linga::F) => rows1(&[
            ["sarvA", "sarve", "sarvAH"],
            ["sarvAm", "sarve", "sarvAH"],
            ["sarvayA", "sarvAByAm", "sarvABiH"],
            ["sarvasyE", "sarvAByAm", "sarvAByaH"],
            ["sarvasyAH", "sarvAByAm", "sarvAByaH"],
            ["sarvasyAH", "sarvayoH", "sarvAsAm"],
            ["sarvasyAm", "sarvayoH", "sarvAsu"],
            ["sarve", "sarve", "sarvAH"],
        ]),
        // गो/नौ/रै diphthong words (ADDITIONS 2.5), gridded verbatim from
        // the site (@go1/@go2, @nau1, @rai1/@rai2): the उभयवृद्धि stem
        // गाव्/नाव्, प्रथमा-सि as the bare diphthong + visarga, and the
        // गो-distinct accusative-bahu गाः.
        ("go", Linga::M | Linga::F) => rows1(&[
            ["gOH", "gAvO", "gAvaH"],
            ["gAm", "gAvO", "gAH"],
            ["gavA", "goByAm", "goBiH"],
            ["gave", "goByAm", "goByaH"],
            ["goH", "goByAm", "goByaH"],
            ["goH", "gavoH", "gavAm"],
            ["gavi", "gavoH", "gozu"],
            ["gOH", "gAvO", "gAvaH"],
        ]),
        ("nO", Linga::F) => rows1(&[
            ["nOH", "nAvO", "nAvaH"],
            ["nAvam", "nAvO", "nAvaH"],
            ["nAvA", "nOByAm", "nOBiH"],
            ["nAve", "nOByAm", "nOByaH"],
            ["nAvaH", "nOByAm", "nOByaH"],
            ["nAvaH", "nAvoH", "nAvAm"],
            ["nAvi", "nAvoH", "nOzu"],
            ["nOH", "nAvO", "nAvaH"],
        ]),
        ("rE", Linga::M | Linga::F) => rows1(&[
            ["rAH", "rAyO", "rAyaH"],
            ["rAyam", "rAyO", "rAyaH"],
            ["rAyA", "rAByAm", "rABiH"],
            ["rAye", "rAByAm", "rAByaH"],
            ["rAyaH", "rAByAm", "rAByaH"],
            ["rAyaH", "rAyoH", "rAyAm"],
            ["rAyi", "rAyoH", "rAsu"],
            ["rAH", "rAyO", "rAyaH"],
        ]),
        // प्रण-नाम-श्री (ADDITIONS 2.6) — सर्व-family स्म-शेषों grid
        // (@anya1/@anya2, @itara1/@itara2, @katara1/@katara2,
        // @katama1/@katama2, @ubhaya1/@ubhaya2, @ubha1/@ubha2,
        // @anyA1) with the site's alternant pairs in the singular slots
        // they show them: प्रथमा-/सम्बोधन-eka अन्यद्-अन्यत् (neuters) and
        // पञ्चमी-eka स्माद्-स्मात् (every member). उभ duals-only — the site
        // leaves eka/bahu cells empty (filtered slots, rendered —).
        ("anya", Linga::M) => {
            let mut rows = rows1(&[
                ["anyaH", "anyO", "anye"],
                ["anyam", "anyO", "anyAn"],
                ["anyena", "anyAByAm", "anyEH"],
                ["anyasmE", "anyAByAm", "anyeByaH"],
                ["anyasmAd", "anyAByAm", "anyeByaH"],
                ["anyasya", "anyayoH", "anyezAm"],
                ["anyasmin", "anyayoH", "anyezu"],
                ["anya", "anyO", "anye"],
            ]);
            rows[4].0 = vec!["anyasmAd".to_string(), "anyasmAt".to_string()];
            rows
        }
        ("anya", Linga::N) => {
            let mut rows = rows1(&[
                ["anyat", "anye", "anyAni"],
                ["anyat", "anye", "anyAni"],
                ["anyena", "anyAByAm", "anyEH"],
                ["anyasmE", "anyAByAm", "anyeByaH"],
                ["anyasmAd", "anyAByAm", "anyeByaH"],
                ["anyasya", "anyayoH", "anyezAm"],
                ["anyasmin", "anyayoH", "anyezu"],
                ["anyat", "anye", "anyAni"],
            ]);
            rows[0].0 = vec!["anyad".to_string(), "anyat".to_string()];
            rows[1].0 = vec!["anyad".to_string(), "anyat".to_string()];
            rows[4].0 = vec!["anyasmAd".to_string(), "anyasmAt".to_string()];
            rows[7].0 = vec!["anyad".to_string(), "anyat".to_string()];
            rows
        }
        ("anyA", Linga::F) => rows1(&[
            ["anyA", "anye", "anyAH"],
            ["anyAm", "anye", "anyAH"],
            ["anyayA", "anyAByAm", "anyABiH"],
            ["anyasyE", "anyAByAm", "anyAByaH"],
            ["anyasyAH", "anyAByAm", "anyAByaH"],
            ["anyasyAH", "anyayoH", "anyAsAm"],
            ["anyasyAm", "anyayoH", "anyAsu"],
            ["anye", "anye", "anyAH"],
        ]),
        ("itara", Linga::M) => {
            let mut rows = rows1(&[
                ["itaraH", "itarO", "itare"],
                ["itaram", "itarO", "itarAn"],
                ["itareRa", "itarAByAm", "itarEH"],
                ["itarasmE", "itarAByAm", "itareByaH"],
                ["itarasmAd", "itarAByAm", "itareByaH"],
                ["itarasya", "itarayoH", "itarezAm"],
                ["itarasmin", "itarayoH", "itarezu"],
                ["itara", "itarO", "itare"],
            ]);
            rows[4].0 = vec!["itarasmAd".to_string(), "itarasmAt".to_string()];
            rows
        }
        ("itara", Linga::N) => {
            let mut rows = rows1(&[
                ["itarat", "itare", "itarARi"],
                ["itarat", "itare", "itarARi"],
                ["itareRa", "itarAByAm", "itarEH"],
                ["itarasmE", "itarAByAm", "itareByaH"],
                ["itarasmAd", "itarAByAm", "itareByaH"],
                ["itarasya", "itarayoH", "itarezAm"],
                ["itarasmin", "itarayoH", "itarezu"],
                ["itarat", "itare", "itarARi"],
            ]);
            rows[0].0 = vec!["itarad".to_string(), "itarat".to_string()];
            rows[1].0 = vec!["itarad".to_string(), "itarat".to_string()];
            rows[4].0 = vec!["itarasmAd".to_string(), "itarasmAt".to_string()];
            rows[7].0 = vec!["itarad".to_string(), "itarat".to_string()];
            rows
        }
        ("katara", Linga::M) => {
            let mut rows = rows1(&[
                ["kataraH", "katarO", "katare"],
                ["kataram", "katarO", "katarAn"],
                ["katareRa", "katarAByAm", "katarEH"],
                ["katarasmE", "katarAByAm", "katareByaH"],
                ["katarasmAd", "katarAByAm", "katareByaH"],
                ["katarasya", "katarayoH", "katarezAm"],
                ["katarasmin", "katarayoH", "katarezu"],
                ["katara", "katarO", "katare"],
            ]);
            rows[4].0 = vec!["katarasmAd".to_string(), "katarasmAt".to_string()];
            rows
        }
        ("katara", Linga::N) => {
            let mut rows = rows1(&[
                ["katarat", "katare", "katarARi"],
                ["katarat", "katare", "katarARi"],
                ["katareRa", "katarAByAm", "katarEH"],
                ["katarasmE", "katarAByAm", "katareByaH"],
                ["katarasmAd", "katarAByAm", "katareByaH"],
                ["katarasya", "katarayoH", "katarezAm"],
                ["katarasmin", "katarayoH", "katarezu"],
                ["katarat", "katare", "katarARi"],
            ]);
            rows[0].0 = vec!["katarad".to_string(), "katarat".to_string()];
            rows[1].0 = vec!["katarad".to_string(), "katarat".to_string()];
            rows[4].0 = vec!["katarasmAd".to_string(), "katarasmAt".to_string()];
            rows[7].0 = vec!["katarad".to_string(), "katarat".to_string()];
            rows
        }
        ("katama", Linga::M) => {
            let mut rows = rows1(&[
                ["katamaH", "katamO", "katame"],
                ["katamam", "katamO", "katamAn"],
                ["katamena", "katamAByAm", "katamEH"],
                ["katamasmE", "katamAByAm", "katameByaH"],
                ["katamasmAd", "katamAByAm", "katameByaH"],
                ["katamasya", "katamayoH", "katamezAm"],
                ["katamasmin", "katamayoH", "katamezu"],
                ["katama", "katamO", "katame"],
            ]);
            rows[4].0 = vec!["katamasmAd".to_string(), "katamasmAt".to_string()];
            rows
        }
        ("katama", Linga::N) => {
            let mut rows = rows1(&[
                ["katamat", "katame", "katamAni"],
                ["katamat", "katame", "katamAni"],
                ["katamena", "katamAByAm", "katamEH"],
                ["katamasmE", "katamAByAm", "katameByaH"],
                ["katamasmAd", "katamAByAm", "katameByaH"],
                ["katamasya", "katamayoH", "katamezAm"],
                ["katamasmin", "katamayoH", "katamezu"],
                ["katamat", "katame", "katamAni"],
            ]);
            rows[0].0 = vec!["katamad".to_string(), "katamat".to_string()];
            rows[1].0 = vec!["katamad".to_string(), "katamat".to_string()];
            rows[4].0 = vec!["katamasmAd".to_string(), "katamasmAt".to_string()];
            rows[7].0 = vec!["katamad".to_string(), "katamat".to_string()];
            rows
        }
        ("uBaya", Linga::M) => {
            let mut rows = rows1(&[
                ["uBayaH", "uBayO", "uBaye"],
                ["uBayam", "uBayO", "uBayAn"],
                ["uBayena", "uBayAByAm", "uBayEH"],
                ["uBayasmE", "uBayAByAm", "uBayeByaH"],
                ["uBayasmAd", "uBayAByAm", "uBayeByaH"],
                ["uBayasya", "uBayayoH", "uBayezAm"],
                ["uBayasmin", "uBayayoH", "uBayezu"],
                ["uBaya", "uBayO", "uBaye"],
            ]);
            rows[4].0 = vec!["uBayasmAd".to_string(), "uBayasmAt".to_string()];
            rows
        }
        ("uBaya", Linga::N) => {
            let mut rows = rows1(&[
                ["uBayam", "uBaye", "uBayAni"],
                ["uBayam", "uBaye", "uBayAni"],
                ["uBayena", "uBayAByAm", "uBayEH"],
                ["uBayasmE", "uBayAByAm", "uBayeByaH"],
                ["uBayasmAd", "uBayAByAm", "uBayeByaH"],
                ["uBayasya", "uBayayoH", "uBayezAm"],
                ["uBayasmin", "uBayayoH", "uBayezu"],
                ["uBaya", "uBaye", "uBayAni"],
            ]);
            rows[4].0 = vec!["uBayasmAd".to_string(), "uBayasmAt".to_string()];
            rows
        }
        ("uBa", Linga::M) => rows1(&[
            ["", "uBO", ""],
            ["", "uBO", ""],
            ["", "uBAByAm", ""],
            ["", "uBAByAm", ""],
            ["", "uBAByAm", ""],
            ["", "uBayoH", ""],
            ["", "uBayoH", ""],
            ["", "uBO", ""],
        ]),
        ("uBa", Linga::N) => rows1(&[
            ["", "uBe", ""],
            ["", "uBe", ""],
            ["", "uBAByAm", ""],
            ["", "uBAByAm", ""],
            ["", "uBAByAm", ""],
            ["", "uBayoH", ""],
            ["", "uBayoH", ""],
            ["", "uBe", ""],
        ]),
        ("tad", Linga::M) => rows1(&[
            ["saH", "tO", "te"],
            ["tam", "tO", "tAn"],
            ["tena", "tAByAm", "tEH"],
            ["tasmE", "tAByAm", "teByaH"],
            ["tasmAt", "tAByAm", "teByaH"],
            ["tasya", "tayoH", "tezAm"],
            ["tasmin", "tayoH", "tezu"],
            ["saH", "tO", "te"],
        ]),
        ("tad", Linga::N) => {
            let rows = rows1(&[
                ["tat", "te", "tAni"],
                ["tat", "te", "tAni"],
                ["tena", "tAByAm", "tEH"],
                ["tasmE", "tAByAm", "teByaH"],
                ["tasmAt", "tAByAm", "teByaH"],
                ["tasya", "tayoH", "tezAm"],
                ["tasmin", "tayoH", "tezu"],
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
            ["tasyAH", "tayoH", "tAsAm"],
            ["tasyAm", "tayoH", "tAsu"],
            ["sA", "te", "tAH"],
        ]),
        ("etad", Linga::M) => rows1(&[
            ["ezaH", "etO", "ete"],
            ["etam", "etO", "etAn"],
            ["etena", "etAByAm", "etEH"],
            ["etasmE", "etAByAm", "eteByaH"],
            ["etasmAt", "etAByAm", "eteByaH"],
            ["etasya", "etayoH", "etezAm"],
            ["etasmin", "etayoH", "etezu"],
            ["ezaH", "etO", "ete"],
        ]),
        ("etad", Linga::N) => {
            let rows = rows1(&[
                ["etat", "ete", "etAni"],
                ["etat", "ete", "etAni"],
                ["etena", "etAByAm", "etEH"],
                ["etasmE", "etAByAm", "eteByaH"],
                ["etasmAt", "etAByAm", "eteByaH"],
                ["etasya", "etayoH", "etezAm"],
                ["etasmin", "etayoH", "etezu"],
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
            ["etasyAH", "etayoH", "etAsAm"],
            ["etasyAm", "etayoH", "etAsu"],
            ["ezA", "ete", "etAH"],
        ]),
        ("yad", Linga::M) => rows1(&[
            ["yaH", "yO", "ye"],
            ["yam", "yO", "yAn"],
            ["yena", "yAByAm", "yEH"],
            ["yasmE", "yAByAm", "yeByaH"],
            ["yasmAt", "yAByAm", "yeByaH"],
            ["yasya", "yayoH", "yezAm"],
            ["yasmin", "yayoH", "yezu"],
            ["yaH", "yO", "ye"],
        ]),
        ("yad", Linga::N) => {
            let rows = rows1(&[
                ["yat", "ye", "yAni"],
                ["yat", "ye", "yAni"],
                ["yena", "yAByAm", "yEH"],
                ["yasmE", "yAByAm", "yeByaH"],
                ["yasmAt", "yAByAm", "yeByaH"],
                ["yasya", "yayoH", "yezAm"],
                ["yasmin", "yayoH", "yezu"],
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
            ["yasyAH", "yayoH", "yAsAm"],
            ["yasyAm", "yayoH", "yAsu"],
            ["yA", "ye", "yAH"],
        ]),
        ("kim", Linga::M) => rows1(&[
            ["kaH", "kO", "ke"],
            ["kam", "kO", "kAn"],
            ["kena", "kAByAm", "kEH"],
            ["kasmE", "kAByAm", "keByaH"],
            ["kasmAt", "kAByAm", "keByaH"],
            ["kasya", "kayoH", "kezAm"],
            ["kasmin", "kayoH", "kezu"],
            ["kaH", "kO", "ke"],
        ]),
        ("kim", Linga::N) => rows1(&[
            ["kim", "ke", "kAni"],
            ["kim", "ke", "kAni"],
            ["kena", "kAByAm", "kEH"],
            ["kasmE", "kAByAm", "keByaH"],
            ["kasmAt", "kAByAm", "keByaH"],
            ["kasya", "kayoH", "kezAm"],
            ["kasmin", "kayoH", "kezu"],
            ["kim", "ke", "kAni"],
        ]),
        ("kim", Linga::F) => rows1(&[
            ["kA", "ke", "kAH"],
            ["kAm", "ke", "kAH"],
            ["kayA", "kAByAm", "kABiH"],
            ["kasyE", "kAByAm", "kAByaH"],
            ["kasyAH", "kAByAm", "kAByaH"],
            ["kasyAH", "kayoH", "kAsAm"],
            ["kasyAm", "kayoH", "kAsu"],
            ["kA", "ke", "kAH"],
        ]),
        ("idam", Linga::M) => rows1(&[
            ["ayam", "imO", "ime"],
            ["imam", "imO", "imAn"],
            ["anena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAt", "AByAm", "eByaH"],
            ["asya", "anayoH", "ezAm"],
            ["asmin", "anayoH", "ezu"],
            ["ayam", "imO", "ime"],
        ]),
        ("idam", Linga::N) => rows1(&[
            ["idam", "ime", "imAni"],
            ["idam", "ime", "imAni"],
            ["anena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAt", "AByAm", "eByaH"],
            ["asya", "anayoH", "ezAm"],
            ["asmin", "anayoH", "ezu"],
            ["idam", "ime", "imAni"],
        ]),
        ("idam", Linga::F) => rows1(&[
            ["iyam", "ime", "imAH"],
            ["imAm", "ime", "imAH"],
            ["anayA", "AByAm", "ABiH"],
            ["asyE", "AByAm", "AByaH"],
            ["asyAH", "AByAm", "AByaH"],
            ["asyAH", "anayoH", "AsAm"],
            ["asyAm", "anayoH", "Asu"],
            ["iyam", "ime", "imAH"],
        ]),
        ("adas", Linga::M) => rows1(&[
            ["asO", "amU", "amI"],
            ["amum", "amU", "amUn"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAt", "amUByAm", "amIByaH"],
            ["amuzya", "amuyoH", "amIzAm"],
            ["amuzmin", "amuyoH", "amIzu"],
            ["asO", "amU", "amI"],
        ]),
        ("adas", Linga::N) => rows1(&[
            ["adaH", "amU", "amUni"],
            ["adaH", "amU", "amUni"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAt", "amUByAm", "amIByaH"],
            ["amuzya", "amuyoH", "amIzAm"],
            ["amuzmin", "amuyoH", "amIzu"],
            ["adaH", "amU", "amUni"],
        ]),
        ("adas", Linga::F) => rows1(&[
            ["asO", "amU", "amUH"],
            ["amUm", "amU", "amUH"],
            ["amuyA", "amUByAm", "amUBiH"],
            ["amuzyE", "amUByAm", "amUByaH"],
            ["amuzyAH", "amUByAm", "amUByaH"],
            ["amuzyAH", "amuyoH", "amUzAm"],
            ["amuzyAm", "amuyoH", "amUzu"],
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
            return decline("Bavat", Linga::M, &Class::AtM(AtMStyle::Num));
        }
        ("Bavat", Linga::N) => {
            return decline("Bavat", Linga::N, &Class::AtN(AtNStyle::Num));
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
        (DVI, vec!["yuvayoH"]),
        (BAHU, vec!["yuzmAkam"]),
        (EKA, vec!["tvayi"]),
        (DVI, vec!["yuvayoH"]),
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
        (DVI, vec!["AvayoH"]),
        (BAHU, vec!["asmAkam"]),
        (EKA, vec!["mayi"]),
        (DVI, vec!["AvayoH"]),
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
        vec!["dvayoH".to_string()],
        vec!["dvayoH".to_string()],
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

/// Known pronoun/numeral stems. गो/नौ/रै (ADDITIONS 2.5) are gridded
/// diphthong words — 'go'/'nO'/'rE' in SLP1. अन्य/इतर/कतर/कतम/उभय/उभ/अन्या
/// (ADDITIONS 2.6) are the सर्व-family स्म-ending pronominals the site
/// grids explicitly.
pub const KNOWN_WORDS: &[&str] = &[
    "sarva", "tad", "etad", "yad", "kim", "idam", "adas", "yuzmad", "asmad", "Bavat", "eka", "dvi",
    "tri", "catur", "paJcan", "zaz", "saptan", "azwan", "navan", "daSan", "go", "nO", "rE", "anya",
    "anyA", "itara", "katara", "katama", "uBaya", "uBa",
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
    } else if let Some(p) = stem.strip_suffix("an") {
        // Manual `-an`: an-stems (`rAjan`). Participle `-an` (`Bavan`)
        // always resolves through the krt-exact path, so manual never
        // needs at-m here; `-in` is in-stem only; `-van`/`-man` use the
        // ङ्वनिप् paradigms (`yajvan`/`sadman`).
        if linga == Linga::M {
            // Cluster-final `-van` (यज्वन्/धन्वन्/पर्वन्) is the ङ्वनिप्
            // keep family; an open `व्` (ग्रावन्/पीवन्) elides like the
            // rest of the drop family (`an_weak`).
            if stem.ends_with("van") && an_weak(p) != AnWeak::Drop {
                out.push(Class::VanM);
            } else {
                out.push(Class::AnM);
            }
        } else if linga == Linga::N {
            // Every `-an` neuter follows the मनिन् shape (`nAman`/`karman`
            // elide or keep by the same spelling rule; `dhanvan`/`parvan`
            // keep).
            out.push(Class::ManN);
        } else if linga == Linga::F {
            out.push(Class::AnF);
        }
    } else if stem.ends_with("in") {
        match linga {
            Linga::M => out.push(Class::InM),
            Linga::N => out.push(Class::InN),
            Linga::F => out.push(Class::InF),
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
            // नदीसंज्ञा is the Pāṇinian default for lexical `-I` stems;
            // the कृदन्त/उगित् flavour is the second candidate — a bare
            // stem cannot say which is meant, so both tables are offered.
            out.push(Class::IIFN);
            out.push(Class::IIF);
        }
    } else if stem.ends_with('u') {
        match linga {
            Linga::M | Linga::F => out.push(Class::UMF),
            Linga::N => out.push(Class::UN),
        }
    } else if stem.ends_with('U') {
        if linga == Linga::F {
            out.push(Class::UUFN);
            out.push(Class::UUF);
        }
    } else if stem.ends_with('f') {
        if linga != Linga::N {
            out.push(Class::RFM);
        }
    } else if stem.ends_with('j') {
        match linga {
            Linga::M | Linga::F => out.push(Class::Jvs),
            Linga::N => out.push(Class::JvsN),
        }
    } else if stem.ends_with('t')
        || stem.ends_with('d')
        || stem.ends_with('D')
        || stem.ends_with('B')
        || stem.ends_with('c')
    {
        match linga {
            Linga::M => {
                // अ-उपधा त् participles/adjectives (गच्छत्, भवन्त) keep the
                // गच्छन् shape; bare consonant-final nouns (मरुत्, बुध्)
                // take the सरित् paradigm with its pausa pair. The वतुप्
                // nominalization (आत्मवत् → आत्मवान्, site @Atmavat1) is
                // the vṛddhi-strong shape — but the ऋ-वत् participle
                // (भवत् → भवन्, site @Bhavat1) ends the same way, so both
                // candidates are offered; the site's grid decides for the
                // words it lists.
                if stem.ends_with("vat") || stem.ends_with("mat") || stem.ends_with("yat") {
                    out.push(Class::VantM);
                    out.push(Class::AtM(AtMStyle::Num));
                } else if stem.ends_with("at") || stem.ends_with("ant") {
                    out.push(Class::AtM(AtMStyle::Num));
                } else {
                    out.push(Class::Cons);
                }
            }
            Linga::F => out.push(Class::Cons),
            Linga::N => {
                // Neuter participles keep गच्छत्/गच्छन् (site @gacchat3);
                // no neuter noun grids for भ्/च् finals.
                if stem.ends_with('t') || stem.ends_with('d') {
                    out.push(Class::AtN(AtNStyle::Num));
                }
            }
        }
    } else if (stem.ends_with("us")
        || stem.ends_with("is")
        || stem.ends_with("uz")
        || stem.ends_with("iz"))
        && linga == Linga::N
    {
        // उस्/इस्-स्वादि neuters (आयुष्, हविस्, धनुस्); the `uz`/`iz`
        // spellings are the षत्व forms (आयुष् = Ayuz). Neuter only.
        out.push(Class::UsN);
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
            // Re-derive the exact class from the krt entry: site reference
            // paradigm first (`dhatu_id` is the site's `baseindex` for the
            // gaṇa-dependent overrides), then the shape-based fallback.
            let exact = krt_class_ctx(
                &first.family,
                &first.krt,
                &first.field,
                stem,
                &first.dhatu_id,
            )
            .or_else(|| krt_class(&first.family, &first.krt, &first.field, stem));
            if let Some((class, weak)) = exact {
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

/// Site reference paradigm for one कृत् cell — the port of
/// ashtadhyayi.com's `kritPratyayaOrder` `refP`/`refS`/`refN` fields plus its
/// click-time overrides (`createSubantaForms`). The site never declines the
/// clicked cell: it declines the row's *base* through one of these hardcoded
/// reference tables (`@deva1`, `@gacchat1`, …), so the engine mirrors that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KrtParadigm {
    /// @deva1 — a-stem masculine (`rAma`).
    DevaM,
    /// @phala1 — a-stem neuter (`Pala`).
    PhalaN,
    /// @mAlA1 — A-stem feminine (`mAlA`).
    MAlAF,
    /// @devI1 — I-stem feminine (`devI`).
    DevIF,
    /// @gati1 — ङित् i-stem feminine (`gati`).
    GatiF,
    /// @sAdhu1 — u-stem masculine (`sADu`).
    SAdhuM,
    /// @dhenu1 — u-stem feminine (`Denu`).
    DhenuF,
    /// @kaTu4 — u-stem neuter (`kaTu`).
    KaTuN,
    /// @zazin1 — in-stem masculine (`zazin`/दोहिन्).
    InM,
    /// @kSamin2 — in-stem neuter (क्षमिन्).
    InN,
    /// @gacchat1 — शतृ m with नुम् (`gacCat`).
    GacchatM,
    /// @dadat1 — अभ्यस्त शतृ m (`dadat`).
    DadatM,
    /// @pacat1 — शतृ n strong dual/plural (`pacat`).
    PacatN,
    /// @viyat1 — शतृ n weak dual (`viyat`, वत्-stems).
    ViyatN,
    /// @tudat1 — शतृ n both dual stems (`tudat`).
    TudatN,
    /// @dadat2 — अभ्यस्त शतृ n (`dadat`).
    DadatN,
    /// @bhavat1 — vant-stem masculine (`Bavat`).
    BhavatM,
    /// @phalavat2 — vant-stem neuter (`Palavat`).
    PhalavatN,
    /// @sedivas1 — vas-stem masculine, क्वसु (`sedivas`/ददिवस्).
    SedivasM,
    /// @vidvas2 — vas-stem neuter, क्वसु (`vidvas`).
    VidvasN,
    /// @dhAtR1 — f-stem masculine (`dAtf`).
    DhAtRM,
    /// @dhAtR2 — f-stem neuter (`dAtf`).
    DhAtRN,
    /// @svapnaj1/@ruj1 — j-stem masculine/feminine (pausa g/k).
    Jvs,
    /// @svapnaj2 — j-stem neuter.
    JvsN,
    /// @yajvan1 — van-stem masculine (`yajvan`).
    VanM,
    /// @sadman1 — man-stem neuter (`sadman`).
    ManN,
}

impl KrtParadigm {
    fn class(self) -> Class {
        match self {
            Self::DevaM => Class::AM,
            Self::PhalaN => Class::AN,
            Self::MAlAF => Class::AAF,
            Self::DevIF => Class::IIF,
            Self::GatiF => Class::GatiF,
            Self::SAdhuM => Class::UMF,
            Self::DhenuF => Class::UF,
            Self::KaTuN => Class::UN,
            Self::InM => Class::InM,
            Self::InN => Class::InN,
            Self::GacchatM => Class::AtM(AtMStyle::Num),
            Self::DadatM => Class::AtM(AtMStyle::Abhyasta),
            Self::PacatN => Class::AtN(AtNStyle::Num),
            Self::ViyatN | Self::PhalavatN => Class::AtN(AtNStyle::WeakDual),
            Self::TudatN => Class::AtN(AtNStyle::BothDual),
            Self::DadatN => Class::AtN(AtNStyle::Abhyasta),
            Self::BhavatM => Class::VantM,
            Self::SedivasM => Class::VasM,
            Self::VidvasN => Class::VasN,
            Self::DhAtRM => Class::RFM,
            Self::DhAtRN => Class::RFN,
            Self::Jvs => Class::Jvs,
            Self::JvsN => Class::JvsN,
            Self::VanM => Class::VanM,
            Self::ManN => Class::ManN,
        }
    }

    /// Declension stem from the clicked value. The site sends the row base
    /// for m/n and the f form when it ends आ/ई; tests may send cell surfaces.
    /// `None` when the value's shape cannot read as this paradigm (site
    /// quirks like अथुच्'s u-stem cells typed as `Bavat`).
    fn weak(self, value: &str) -> Option<String> {
        let v = value.trim();
        if v.is_empty() || v.contains('/') {
            return None;
        }
        let bare = v.strip_suffix('H').unwrap_or(v);
        let shaped = match self {
            Self::DevaM => bare.ends_with('a'),
            Self::PhalaN => v.strip_suffix('m').unwrap_or(v).ends_with('a'),
            Self::MAlAF => v.ends_with('A'),
            Self::DevIF => v.ends_with('I'),
            Self::GatiF => bare.ends_with('i'),
            Self::SAdhuM | Self::DhenuF | Self::KaTuN => bare.ends_with('u'),
            Self::GacchatM
            | Self::PacatN
            | Self::ViyatN
            | Self::TudatN
            | Self::DadatN
            | Self::PhalavatN => {
                bare.ends_with('t')
                    || bare.ends_with('d')
                    || bare.ends_with("an")
                    || bare.ends_with("An")
                    || bare.ends_with("ant")
                    || bare.ends_with("Ant")
            }
            Self::DadatM => bare.ends_with('t') || bare.ends_with('d'),
            Self::BhavatM => v.ends_with("vAn") || v.ends_with("vant") || v.ends_with("vat"),
            Self::SedivasM => v.ends_with('s') || v.ends_with("An"),
            Self::VidvasN => v.ends_with('s') || v.ends_with("at") || v.ends_with("ad"),
            Self::DhAtRM | Self::DhAtRN => v.ends_with('f') || v.ends_with('A'),
            Self::Jvs | Self::JvsN => v.ends_with('j') || v.ends_with('k') || v.ends_with('g'),
            Self::VanM => v.ends_with("an") || v.ends_with('A'),
            Self::ManN => v.ends_with("an") || v.ends_with('a'),
            Self::InM | Self::InN => v.ends_with("in") || v.ends_with('i') || v.ends_with('I'),
        };
        shaped.then(|| weak_for(&self.class(), v))
    }
}

/// Site कृत् cell slot: the reference paradigm, or known-but-absent (the
/// site renders no click for that linga, e.g. ल्युट् masculine).
enum KrtRefSlot {
    Paradigm(KrtParadigm),
    Absent,
}

/// Override lists from the site bundle (verbatim, including its `04.001`
/// spelling — the ids use 4-digit suffixes, so that entry never matches).
const SATF_ABHYASTA_IDS: &[&str] = &[
    "02.0066", "02.0067", "02.0068", "02.0069", "02.0070", "02.0071", "02.0072",
];
const SATF_TUDATI_IDS: &[&str] = &[
    "02.0046", "02.0047", "02.0048", "02.0049", "02.0050", "02.0051", "02.0052", "02.0053",
    "02.0054", "02.0055", "02.0056", "02.0057", "02.0067",
];
const AC_DEVI_IDS: &[&str] = &[
    "01.0056", "01.0791", "01.1112", "01.0640", "01.1087", "01.1124", "10.0001", "01.0736",
    "01.1085", "03.0018", "04.001", "06.0008", "10.0371", "10.0230", "10.0249",
];

/// Site reference paradigm for (family, kṛt, field), with the site's
/// `baseindex` (dhātu id) overrides applied. `None` = कृत् not in the site
/// table (no click at all); `Some(Absent)` = कृत् known, linga not declined.
#[must_use]
fn krt_ref_slot(family: &str, krt: &str, field: &str, baseindex: &str) -> Option<KrtRefSlot> {
    use KrtParadigm as P;
    // Site tab ids: `krut` = shuddha (the default कृदन्ते view), `yangluk_krut`
    // = yangluk; nich/san/yang take the entry refs without overrides.
    // (`forms::normalize_krut_engine_family` yields these ids; raw UI family
    // names are accepted too.)
    let is_krut = family == "krut" || family == "shuddha";
    let yangluk = family == "yangluk_krut" || family == "yangluk";
    let (p, s, n): (Option<P>, Option<P>, Option<P>) = match krt {
        // अव्ययम् (indeclinable): no click slots.
        "ktvA" | "lyap" | "tumun" | "Ramul" => (None, None, None),
        "lyuw" | "itra" => (None, None, Some(P::PhalaN)),
        "tfc" => (Some(P::DhAtRM), Some(P::DevIF), Some(P::DhAtRN)),
        // अतृन्: site refs @bhavat2/@devI1/@jagat1 — @bhavat2 is the
        // `त्`->`न्` strong (`jaran`) twin of @gacchat1.
        "atfn" => (Some(P::GacchatM), Some(P::DevIF), Some(P::ViyatN)),
        "ktavatu" => (Some(P::BhavatM), Some(P::DevIF), Some(P::PhalavatN)),
        "kvasu" => (Some(P::SedivasM), Some(P::DevIF), Some(P::VidvasN)),
        "najiN" => (Some(P::Jvs), Some(P::Jvs), Some(P::JvsN)),
        "GinuR" => (Some(P::InM), Some(P::DevIF), Some(P::InN)),
        "Nvanip" => (Some(P::VanM), Some(P::DevIF), Some(P::ManN)),
        "u" | "Aluc" | "Aru" | "knu" | "kru" | "gsnu" => {
            (Some(P::SAdhuM), Some(P::DhenuF), Some(P::KaTuN))
        }
        "aTuc" => (Some(P::BhavatM), None, None),
        "GaY" | "ap" | "varac" | "nan" | "naN" => (Some(P::DevaM), None, None),
        "ktin" => (None, Some(P::GatiF), None),
        "Takan" | "Rvuc" | "a" | "aN" => (None, Some(P::MAlAF), None),
        "zwran" | "zvun" => (None, Some(P::DevIF), None),
        "Ryuw" | "zAkan" => (Some(P::DevaM), Some(P::DevIF), Some(P::PhalaN)),
        "Satf" => {
            let m = if (is_krut
                && (baseindex.starts_with("03") || SATF_ABHYASTA_IDS.contains(&baseindex)))
                || yangluk
            {
                P::DadatM
            } else {
                P::GacchatM
            };
            let n = if is_krut {
                if baseindex.starts_with("01")
                    || baseindex.starts_with("04")
                    || baseindex.starts_with("10")
                {
                    P::PacatN
                } else if baseindex.starts_with("03") || SATF_ABHYASTA_IDS.contains(&baseindex) {
                    P::DadatN
                } else if baseindex.starts_with("06") || SATF_TUDATI_IDS.contains(&baseindex) {
                    P::TudatN
                } else {
                    P::ViyatN
                }
            } else {
                P::PacatN
            };
            (Some(m), Some(P::DevIF), Some(n))
        }
        "sya-Satf" => (Some(P::GacchatM), Some(P::DevIF), Some(P::TudatN)),
        "ac" => {
            let f = if is_krut && AC_DEVI_IDS.contains(&baseindex) {
                P::DevIF
            } else {
                P::MAlAF
            };
            (Some(P::DevaM), Some(f), Some(P::PhalaN))
        }
        // Default a/ā-stems: ण्वुल्, यत्, क्यप्, ण्यत्, तव्य, अनीयर्, क्त,
        // शानच् (+सya/भावकर्म), उकञ्, चानश्, शानन्, ण, र, श, क्त्रि, क्मरच्,
        // घुरच्, कुरच्, क्वरप्, क्लुकन्, क्रुकन्, वुञ्, वुन्, ल्यु.
        "Rvul"
        | "yat"
        | "kyap"
        | "Ryat"
        | "tavya"
        | "anIyar"
        | "kta"
        | "SAnac"
        | "sya-SAnac"
        | "BAvakarma-SAnac"
        | "sya-BAvakarma-SAnac"
        | "ukaY"
        | "cAnaS"
        | "SAnan"
        | "Ra"
        | "ra"
        | "Sa"
        | "ktri"
        | "kmarac"
        | "Gurac"
        | "kurac"
        | "kvarap"
        | "klukan"
        | "krukan"
        | "vuY"
        | "vun"
        | "lyu" => (Some(P::DevaM), Some(P::MAlAF), Some(P::PhalaN)),
        _ => return None,
    };
    Some(match field {
        "m" => p.map_or(KrtRefSlot::Absent, KrtRefSlot::Paradigm),
        "f" => s.map_or(KrtRefSlot::Absent, KrtRefSlot::Paradigm),
        "n" => n.map_or(KrtRefSlot::Absent, KrtRefSlot::Paradigm),
        _ => return None,
    })
}

/// Reference-paradigm class + declension stem for a कृत् cell, with the
/// site's value conventions (m/n: row base; f: f-form when it ends आ/ई).
#[must_use]
pub fn krt_class_ctx(
    family: &str,
    krt: &str,
    field: &str,
    value: &str,
    baseindex: &str,
) -> Option<(Class, String)> {
    match krt_ref_slot(family, krt, field, baseindex)? {
        KrtRefSlot::Paradigm(kind) => {
            let weak = kind.weak(value)?;
            Some((kind.class(), weak))
        }
        KrtRefSlot::Absent => None,
    }
}

/// Exact single table for a krt click-through (dhatu + krt + field known):
/// the site's reference-paradigm path first, the shape-based class as a
/// fallback for site quirks (अथुच् u-stems typed `Bavat`, क्वरप् ई-cells
/// typed `mAlA`). `None` for indeclinables / unknown shapes / lingas the
/// site does not decline for this कृत् (`not possible` in UI).
#[must_use]
pub fn krt_table(
    family: &str,
    krt: &str,
    field: &str,
    value: &str,
    baseindex: &str,
) -> Option<Paradigm> {
    let linga = Linga::parse(field)?;
    match krt_ref_slot(family, krt, field, baseindex) {
        Some(KrtRefSlot::Paradigm(kind)) => {
            if let Some(weak) = kind.weak(value) {
                if let Ok(tab) = decline(&weak, linga, &kind.class()) {
                    return Some(tab);
                }
            }
        }
        Some(KrtRefSlot::Absent) => return None,
        None => {}
    }
    let (class, weak) = krt_class(family, krt, field, value)?;
    decline(&weak, linga, &class).ok()
}

/// Weak/declension stem for a class from a citation stem.
#[must_use]
pub fn weak_for(class: &Class, stem: &str) -> String {
    match class {
        // Masculine a-stem `-aH` citation (nom. sg.): decline the bare stem.
        Class::AM if stem.ends_with("aH") => stem[..stem.len() - 1].to_string(),
        Class::AtM(_) if stem.ends_with("an") || stem.ends_with("An") => {
            format!("{}at", &stem[..stem.len() - 2])
        }
        Class::AtM(_) if stem.ends_with("ant") || stem.ends_with("Ant") => {
            format!("{}at", &stem[..stem.len() - 3])
        }
        Class::AtM(_) | Class::AtN(_) if stem.ends_with('d') => normalize_dental(stem),
        Class::AN if stem.ends_with("am") => stem[..stem.len() - 1].to_string(),
        Class::VasM if stem.ends_with("An") => {
            format!("{}as", &stem[..stem.len() - 2])
        }
        Class::VasN if stem.ends_with("at") || stem.ends_with("ad") => {
            format!("{}as", &stem[..stem.len() - 2])
        }
        Class::InM if stem.ends_with('I') => format!("{}in", &stem[..stem.len() - 1]),
        Class::InN if stem.ends_with('i') && !stem.ends_with("in") => format!("{stem}n"),
        Class::VanM if stem.ends_with('A') => format!("{}an", &stem[..stem.len() - 1]),
        Class::ManN if stem.ends_with('a') && !stem.ends_with("an") => format!("{stem}n"),
        Class::Jvs | Class::JvsN if stem.ends_with('k') || stem.ends_with('g') => {
            format!("{}j", &stem[..stem.len() - 1])
        }
        Class::RFM | Class::RFN if stem.ends_with('A') => {
            format!("{}f", &stem[..stem.len() - 1])
        }
        Class::VantM if stem.ends_with("vAn") || stem.ends_with("vant") => {
            format!("{}vat", &stem[..stem.len() - 3])
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
            Class::AtM(AtMStyle::Num)
        }
        ("m", _) if plain.ends_with('t') || plain.ends_with('d') => Class::AtM(AtMStyle::Num),
        ("n", _) if plain.ends_with('t') || plain.ends_with('d') => Class::AtN(AtNStyle::Num),
        ("m", _) if plain.ends_with("in") => Class::InM,
        ("n", _) if plain.ends_with("in") => Class::InN,
        ("m" | "f", _) if plain.ends_with('j') => Class::Jvs,
        ("n", _) if plain.ends_with('j') => Class::JvsN,
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
                // Feminine twins: `AnF`/`InF` are the `AnM`/`InM` shapes
                // with a feminine guard, so each masculine strip also gets
                // the feminine reading candidate (verification filters).
                let fem = match class {
                    Class::AnM => Some(Class::AnF),
                    Class::InM => Some(Class::InF),
                    _ => None,
                };
                for linga in [Linga::M, Linga::F, Linga::N] {
                    match fem.clone() {
                        Some(fe) if linga == Linga::F => push(stem.clone(), linga, fe),
                        _ => push(stem.clone(), linga, class.clone()),
                    }
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
                Class::AN
                | Class::IN
                | Class::UN
                | Class::AtN(_)
                | Class::AsN
                | Class::InN
                | Class::RFN
                | Class::VasN
                | Class::JvsN
                | Class::ManN => Linga::N,
                Class::AAF
                | Class::IIF
                | Class::IIFN
                | Class::UUF
                | Class::UUFN
                | Class::UF
                | Class::AnF
                | Class::InF
                | Class::GatiF => Linga::F,
                Class::Word(_) => continue,
                _ => Linga::M,
            };
            // Known words handled separately below with exact tables.
            push(probe.clone(), linga, class);
        }
    }
    // 2b. नदीसंज्ञक `-I`/`-U` base-word readings: the bare stem is no
    // declined cell of those paradigms (prathamA-ekavacana takes the
    // visarga), so state the citation reading directly — the word view's
    // premise ("this word as base word").
    for probe in &variants {
        if *probe != form {
            continue;
        }
        for class in classes_for(probe, Linga::F) {
            if !matches!(class, Class::IIFN | Class::UUFN) {
                continue;
            }
            let key = (probe.clone(), "f".to_string(), "citation".to_string());
            if seen.contains_key(&key) || decline(probe, Linga::F, &class).is_err() {
                continue;
            }
            seen.insert(key, true);
            out.push(Analysis {
                stem: probe.clone(),
                linga: Linga::F,
                class,
                slot: "citation".to_string(),
                refs: vec![],
            });
        }
    }
    // 2c. उस्/इस्-स्वादि neuter citations: the stem itself is not a
    // declined cell of that paradigm (the सि is root+विसर्ग), so state the
    // citation reading directly — the word view's premise ("this word as
    // base word").
    for probe in &variants {
        if *probe != form {
            continue;
        }
        for class in classes_for(probe, Linga::N) {
            if !matches!(class, Class::UsN) {
                continue;
            }
            let key = (probe.clone(), "n".to_string(), "citation".to_string());
            if seen.contains_key(&key) || decline(probe, Linga::N, &class).is_err() {
                continue;
            }
            seen.insert(key, true);
            out.push(Analysis {
                stem: probe.clone(),
                linga: Linga::N,
                class,
                slot: "citation".to_string(),
                refs: vec![],
            });
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
    (Class::AM, "yoH", ""),
    (Class::AM, "aH", "a"),
    (Class::AM, "AH", "a"),
    (Class::AM, "EH", "a"),
    (Class::AM, "am", "a"),
    (Class::AM, "O", "a"),
    (Class::AM, "At", "a"),
    (Class::AM, "Ad", "a"),
    (Class::AM, "ezu", "a"),
    (Class::AM, "e", "a"),
    (Class::AN, "AByAm", "a"),
    (Class::AN, "eByaH", "a"),
    (Class::AN, "asya", "a"),
    (Class::AN, "AnAm", "a"),
    (Class::AN, "Ani", "a"),
    (Class::AN, "ena", "a"),
    (Class::AN, "Aya", "a"),
    (Class::AN, "yoH", ""),
    (Class::AN, "At", "a"),
    (Class::AN, "Ad", "a"),
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
    (Class::AAF, "ayoH", "A"),
    (Class::AAF, "yoH", "A"),
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
    (Class::IMF, "yoH", "i"),
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
    (Class::IIF, "yoH", "I"),
    (Class::IIF, "yaH", "I"),
    (Class::IIF, "Im", "I"),
    (Class::IIF, "IH", "I"),
    (Class::IIF, "i", "I"),
    // नदीसंज्ञक `-I` stems differ from `I-f` only in prathamA-ekavacana
    // (the visarga); this one rule unlocks exactly that reading — all
    // other cells are shared and dedupe under `I-f`.
    (Class::IIFN, "IH", "I"),
    // u-stems.
    (Class::UMF, "uByAm", "u"),
    (Class::UMF, "uByaH", "u"),
    (Class::UMF, "uBiH", "u"),
    (Class::UMF, "URAm", "u"),
    (Class::UMF, "UnAm", "u"),
    (Class::UMF, "avaH", "u"),
    (Class::UMF, "ave", "u"),
    (Class::UMF, "unA", "u"),
    (Class::UMF, "voH", "u"),
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
    (Class::UN, "unoH", "u"),
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
    (Class::UUF, "voH", "U"),
    (Class::UUF, "vaH", "U"),
    (Class::UUF, "Um", "U"),
    (Class::UUF, "UH", "U"),
    (Class::UUF, "u", "U"),
    // नदीसंज्ञक `-U` stems differ from `U-f` only in prathamA-ekavacana.
    (Class::UUFN, "UH", "U"),
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
    // t-stems (शतृ m; site @gacchat1/@dadat1).
    (Class::AtM(AtMStyle::Num), "ByAm", "at"),
    (Class::AtM(AtMStyle::Num), "BiH", "at"),
    (Class::AtM(AtMStyle::Num), "dByAm", "t"),
    (Class::AtM(AtMStyle::Num), "dBiH", "t"),
    (Class::AtM(AtMStyle::Num), "antO", "at"),
    (Class::AtM(AtMStyle::Num), "antaH", "at"),
    (Class::AtM(AtMStyle::Num), "antam", "at"),
    (Class::AtM(AtMStyle::Num), "an", "at"),
    (Class::AtM(AtMStyle::Num), "AntO", "At"),
    (Class::AtM(AtMStyle::Num), "AntaH", "At"),
    (Class::AtM(AtMStyle::Num), "Antam", "At"),
    (Class::AtM(AtMStyle::Num), "An", "At"),
    (Class::AtM(AtMStyle::Num), "atA", "at"),
    (Class::AtM(AtMStyle::Num), "ate", "at"),
    (Class::AtM(AtMStyle::Num), "ataH", "at"),
    (Class::AtM(AtMStyle::Num), "atoH", "at"),
    (Class::AtM(AtMStyle::Num), "atAm", "at"),
    (Class::AtM(AtMStyle::Num), "ati", "at"),
    (Class::AtM(AtMStyle::Num), "atsu", "at"),
    (Class::AtM(AtMStyle::Num), "AtA", "At"),
    (Class::AtM(AtMStyle::Num), "Ate", "At"),
    (Class::AtM(AtMStyle::Num), "AtaH", "At"),
    (Class::AtM(AtMStyle::Num), "AtoH", "At"),
    (Class::AtM(AtMStyle::Num), "AtAm", "At"),
    (Class::AtM(AtMStyle::Num), "Ati", "At"),
    (Class::AtM(AtMStyle::Num), "Atsu", "At"),
    // अभ्यस्त शतृ (@dadat1: weak throughout; `dadatO`, `dadatam`, `dadate`).
    (Class::AtM(AtMStyle::Abhyasta), "atO", "at"),
    (Class::AtM(AtMStyle::Abhyasta), "atam", "at"),
    (Class::AtM(AtMStyle::Abhyasta), "ate", "at"),
    // t-stems neuter (शतृ n; @pacat1 strong dual/plural, @viyat1 weak dual,
    // @tudat1/@dadat2 sparse styles).
    (Class::AtN(AtNStyle::Num), "ByAm", "at"),
    (Class::AtN(AtNStyle::Num), "BiH", "at"),
    (Class::AtN(AtNStyle::Num), "dByAm", "t"),
    (Class::AtN(AtNStyle::Num), "dBiH", "t"),
    (Class::AtN(AtNStyle::Num), "antI", "at"),
    (Class::AtN(AtNStyle::Num), "anti", "at"),
    (Class::AtN(AtNStyle::Num), "ante", "at"),
    (Class::AtN(AtNStyle::Num), "Anti", "At"),
    (Class::AtN(AtNStyle::Num), "AtI", "At"),
    (Class::AtN(AtNStyle::Num), "atA", "at"),
    (Class::AtN(AtNStyle::Num), "AtA", "At"),
    (Class::AtN(AtNStyle::Num), "ate", "at"),
    (Class::AtN(AtNStyle::Num), "Ate", "At"),
    (Class::AtN(AtNStyle::Num), "ataH", "at"),
    (Class::AtN(AtNStyle::Num), "AtaH", "At"),
    (Class::AtN(AtNStyle::Num), "atoH", "at"),
    (Class::AtN(AtNStyle::Num), "AtoH", "At"),
    (Class::AtN(AtNStyle::Num), "atAm", "at"),
    (Class::AtN(AtNStyle::Num), "AtAm", "At"),
    (Class::AtN(AtNStyle::Num), "ati", "at"),
    (Class::AtN(AtNStyle::Num), "Ati", "At"),
    (Class::AtN(AtNStyle::Num), "atsu", "at"),
    (Class::AtN(AtNStyle::Num), "Atsu", "At"),
    (Class::AtN(AtNStyle::WeakDual), "atI", "at"),
    (Class::AtN(AtNStyle::Abhyasta), "ati", "at"),
    // n-stems.
    (Class::AnM, "AnO", "an"),
    (Class::AnM, "AnaH", "an"),
    (Class::AnM, "Anam", "an"),
    // The instr/dat/abl dual + plural and the locative plural attach to
    // `p` + short `a` (राजभ्याम्, आत्मभिः, उक्षसु — the long-ā forms belong
    // to the ā-stems).
    (Class::AnM, "aByAm", "an"),
    (Class::AnM, "aBiH", "an"),
    (Class::AnM, "aByaH", "an"),
    // Elided weak stems (उक्ष्णः): the ण्/न् is the stem's own, so the strip
    // keeps pre and appends `an` (retroflex inputs arrive via the R→n
    // variant pass; verification re-voices).
    (Class::AnM, "nA", "an"),
    (Class::AnM, "ne", "an"),
    (Class::AnM, "naH", "an"),
    (Class::AnM, "ni", "an"),
    (Class::AnM, "noH", "an"),
    (Class::AnM, "nAm", "an"),
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
    (Class::InM, "inoH", "in"),
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
    (Class::AsN, "asoH", "as"),
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
    (Class::AsM, "asoH", "as"),
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
    (Class::VasM, "uzA", "ivas"),
    (Class::VasM, "uze", "ivas"),
    (Class::VasM, "uzaH", "ivas"),
    (Class::VasM, "uzoH", "ivas"),
    (Class::VasM, "uzAm", "ivas"),
    (Class::VasM, "uzi", "ivas"),
    (Class::VasM, "vadByAm", "vas"),
    (Class::VasM, "vadBiH", "vas"),
    (Class::VasM, "vadByaH", "vas"),
    (Class::VasM, "vadsu", "vas"),
    (Class::VasM, "dByAm", "s"),
    (Class::VasM, "dsu", "s"),
    // उस्/इस्-स्वादि neuters (आयुष् @AyuS1). The citation is the root
    // plus its ष्/स्, and every ending but the visarga one carries the
    // षत्व (`zA`, `zI`, `ze`, `zaH`, `zoH`, `zAm`, `zi`, `zzu`) — so the
    // stem is the stripped prefix plus the citation letter. The rules are
    // root-vowel-specific (u-root → `uz`, i-root → `is`), so each form
    // reads back to exactly one citation: आयुष् for the u-roots, हविस्
    // for the i-roots. The र्-discard भ-endings (`urByAm`/`urBiH`/
    // `urByaH`) and the loc-pl pair (`uHzu`/`uzzu`) work the same way; the
    // visarga prathamā (`AyuH`) has no ष in it, so it needs its own rules.
    (Class::UsN, "uzI", "uz"),
    (Class::UsN, "uzA", "uz"),
    (Class::UsN, "uze", "uz"),
    (Class::UsN, "uzaH", "uz"),
    (Class::UsN, "uzoH", "uz"),
    (Class::UsN, "uzAm", "uz"),
    (Class::UsN, "uzi", "uz"),
    (Class::UsN, "uzzu", "uz"),
    (Class::UsN, "uHzu", "uz"),
    (Class::UsN, "urByAm", "uz"),
    (Class::UsN, "urBiH", "uz"),
    (Class::UsN, "urByaH", "uz"),
    (Class::UsN, "uH", "uz"),
    (Class::UsN, "izI", "is"),
    (Class::UsN, "izA", "is"),
    (Class::UsN, "ize", "is"),
    (Class::UsN, "izaH", "is"),
    (Class::UsN, "izoH", "is"),
    (Class::UsN, "izAm", "is"),
    (Class::UsN, "izi", "is"),
    (Class::UsN, "izzu", "is"),
    (Class::UsN, "iHzu", "is"),
    (Class::UsN, "irByAm", "is"),
    (Class::UsN, "irBiH", "is"),
    (Class::UsN, "irByaH", "is"),
    (Class::UsN, "iH", "is"),
    // Consonant-final nouns (सरित्-family). The strong cells keep the
    // input final, so the plain endings rebuild the stem as-is; the
    // प्रथमा-एकवचन pair and the भ-endings voice that final, and सु
    // devoices it — so those rules restore the final the stem actually
    // carries (`t`/`d`/`D`, `p`/`B`, `k`/`c`).
    (Class::Cons, "O", ""),
    (Class::Cons, "aH", ""),
    (Class::Cons, "am", ""),
    (Class::Cons, "A", ""),
    (Class::Cons, "e", ""),
    (Class::Cons, "oH", ""),
    (Class::Cons, "Am", ""),
    (Class::Cons, "i", ""),
    (Class::Cons, "d", "t"),
    (Class::Cons, "d", "d"),
    (Class::Cons, "d", "D"),
    (Class::Cons, "b", "p"),
    (Class::Cons, "b", "B"),
    (Class::Cons, "g", "k"),
    (Class::Cons, "g", "c"),
    (Class::Cons, "dByAm", "t"),
    (Class::Cons, "dByAm", "d"),
    (Class::Cons, "dByAm", "D"),
    (Class::Cons, "dBiH", "t"),
    (Class::Cons, "dBiH", "d"),
    (Class::Cons, "dBiH", "D"),
    (Class::Cons, "dByaH", "t"),
    (Class::Cons, "dByaH", "d"),
    (Class::Cons, "dByaH", "D"),
    (Class::Cons, "bByAm", "p"),
    (Class::Cons, "bByAm", "B"),
    (Class::Cons, "bBiH", "p"),
    (Class::Cons, "bBiH", "B"),
    (Class::Cons, "bByaH", "p"),
    (Class::Cons, "bByaH", "B"),
    (Class::Cons, "gByAm", "k"),
    (Class::Cons, "gByAm", "c"),
    (Class::Cons, "gBiH", "k"),
    (Class::Cons, "gBiH", "c"),
    (Class::Cons, "gByaH", "k"),
    (Class::Cons, "gByaH", "c"),
    (Class::Cons, "tsu", "t"),
    (Class::Cons, "tsu", "d"),
    (Class::Cons, "tsu", "D"),
    (Class::Cons, "psu", "p"),
    (Class::Cons, "psu", "B"),
    (Class::Cons, "kzu", "k"),
    (Class::Cons, "kzu", "c"),
    // तुप् nominalizations (वतुप्/मतुप्/यतुप्): the vṛddhi strong reads
    // back by swapping the suffix letter (आत्मवान् → आत्मवत्), the
    // `-ant` strong by the same swap (आत्मवन्तौ → आत्मवत्), and the
    // plain endings rebuild the stem as-is. The भ-endings keep the
    // द-voiced weak stem, so the suffix carries the `ad` and the tail
    // restores the `at` (आत्मवद्भ्याम् → आत्मवत्).
    (Class::VantM, "vAn", "vat"),
    (Class::VantM, "vantO", "vat"),
    (Class::VantM, "vantam", "vat"),
    (Class::VantM, "vantAH", "vat"),
    (Class::VantM, "van", "vat"),
    (Class::VantM, "mAn", "mat"),
    (Class::VantM, "mantO", "mat"),
    (Class::VantM, "mantam", "mat"),
    (Class::VantM, "mantAH", "mat"),
    (Class::VantM, "man", "mat"),
    (Class::VantM, "yAn", "yat"),
    (Class::VantM, "yantO", "yat"),
    (Class::VantM, "yantam", "yat"),
    (Class::VantM, "yantAH", "yat"),
    (Class::VantM, "yan", "yat"),
    (Class::VantM, "adByAm", "at"),
    (Class::VantM, "adBiH", "at"),
    (Class::VantM, "adByaH", "at"),
    (Class::VantM, "aH", ""),
    (Class::VantM, "A", ""),
    (Class::VantM, "e", ""),
    (Class::VantM, "oH", ""),
    (Class::VantM, "Am", ""),
    (Class::VantM, "i", ""),
    (Class::VantM, "su", ""),
    // क्वसु neuter (@vidvas2: `-vat`/`-vad`, यण् weak, `-vAMsi`).
    (Class::VasN, "AMsi", "as"),
    (Class::VasN, "vat", "vas"),
    (Class::VasN, "vad", "vas"),
    (Class::VasN, "yuzA", "ivas"),
    (Class::VasN, "yuze", "ivas"),
    (Class::VasN, "yuzaH", "ivas"),
    (Class::VasN, "yuzoH", "ivas"),
    (Class::VasN, "yuzAm", "ivas"),
    (Class::VasN, "yuzi", "ivas"),
    (Class::VasN, "yuzI", "ivas"),
    (Class::VasN, "uzA", "vas"),
    (Class::VasN, "uze", "vas"),
    (Class::VasN, "uzaH", "vas"),
    (Class::VasN, "uzoH", "vas"),
    (Class::VasN, "uzAm", "vas"),
    (Class::VasN, "uzi", "vas"),
    (Class::VasN, "uzI", "vas"),
    (Class::VasN, "vadByAm", "vas"),
    (Class::VasN, "vadBiH", "vas"),
    (Class::VasN, "vadByaH", "vas"),
    (Class::VasN, "vatsu", "vas"),
    // ऋ-stem neuter (तृच् n; @dhAtR2) — shared forms also surface through
    // the RFM rules above; these add the neuter readings.
    (Class::RFN, "fRI", "f"),
    (Class::RFN, "FRI", "f"),
    (Class::RFN, "fRA", "f"),
    (Class::RFN, "rA", "f"),
    (Class::RFN, "fRe", "f"),
    (Class::RFN, "re", "f"),
    (Class::RFN, "fRaH", "f"),
    (Class::RFN, "uH", "f"),
    (Class::RFN, "fRoH", "f"),
    (Class::RFN, "roH", "f"),
    (Class::RFN, "FRAm", "f"),
    (Class::RFN, "fRi", "f"),
    (Class::RFN, "ari", "f"),
    (Class::RFN, "fzu", "f"),
    // ङित् i-stem feminine (क्तिन्; @gati1) — shared `iByAm`/`inA`/… forms
    // are covered by the IMF rules above.
    (Class::GatiF, "IH", "i"),
    (Class::GatiF, "yA", "i"),
    (Class::GatiF, "yE", "i"),
    (Class::GatiF, "yAH", "i"),
    (Class::GatiF, "yAm", "i"),
    // in-stem neuter (घिनुण् n; @kSamin2).
    (Class::InN, "iRI", "in"),
    (Class::InN, "IRi", "in"),
    (Class::InN, "iRA", "in"),
    (Class::InN, "iRe", "in"),
    (Class::InN, "iRaH", "in"),
    (Class::InN, "iRoH", "in"),
    (Class::InN, "IRAm", "in"),
    (Class::InN, "iRi", "in"),
    (Class::InN, "izu", "in"),
    // j-stems (नजिङ्; @svapnaj1/@ruj1/@svapnaj2 — पausa `-g`/`-k`).
    (Class::Jvs, "ak", "aj"),
    (Class::Jvs, "ag", "aj"),
    (Class::Jvs, "ajO", "aj"),
    (Class::Jvs, "ajaH", "aj"),
    (Class::Jvs, "ajam", "aj"),
    (Class::Jvs, "ajA", "aj"),
    (Class::Jvs, "agByAm", "aj"),
    (Class::Jvs, "agBiH", "aj"),
    (Class::Jvs, "aje", "aj"),
    (Class::Jvs, "agByaH", "aj"),
    (Class::Jvs, "ajoH", "aj"),
    (Class::Jvs, "ajAm", "aj"),
    (Class::Jvs, "aji", "aj"),
    (Class::Jvs, "akzu", "aj"),
    (Class::JvsN, "ak", "aj"),
    (Class::JvsN, "ag", "aj"),
    (Class::JvsN, "ajI", "aj"),
    (Class::JvsN, "Yji", "aj"),
    (Class::JvsN, "ajA", "aj"),
    (Class::JvsN, "agByAm", "aj"),
    (Class::JvsN, "agBiH", "aj"),
    (Class::JvsN, "aje", "aj"),
    (Class::JvsN, "agByaH", "aj"),
    (Class::JvsN, "ajoH", "aj"),
    (Class::JvsN, "ajAm", "aj"),
    (Class::JvsN, "aji", "aj"),
    (Class::JvsN, "akzu", "aj"),
    // van-stem masculine (ङ्वनिप्; @yajvan1).
    (Class::VanM, "A", "an"),
    (Class::VanM, "nA", "n"),
    (Class::VanM, "ne", "n"),
    (Class::VanM, "naH", "n"),
    (Class::VanM, "noH", "n"),
    (Class::VanM, "nAm", "n"),
    (Class::VanM, "ni", "n"),
    (Class::VanM, "su", "n"),
    // man-stem neuter (ङ्वनिप्; @sadman1).
    (Class::ManN, "nI", "n"),
    (Class::ManN, "Ani", "an"),
    (Class::ManN, "nA", "n"),
    (Class::ManN, "ne", "n"),
    (Class::ManN, "naH", "n"),
    (Class::ManN, "noH", "n"),
    (Class::ManN, "nAm", "n"),
    (Class::ManN, "ni", "n"),
    (Class::ManN, "su", "n"),
    // u-stem feminine ङित् (उ f; @dhenu1) — guna/यण् alternations.
    (Class::UF, "vA", "u"),
    (Class::UF, "vE", "u"),
    (Class::UF, "vAH", "u"),
    (Class::UF, "voH", "u"),
    (Class::UF, "vAm", "u"),
    (Class::UF, "vO", "u"),
];
