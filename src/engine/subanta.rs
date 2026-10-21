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

/// The optional-form flavours of the a-stem masculine: the site lists
/// the वृद्धि-forms beside the plain ones in some slots, and the words
/// differ in which slots they appear in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AOpt {
    /// The ए-form as an alternant in the nominative plural (अर्धाः-अर्धे;
    /// site @ardha1).
    E,
    /// The same plus the ए + the ण in the instrumental (चरमेण; site
    /// @carama1).
    EN,
    /// The same plus the स्म-forms in the cat/paJ/sap (अपरस्मै; site
    /// @apara1).
    ENSm,
    /// The स्म-forms with the plain instrumental (नेमस्मै, नेमेन; site
    /// @nema1).
    Sm,
    /// The स्म-forms with the ए-pair (दक्षिण; site @dakSiNa1): the
    /// nominative plural keeps the आ- and ए-forms (दक्षिणाः-दक्षिणे), the
    /// cat the स्म-form, the ablative the four alternants and the locative
    /// the ए- and स्म-forms.
    SmE,
    /// The स्म-forms with the ए + the ण and without the nominative pair
    /// (एकतर; site @ekatara1): the ablative carries only the two
    /// स्म-alternants and the locative the स्म-form.
    SmEN,
    /// The स्म-alternant beside the plain form (द्वितीय; site
    /// @dvitIya1): the cat (द्वितीयाय-द्वितीयस्मै), the four-way
    /// ablative (द्वितीयाद्-द्वितीयस्माद्-द्वितीयस्मात्-द्वितीयात्)
    /// and the locative (द्वितीयये-द्वितीयस्मिन्).
    SmPairs,
    /// The comparative flavour with the plain instrumental (एकतम; site
    /// @ekatama1): as `Comp`, but the instrumental keeps the plain form
    /// (एकतमेन).
    CompS,
    /// The comparative flavour (अन्यतर; site @anyatara1): the स्म-forms
    /// (अन्यतरस्मै, अन्यतरस्माद्-अन्यतरस्मात्, अन्यतरस्मिन्), the
    /// ए + the ण instrumental (अन्यतरेण), the ए-form in the genitive
    /// plural and the nominative and vocative plurals (अन्यतरे).
    Comp,
}

impl AOpt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::E => "e",
            Self::EN => "en",
            Self::ENSm => "en-sm",
            Self::Sm => "sm",
            Self::SmE => "sm-e",
            Self::SmEN => "sm-en",
            Self::SmPairs => "sm-pairs",
            Self::Comp => "comp",
            Self::CompS => "comps",
        }
    }
}

/// The flavours of the compounded pronouns (अतियुष्मद्, अत्यस्मद्,
/// परमयुष्मद्, परमास्मद्): the site grids the pronoun's own paradigm
/// with the base in front of it, in three flavours per pronoun plus the
/// पुरुष compounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PronFlavour {
    /// अतियुष्मद् — the त्व-forms (अतित्वम्, अतित्वाम्).
    YuzTv,
    /// अतियुष्मद् — the युव-forms (अतियुवाम्).
    YuzYuv,
    /// अतियुष्मद् — the युष्म-forms (अतियुष्माम्).
    YuzYuzm,
    /// अत्यस्मद् — the म-forms (अतिमाम्).
    AsmMa,
    /// अत्यस्मद् — the आव-forms (अत्यावाम्).
    AsmAv,
    /// अत्यस्मद् — the स्म-forms (अत्यस्माम्).
    AsmSm,
    /// परमयुष्मद् — the pronoun's own grid, the prefix on every
    /// alternative (परमत्वाम्-परमत्वा).
    YuzPara,
    /// परमास्मद् — the pronoun's own grid, the prefix on every
    /// alternative (परममाम्-परममा).
    AsmPara,
}

/// The strong-cell flavours of the masculine ई-स्वादि stems: the site
/// writes the गुण-forms, the वृद्धि or a mixture word by word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IIM2Opt {
    /// The ग्य-forms in the accusative and the locative (अग्रण्यम्,
    /// अग्रण्याम्; site @agraNI1).
    Guna,
    /// The same, with the ि-locative (जलप्यि; site @jalapI1).
    GunaI,
    /// The same, with the उ-augment in the ablative and genitive singular
    /// (क्षाम्युः; site @kSAmI1).
    GunaU,
    /// The वृद्धि: the िय-forms in the strong cells (दुर्धियौ, दुर्धियि;
    /// site @durdhI2).
    Vridhi,
    /// The गुण-forms in the strong cells with the आ-forms beside them
    /// (दुर्ध्यै, दुर्ध्याः, दुर्धीनाम्; site @durdhI1): the dative
    /// singular takes the ऐ-form, the ablative and genitive singular the
    /// आ-form, the genitive plural the ई-form and the vocative the
    /// ि-form.
    GunaA,
    /// The वृद्धि with the आ-form in the locative singular (नियाम्;
    /// site @nI1).
    VridhiA,
    /// The गुण-forms in the strong cells with the bare stem in the
    /// nominative (कुमारी, सखी; site @kumArI1/@sakhI3): the nominative
    /// singular is the stem itself (कुमारी), the dative singular the
    /// ऐ-form (कुमार्यै), the ablative and genitive singular the
    /// आ-form (कुमार्याः) and the vocative the ि-form (हे कुमारि).
    GunaB,
}

/// The strong-cell flavours of the masculine ऊ-स्वादि stems: the site
/// writes the व्व-forms, with or without the plain accusative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UUM2Opt {
    /// The व्व-forms everywhere (उल्ल्वम्, उल्ल्वः; site @ullU1).
    Vva,
    /// The व्व-forms, but the accusative keeps the ऊ-forms
    /// (अग्रेदिधिषूम्, अग्रेदिधिषून्; site @agredidhiSU1).
    VvaU,
}

impl IIM2Opt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Guna => "guna",
            Self::GunaI => "gunai",
            Self::GunaU => "gunau",
            Self::Vridhi => "vridhi",
            Self::GunaA => "guna-a",
            Self::VridhiA => "vridhi-a",
            Self::GunaB => "guna-b",
        }
    }
}

impl UUM2Opt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Vva => "vva",
            Self::VvaU => "vvau",
        }
    }
}

/// The flavours of the feminine a-stems: the site writes the स्य-augment
/// pairs (the दिग्-स्वादि and the ordinals) and the bare vocative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AAFSOpt {
    /// The स्य-pairs in the dative, ablative, genitive and locative
    /// singular (उत्तरपश्चिमायै-उत्तरपश्चिमस्यै; site @uttarapazcimA1).
    Sy,
    /// The same, with the genitive plural (उत्तरपश्चिमानाम्-
    /// उत्तरपश्चिमासाम्).
    SyGen,
    /// The bare vocative (हे अक्क; site @akkA1).
    Bare,
}

impl AAFSOpt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Sy => "sy",
            Self::SyGen => "sygen",
            Self::Bare => "bare",
        }
    }
}

/// The flavours of the नदीसंज्ञक feminine ई- and ऊ-स्वादि stems: the
/// site writes the िय-/उव-forms with the alternant pairs in some
/// cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NV2Opt {
    /// The pairs in the accusative singular and plural (कुलस्त्रियम्-
    /// कुलस्त्रीम्; site @kulastrI1).
    Eka,
    /// The pairs in the dative, ablative, genitive and locative
    /// (जयश्रिये-जयश्रियै; site @jayazrI1).
    Strong,
    /// The plain forms (पर्यन्तभुवौ; site @paryantabhU1).
    Solo,
    /// The `Strong` pairs with the dental genitive plural alternant
    /// (धियाम्-धीनाम्; site @dhI1).
    StrongN,
    /// The व्व-forms of the feminine ऊ-stems (खलप्वम्; site @khalapU2).
    Vva,
}

impl NV2Opt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Eka => "eka",
            Self::Strong => "strong",
            Self::Solo => "solo",
            Self::StrongN => "strongn",
            Self::Vva => "vva",
        }
    }
}

/// The optional-form flavours of the neuter a-stem: the site's grids
/// differ in the nominative, the ablative and the instrumental.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ANOpt {
    /// The अपर-family flavour (अधर; site @adhara2): the plain nominative
    /// (अधरम्), the ए + the ण instrumental (अधरेण), the four-way
    /// ablative (अधराद्-अधरस्माद्-अधरस्मात्-अधरात्) and the ए- and
    /// स्म-locative (अधरे-अधरस्मिन्).
    Apara,
    /// The एकतर flavour (एकतर; site @ekatara2): as `Apara`, but the
    /// ablative carries only the two स्म-alternants (एकतरस्माद्-एकतरस्मात्)
    /// and the locative the स्म-form alone (एकतरस्मिन्).
    Tara,
    /// The स्म-alternant beside the plain form (द्वितीय; site @dvitIya2):
    /// the cat (द्वितीयाय-द्वितीयस्मै), the four-way ablative and the
    /// locative (द्वितीयये-द्वितीयस्मिन्).
    SmPairs,
    /// The comparative flavour with the plain instrumental (एकतम; site
    /// @ekatama2): as `Comp`, but the instrumental keeps the plain form
    /// (एकतमेन).
    CompS,
    /// The comparative flavour (अन्यतर; site @anyatara2): as the
    /// masculine, with the द/त pair in the nominative, accusative and
    /// vocative singular (अन्यतरद्-अन्यतरत्).
    Comp,
}

impl PronFlavour {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::YuzTv => "tv",
            Self::YuzYuv => "yuv",
            Self::YuzYuzm => "yuzm",
            Self::AsmMa => "ma",
            Self::AsmAv => "av",
            Self::AsmSm => "sm",
            Self::YuzPara => "yuzpara",
            Self::AsmPara => "asmpara",
        }
    }
}

impl ANOpt {
    /// The class label of this flavour.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Apara => "a",
            Self::Tara => "t",
            Self::SmPairs => "sm-pairs",
            Self::Comp => "comp",
            Self::CompS => "comps",
        }
    }
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
    /// `a`-stem masculine with the site's optional वृद्धि-forms (अर्ध;
    /// site @ardha1): the nominative plural carries अर्धाः-अर्धे.
    AMo(AOpt),
    /// The neuter of the वतुप् stems (अहङ्कारवत्; site @ahaGkAravat2):
    /// the द/त pair (अहङ्कारवद्-अहङ्कारवत्).
    VantN,
    /// `a`-stem neuter (`vana`).
    AN,
    /// The neuter a-stem with the site's optional वृद्धि-forms (अधर; site
    /// @adhara2): the nominative may be a द/त pair.
    ANo(ANOpt),
    /// `A`-stem neuter (श्रीपा; site @zrIpA1): the णि-form plural
    /// (श्रीपाणि).
    AAN,
    /// `A`-stem masculine (कीलालपा; site @kIlAlapA1): the visarga
    /// nominative (कीलालपाः).
    AAM,
    /// `c`-stem masculine of the pair flavour (जलमुच्; site @jalamuc1):
    /// the ग/क pair (जलमुग्-जलमुक्) — `q` for the ड/ट pair (सुवृड्-सुवृट्).
    AcM2(char),
    /// `O`-stem masculine (जनौ; site @janau1): the औ-form nominative
    /// (जनौः) and the व-form in the dual/plural (जनावौ).
    AuM,
    /// `A`-stem feminine (`ramA`).
    AAF,
    /// The नदीसंज्ञक feminine ई- and ऊ-स्वादि stems in the िय-/उव-
    /// flavours (धी, धू; site @dhI1/@dhU1).
    NV2(NV2Opt),
    /// The feminine a-stems with the स्य-augment pairs (उत्तरपश्चिमा;
    /// site @uttarapazcimA1) or the bare vocative (हे अक्क).
    AAFS(AAFSOpt),
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
    /// `I`-stem masculine (`amI`, `agraNI`; site @amI1): the nominative
    /// takes the visarga (`amIaH`) and the y-forms carry the accusative
    /// and the other case endings (`amya`, `amyeH`, `amyaH`).
    IIM,
    /// `I`-stem masculine with the गुण-forms in the strong cells
    /// (`agraNI`; site @agraNI1): the accusative and the locative
    /// singular take the ग्य-forms (`agraNyam`, `agraNyAm`).
    IIM2(IIM2Opt),
    /// `u`-stem masculine/feminine (`guru`).
    UMF,
    /// `u`-stem feminine with ङित् endings (`Denu`): `DenvA`, `Denave`/
    /// `DenvE` (the ashtadhyayi.com उ-f paradigm).
    UF,
    /// `u`-stem neuter (`maDu`).
    UN,
    /// `u`-stem neuter without the गुण/व् alternants (मधु; site
    /// @madhu1): the plain forms only.
    UNp,
    /// `i`-stem neuter with the य-forms as alternants (अनादि; site
    /// @anAdi1): अनादिने-अनादये.
    INy,
    /// `i`-stem neuter with the ऋ-form in the genitive dual (भूरि; site
    /// @bhUri1): भूरिणोः-भूर्योः.
    INr,
    /// `U`-stem feminine कृदन्त/उगित् flavour (`vaDU`): prathamA-ekavacana
    /// is the bare stem.
    UUF,
    /// `U`-stem feminine lexical नदीसंज्ञक flavour (a `vaDU` शब्द):
    /// prathamA-ekavacana takes the visarga (`vaDUH`); every other cell
    /// equals [`Class::UUF`].
    UUFN,
    /// `U`-stem masculine (`agnibhU`; site @agnibhU1): the nominative takes
    /// the visarga (`agnibhUaH`) and the v-forms carry the endings
    /// (`agnibhuvA`, `agnibhuvE`, `agnibhuvAH`).
    UUM,
    /// `U`-stem masculine with the व्-forms in the strong cells (`ullU`;
    /// site @ullU1): the व्व-forms (`ullvaO`, `ullvam`).
    UUM2(UUM2Opt),
    /// The neuter of a त-स्वादि stem (आत्मवत्, आयुष्मत्; site
    /// @Atmavat2): the प्रथमा-/द्वितीया-एकवचन carry the द/त pair
    /// (आत्मवद्-आत्मवत्), the rest the ई-forms (आत्मवती, आत्मवन्ति).
    TaN,
    /// The neuter of a त-स्वादि stem in the site's mixed flavours (तुदत्;
    /// site @tudat1/@dadat2/@mahat2): `D` the dual carries the न्ती-pair
    /// (तुदन्ती-तुदती), `P` the plural the न्ति-pair (ददन्ति-ददति), `A`
    /// the plural the आ-form (महान्ति).
    TaN2(char),
    /// Masculine च्-final nouns (अर्वाच्; site @arvAc2): the ङ्-inserted
    /// nominative (अर्वाङ्) and the ञ्च-forms (अर्वाञ्चौ, अर्वाञ्चः).
    AccM,
    /// Masculine च्-final nouns of the ञ्च-स्वादि family whose weak
    /// stem is the वृद्धि-form (उदच्; site @udac1): the strong cells
    /// keep the ञ्च-forms (उदञ्चौ), the weak cells the वृद्धि-form
    /// (उदीचः) — the flavour picks the consonant the वृद्धि-form ends
    /// in (`I` ईच, `U` ऊच, `S` श्च).
    AccMV(char),
    /// Masculine ज्-final nouns (खन्ज्; site @khanj1): the bare-stem
    /// nominative (खन्) and the ञ्ज-forms (खञ्जौ, खञ्जः).
    AjjM,
    /// The व-स्वादि feminine u-stems (अभ्रमु; site @abhramuh1): the
    /// व-forms (अभ्रम्वा) and the paired alternants (अभ्रमवे-अभ्रम्वै).
    UvF,
    /// The व-स्वादि feminine u-stems with the dental genitive plural
    /// (अलावूनाम्; site @alAvu1).
    UvFd,
    /// The neuter u-stems (अगुरु; site @aguru2): the ण-forms in the
    /// cat/paJ/zaz/sap (अगुरुणे, अगुरूणाम्).
    UnN,
    /// The अ-स्वादि masculine/feminine (गुप्, अप्; site @gup1/@ap1):
    /// the masculine nominative carries the जश् pause pair (गुब्-गुप्),
    /// the feminine the आ-form (आपः).
    ApM,
    /// The ओ-स्वादि compound plurals (आगस्त्य → अगस्तयः; site @Agastya1
    /// and its kin): the plural takes the compound's reduced stem with
    /// the endings of the a-stem masculine, the इ-स्वादि masculine or the
    /// उ-स्वादि masculine.
    OPlur(&'static str, char),
    /// One of the site-verbatim grids (`SITE_GRIDS`): the irregular words
    /// the site grids in full — the ᳶ-स्वादि roots, the द्यो/दिव्
    /// compounds, the diphthong stems and the कृत्-स्वादि participles.
    SiteGrid(usize),
    /// The neuter i-stems in the ण-form (अतिस्त्रि; site @atistri3): the
    /// dvi/plural take the ण-forms (अतिस्त्रिणी) and the cat/paJ/sap carry
    /// both alternants (अतिस्त्रिणे-अतिस्त्रये).
    INu,
    /// The neuter i-stems with the ण-forms in the oblique cases (अक्षि;
    /// site @akSi1): the instrumental, dative, ablative, genitive and
    /// locative take the ण-form (अक्ष्णा, अस्थ्ना) and the locative
    /// singular and the vocative carry both alternants (अक्ष्णि-अक्षणि,
    /// हे अक्षि-हे अक्षे).
    INv,
    /// The masculine ह्-final nouns of the ड/ट flavour (लिह्; site @lih1):
    /// the ड/ट pair in the प्रथमा-एकवचन (लिड्-लिट्).
    HasM,
    /// The feminine ह्-final nouns of the द/त flavour (उपानह्; site
    /// @upAnah1): the द/त pair (उपानद्-उपानत्).
    HasF,
    /// The feminine र्-final nouns of the vowel flavours (गिर्; site
    /// @gir1): the ई-form in the nominative (गीः) — `i` for the ई and
    /// `u` for the ऊ.
    RirF(char),
    /// The feminine र्-final nouns of the plain flavour (द्वार्; site
    /// @dvAr1): the आ-form in the nominative (द्वाः).
    RarF,
    /// The masculine nouns whose nominative is the bare stem (कमल्; site
    /// @kamal1): `l` for the ल्-final (कमल्) and `R` for the ण्-final
    /// (सुगण्).
    BaraM(char),
    /// The neuter र्-final nouns (वार्; site @vAr1): the ई-form dual
    /// (वारी).
    RarN,
    /// The neuter ह्-final nouns (स्वनडुह्; site @svanaDuh1): the द/त
    /// pair (स्वनडुद्-स्वनडुत्) and the ई-form dual.
    HasN,
    /// The feminine ह्-final nouns of the ग/क flavour (उष्णिह्; site
    /// @uSNih1): the ग/क pair (उष्णिग्-उष्णिक्).
    HihF,
    /// The feminine ह्-final nouns of the धुग्/धुक् flavour (कामदुह्; site
    /// @kAmaduh1): the धुग्-धुक् pair.
    DuhF,
    /// The masculine ह्-final nouns of the four-alternant flavour (मुह्;
    /// site @muh1): the ड/ग/क/ट quartet (मुड्-मुग्-मुक्-मुट्).
    HuhM,
    /// The masculine डु-वृद्धि ह्-final nouns (अनडुह्; site @anaDuh1): the
    /// वृद्धि nominative (अनड्वान्).
    HasVM,
    /// The masculine औ-स्वादि ह्-final nouns (विश्ववाह्; site
    /// @vizvavAh1): the औ-form in the plural slots (विश्वौहः).
    HasAM,
    /// The masculine ह्-final nouns of the two-alternant flavour (दुह्;
    /// site @duh1): the ग/क pair (धुग्-धुक्).
    DuhM,
    /// The masculine ष्-final nouns (द्विष्; site @dviS1): the ड/ट pair in
    /// the प्रथमा-एकवचन (द्विड्-द्विट्) and the ट्सु locative plural.
    Zam,
    /// The masculine ष्-final nouns of the visarga flavour (चिकीर्ष्;
    /// site @cikIrS1): the nominative is the visarga form (चिकीः), the
    /// भ-endings the र-form (चिकीर्भ्याम्) — the flavour picks the
    /// nominative (`C` plain, `V` the वृद्धि-form, `R` the रुत्व-dropped
    /// form) and with it the locative plural.
    ZamC(char),
    /// The neuter ऋ-chart nouns (दृश्; site @dRz3): the ई-form dual
    /// (दृशी) and the ंशि-form plural (दृंशि).
    RcN,
    /// The neuter द्-final nouns (उद्भिद्; site @udbhid2): the द/त pair
    /// (उद्भिद्-उद्भित्) and the न्दि-form plural (उद्भिन्दि) — `true`
    /// for the न्दि-form, `false` for the plain one (बेभिदि).
    DadN(bool),
    /// The ऋ-chart nouns with two alternants (दिश्; site @diz1): the ग/क
    /// pair (दिग्-दिक्).
    Rc2,
    /// The श्-final nouns of the ड/ट flavour (विश्; site @viz1): the
    /// nominative carries विड्-विट्.
    Sam,
    /// The feminine ष्-final nouns of the ड/ट flavour (तृष्; site @tRS1):
    /// the ड/ट pair (तृड्-तृट्).
    ZamF,
    /// The feminine ष्-final nouns of the vowel flavours (छदिष्; site
    /// @chadiS1): the ष् becomes the stem's own vowel — `i` (छदिः), `I`
    /// (आशीः) or `u` (सजूः).
    ZvF(char),
    /// The masculine ष्-final nouns of the ग/क flavour (तक्ष्; site
    /// @takS2): the ग्/क् pair (तग्-तक्) and the क्षु locative plural.
    ZagM,
    /// The neuter च्-final nouns (अर्वाच्; site @arvAc1): the ग/क pair in
    /// the प्रथमा-एकवचन and the ई-forms (अर्वाची, अर्वाञ्चि).
    AcN,
    /// The neuter च्-final nouns whose weak stem is the वृद्धि-form
    /// (न्यच् → नीची; site @nyac2) or the ओ-form (गवाच् → गोची; site
    /// @gavAc1) — the flavour picks the weak stem (`I` ईच, `U` ऊच,
    /// `S` श्च, `O` the ओ-form).
    AcNV(char),
    /// The neuter न्च-final nouns of the ङ्-inserted family (गोन्च्;
    /// site @gonc1): the nominative takes the ङ्-inserted form (गोङ्),
    /// the dual and plural the ञ्च-forms (गोञ्ची, गोञ्चि) and the
    /// locative plural the ङ्-क्षु pair (गोङ्षु-गोङ्क्षु-गोङ्ख्षु).
    AncN,
    /// The feminine अस्-स्वादि (सदस् → सदाः; site @sadas2): the same
    /// shape as the masculine, with the आ-form in the nominative.
    AsF,
    /// The अस्-स्वादि masculine of the plain flavour (पिण्डग्रस्; site
    /// @piNDagras1): the nominative is the visarga form (पिण्डग्रः)
    /// where the आ-flavour writes the आ-form (पिण्डग्राः).
    AsMV,
    /// The अस्-स्वादि masculine of the pause flavour (ध्वस्; site
    /// @dhvas1): the nominative carries the द्/त् pair (ध्वद्-ध्वत्),
    /// the भ-endings the द-form (ध्वद्भ्याम्) and the locative plural
    /// the त्सु (ध्वत्सु).
    AsMP,
    /// The अस्-स्वादि masculine of the वृद्धि flavour (सुतुस्; site
    /// @sutus1): the nominative takes the वृद्धि-form (सुतूः), the
    /// भ-endings the वृद्धि + the र् (सुतूर्भ्याम्).
    AsMW,
    /// The स्-final masculine nouns of the visarga flavour (ओकास्; site
    /// @okAs1): the nominative is the visarga form (ओकाः), the भ-endings
    /// take the stem without the ओ-augment (ओकाभ्याम्) and the locative
    /// plural both alternants (ओकाःसु-ओकास्सु).
    SasV,
    /// The anusvāra-स्वादि (अणीयस्; site @aNIyas1): the nominal forms
    /// take the anusvāra (अणीयान्, अणीयांसौ), the rest the स-forms.
    AsAnM,
    /// The e-form of the अस्-स्वादि (अनेहस्; site @anehas1): the
    /// nominative is the bare stem (अनेहा).
    AsEM,
    /// The स्म-ending a-stem masculine (अनेक; site @aneka1): the
    /// nominative plural takes the ए-form, the cat/paJ/sap the स्म-forms.
    AsmM,
    /// The स्म-ending a-stem neuter (अनेक; site @aneka2).
    AsmN,
    /// The स्म-ending a-stem feminine (अधरा; site @adharA1): the cat/paJ
    /// take the स-form (अधरस्यै, अधरस्याः).
    AsmF,
    /// The ऋ-स्वादि masculine/feminine with the आ-form in the nominative
    /// (माता, मातरौ; site @mAtR1/@pitR1).
    RFA,
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
    /// The masculine at-participles with the द/त pair (चकासत्; site
    /// @cakAsat1): the nominative carries चकासद्-चकासत्.
    AtM2,
    /// `t`-stem neuter (`Bavat`/`Bavad`, `viyat`/`tudat` styles).
    AtN(AtNStyle),
    /// `an`-stem masculine (`rAjan`).
    AnM,
    /// `in`-stem masculine (`balin`).
    InM,
    /// The इ-स्वादि masculine with the वृद्धि-सर्वधातुकारी in the
    /// nominative and accusative slots (पथिन्; site @pathin1): the
    /// सर्वधातुकारी-forms (पन्थाः, पन्थानौ, पन्थानम्) there and the
    /// plain root (पथा, पथिभ्याम्, पथे) in the other cases.
    InV,
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
    /// The plain flavour of the masculine वतुप् stems (त्वत्; site
    /// @tvat1): the nominative carries the द/त pair (त्वद्-त्वत्) and
    /// the rest keeps the plain stem (त्वतौ, त्वतः).
    VantMP,
    /// The masculine अन्-stems of the ण-सर्वधातुकारी family (अर्यमन्;
    /// site @aryaman1): the dual and plural take the ण-सर्वधातुकारी
    /// (अर्यमणौ, अर्यमणः) and the oblique cases the ण-weak (अर्यम्णः).
    AnV,
    /// `j`-stem masculine/feminine (नजिङ्): pausa `svapnag`/`svapnak`,
    /// `svapnagByAm`, `svapnakzu`.
    Jvs,
    /// The masculine ज्-final nouns of the ड/ट flavour (राज्; site @rAj1):
    /// the nominative carries the ड/ट pair (राड्-राट्).
    JasM,
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
    /// The compounded pronouns (अतियुष्मद्, अत्यस्मद्; site
    /// @atiyuSmad1): the base in front of the pronoun's paradigm.
    Pron(PronFlavour),
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
            Self::AMo(opt) => format!("a-m-{}", opt.label()),
            Self::VantN => "vant-n".to_string(),
            Self::AN => "a-n".to_string(),
            Self::ANo(opt) => format!("a-n-{}", opt.label()),
            Self::AAN => "A-n".to_string(),
            Self::AAM => "A-m".to_string(),
            Self::AcM2(pair) => format!("ac-m-{pair}"),
            Self::AuM => "O-m".to_string(),
            Self::AAF => "A-f".to_string(),
            Self::NV2(o) => format!("nadi-{}", o.label()),
            Self::AAFS(o) => format!("A-f-{}", o.label()),
            Self::IMF => "i-m/f".to_string(),
            Self::IN => "i-n".to_string(),
            Self::IIF => "I-f".to_string(),
            Self::IIFN => "I-f-nadi".to_string(),
            Self::IIM => "I-m".to_string(),
            Self::IIM2(o) => format!("I-m-{}", o.label()),
            Self::UMF => "u-m/f".to_string(),
            Self::UF => "u-f".to_string(),
            Self::UN => "u-n".to_string(),
            Self::UNp => "u-n-plain".to_string(),
            Self::INy => "i-n-y".to_string(),
            Self::INr => "i-n-r".to_string(),
            Self::UUF => "U-f".to_string(),
            Self::UUFN => "U-f-nadi".to_string(),
            Self::UUM => "U-m".to_string(),
            Self::UUM2(o) => format!("U-m-{}", o.label()),
            Self::TaN => "ta-n".to_string(),
            Self::TaN2(v) => format!("ta-n2-{v}"),
            Self::AccM => "ac-m".to_string(),
            Self::AccMV(v) => format!("ac-m-v{v}"),
            Self::AjjM => "aj-m".to_string(),
            Self::UvF => "uv-f".to_string(),
            Self::UvFd => "uv-f-d".to_string(),
            Self::UnN => "un-n".to_string(),
            Self::ApM => "ap-m".to_string(),
            Self::OPlur(x, fam) => format!("oplur-{fam}-{x}"),
            Self::SiteGrid(_) => "site grid".to_string(),
            Self::INu => "i-nu".to_string(),
            Self::INv => "i-nv".to_string(),
            Self::HasM => "has-m".to_string(),
            Self::HasF => "has-f".to_string(),
            Self::RirF(v) => format!("rir-f-{v}"),
            Self::RarF => "rar-f".to_string(),
            Self::BaraM(c) => format!("bara-m-{c}"),
            Self::RarN => "rar-n".to_string(),
            Self::HasN => "has-n".to_string(),
            Self::HihF => "hih-f".to_string(),
            Self::DuhF => "duh-f".to_string(),
            Self::HuhM => "huh-m".to_string(),
            Self::HasVM => "has-vm".to_string(),
            Self::HasAM => "has-am".to_string(),
            Self::DuhM => "duh-m".to_string(),
            Self::Zam => "zam".to_string(),
            Self::ZamC(v) => format!("zam-c-{v}"),
            Self::RcN => "rc-n".to_string(),
            Self::DadN(ndi) => {
                if *ndi {
                    "dad-n".to_string()
                } else {
                    "dad-p".to_string()
                }
            }
            Self::Rc2 => "rc-2".to_string(),
            Self::Sam => "sam".to_string(),
            Self::ZamF => "zam-f".to_string(),
            Self::ZvF(v) => format!("zv-f-{v}"),
            Self::ZagM => "zag-m".to_string(),
            Self::AcN => "ac-n".to_string(),
            Self::AcNV(v) => format!("ac-n-v{v}"),
            Self::AncN => "anc-n".to_string(),
            Self::AsAnM => "asan-m".to_string(),
            Self::AsMV => "asv-m".to_string(),
            Self::AsMP => "asp-m".to_string(),
            Self::AsMW => "asw-m".to_string(),
            Self::SasV => "sas-v".to_string(),
            Self::AsEM => "ase-m".to_string(),
            Self::AsmM => "asm-m".to_string(),
            Self::AsmN => "asm-n".to_string(),
            Self::AsmF => "asm-f".to_string(),
            Self::RFM => "f-m/f".to_string(),
            Self::RFA => "rf-a".to_string(),
            Self::RFN => "f-n".to_string(),
            Self::GatiF => "i-f".to_string(),
            Self::AtM(AtMStyle::Num) => "at-m".to_string(),
            Self::AtM2 => "at-m-2".to_string(),
            Self::AtM(AtMStyle::Abhyasta) => "at-m-abhyasta".to_string(),
            Self::AtN(AtNStyle::Num) => "at-n".to_string(),
            Self::AtN(AtNStyle::WeakDual) => "at-n-viyat".to_string(),
            Self::AtN(AtNStyle::BothDual) => "at-n-tudat".to_string(),
            Self::AtN(AtNStyle::Abhyasta) => "at-n-abhyasta".to_string(),
            Self::AnM => "an-m".to_string(),
            Self::AnV => "an-v".to_string(),
            Self::AnF => "an-f".to_string(),
            Self::InM => "in-m".to_string(),
            Self::InV => "in-m-vr".to_string(),
            Self::InF => "in-f".to_string(),
            Self::InN => "in-n".to_string(),
            Self::AsN => "as-n".to_string(),
            Self::AsF => "as-f".to_string(),
            Self::AsM => "as-m".to_string(),
            Self::Cons => "cons".to_string(),
            Self::Pron(f) => format!("pron-{}", f.label()),
            Self::UsN => "us-n".to_string(),
            Self::VasM => "vas-m".to_string(),
            Self::VasN => "vas-n".to_string(),
            Self::VantM => "vant-m".to_string(),
            Self::VantMP => "vant-m-plain".to_string(),
            Self::Jvs => "j-m/f".to_string(),
            Self::JasM => "jas-m".to_string(),
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
        Class::VantN => {
            // The neuter of the वतुप् stems (अहङ्कारवत्; site
            // @ahaGkAravat2): the प्रथमा-/द्वितीया-एकवचन carry the द/त
            // pair (अहङ्कारवद्-अहङ्कारवत्), the dual the ई-form
            // (अहङ्कारवती) and the plural the न्ति-form (अहङ्कारवन्ति);
            // the भ-endings voice the stem (अहङ्कारवद्भ्याम्) and the
            // locative plural the त्सु (अहङ्कारवत्सु).
            if linga != Linga::N
                || !(stem.ends_with("vat") || stem.ends_with("mat") || stem.ends_with("yat"))
                || stem.len() < 5
            {
                return Err(NOT_POSSIBLE);
            }
            // The base drops the वतुप्'s त (अहङ्कारवद्-अहङ्कारवत्), and
            // the न्ति-form builds on it (अहङ्कारवन्ति).
            let base = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(base, "d"), f(stem, "I"), f(base, "nti")),
                (f(base, "d"), f(stem, "I"), f(base, "nti")),
                (f(stem, "A"), f(base, "dByAm"), f(base, "dBiH")),
                (f(stem, "e"), f(base, "dByAm"), f(base, "dByaH")),
                (f(stem, "aH"), f(base, "dByAm"), f(base, "dByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(base, "d"), f(stem, "I"), f(base, "nti")),
            ]);
            put(&mut cells, 6, 2, vec![f(base, "tsu")]);
            // The प्रथमा-, द्वितीया- and सम्बोधन-एकवचन carry the द/त pair.
            put(&mut cells, 0, 0, vec![f(base, "d"), stem.to_string()]);
            put(&mut cells, 1, 0, vec![f(base, "d"), stem.to_string()]);
            put(&mut cells, 7, 0, vec![f(base, "d"), stem.to_string()]);
            cells
        }
        Class::AMo(opt) => {
            // The a-stem masculine with the site's optional वृद्धि-forms
            // (अर्ध; site @ardha1): the nominative and vocative plurals
            // carry the आ- and ए-forms (अर्धाः-अर्धे), the instrumental
            // may take the ए + the ण (चरमेण) and the cat/paJ/sap the
            // स्म-forms (अपरस्मै, अपराद्-अपरस्माद्-अपरस्मात्-अपरात्,
            // अपरे-अपरस्मिन्).
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
            // The पञ्चमी-एकवचन carries both alternants.
            put(&mut cells, 4, 0, vec![f(p, "Ad"), f(p, "At")]);
            // The nominative and vocative plurals carry the ए-form too.
            put(&mut cells, 0, 2, vec![f(p, "AH"), f(p, "e")]);
            put(&mut cells, 7, 2, vec![f(p, "AH"), f(p, "e")]);
            match opt {
                AOpt::E => {}
                AOpt::EN => {
                    // The instrumental takes the ए + the ण (चरमेण) and the
                    // genitive plural the ण-form (चरमाणाम्).
                    cells[6].forms = vec![f(p, "eRa")];
                    cells[17].forms = vec![f(p, "ARAm")];
                }
                AOpt::ENSm => {
                    // The instrumental takes the ए + the ण, the cat the
                    // स्म-form, the ablative the four alternants, the
                    // locative the ए- and स्म-forms and the genitive plural
                    // the ए-form (अपरेषाम्).
                    cells[6].forms = vec![f(p, "eRa")];
                    cells[9].forms = vec![f(p, "asmE")];
                    put(
                        &mut cells,
                        4,
                        0,
                        vec![f(p, "Ad"), f(p, "asmAd"), f(p, "asmAt"), f(p, "At")],
                    );
                    put(&mut cells, 6, 0, vec![f(p, "e"), f(p, "asmin")]);
                    cells[17].forms = vec![f(p, "ezAm")];
                }
                AOpt::Comp | AOpt::CompS => {
                    // The comparative: the स्म-forms, the ए + the ण
                    // instrumental, the ए-form in the genitive plural and
                    // the nominative and vocative plurals.
                    cells[2].forms = vec![f(p, "e")];
                    cells[23].forms = vec![f(p, "e")];
                    if !matches!(opt, AOpt::CompS) {
                        cells[6].forms = vec![f(p, "eRa")];
                    }
                    cells[9].forms = vec![f(p, "asmE")];
                    put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
                    cells[17].forms = vec![f(p, "ezAm")];
                    cells[18].forms = vec![f(p, "asmin")];
                }
                AOpt::SmPairs => {
                    // The स्म-alternant beside the plain form: the cat,
                    // the four-way ablative and the locative; the
                    // nominative and vocative plurals stay plain.
                    cells[2].forms = vec![f(p, "AH")];
                    cells[23].forms = vec![f(p, "AH")];
                    cells[9].forms = vec![f(p, "Aya"), f(p, "asmE")];
                    put(
                        &mut cells,
                        4,
                        0,
                        vec![f(p, "Ad"), f(p, "asmAd"), f(p, "asmAt"), f(p, "At")],
                    );
                    put(&mut cells, 6, 0, vec![f(p, "e"), f(p, "asmin")]);
                }
                AOpt::Sm => {
                    // The cat takes the स्म-form, the ablative the two
                    // स्म-alternants, the locative the स्म-form and the
                    // genitive plural the ए-form (नेमेषाम्).
                    cells[9].forms = vec![f(p, "asmE")];
                    put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
                    cells[18].forms = vec![f(p, "asmin")];
                    cells[17].forms = vec![f(p, "ezAm")];
                }
                AOpt::SmE => {
                    // The cat takes the स्म-form, the ablative the four
                    // alternants, the locative the ए- and स्म-forms and the
                    // genitive plural the ए-form (दक्षिणेषाम्).
                    cells[9].forms = vec![f(p, "asmE")];
                    put(
                        &mut cells,
                        4,
                        0,
                        vec![f(p, "Ad"), f(p, "asmAd"), f(p, "asmAt"), f(p, "At")],
                    );
                    put(&mut cells, 6, 0, vec![f(p, "e"), f(p, "asmin")]);
                    cells[17].forms = vec![f(p, "ezAm")];
                }
                AOpt::SmEN => {
                    // The nominative plural keeps the ए-form alone
                    // (एकतरे), the instrumental takes the ए + the ण
                    // (एकतरेण), the cat the स्म-form, the ablative the two
                    // स्म-alternants and the locative the स्म-form.
                    cells[2].forms = vec![f(p, "e")];
                    cells[23].forms = vec![f(p, "e")];
                    cells[6].forms = vec![f(p, "eRa")];
                    cells[9].forms = vec![f(p, "asmE")];
                    put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
                    cells[18].forms = vec![f(p, "asmin")];
                    cells[17].forms = vec![f(p, "ezAm")];
                }
            }
            cells
        }
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
            // The ह्न-final stems carry three alternants in the locative
            // singular (अष्टाह्ने-अष्टाह्नि-अष्टाहनि): the plain form, the
            // ण-form and the form with the ह's halant dropped.
            if stem.ends_with("hna") {
                let hna = &p[..p.len() - 1];
                put(
                    &mut cells,
                    6,
                    0,
                    vec![f(p, "e"), f(p, "i"), format!("{hna}ani")],
                );
            }
            cells
        }
        Class::AuM => {
            // The masculine औ-स्वादि nouns (जनौ; site @janau1): the
            // nominative and the vocative take the औ-form (जनौः), the
            // dual/plural the व-form (जनावौ, जनावः), the भ-endings the
            // औ-form (जनौभ्याम्) and the locative plural the षु-form
            // (जनौषु).
            if linga != Linga::M || !stem.ends_with('O') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The व-form: the व after the stem's vowel (जनाव).
            let v_form = f(base, "Av");
            let au_form = stem.to_string();
            table(&[
                (f(&au_form, "H"), f(&v_form, "O"), f(&v_form, "aH")),
                (f(&v_form, "am"), f(&v_form, "O"), f(&v_form, "aH")),
                (f(&v_form, "A"), f(&au_form, "ByAm"), f(&au_form, "BiH")),
                (f(&v_form, "e"), f(&au_form, "ByAm"), f(&au_form, "ByaH")),
                (f(&v_form, "aH"), f(&au_form, "ByAm"), f(&au_form, "ByaH")),
                (f(&v_form, "aH"), f(&v_form, "oH"), f(&v_form, "Am")),
                (f(&v_form, "i"), f(&v_form, "oH"), f(stem, "zu")),
                (f(&au_form, "H"), f(&v_form, "O"), f(&v_form, "aH")),
            ])
        }
        Class::AAM => {
            // The masculine आ-stem nouns (कीलालपा; site @kIlAlapA1): the
            // nominative and the vocative take the visarga (कीलालपाः), the
            // instrumental the आ (कीलालपा), the भ-endings the plain stem
            // (कीलालपाभ्याम्) and the locative plural the आसु
            // (कीलालपासु).
            if linga != Linga::M || !stem.ends_with('A') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            // The stem's own आ is the base's vowel, so the endings that
            // carry a matra replace it (कीलालपौ, not कीलालपाऔ).
            let base = &stem[..stem.len() - 1];
            table(&[
                (f(stem, "H"), f(base, "O"), f(stem, "H")),
                (f(stem, "m"), f(base, "O"), f(base, "aH")),
                (f(base, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(base, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(base, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(base, "aH"), f(base, "oH"), f(base, "Am")),
                (f(base, "i"), f(base, "oH"), f(stem, "su")),
                (f(stem, "H"), f(base, "O"), f(stem, "H")),
            ])
        }
        Class::AAN => {
            // The neuter आ-stem nouns (श्रीपा; site @zrIpA1): the
            // nominative takes the म् (श्रीपम्), the dual the ए (श्रीपे),
            // the plural the णि (श्रीपाणि) and the instrumental the ए +
            // the ण (श्रीपेण).
            if linga != Linga::N || !stem.ends_with('A') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(base, "am"), f(base, "e"), f(base, "ARi")),
                (f(base, "am"), f(base, "e"), f(base, "ARi")),
                (f(base, "eRa"), f(stem, "ByAm"), f(base, "EH")),
                (f(base, "Aya"), f(stem, "ByAm"), f(base, "eByaH")),
                (f(base, "Ad"), f(stem, "ByAm"), f(base, "eByaH")),
                (f(base, "asya"), f(base, "ayoH"), f(base, "ARAm")),
                (f(base, "e"), f(base, "ayoH"), f(base, "ezu")),
                (f(base, "a"), f(base, "e"), f(base, "ARi")),
            ]);
            // The पञ्चमी-एकवचन carries both alternants (श्रीपाद्-श्रीपात्).
            put(&mut cells, 4, 0, vec![f(base, "Ad"), f(base, "At")]);
            cells
        }
        Class::ANo(opt) => {
            // The neuter a-stem with the site's optional वृद्धि-forms
            // (अधर; site @adhara2): the nominative may be plain or a
            // द/त pair, the instrumental takes the ए + the ण, the cat the
            // स्म-form, the ablative the स्म-alternants and the locative the
            // ए- or स्म-form.
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
            match opt {
                ANOpt::Apara => {
                    // The instrumental takes the ए + the ण (अधरेण), the cat
                    // the स्म-form, the ablative the four alternants and the
                    // locative the ए- and स्म-forms.
                    cells[6].forms = vec![f(p, "eRa")];
                    cells[9].forms = vec![f(p, "asmE")];
                    put(
                        &mut cells,
                        4,
                        0,
                        vec![f(p, "Ad"), f(p, "asmAd"), f(p, "asmAt"), f(p, "At")],
                    );
                    cells[17].forms = vec![f(p, "ezAm")];
                    put(&mut cells, 6, 0, vec![f(p, "e"), f(p, "asmin")]);
                }
                ANOpt::Comp | ANOpt::CompS => {
                    // The comparative: the स्म-forms, the ए + the ण
                    // instrumental, the ए-form in the genitive plural and
                    // the द/त pair in the nominative, accusative and
                    // vocative singular.
                    let pair = vec![f(p, "ad"), f(p, "at")];
                    cells[0].forms.clone_from(&pair);
                    cells[3].forms.clone_from(&pair);
                    cells[21].forms = pair;
                    if !matches!(opt, ANOpt::CompS) {
                        cells[6].forms = vec![f(p, "eRa")];
                    }
                    cells[9].forms = vec![f(p, "asmE")];
                    put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
                    cells[17].forms = vec![f(p, "ezAm")];
                    cells[18].forms = vec![f(p, "asmin")];
                }
                ANOpt::SmPairs => {
                    // The स्म-alternant beside the plain form: the cat,
                    // the four-way ablative and the locative.
                    cells[9].forms = vec![f(p, "Aya"), f(p, "asmE")];
                    put(
                        &mut cells,
                        4,
                        0,
                        vec![f(p, "Ad"), f(p, "asmAd"), f(p, "asmAt"), f(p, "At")],
                    );
                    put(&mut cells, 6, 0, vec![f(p, "e"), f(p, "asmin")]);
                }
                ANOpt::Tara => {
                    // The instrumental takes the ए + the ण (एकतरेण), the
                    // cat the स्म-form, the ablative the two स्म-alternants
                    // and the locative the स्म-form.
                    cells[6].forms = vec![f(p, "eRa")];
                    cells[9].forms = vec![f(p, "asmE")];
                    put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
                    cells[17].forms = vec![f(p, "ezAm")];
                    cells[18].forms = vec![f(p, "asmin")];
                }
            }
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
        Class::AAFS(opt) => {
            // The feminine a-stems in the flavours the site writes word by
            // word: the स्य-augment pairs of the दिग्-स्वादि and the
            // ordinals (उत्तरपश्चिमायै-उत्तरपश्चिमस्यै) or the bare
            // vocative (हे अक्क).
            if linga != Linga::F || !stem.ends_with('A') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(p, "e"), f(p, "AH")),
                (f(p, "Am"), f(p, "e"), f(p, "AH")),
                (f(p, "ayA"), f(p, "AByAm"), f(p, "ABiH")),
                (f(p, "AyE"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "AyAH"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "AyAH"), f(p, "ayoH"), f(p, "AnAm")),
                (f(p, "AyAm"), f(p, "ayoH"), f(p, "Asu")),
                (f(p, "e"), f(p, "e"), f(p, "AH")),
            ]);
            if matches!(opt, AAFSOpt::Bare) {
                // The vocative takes the bare stem (हे अक्क) — the
                // consonant keeps its inherent written.
                let bare = match p.chars().last() {
                    Some(c)
                        if !matches!(
                            c,
                            'a' | 'A'
                                | 'i'
                                | 'I'
                                | 'u'
                                | 'U'
                                | 'f'
                                | 'F'
                                | 'x'
                                | 'X'
                                | 'e'
                                | 'E'
                                | 'o'
                                | 'O'
                        ) =>
                    {
                        format!("{p}a")
                    }
                    _ => p.to_string(),
                };
                put(&mut cells, 7, 0, vec![bare]);
            } else {
                // The स्य-augment pairs (the site writes the plain form
                // first and the स्य-form second; the स-form is spelled
                // with the य carrying the vowel).
                put(&mut cells, 3, 0, vec![f(p, "AyE"), f(p, "asyE")]);
                put(&mut cells, 4, 0, vec![f(p, "AyAH"), f(p, "asyAH")]);
                put(&mut cells, 5, 0, vec![f(p, "AyAH"), f(p, "asyAH")]);
                put(&mut cells, 6, 0, vec![f(p, "AyAm"), f(p, "asyAm")]);
                // The genitive plural pairs: the dental (उत्तरपश्चिमानाम्)
                // or the retroflex (उत्तरपूर्वाणाम्) with the साम्
                // alternant — the ण-final compounds take the retroflex.
                let gen = if matches!(opt, AAFSOpt::SyGen) {
                    f(stem, "RAm")
                } else {
                    f(stem, "nAm")
                };
                put(&mut cells, 5, 2, vec![gen, f(stem, "sAm")]);
            }
            cells
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
        Class::OPlur(x, family) => {
            // The ओ-स्वादि compound plurals (आगस्त्य → अगस्तयः; site
            // @Agastya1 and its kin): the singular keeps the plain stem
            // (आगस्त्यः, औडुलोमिः) and the plural takes the compound's
            // reduced stem (अगस्त, उडुलोम) with the endings of the a-stem
            // masculine (कुण्डिनाः), the इ-स्वादि masculine (अगस्तयः) or
            // the उ-स्वादि masculine (भृगवः).
            if linga != Linga::M {
                return Err(NOT_POSSIBLE);
            }
            let (nom, acc, instr, dat, gen, loc) = match family {
                'i' => (
                    f(x, "ayaH"),
                    f(x, "In"),
                    f(x, "iBiH"),
                    f(x, "iByaH"),
                    f(x, "InAm"),
                    f(x, "izu"),
                ),
                'u' => (
                    f(x, "avaH"),
                    f(x, "Un"),
                    f(x, "uBiH"),
                    f(x, "uByaH"),
                    f(x, "UnAm"),
                    f(x, "uzu"),
                ),
                _ => (
                    f(x, "AH"),
                    f(x, "An"),
                    f(x, "EH"),
                    f(x, "eByaH"),
                    f(x, "AnAm"),
                    f(x, "ezu"),
                ),
            };
            let mut cells = if let Some(p) = stem.strip_suffix('i') {
                // The i-stem singular (औडुलोमिः, हे औडुलोमे).
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
            } else if let Some(p) = stem.strip_suffix('a') {
                // The a-stem singular (आगस्त्यः, आगस्त्येन).
                table(&[
                    (f(p, "aH"), f(p, "O"), f(p, "AH")),
                    (f(p, "am"), f(p, "O"), f(p, "An")),
                    (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                    (f(p, "Aya"), f(p, "AByAm"), f(p, "eByaH")),
                    (f(p, "At"), f(p, "AByAm"), f(p, "eByaH")),
                    (f(p, "asya"), f(p, "ayoH"), f(p, "AnAm")),
                    (f(p, "e"), f(p, "ayoH"), f(p, "ezu")),
                    (stem.to_string(), f(p, "O"), f(p, "AH")),
                ])
            } else {
                return Err(NOT_POSSIBLE);
            };
            put(&mut cells, 0, 2, vec![nom.clone()]);
            put(&mut cells, 1, 2, vec![acc]);
            put(&mut cells, 2, 2, vec![instr]);
            put(&mut cells, 3, 2, vec![dat.clone()]);
            put(&mut cells, 4, 2, vec![dat]);
            put(&mut cells, 5, 2, vec![gen]);
            put(&mut cells, 6, 2, vec![loc]);
            put(&mut cells, 7, 2, vec![nom]);
            if let Some(p) = stem.strip_suffix('a') {
                // The ablative singular carries the वृद्धि pair
                // (आगस्त्याद्-आगस्त्यात्).
                put(&mut cells, 4, 0, vec![f(p, "Ad"), f(p, "At")]);
            }
            cells
        }
        Class::ApM => {
            // The अ-स्वादि masculine/feminine (गुप्, अप्; site @gup1/@ap1):
            // the masculine nominative carries the जश् pause pair
            // (गुब्-गुप्) and the feminine the आ-form (आपः); the भ-endings
            // voice the stem (गुब्भ्याम्, अद्भिः) and the locative plural
            // takes the षु-form (गुप्सु, अप्सु).
            if !matches!(linga, Linga::M | Linga::F) || !stem.ends_with('p') || stem.len() < 2 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "b");
            let plain = stem.to_string();
            let fem = if linga == Linga::F {
                // the आ-form (आपः)
                format!("A{}", &stem[1..])
            } else {
                plain.clone()
            };
            let mut cells = table(&[
                (f(&fem, "aH"), f(&plain, "O"), f(&plain, "aH")),
                (f(&fem, "am"), f(&plain, "O"), f(&plain, "aH")),
                (f(&plain, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(&plain, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&plain, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&plain, "aH"), f(&plain, "oH"), f(&plain, "Am")),
                (f(&plain, "i"), f(&plain, "oH"), f(&plain, "su")),
                (f(&fem, "aH"), f(&plain, "O"), f(&plain, "aH")),
            ]);
            if linga == Linga::M {
                put(&mut cells, 0, 0, vec![voiced.clone(), plain.clone()]);
                put(&mut cells, 7, 0, vec![voiced, plain]);
            }
            cells
        }
        Class::INu => {
            // The neuter i-stems in the ण-form (अतिस्त्रि; site @atistri3):
            // the dvi/plural take the ण-forms (अतिस्त्रिणी, अतिस्त्रीणि)
            // and the cat/paJ/sap carry both alternants
            // (अतिस्त्रिणे-अतिस्त्रये). Distinct from the इ-न neuter with
            // its नु augment ([`Class::IN`], the corpus's own flavour). The
            // य/ए-forms drop the stem's ि (the sandhi), so they build on
            // the stem without it.
            if linga != Linga::N || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
                (f(stem, "RA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(stem, "Re"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "yoH"), f(p, "IRAm")),
                (f(stem, "Ri"), f(p, "ayoH"), f(stem, "zu")),
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
            ]);
            // The cat, paJ, zaz and sap carry both alternants.
            put(&mut cells, 3, 0, vec![f(stem, "Re"), f(p, "aye")]);
            put(&mut cells, 4, 0, vec![f(stem, "RaH"), f(p, "eH")]);
            put(&mut cells, 5, 0, vec![f(stem, "RaH"), f(p, "eH")]);
            put(&mut cells, 6, 0, vec![f(stem, "Ri"), f(p, "O")]);
            // The genitive and locative duals carry both alternants.
            put(&mut cells, 5, 1, vec![f(stem, "RoH"), f(stem, "yoH")]);
            put(&mut cells, 6, 1, vec![f(stem, "RoH"), f(stem, "yoH")]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "e")]);
            cells
        }
        Class::INv => {
            // The neuter i-stems with the ण-forms in the oblique cases
            // (अक्षि; site @akSi1/@asthi1/@dadhi1): the ि → the ण
            // (अक्ष्णा) or, where the न-स्वादि takes its place, the ि →
            // the न with the ण inserted (अस्थ्ना); the nominative and
            // accusative keep the plain stem, the locative singular and
            // the vocative carry both alternants (अक्ष्णि-अक्षणि).
            if linga != Linga::N || !stem.ends_with('i') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            // The ष-final stems take the ण in the oblique cases
            // (अक्ष्णा, अक्ष्णि), the others the न (अस्थ्ना, अस्थ्नि).
            let zi = stem.ends_with("zi");
            let rs = if zi { format!("{p}R") } else { format!("{p}n") };
            let (dual, plural, alt) = if zi {
                (f(stem, "RI"), f(p, "IRi"), format!("{p}aRi"))
            } else {
                (f(stem, "nI"), format!("{p}Ini"), format!("{p}ani"))
            };
            let mut cells = table(&[
                (stem.to_string(), dual.clone(), plural.clone()),
                (stem.to_string(), dual.clone(), plural.clone()),
                (f(&rs, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(&rs, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&rs, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&rs, "aH"), f(&rs, "oH"), f(&rs, "Am")),
                (f(&rs, "i"), f(&rs, "oH"), f(stem, "zu")),
                (stem.to_string(), dual, plural),
            ]);
            // The locative singular and the vocative carry both
            // alternants (अक्ष्णि-अक्षणि, हे अक्षि-हे अक्षे).
            put(&mut cells, 6, 0, vec![f(&rs, "i"), alt]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "e")]);
            cells
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
        Class::IIM => {
            // `I`-stem masculine (`amI`; site @amI1): the nominative takes
            // the visarga, the y-forms carry the endings.
            if linga != Linga::M || !stem.ends_with('I') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(stem, "H"), f(p, "yO"), f(p, "yaH")),
                (f(stem, "m"), f(p, "yO"), f(stem, "n")),
                (f(p, "yA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(p, "ye"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "yaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "yaH"), f(p, "yoH"), f(p, "yAm")),
                (stem.to_string(), f(p, "yoH"), f(stem, "zu")),
                (f(stem, "H"), f(p, "yO"), f(p, "yaH")),
            ])
        }
        Class::IIM2(opt) => {
            // The masculine ई-स्वादि in the strong-cell flavours the
            // site writes word by word: the गुण-forms (the ग्य-forms) in
            // the accusative and the locative (अग्रण्यम्, अग्रण्याम्)
            // or the वृद्धि (the िय-forms) in every strong cell (दुर्धियौ,
            // दुर्धियि).
            if linga != Linga::M || !stem.ends_with('I') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            if matches!(opt, IIM2Opt::Vridhi | IIM2Opt::VridhiA) {
                let v = f(p, "iy");
                let loc = if matches!(opt, IIM2Opt::VridhiA) {
                    // The आ-form in the locative singular (नियाम्).
                    f(&v, "Am")
                } else {
                    f(&v, "i")
                };
                table(&[
                    (f(stem, "H"), f(&v, "O"), f(&v, "aH")),
                    (f(&v, "am"), f(&v, "O"), f(&v, "aH")),
                    (f(&v, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                    (f(&v, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                    (f(&v, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                    (f(&v, "aH"), f(&v, "oH"), f(&v, "Am")),
                    (loc, f(&v, "oH"), f(stem, "zu")),
                    (f(stem, "H"), f(&v, "O"), f(&v, "aH")),
                ])
            } else {
                // The गुण flavour: the ग्य-forms in the accusative
                // singular and plural and in the locative singular; the
                // ablative and genitive singular take the उ-augment in
                // the site.
                let loc = if matches!(opt, IIM2Opt::Guna | IIM2Opt::GunaA | IIM2Opt::GunaB) {
                    f(p, "yAm")
                } else {
                    f(p, "yi")
                };
                let mut cells = table(&[
                    (f(stem, "H"), f(p, "yO"), f(p, "yaH")),
                    (f(p, "yam"), f(p, "yO"), f(p, "yaH")),
                    (f(p, "yA"), f(stem, "ByAm"), f(stem, "BiH")),
                    (f(p, "ye"), f(stem, "ByAm"), f(stem, "ByaH")),
                    (f(p, "yaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                    (f(p, "yaH"), f(p, "yoH"), f(p, "yAm")),
                    (loc, f(p, "yoH"), f(stem, "zu")),
                    (f(stem, "H"), f(p, "yO"), f(p, "yaH")),
                ]);
                if matches!(opt, IIM2Opt::GunaU) {
                    put(&mut cells, 4, 0, vec![f(p, "yuH")]);
                    put(&mut cells, 5, 0, vec![f(p, "yuH")]);
                }
                if matches!(opt, IIM2Opt::GunaA | IIM2Opt::GunaB) {
                    // The ऐ-form in the dative singular (दुर्ध्यै), the
                    // आ-forms in the ablative and genitive singular
                    // (दुर्ध्याः), the ई-form in the genitive plural
                    // (दुर्धीनाम्) and the ि-form in the vocative
                    // (हे दुर्धि).
                    put(&mut cells, 3, 0, vec![f(p, "yE")]);
                    put(&mut cells, 4, 0, vec![f(p, "yAH")]);
                    put(&mut cells, 5, 0, vec![f(p, "yAH")]);
                    put(&mut cells, 5, 2, vec![f(stem, "nAm")]);
                    put(&mut cells, 7, 0, vec![f(p, "i")]);
                }
                if matches!(opt, IIM2Opt::GunaB) {
                    // The bare stem in the nominative singular (कुमारी).
                    put(&mut cells, 0, 0, vec![stem.to_string()]);
                }
                cells
            }
        }
        Class::NV2(opt) => {
            // The नदीसंज्ञक feminine ई- and ऊ-स्वादि stems (धी, धू; site
            // @dhI1/@dhU1): the site writes the िय- and उव-forms in the
            // strong cells (धियौ, धुवौ) and, word by word, the
            // alternant pairs beside them (जयश्रिये-जयश्रियै,
            // कुलस्त्रियम्-कुलस्त्रीम्).
            if linga != Linga::F {
                return Err(NOT_POSSIBLE);
            }
            let (strong, voc) = if let Some(p) = stem.strip_suffix('I') {
                (format!("{p}iy"), format!("{p}i"))
            } else if let Some(p) = stem.strip_suffix('U') {
                // The उव-forms (धुवौ) or the व्व-forms (खलप्वौ).
                let strong = if matches!(opt, NV2Opt::Vva) {
                    format!("{p}v")
                } else {
                    format!("{p}uv")
                };
                (strong, format!("{p}u"))
            } else if let Some(p) = stem.strip_suffix('i') {
                (format!("{stem}y"), format!("{p}e"))
            } else {
                return Err(NOT_POSSIBLE);
            };
            // The genitive plural alternant: the णत्व after the ई-/इ-
            // vowels (जयश्रीणाम्) but not after the ऊ (धूनाम्).
            let gen = if stem.ends_with('U') {
                f(stem, "nAm")
            } else {
                f(stem, "RAm")
            };
            let mut cells = table(&[
                (f(stem, "H"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(&strong, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&strong, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&strong, "aH"), f(&strong, "oH"), f(&strong, "Am")),
                (f(&strong, "i"), f(&strong, "oH"), f(stem, "zu")),
                (voc.clone(), f(&strong, "O"), f(&strong, "aH")),
            ]);
            match opt {
                NV2Opt::Eka => {
                    // The pairs in the accusative singular and plural;
                    // the vocative drops the ending (हे कुलस्त्रि).
                    put(&mut cells, 0, 0, vec![stem.to_string()]);
                    put(&mut cells, 1, 0, vec![f(&strong, "am"), f(stem, "m")]);
                    put(&mut cells, 1, 2, vec![f(&strong, "aH"), f(stem, "H")]);
                    put(&mut cells, 3, 0, vec![f(&strong, "E")]);
                    put(&mut cells, 4, 0, vec![f(&strong, "AH")]);
                    put(&mut cells, 5, 0, vec![f(&strong, "AH")]);
                    put(&mut cells, 5, 2, vec![gen]);
                    put(&mut cells, 6, 0, vec![f(&strong, "Am")]);
                    put(&mut cells, 7, 0, vec![voc]);
                }
                NV2Opt::Strong | NV2Opt::StrongN => {
                    // The pairs in the dative, ablative, genitive and
                    // locative.
                    put(&mut cells, 3, 0, vec![f(&strong, "e"), f(&strong, "E")]);
                    put(&mut cells, 4, 0, vec![f(&strong, "aH"), f(&strong, "AH")]);
                    put(&mut cells, 5, 0, vec![f(&strong, "aH"), f(&strong, "AH")]);
                    let gen = if matches!(opt, NV2Opt::StrongN) {
                        f(stem, "nAm")
                    } else {
                        gen
                    };
                    put(&mut cells, 5, 2, vec![f(&strong, "Am"), gen]);
                    put(&mut cells, 6, 0, vec![f(&strong, "i"), f(&strong, "Am")]);
                    put(&mut cells, 7, 0, vec![f(stem, "H")]);
                }
                NV2Opt::Solo | NV2Opt::Vva => {
                    put(&mut cells, 7, 0, vec![f(stem, "H")]);
                }
            }
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
        Class::UvF => {
            // The व-स्वादि feminine u-stems (अभ्रमु; site @abhramuh1): the
            // instrumental and the case endings take the व-forms
            // (अभ्रम्वा) and the cat/paJ/sap carry both alternants
            // (अभ्रमवे-अभ्रम्वै). The sandhi is spelled out: the उ of the
            // stem takes the व (व) or the ऊ (ऊ).
            if linga != Linga::F || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let u = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(stem, "H"), f(u, "U"), f(u, "avaH")),
                (f(stem, "m"), f(u, "U"), f(u, "UH")),
                (f(u, "vA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(u, "ave"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(u, "oH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(u, "oH"), f(u, "voH"), f(u, "URAm")),
                (f(u, "O"), f(u, "voH"), f(stem, "zu")),
                (f(u, "o"), f(u, "U"), f(u, "avaH")),
            ]);
            // The cat, paJ, zaz and sap carry both alternants.
            put(&mut cells, 3, 0, vec![f(u, "ave"), f(u, "vE")]);
            put(&mut cells, 4, 0, vec![f(u, "oH"), f(u, "vAH")]);
            put(&mut cells, 5, 0, vec![f(u, "oH"), f(u, "vAH")]);
            put(&mut cells, 6, 0, vec![f(u, "O"), f(u, "vAm")]);
            cells
        }
        Class::UvFd => {
            // The व-स्वादि feminine u-stems of the dental genitive plural
            // (अलावु; site @alAvu1): the same shape as `UvF` — the
            // instrumental and the case endings take the व-forms (अलाव्वा)
            // and the cat/paJ/sap carry both alternants — but the genitive
            // plural keeps the dental न (अलावूनाम्) where `UvF` has the
            // ण-form (अभ्रमूणाम्, site @abhramu1).
            if linga != Linga::F || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let u = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(stem, "H"), f(u, "U"), f(u, "avaH")),
                (f(stem, "m"), f(u, "U"), f(u, "UH")),
                (f(u, "vA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(u, "ave"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(u, "oH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(u, "oH"), f(u, "voH"), f(u, "UnAm")),
                (f(u, "O"), f(u, "voH"), f(stem, "zu")),
                (f(u, "o"), f(u, "U"), f(u, "avaH")),
            ]);
            // The cat, paJ, zaz and sap carry both alternants.
            put(&mut cells, 3, 0, vec![f(u, "ave"), f(u, "vE")]);
            put(&mut cells, 4, 0, vec![f(u, "oH"), f(u, "vAH")]);
            put(&mut cells, 5, 0, vec![f(u, "oH"), f(u, "vAH")]);
            put(&mut cells, 6, 0, vec![f(u, "O"), f(u, "vAm")]);
            cells
        }
        Class::UnN => {
            // The neuter u-stems (अगुरु; site @aguru2): the cat/paJ/zaz/sap
            // take the ण-forms (अगुरुणे, अगुरूणाम्) — the उ of the stem
            // sandhis into ऊ before the ण.
            if linga != Linga::N || !stem.ends_with('u') {
                return Err(NOT_POSSIBLE);
            }
            let u = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(stem, "RI"), f(u, "URi")),
                (stem.to_string(), f(stem, "RI"), f(u, "URi")),
                (f(stem, "RA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(stem, "Re"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "RoH"), f(u, "URAm")),
                (f(stem, "Ri"), f(stem, "RoH"), f(stem, "zu")),
                (stem.to_string(), f(stem, "RI"), f(u, "URi")),
            ]);
            // The vocative carries the stem and its ए-form.
            put(&mut cells, 7, 0, vec![stem.to_string(), f(u, "o")]);
            cells
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
        Class::Pron(flavour) => {
            // The compounded pronouns (अतियुष्मद्, अत्यस्मद्,
            // परमयुष्मद्, परमास्मद्). The site grids the pronoun's own
            // paradigm with the base in front of it, in two shapes:
            //
            // * the अधिक compounds (अतियुष्मद्, अत्यस्मद्) — one form
            //   per cell, the pronoun's त्व/युव/युष्म flavours
            //   (अतित्वाम्, अतियुवाम्, अतियुष्माम्) or its म/आव/स्म
            //   flavours (अतिमाम्, अत्यावाम्, अत्यस्माम्); a few
            //   cells stay with the first flavour (see the helpers).
            // * the पुरुष compounds (परमयुष्मद्, परमास्मद्) — the
            //   pronoun's own grid with its pairs, the base on every
            //   alternative (परमत्वाम्-परमत्वा).
            if linga != Linga::M {
                return Err(NOT_POSSIBLE);
            }
            match flavour {
                PronFlavour::YuzTv | PronFlavour::YuzYuv | PronFlavour::YuzYuzm => {
                    let Some(base) = stem.strip_suffix("yuzmad") else {
                        return Err(NOT_POSSIBLE);
                    };
                    if base.is_empty() {
                        return Err(NOT_POSSIBLE);
                    }
                    let pstem = match flavour {
                        PronFlavour::YuzYuv => "yuv",
                        PronFlavour::YuzYuzm => "yuzm",
                        _ => "tv",
                    };
                    yuz_compound_cells(base, pstem)
                }
                PronFlavour::AsmMa | PronFlavour::AsmAv | PronFlavour::AsmSm => {
                    let Some(base) = stem.strip_suffix("asmad") else {
                        return Err(NOT_POSSIBLE);
                    };
                    // The stem carries the sandhi य (अत्यस्मद् = अति + अस्मद्).
                    let Some(rest) = base.strip_suffix('y') else {
                        return Err(NOT_POSSIBLE);
                    };
                    // The म-forms take the plain अति (अतिमाम्), the आव-
                    // and स्म-forms the sandhi (अत्यावाम्, अत्यस्माम्).
                    let ati = format!("{rest}i");
                    let sandhi = base.to_string();
                    let pstem = match flavour {
                        PronFlavour::AsmAv => "Av",
                        PronFlavour::AsmSm => "sm",
                        _ => "m",
                    };
                    asm_compound_cells(&ati, &sandhi, pstem)
                }
                PronFlavour::YuzPara | PronFlavour::AsmPara => {
                    let pron = if matches!(flavour, PronFlavour::YuzPara) {
                        "yuzmad"
                    } else {
                        "asmad"
                    };
                    let Some((plain, sandhi)) = para_bases(stem) else {
                        return Err(NOT_POSSIBLE);
                    };
                    pron_para_cells(&plain, &sandhi, pron)
                }
            }
        }
        Class::Cons => {
            // Full consonant-final noun (सरित्/विद्युत्/संविद्/उपनिषद्/
            // युध्/ककुभ्/वाच्; site @sarit1/@kakubh1/@vAc1). One shape for
            // the finals {t, d, D, B, c, th, S}: प्रथमा- and सम्बोधन-सि carry
            // the pausa pairs (सरिद्-सरित्, युद्-युत्, ककुब्-ककुप्,
            // वाग्-वाक्, अग्निमद्-अग्निमत्), the भ्-endings voice the final
            // (सरिद्भ्याम्, ककुब्भ्याम्, वाग्भ्याम्), and सु devoices it
            // (सरित्सु, ककुप्सु; वाक्षु for च्). The ऋ-chart's श्-final
            // carries a third alternant (अदृग्-अदृख्-अदृक्). Strong cells
            // keep the input final (सरितौ/संविदौ/युधौ/ककुभौ). Feminine
            // reaches every final; masculine reaches {t, d, D, B, th, S}
            // (मरुत्, बुध्, गर्दभ्, अग्निमथ्, अदृश्) — the site's masculine
            // च्-final words (अर्वाच्, अन्वच्) are the ञ्च-स्वादि family
            // (अर्वाञ्चः), a different paradigm, so च् stays feminine-only.
            // Participle अ-उपधा spellings stay with AtM.
            // The ऋ-chart (ऋक्/ऋग्/ऋच्, site @Rc1) is the same shape with
            // a one-letter base, so those two-letter stems are admitted.
            let r_chart = matches!(stem, "fc" | "fk" | "fg");
            if !matches!(linga, Linga::M | Linga::F) || (stem.len() < 3 && !r_chart) {
                return Err(NOT_POSSIBLE);
            }
            // The masculine अ-उपधा participles (गच्छत्, भवन्त) keep the
            // AtM shape; the routing sends them there, and the feminine
            // numeral family (त्रिंशत्) is the Cons shape.
            if linga == Linga::M && (stem.ends_with("at") || stem.ends_with("ant")) {
                return Err(NOT_POSSIBLE);
            }
            // (base length, voiced, middle alternant, voiceless, सु suffix).
            // थ् is `T` in SLP1 and alternates like त् (अग्निमद्-अग्निमत्);
            // the श्-final ऋ-chart adds the ख् alternant between the voiced
            // and the pause (अदृग्-अदृख्-अदृक्).
            let (base_len, voiced, middle, voiceless, su_suffix) = {
                let Some(c) = stem.chars().next_back() else {
                    return Err(NOT_POSSIBLE);
                };
                match c {
                    't' | 'd' | 'D' | 'T' => (1, "d", None, "t", "su"),
                    'B' => (1, "b", None, "p", "su"),
                    'c' => (1, "g", None, "k", "zu"),
                    'S' => (1, "g", Some("K"), "k", "zu"),
                    _ => return Err(NOT_POSSIBLE),
                }
            };
            if linga == Linga::M && stem.ends_with('c') {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - base_len];
            // The भ्-final family voices its internal द/ब/ग/ज in the weak
            // forms (the जश्-व्यवस्था): गर्दभ् → गर्धब्-गर्धप्,
            // गर्धब्भ्याम्, गर्धप्सु — the strong cells keep गर्दभः.
            let weak_base = if stem.ends_with('B') {
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
            // The प्रथमा- and सम्बोधन-सि pausa pairs (सरिद्-सरित्; the
            // ऋ-chart's श्-final adds the ख् alternant — अदृग्-अदृख्-अदृक्).
            let mut pair = vec![voiced_stem.clone()];
            if let Some(m) = middle {
                pair.push(f(&weak_base, m));
            }
            pair.push(pausa_stem.clone());
            put(&mut cells, 0, 0, pair);
            let mut samb_pair = vec![f(&weak_base, voiced)];
            if let Some(m) = middle {
                samb_pair.push(f(&weak_base, m));
            }
            samb_pair.push(pausa_stem);
            put(&mut cells, 7, 0, samb_pair);
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
        Class::UNp => {
            // The u-stem neuter without the गुण/व् alternants (मधु; site
            // @madhu1): the plain forms only (मधु, मधुनी, मधूनि,
            // मधुना, मधुने, मधुनः, मधूनाम्, मधुषु).
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
            // The vocative singular carries the stem and the ओ-form.
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "o")]);
            cells
        }
        Class::INr => {
            // The neuter i-stems in the ण-form with the ऋ-form in the
            // genitive and locative duals (भूरि; site @bhUri1): the
            // य-forms drop the stem's ि (the sandhi), so they build on
            // the stem without it — भूरिणोः-भूर्योः.
            if linga != Linga::N || !stem.ends_with('i') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
                (f(stem, "RA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(stem, "Re"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "RaH"), f(stem, "yoH"), f(p, "IRAm")),
                (f(stem, "Ri"), f(p, "ayoH"), f(stem, "zu")),
                (stem.to_string(), f(stem, "RI"), f(p, "IRi")),
            ]);
            put(&mut cells, 3, 0, vec![f(stem, "Re"), f(p, "aye")]);
            put(&mut cells, 4, 0, vec![f(stem, "RaH"), f(p, "eH")]);
            put(&mut cells, 5, 0, vec![f(stem, "RaH"), f(p, "eH")]);
            put(&mut cells, 6, 0, vec![f(stem, "Ri"), f(p, "O")]);
            // The genitive and locative duals carry both alternants, the
            // य-form without the stem's ि.
            put(&mut cells, 5, 1, vec![f(stem, "RoH"), f(p, "yoH")]);
            put(&mut cells, 6, 1, vec![f(stem, "RoH"), f(p, "yoH")]);
            put(&mut cells, 7, 0, vec![stem.to_string(), f(p, "e")]);
            cells
        }
        Class::INy => {
            // The i-stem neuter with the य-forms as alternants (अनादि;
            // site @anAdi1): the locative singular carries the इ- and
            // य-forms (अनादिने-अनादये) and the nominative the इ- and
            // ए-forms (अनादिनः-अनादेः).
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
            put(&mut cells, 3, 0, vec![f(p, "ine"), f(p, "aye")]);
            put(&mut cells, 4, 0, vec![f(p, "inaH"), f(p, "eH")]);
            put(&mut cells, 5, 0, vec![f(p, "inaH"), f(p, "eH")]);
            put(&mut cells, 5, 1, vec![f(p, "inoH"), f(p, "yoH")]);
            put(&mut cells, 6, 0, vec![f(p, "ini"), f(p, "O")]);
            put(&mut cells, 6, 1, vec![f(p, "inoH"), f(p, "yoH")]);
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
        Class::UUM => {
            // `U`-stem masculine (`agnibhU`; site @agnibhU1): the
            // nominative takes the visarga, the v-forms keep the उ
            // (`agnibhuvAH`).
            if linga != Linga::M || !stem.ends_with('U') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (f(stem, "H"), f(p, "uvO"), f(p, "uvaH")),
                (f(p, "uvam"), f(p, "uvO"), f(p, "uvaH")),
                (f(p, "uvA"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(p, "uve"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "uvaH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(p, "uvaH"), f(p, "uvoH"), f(p, "uvAm")),
                (f(p, "uvi"), f(p, "uvoH"), f(stem, "zu")),
                (f(stem, "H"), f(p, "uvO"), f(p, "uvaH")),
            ])
        }
        Class::UUM2(opt) => {
            // The masculine ऊ-स्वादि with the व्व-forms in the strong
            // cells (उल्ल्वौ, उल्ल्वम्; site @ullU1) — the accusative
            // sometimes keeps the ऊ-forms (अग्रेदिधिषूम्; site
            // @agredidhiSU1).
            if linga != Linga::M || !stem.ends_with('U') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let v = f(p, "v");
            let (acc_sg, acc_pl) = match opt {
                UUM2Opt::Vva => (f(&v, "am"), f(&v, "aH")),
                UUM2Opt::VvaU => (f(stem, "m"), f(stem, "n")),
            };
            table(&[
                (f(stem, "H"), f(&v, "O"), f(&v, "aH")),
                (acc_sg, f(&v, "O"), acc_pl),
                (f(&v, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(&v, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&v, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&v, "aH"), f(&v, "oH"), f(&v, "Am")),
                (f(&v, "i"), f(&v, "oH"), f(stem, "zu")),
                (f(stem, "H"), f(&v, "O"), f(&v, "aH")),
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
        Class::RFA => {
            // The ऋ-स्वादि masculine/feminine (मातृ, पितृ; site @mAtR1/
            // @pitR1): the nominative takes the आ-form (माता, हे मातः),
            // the dual and plural the र-form (मातरौ, मातरः), the
            // accusative plural the ऋ-वृद्धि (पितॄन्, मातॄः), the
            // ablative and genitive singular the उ-form (मातुः), the
            // genitive plural the णाम् (मातॄणाम्) and the instrumental
            // the र-form (मात्रा).
            if !matches!(linga, Linga::M | Linga::F) || !stem.ends_with('f') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            // The root without the ऋ (मात्, पित्).
            let mut root = stem[..stem.len() - 1].to_string();
            if root.ends_with('a') {
                root.pop();
            }
            // The strong stem keeps the root's own vowel (मातर-) and the
            // weak stem closes it (मात्र-).
            let strong = format!("{root}ar");
            let weak = format!("{root}r");
            // The accusative plural: the masculine takes the ऋ-वृद्धि + न्
            // (पितॄन्), the feminine the ऋ-वृद्धि + the visarga (मातॄः).
            let acc_pl = if linga == Linga::M {
                f(&root, "Fn")
            } else {
                f(&root, "FH")
            };
            table(&[
                (f(&root, "A"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), acc_pl),
                (f(&weak, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(&weak, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&root, "uH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(&root, "uH"), f(&weak, "oH"), f(&root, "FRAm")),
                (f(&strong, "i"), f(&weak, "oH"), f(stem, "zu")),
                (f(&root, "H"), f(&strong, "O"), f(&strong, "aH")),
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
        Class::AtM2 => {
            // The masculine at-participles with the द/त pair (चकासत्; site
            // @cakAsat1): the प्रथमा- and सम्बोधन-एकवचन carry the pair
            // (चकासद्-चकासत्) and the rest keeps the plain stem
            // (चकासतौ, चकासता, चकासद्भ्याम्, चकासत्सु).
            if linga != Linga::M || !stem.ends_with('t') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_dental(stem);
            let d = format!("{}d", &w[..w.len() - 1]);
            let mut cells = table(&[
                (w.clone(), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(&w, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (w.clone(), f(&w, "O"), f(&w, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![d.clone(), w.clone()]);
            put(&mut cells, 7, 0, vec![d, w]);
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
        Class::VantMP => {
            // The plain flavour of the masculine वतुप् stems (त्वत्; site
            // @tvat1): the nominative carries the द/त pair (त्वद्-त्वत्)
            // where the वृद्धि flavour writes the वृद्धि-form (आत्मवान्),
            // the rest keeps the plain stem (त्वतौ, त्वतः) and the
            // भ-endings the द-form (त्वद्भ्याम्).
            if linga != Linga::M
                || !(stem.ends_with("vat") || stem.ends_with("mat") || stem.ends_with("yat"))
                || stem.len() < 4
            {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let d = format!("{p}d");
            let mut cells = table(&[
                (d.clone(), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&d, "ByAm"), f(&d, "BiH")),
                (f(stem, "e"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(&d, "ByAm"), f(&d, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (d.clone(), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![d.clone(), stem.to_string()]);
            put(&mut cells, 7, 0, vec![d, stem.to_string()]);
            put(&mut cells, 6, 2, vec![f(p, "tsu")]);
            cells
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
        Class::AnV => {
            // The masculine अन्-stems of the ण-सर्वधातुकारी family
            // (अर्यमन्; site @aryaman1): the dual and plural take the
            // ण-सर्वधातुकारी (अर्यमणौ, अर्यमणः), the oblique cases the
            // ण-weak (अर्यम्णः, अर्यम्णोः, अर्यम्णाम्), the locative
            // singular both alternants (अर्यम्णि-अर्यमणि) and the
            // भ-endings and the locative plural the plain stem
            // (अर्यमभ्याम्, अर्यमसु).
            if linga != Linga::M || !stem.ends_with("an") || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let root = &stem[..stem.len() - 2];
            // The ण-सर्वधातुकारी keeps the root's own vowel (अर्यमण-) and
            // the ण-weak closes it (अर्यम्ण-).
            let strong = format!("{root}aR");
            let weak = format!("{root}R");
            let plain = format!("{root}a");
            let mut cells = table(&[
                (f(root, "A"), f(&strong, "O"), f(&strong, "aH")),
                (f(&strong, "am"), f(&strong, "O"), f(&weak, "aH")),
                (f(&weak, "A"), f(&plain, "ByAm"), f(&plain, "BiH")),
                (f(&weak, "e"), f(&plain, "ByAm"), f(&plain, "ByaH")),
                (f(&weak, "aH"), f(&plain, "ByAm"), f(&plain, "ByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(&plain, "su")),
                (stem.to_string(), f(&strong, "O"), f(&strong, "aH")),
            ]);
            put(&mut cells, 6, 0, vec![f(&weak, "i"), f(&strong, "i")]);
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
        Class::InV => {
            // The इ-स्वादि masculine with the वृद्धि-सर्वधातुकारी in
            // the nominative and accusative slots (पथिन्; site
            // @pathin1): the site writes पन्थाः, पन्थानौ, पन्थानः and
            // पन्थानम् there and the plain root (पथा, पथिभ्याम्, पथे)
            // in the other cases.
            if linga != Linga::M || !stem.ends_with("in") {
                return Err(NOT_POSSIBLE);
            }
            let root = &stem[..stem.len() - 2];
            // The सर्वधातुकारी-stem: the root with the न् (the ण in the
            // ऋ-class) before its last consonant, plus the आ + the न्
            // (पन्थान्).
            let Some(last) = root.chars().last() else {
                return Err(NOT_POSSIBLE);
            };
            let vr = if root.contains('f') {
                // The ऋ-class roots take the ण-सर्वधातुकारी (ऋभुक्षाण्).
                format!("{root}AR")
            } else {
                // The न् goes before the last consonant (पन्थान्).
                format!("{}n{last}An", &root[..root.len() - 1])
            };
            // The nominative singular drops the final न् again (पन्थाः).
            let vr_a = &vr[..vr.len() - 1];
            table(&[
                (f(vr_a, "H"), f(&vr, "O"), f(&vr, "aH")),
                (f(&vr, "am"), f(&vr, "O"), f(root, "aH")),
                (f(root, "A"), f(root, "iByAm"), f(root, "iBiH")),
                (f(root, "e"), f(root, "iByAm"), f(root, "iByaH")),
                (f(root, "aH"), f(root, "iByAm"), f(root, "iByaH")),
                (f(root, "aH"), f(root, "oH"), f(root, "Am")),
                (f(root, "i"), f(root, "oH"), f(root, "izu")),
                (f(vr_a, "H"), f(&vr, "O"), f(&vr, "aH")),
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
            let mut cells = table(&[
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(q, "aH"), f(&w, "I"), f(q, "AMsi")),
            ]);
            // The locative plural carries both alternants (अनःसु-अनस्सु,
            // site @anas1): the visarga form and the doubled स् form.
            put(&mut cells, 6, 2, vec![f(q, "aHsu"), f(&w, "su")]);
            cells
        }
        Class::TaN => {
            // The neuter of a त-स्वादि stem (आत्मवत्, आयुष्मत्; site
            // @Atmavat2/@AyuSmat2): the प्रथमा-/द्वितीया-एकवचन carry the
            // द/त pair (आत्मवद्-आत्मवत्) and the rest the ई-forms
            // (आत्मवती, आत्मवन्ति, आत्मवता, आत्मवते, आत्मवति).
            if linga != Linga::N || !stem.ends_with('t') || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "d");
            let mut cells = table(&[
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
            ]);
            // The प्रथमा- and द्वितीया-एकवचन carry the द/त pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), stem.to_string()]);
            put(&mut cells, 1, 0, vec![voiced.clone(), stem.to_string()]);
            put(&mut cells, 7, 0, vec![voiced, stem.to_string()]);
            cells
        }
        Class::AsmM => {
            // The स्म-ending a-stem masculine (अनेक; site @aneka1): the
            // nominative plural takes the ए-form (अनेके), the cat/paJ/sap
            // the स्म-forms (अनेकस्मै, अनेकस्माद्-अनेकस्मात्, अनेकस्मिन्).
            if linga != Linga::M || !stem.ends_with('a') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "aH"), f(p, "O"), f(p, "e")),
                (f(p, "am"), f(p, "O"), f(p, "An")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "asmE"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asmAd"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayoH"), f(p, "ezAm")),
                (f(p, "asmin"), f(p, "ayoH"), f(p, "ezu")),
                (stem.to_string(), f(p, "O"), f(p, "e")),
            ]);
            // The ablative singular carries both स्म-forms.
            put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
            cells
        }
        Class::AsmN => {
            // The स्म-ending a-stem neuter (अनेक; site @aneka2).
            if linga != Linga::N || !stem.ends_with('a') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "am"), f(p, "e"), f(p, "Ani")),
                (f(p, "ena"), f(p, "AByAm"), f(p, "EH")),
                (f(p, "asmE"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asmAd"), f(p, "AByAm"), f(p, "eByaH")),
                (f(p, "asya"), f(p, "ayoH"), f(p, "ezAm")),
                (f(p, "asmin"), f(p, "ayoH"), f(p, "ezu")),
                (stem.to_string(), f(p, "e"), f(p, "Ani")),
            ]);
            // The ablative singular carries both स्म-forms.
            put(&mut cells, 4, 0, vec![f(p, "asmAd"), f(p, "asmAt")]);
            cells
        }
        Class::AsmF => {
            // The स्म-ending a-stem feminine (अधरा; site @adharA1): the
            // cat/paJ/sap take the स-form (अधरस्यै, अधरस्याः, अधरस्याम्),
            // spelled with the य carrying the vowel (`asyE`).
            if linga != Linga::F || !stem.ends_with('A') {
                return Err(NOT_POSSIBLE);
            }
            let p = &stem[..stem.len() - 1];
            table(&[
                (stem.to_string(), f(p, "e"), f(p, "AH")),
                (f(p, "Am"), f(p, "e"), f(p, "AH")),
                (f(p, "ayA"), f(p, "AByAm"), f(p, "ABiH")),
                (f(p, "asyE"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "asyAH"), f(p, "AByAm"), f(p, "AByaH")),
                (f(p, "asyAH"), f(p, "ayoH"), f(p, "AsAm")),
                (f(p, "asyAm"), f(p, "ayoH"), f(p, "Asu")),
                (f(p, "e"), f(p, "e"), f(p, "AH")),
            ])
        }
        Class::HasVM => {
            // The masculine डु-वृद्धि ह्-final nouns (अनडुह्; site @anaDuh1):
            // the प्रथमा-/द्वितीया-/सम्बोधन-एकवचन and the dual take the
            // वृद्धि (अनड्वान्, अनड्वाहौ), the भ-endings the उपधा-विवचन
            // द-form (अनडुद्भ्याम्) and the locative plural the त-form
            // (अनडुत्सु).
            if linga != Linga::M || !stem.ends_with("quh") || stem.len() < 5 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let root = &base[..base.len() - 2];
            // The वृद्धि: the डु becomes the ड्व with the आ (अनडु →
            // अनड्वा), and the ह् of the stem stays — except before the
            // न् of the nominative, where the ह् is elided (अनड्वान्).
            let vrdh = f(root, "qvAh");
            let vrdh_short = f(root, "qvA");
            // The उपधा-विवचन: the ह् before the भ-ending is the द् and
            // before the स the त् (अनडुद्भ्याम्, अनडुत्सु).
            let voiced = f(base, "d");
            let pause = f(base, "t");
            let mut cells = table(&[
                (f(&vrdh_short, "n"), f(&vrdh, "O"), f(&vrdh, "aH")),
                (f(&vrdh, "am"), f(&vrdh, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(root, "qvan"), f(&vrdh, "O"), f(&vrdh, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(&pause, "su")]);
            cells
        }
        Class::HasAM => {
            // The masculine औ-स्वादि ह्-final nouns (विश्ववाह्; site
            // @vizvavAh1): the प्रथमा-/सम्बोधन-एकवचन carry the ड/ट pair
            // (विश्ववाड्-विश्ववाट्), the plural slots the औ-form
            // (विश्वौहः, विश्वौहाम्) and the locative plural the ट्सु
            // pair (विश्ववाट्सु-विश्ववाट्त्सु).
            if linga != Linga::M || !stem.ends_with('h') || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            if !base.ends_with("vA") {
                return Err(NOT_POSSIBLE);
            }
            // The औ-form: the वा becomes the औ and the व is elided
            // (विश्ववा → विश्वौ). The vowel before the व is the inherent
            // one of the preceding consonant (विश्व), so it goes too.
            let head = &base[..base.len() - 2];
            let au = format!("{}Oh", &head[..head.len() - 1]);
            let alt = retroflex_cluster(base);
            let voiced = f(&alt, "q");
            let pause = f(&alt, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(&au, "aH")),
                (f(&au, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(&au, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&au, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&au, "aH"), f(&au, "oH"), f(&au, "Am")),
                (f(&au, "i"), f(&au, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 6, 2, vec![f(&alt, "wsu"), f(&alt, "wtsu")]);
            cells
        }
        Class::HuhM => {
            // The masculine ह्-final nouns of the four-alternant flavour
            // (मुह्; site @muh1): the प्रथमा-/सम्बोधन-एकवचन carry the
            // ड/ग/क/ट quartet (मुड्-मुग्-मुक्-मुट्), the भ-endings the
            // ड/ग pair (मुड्भ्याम्-मुग्भ्याम्) and the locative plural the
            // ट्सु-क्षु-ट्त्सु triple (मुट्सु-मुक्षु-मुट्त्सु).
            if linga != Linga::M || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let alt = retroflex_cluster(base);
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&alt, "qByAm"), f(&alt, "qBiH")),
                (f(stem, "e"), f(&alt, "qByAm"), f(&alt, "qByaH")),
                (f(stem, "aH"), f(&alt, "qByAm"), f(&alt, "qByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(
                &mut cells,
                0,
                0,
                vec![f(&alt, "q"), f(&alt, "g"), f(&alt, "k"), f(&alt, "w")],
            );
            put(
                &mut cells,
                7,
                0,
                vec![f(&alt, "q"), f(&alt, "g"), f(&alt, "k"), f(&alt, "w")],
            );
            put(&mut cells, 2, 1, vec![f(&alt, "qByAm"), f(&alt, "gByAm")]);
            put(&mut cells, 2, 2, vec![f(&alt, "qBiH"), f(&alt, "gBiH")]);
            put(&mut cells, 3, 1, vec![f(&alt, "qByAm"), f(&alt, "gByAm")]);
            put(&mut cells, 3, 2, vec![f(&alt, "qByaH"), f(&alt, "gByaH")]);
            put(&mut cells, 4, 1, vec![f(&alt, "qByAm"), f(&alt, "gByAm")]);
            put(&mut cells, 4, 2, vec![f(&alt, "qByaH"), f(&alt, "gByaH")]);
            put(
                &mut cells,
                6,
                2,
                vec![f(&alt, "wsu"), f(&alt, "kzu"), f(&alt, "wtsu")],
            );
            cells
        }
        Class::DuhM => {
            // The masculine ह्-final nouns of the two-alternant flavour
            // (दुह्; site @duh1): the प्रथमा-/सम्बोधन-एकवचन carry the
            // ग/क pair (धुग्-धुक्), the भ-endings the ग-form (धुग्भ्याम्)
            // and the locative plural the क्षु (धुक्षु). The द before the
            // ह् is the ध here (दुह् → धुग्).
            if linga != Linga::M || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let alt = retroflex_cluster(base);
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&alt, "gByAm"), f(&alt, "gBiH")),
                (f(stem, "e"), f(&alt, "gByAm"), f(&alt, "gByaH")),
                (f(stem, "aH"), f(&alt, "gByAm"), f(&alt, "gByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![f(&alt, "g"), f(&alt, "k")]);
            put(&mut cells, 7, 0, vec![f(&alt, "g"), f(&alt, "k")]);
            put(&mut cells, 6, 2, vec![f(&alt, "kzu")]);
            cells
        }
        Class::HasF => {
            // The feminine ह्-final nouns of the द/त flavour (उपानह्; site
            // @upAnah1): the प्रथमा-/सम्बोधन-एकवचन carry the द/त pair
            // (उपानद्-उपानत्), the भ-endings voice the stem (उपानद्भ्याम्)
            // and the locative plural the त्सु (उपानत्सु).
            if linga != Linga::F || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            ha_final_cells(
                stem,
                vec![f(base, "d"), f(base, "t")],
                &f(base, "d"),
                vec![f(base, "tsu")],
            )
        }
        Class::HihF => {
            // The feminine ह्-final nouns of the ग/क flavour (उष्णिह्; site
            // @uSNih1): the pair is उष्णिग्-उष्णिक्, the भ-endings the
            // ग-form (उष्णिग्भ्याम्) and the locative plural the क्षु
            // (उष्णिक्षु).
            if linga != Linga::F || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            ha_final_cells(
                stem,
                vec![f(base, "g"), f(base, "k")],
                &f(base, "g"),
                vec![f(base, "kzu")],
            )
        }
        Class::DuhF => {
            // The feminine ह्-final nouns of the धुग्/धुक् flavour (कामदुह्;
            // site @kAmaduh1): the द before the ह् is the ध (कामदु →
            // कामधु), so the pair is कामधुग्-कामधुक्.
            if linga != Linga::F || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let alt = retroflex_cluster(base);
            ha_final_cells(
                stem,
                vec![f(&alt, "g"), f(&alt, "k")],
                &f(&alt, "g"),
                vec![f(&alt, "kzu")],
            )
        }
        Class::HasN => {
            // The neuter ह्-final nouns (स्वनडुह्; site @svanaDuh1): the
            // प्रथमा-/द्वितीया-एकवचन carry the द/त pair (स्वनडुद्-स्वनडुत्),
            // the dual the ई-form (स्वनडुही) and the plural the वृद्धि with
            // the anusvāra (स्वनड्वांहि).
            if linga != Linga::N || !stem.ends_with('h') || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let root = &base[..base.len() - 2];
            // The वृद्धि: the डु becomes the ड्व with the आ.
            let vrdh = f(root, "qvA");
            let voiced = f(base, "d");
            let pause = f(base, "t");
            let mut cells = table(&[
                (f(base, "d"), f(base, "hI"), f(&vrdh, "Mhi")),
                (f(base, "d"), f(base, "hI"), f(&vrdh, "Mhi")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(base, "d"), f(base, "hI"), f(&vrdh, "Mhi")),
            ]);
            put(&mut cells, 0, 0, vec![f(base, "d"), f(base, "t")]);
            put(&mut cells, 1, 0, vec![f(base, "d"), f(base, "t")]);
            put(&mut cells, 7, 0, vec![f(base, "d"), f(base, "t")]);
            put(&mut cells, 6, 2, vec![f(&pause, "su")]);
            cells
        }
        Class::RirF(vowel) => {
            // The feminine र्-final nouns of the vowel flavours (गिर्; site
            // @gir1): the nominative and the vocative take the long vowel
            // with the visarga (गीः), the भ-endings the र-form (गीर्भ्याम्)
            // and the locative plural the र्षु (गीर्षु).
            if linga != Linga::F || !stem.ends_with('r') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The र् is dropped and the vowel lengthened (गिर् → गी,
            // पुर् → पू).
            let long = match (base.chars().last(), *vowel) {
                (Some('i'), 'i') => f(&base[..base.len() - 1], "I"),
                (Some('u'), 'u') => f(&base[..base.len() - 1], "U"),
                _ => return Err(NOT_POSSIBLE),
            };
            let mut cells = table(&[
                (f(&long, "H"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&long, "rByAm"), f(&long, "rBiH")),
                (f(stem, "e"), f(&long, "rByAm"), f(&long, "rByaH")),
                (f(stem, "aH"), f(&long, "rByAm"), f(&long, "rByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(&long, "H"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(&long, "rzu")]);
            cells
        }
        Class::RarF => {
            // The feminine र्-final nouns of the plain flavour (द्वार्; site
            // @dvAr1): the nominative takes the आ-form (द्वाः), the
            // भ-endings the र-form (द्वार्भ्याम्) and the locative plural
            // the र्षु (द्वार्षु).
            if linga != Linga::F || !stem.ends_with('r') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The आ-form: the stem's own आ (द्वा) with the visarga.
            let aa = if base.ends_with('A') {
                f(base, "H")
            } else {
                f(base, "aH")
            };
            let mut cells = table(&[
                (aa.clone(), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(base, "rByAm"), f(base, "rBiH")),
                (f(stem, "e"), f(base, "rByAm"), f(base, "rByaH")),
                (f(stem, "aH"), f(base, "rByAm"), f(base, "rByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (aa, f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(base, "rzu")]);
            cells
        }
        Class::RarN => {
            // The neuter र्-final nouns (वार्; site @vAr1): the nominative
            // takes the आ-form (वाः), the dual the ई-form (वारी) and the
            // plural the plain stem (वारि); the भ-endings take the र-form
            // (वार्भ्याम्) and the locative plural the र्षु (वार्षु).
            if linga != Linga::N || !stem.ends_with('r') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let aa = if base.ends_with('A') {
                f(base, "H")
            } else {
                f(base, "aH")
            };
            let mut cells = table(&[
                (aa.clone(), f(base, "rI"), f(stem, "i")),
                (aa.clone(), f(base, "rI"), f(stem, "i")),
                (f(stem, "A"), f(base, "rByAm"), f(base, "rBiH")),
                (f(stem, "e"), f(base, "rByAm"), f(base, "rByaH")),
                (f(stem, "aH"), f(base, "rByAm"), f(base, "rByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (aa, f(base, "rI"), f(stem, "i")),
            ]);
            put(&mut cells, 6, 2, vec![f(base, "rzu")]);
            cells
        }
        Class::BaraM(final_char) => {
            // The masculine nouns whose nominative is the bare stem
            // (कमल्; site @kamal1): the प्रथमा- and सम्बोधन-एकवचन take
            // the stem itself (कमल्), the भ-endings the final consonant
            // (कमल्भ्याम्) and the locative plural the षु-form (कमल्षु).
            // The ण्-final nouns (सुगण्; site @sugaN1) keep three
            // alternants there (सुगण्सु-सुगण्ट्सु-सुगण्ठ्सु).
            if linga != Linga::M || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let Some(c) = stem.chars().last() else {
                return Err(NOT_POSSIBLE);
            };
            if c != *final_char {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let (voiced, loc) = match c {
                'l' => (f(base, "l"), vec![f(base, "lzu")]),
                'R' => (
                    f(base, "R"),
                    vec![f(base, "Rsu"), f(base, "Rwsu"), f(base, "RWsu")],
                ),
                _ => return Err(NOT_POSSIBLE),
            };
            let mut cells = table(&[
                (stem.to_string(), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (stem.to_string(), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 6, 2, loc);
            cells
        }
        Class::HasM => {
            // The masculine ह्-final nouns of the ड/ट flavour (लिह्; site
            // @lih1): the प्रथमा-/सम्बोधन-एकवचन carry the ड/ट pair
            // (लिड्-लिट्), the भ-endings voice the stem (लिड्भ्याम्) and
            // the locative plural the ट्सु pair (लिट्सु-लिट्त्सु).
            if linga != Linga::M || !stem.ends_with('h') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The ड-form carries the षत्व of a स before the ह्
            // (तुरासाह् → तुराषाड्-तुराषाट्).
            let alt = retroflex_cluster(base);
            let voiced = f(&alt, "q");
            let pause = f(&alt, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            // The प्रथमा- and सम्बोधन-एकवचन carry the ड/ट pair; the
            // locative plural the ट्सु pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 6, 2, vec![f(&alt, "wsu"), f(&alt, "wtsu")]);
            cells
        }
        Class::ZagM => {
            // The masculine ष्-final nouns of the ग/क flavour (तक्ष् →
            // तग्-तक्, site @takS2): the ष् becomes the ग्/क् pair, the
            // भ-endings voice the stem (तग्भ्याम्) and the locative plural
            // the क्षु (तक्षु). A क्ष cluster gives up its ष् to the pair
            // (तक्ष् → तक्, तग्); a plain ष् takes the क् (दधृष् →
            // दधृक्, दधृग्).
            if linga != Linga::M || !stem.ends_with('z') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let (voiced, pause) = match base.strip_suffix('k') {
                Some(reduced) => (f(reduced, "g"), base.to_string()),
                None => (f(base, "g"), f(base, "k")),
            };
            // The locative plural reuses the pause consonant: तक्षु,
            // दधृक्षु.
            let loc_plural = if base.ends_with('k') {
                f(base, "zu")
            } else {
                f(base, "kzu")
            };
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), loc_plural),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            // The प्रथमा- and सम्बोधन-एकवचन carry the ग/क pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced, pause]);
            cells
        }
        Class::RcN => {
            // The neuter ऋ-chart nouns (दृश्; site @dRz3): the
            // प्रथमा-/द्वितीया-एकवचन carry the ऋ-chart's pair
            // (दृग्-दृख्-दृक्), the dual the ई-form (दृशी) and the
            // plural the ंशि-form (दृंशि); the भ-endings voice the stem
            // (दृग्भ्याम्) and the locative plural the क्षु (दृक्षु).
            if linga != Linga::N || !stem.ends_with('S') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "g");
            let middle = f(base, "K");
            let pause = f(base, "k");
            let plural = f(base, "MSi");
            let mut cells = table(&[
                (f(stem, "A"), f(stem, "I"), plural.clone()),
                (f(stem, "A"), f(stem, "I"), plural.clone()),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "A"), f(stem, "I"), plural.clone()),
            ]);
            let pair = vec![voiced, middle, pause];
            put(&mut cells, 0, 0, pair.clone());
            put(&mut cells, 1, 0, pair.clone());
            put(&mut cells, 7, 0, pair);
            put(&mut cells, 6, 2, vec![f(base, "kzu")]);
            cells
        }
        Class::DadN(ndi) => {
            // The neuter द्-final nouns (उद्भिद्; site @udbhid2): the
            // प्रथमा-/द्वितीया-एकवचन carry the द/त pair (उद्भिद्-उद्भित्),
            // the dual the ई-form (उद्भिदी) and the plural the न्दि-form
            // (उद्भिन्दि); the भ-endings keep the stem (उद्भिद्भ्याम्) and
            // the locative plural the त्सु (उद्भित्सु).
            if linga != Linga::N || !stem.ends_with('d') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let pair = vec![stem.to_string(), f(base, "t")];
            // The plural is either the न्दि-form (उद्भिन्दि) or the plain
            // stem (बेभिदि) — the site differs per word.
            let plural = if *ndi { f(base, "ndi") } else { f(stem, "i") };
            let mut cells = table(&[
                (stem.to_string(), f(stem, "I"), plural.clone()),
                (stem.to_string(), f(stem, "I"), plural.clone()),
                (f(stem, "A"), f(stem, "ByAm"), f(stem, "BiH")),
                (f(stem, "e"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "aH"), f(stem, "ByAm"), f(stem, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (stem.to_string(), f(stem, "I"), plural.clone()),
            ]);
            put(&mut cells, 0, 0, pair.clone());
            put(&mut cells, 1, 0, pair.clone());
            put(&mut cells, 7, 0, pair);
            put(&mut cells, 6, 2, vec![f(base, "tsu")]);
            cells
        }
        Class::Sam => {
            // The श्-final nouns of the ड/ट flavour (विश्; site @viz1): the
            // प्रथमा-/सम्बोधन-एकवचन carry the ड/ट pair (विड्-विट्), the
            // भ-endings voice the stem (विड्भ्याम्) and the locative plural
            // the ट्सु pair (विट्सु-विट्त्सु); the rest keeps the plain
            // stem (विशौ, विशः).
            if !matches!(linga, Linga::M | Linga::F) || !stem.ends_with('S') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "q");
            let pause = f(base, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 6, 2, vec![f(base, "wsu"), f(base, "wtsu")]);
            cells
        }
        Class::Rc2 => {
            // The ऋ-chart nouns with two alternants (दिश्; site @diz1):
            // the प्रथमा-/सम्बोधन-एकवचन carry the ग/क pair (दिग्-दिक्),
            // the भ-endings the ग-form (दिग्भ्याम्) and the locative plural
            // the क्षु (दिक्षु).
            if !matches!(linga, Linga::M | Linga::F) || !stem.ends_with('S') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "g");
            let pause = f(base, "k");
            let mut cells = table(&[
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
            ]);
            let pair = vec![voiced, pause];
            put(&mut cells, 0, 0, pair.clone());
            put(&mut cells, 7, 0, pair);
            put(&mut cells, 6, 2, vec![f(base, "kzu")]);
            cells
        }
        Class::ZamF => {
            // The feminine ष्-final nouns of the ड/ट flavour (तृष्; site
            // @tRS1): the प्रथमा-/सम्बोधन-एकवचन carry the ड/ट pair
            // (तृड्-तृट्), the भ-endings voice the stem (तृड्भ्याम्) and
            // the locative plural the ट्सु pair (तृट्सु-तृट्त्सु).
            if linga != Linga::F || !stem.ends_with('z') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "q");
            let pause = f(base, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 6, 2, vec![f(base, "wsu"), f(base, "wtsu")]);
            cells
        }
        Class::ZvF(vowel) => {
            // The feminine ष्-final nouns of the vowel flavours: the ष्
            // becomes the stem's own vowel (site @chadiS1/@AziS1/@sajuS1) —
            // the इ (छदिः), the ई (आशीः) or the ऊ (सजूः). The
            // nominative and the vocative take that vowel with the
            // visarga, the भ-endings the र-form (छदिर्भ्याम्) and the
            // locative plural the षु/ष्षु pair (छदिःषु-छदिष्षु).
            if linga != Linga::F || !stem.ends_with('z') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            // The bare stem without its final matra (छदि → छद).
            let short = match base.chars().last() {
                Some('a' | 'A' | 'i' | 'I' | 'u' | 'U' | 'e' | 'E' | 'o' | 'O') => {
                    &base[..base.len() - 1]
                }
                _ => base,
            };
            // The ष् becomes the stem's own vowel: the इ keeps the ि
            // (छदिः), the ई and the ऊ lengthen it (आशीः, सजूः).
            let rep_bare = match *vowel {
                'i' => base.to_string(),
                'I' => f(short, "I"),
                'u' => f(short, "U"),
                _ => return Err(NOT_POSSIBLE),
            };
            let rep = f(&rep_bare, "H");
            let instr = f(&rep_bare, "r");
            let mut cells = table(&[
                (rep.clone(), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&instr, "ByAm"), f(&instr, "BiH")),
                (f(stem, "e"), f(&instr, "ByAm"), f(&instr, "ByaH")),
                (f(stem, "aH"), f(&instr, "ByAm"), f(&instr, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (rep.clone(), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(&rep, "zu"), f(&rep_bare, "zzu")]);
            cells
        }
        Class::Zam => {
            // The masculine ष्-final nouns (द्विष्; site @dviS1): the
            // प्रथमा-एकवचन carries the ड/ट pair (द्विड्-द्विट्), the
            // भ-endings voice the stem (द्विड्भ्याम्) and the locative
            // plural the ट्सु pair (द्विट्सु-द्विट्त्सु). The site's
            // ण-inserted flavour (दोष् → दोष्णः) is a different shape.
            if linga != Linga::M || !stem.ends_with('z') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            // The ष् becomes the ड्/ट् pair; a क्ष cluster reduces with it
            // (तक्ष् → तड्-तट्, द्विष् → द्विड्-द्विट्).
            let mut base = &stem[..stem.len() - 1];
            if let Some(stripped) = base.strip_suffix('k') {
                base = stripped;
            }
            let voiced = f(base, "q");
            let pause = f(base, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            // The प्रथमा- and सम्बोधन-एकवचन carry the ड/ट pair; the
            // locative plural carries the ट्सु pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced, pause]);
            put(&mut cells, 6, 2, vec![f(base, "wsu"), f(base, "wtsu")]);
            cells
        }
        Class::ZamC(flavour) => {
            // The masculine ष्-final nouns of the visarga flavour (चिकीर्ष्;
            // site @cikIrS1): the nominative is the visarga form (चिकीः),
            // the भ-endings the र-form (चिकीर्भ्याम्) and the locative
            // plural the षु-pair (चिकीःषु-चिकीष्षु). The flavour picks
            // the nominative: `C` the plain stem (जगच्चक्षुः), `V` the
            // वृद्धि-form (पिपठीः), `R` the stem without its रुत्व-र्
            // (चिकीः) — and with it the locative plural.
            if linga != Linga::M || !stem.ends_with('z') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let vriddhi = vriddhi_stem(base);
            let (nom, weak) = match *flavour {
                'V' => (format!("{vriddhi}H"), vriddhi),
                'R' => (
                    format!("{}H", base.strip_suffix('r').unwrap_or(base)),
                    base.to_string(),
                ),
                _ => (format!("{base}H"), base.to_string()),
            };
            let bh = if weak.ends_with('r') {
                weak.clone()
            } else {
                format!("{weak}r")
            };
            let mut cells = table(&[
                (nom.clone(), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&bh, "ByAm"), f(&bh, "BiH")),
                (f(stem, "e"), f(&bh, "ByAm"), f(&bh, "ByaH")),
                (f(stem, "aH"), f(&bh, "ByAm"), f(&bh, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (nom, f(stem, "O"), f(stem, "aH")),
            ]);
            // The locative plural: the षु-pair (पिपठीःषु-पिपठीष्षु) or,
            // where the रुत्व-र् drops, the plain षु (चिकीर्षु).
            if *flavour == 'R' {
                put(&mut cells, 6, 2, vec![f(base, "zu")]);
            } else {
                put(&mut cells, 6, 2, vec![f(&weak, "Hzu"), f(&weak, "zzu")]);
            }
            cells
        }
        Class::AcN => {
            // The neuter च्-final nouns (अर्वाच्; site @arvAc1): the
            // प्रथमा-एकवचन carries the ग/क pair (अर्वाग्-अर्वाक्), the
            // dual/plural the ई-forms (अर्वाची) and the ञ्चि plural
            // (अर्वाञ्चि), the भ-endings voice the stem (अर्वाग्भ्याम्) and
            // the locative plural the क्षु (अर्वाक्षु).
            if linga != Linga::N || !stem.ends_with('c') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "g");
            let pause = f(base, "k");
            let mut cells = table(&[
                (f(stem, "I"), f(stem, "I"), f(base, "Yci")),
                (f(stem, "I"), f(stem, "I"), f(base, "Yci")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(base, "kzu")),
                (f(stem, "I"), f(stem, "I"), f(base, "Yci")),
            ]);
            // The प्रथमा-, द्वितीया- and सम्बोधन-एकवचन carry the ग/क pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 1, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced, pause]);
            cells
        }
        Class::AcNV(flavour) => {
            // The neuter च्-final nouns whose weak stem is not the plain
            // stem (न्यच् → नीची; site @nyac2): the strong cells keep the
            // ञ्च-forms (न्यञ्चि) and the ग/क pair in the nominative
            // (न्यग्-न्यक्), the weak cells the वृद्धि-form (नीचा) or the
            // ओ-form (गोचा) — the flavour picks the weak stem.
            if linga != Linga::N || !stem.ends_with('c') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let weak = if *flavour == 'O' {
                ac_weak_o(stem)
            } else {
                ac_weak(base, *flavour)
            };
            let mut cells = table(&[
                (f(base, "g"), f(&weak, "I"), f(base, "Yci")),
                (f(base, "g"), f(&weak, "I"), f(base, "Yci")),
                (f(&weak, "A"), f(base, "gByAm"), f(base, "gBiH")),
                (f(&weak, "e"), f(base, "gByAm"), f(base, "gByaH")),
                (f(&weak, "aH"), f(base, "gByAm"), f(base, "gByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(base, "kzu")),
                (f(base, "g"), f(&weak, "I"), f(base, "Yci")),
            ]);
            // The प्रथमा-, द्वितीया- and सम्बोधन-एकवचन carry the ग/क pair.
            put(&mut cells, 0, 0, vec![f(base, "g"), f(base, "k")]);
            put(&mut cells, 1, 0, vec![f(base, "g"), f(base, "k")]);
            put(&mut cells, 7, 0, vec![f(base, "g"), f(base, "k")]);
            cells
        }
        Class::AncN => {
            // The neuter न्च-final nouns of the ङ्-inserted family (गोन्च्;
            // site @gonc1): the nominative takes the ङ्-inserted form
            // (गोङ्), the dual and plural the ञ्च-forms (गोञ्ची, गोञ्चि),
            // the भ-endings the ङ्-form (गोङ्भ्याम्) and the locative
            // plural the three ङ्-क्षu alternants (गोङ्षु-गोङ्क्षु-गोङ्ख्षु).
            if linga != Linga::N || !stem.ends_with("nc") || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 2];
            let ang = f(base, "N");
            let mut cells = table(&[
                (ang.clone(), f(base, "YcI"), f(base, "Yci")),
                (ang.clone(), f(base, "YcI"), f(base, "Yci")),
                (f(base, "YcA"), f(&ang, "ByAm"), f(&ang, "BiH")),
                (f(base, "Yce"), f(&ang, "ByAm"), f(&ang, "ByaH")),
                (f(base, "YcaH"), f(&ang, "ByAm"), f(&ang, "ByaH")),
                (f(base, "YcaH"), f(base, "YcoH"), f(base, "YcAm")),
                (f(base, "Yci"), f(base, "YcoH"), f(&ang, "su")),
                (ang.clone(), f(base, "YcI"), f(base, "Yci")),
            ]);
            put(
                &mut cells,
                6,
                2,
                vec![f(&ang, "zu"), f(&ang, "kzu"), f(&ang, "Kzu")],
            );
            cells
        }
        Class::AcM2(pair) => {
            // The masculine च्-final nouns of the pair flavour (जलमुच्;
            // site @jalamuc1): the प्रथमा-/सम्बोधन-एकवचन carry the
            // ग/क pair (जलमुग्-जलमुक्) — or the ड/ट pair (सुवृड्-सुवृट्) —
            // the भ-endings voice the stem (जलमुग्भ्याम्) and the locative
            // plural the क्षु (जलमुक्षु).
            if linga != Linga::M || !stem.ends_with('c') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            // The ड/ट flavour replaces the whole श्च cluster (सुवृश्च् →
            // सुवृड्), the ग/क flavour only the च्.
            let base = match *pair {
                'g' => &stem[..stem.len() - 1],
                'q' => {
                    // The श्च cluster (सुवृश्च्) gives up both letters.
                    if !stem.ends_with("Sc") {
                        return Err(NOT_POSSIBLE);
                    }
                    &stem[..stem.len() - 2]
                }
                _ => return Err(NOT_POSSIBLE),
            };
            let (voiced, pause, loc) = match *pair {
                'g' => (f(base, "g"), f(base, "k"), vec![f(base, "kzu")]),
                _ => (
                    f(base, "q"),
                    f(base, "w"),
                    vec![f(base, "wsu"), f(base, "wtsu")],
                ),
            };
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            let forms = vec![voiced, pause];
            put(&mut cells, 0, 0, forms.clone());
            put(&mut cells, 7, 0, forms);
            put(&mut cells, 6, 2, loc);
            cells
        }
        Class::AccM => {
            // Masculine च्-final nouns (अर्वाच्; site @arvAc2): the
            // nominative takes the ङ्-inserted form (अर्वाङ्), the
            // dual/plural the ञ्च-forms (अर्वाञ्चौ, अर्वाञ्चः), the
            // भ-endings voice the stem (अर्वाग्भ्याम्) and the locative
            // plural the क्षु (अर्वाक्षु).
            if linga != Linga::M || !stem.ends_with('c') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            table(&[
                (f(base, "N"), f(base, "YcO"), f(base, "YcaH")),
                (f(base, "Ycam"), f(base, "YcO"), f(stem, "aH")),
                (f(stem, "A"), f(base, "gByAm"), f(base, "gBiH")),
                (f(stem, "e"), f(base, "gByAm"), f(base, "gByaH")),
                (f(stem, "aH"), f(base, "gByAm"), f(base, "gByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(base, "kzu")),
                (f(base, "N"), f(base, "YcO"), f(base, "YcaH")),
            ])
        }
        Class::AccMV(flavour) => {
            // The masculine च्-final nouns of the ञ्च-स्वादि family whose
            // weak stem is the वृद्धि-form (उदच्; site @udac1): the
            // strong cells keep the ञ्च-forms (उदञ्चौ), the weak cells
            // the वृद्धि-form (उदीचः). The weak stem is the citation
            // minus its च, minus the trailing vowel, minus a trailing
            // semi-vowel, plus the flavour's consonant: उदच् → उद + ईच,
            // न्यच् → न + ईच, अन्वच् → अन + ऊच, तिर्यच् → तिर + श्च.
            if linga != Linga::M || !stem.ends_with('c') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let weak = ac_weak(base, *flavour);
            table(&[
                (f(base, "N"), f(base, "YcO"), f(base, "YcaH")),
                (f(base, "Ycam"), f(base, "YcO"), f(&weak, "aH")),
                (f(&weak, "A"), f(base, "gByAm"), f(base, "gBiH")),
                (f(&weak, "e"), f(base, "gByAm"), f(base, "gByaH")),
                (f(&weak, "aH"), f(base, "gByAm"), f(base, "gByaH")),
                (f(&weak, "aH"), f(&weak, "oH"), f(&weak, "Am")),
                (f(&weak, "i"), f(&weak, "oH"), f(base, "kzu")),
                (f(base, "N"), f(base, "YcO"), f(base, "YcaH")),
            ])
        }
        Class::JasM => {
            // The masculine ज्-final nouns of the ड/ट flavour (राज्; site
            // @rAj1): the nominative carries the ड/ट pair (राड्-राट्), the
            // भ-endings voice the stem (राड्भ्याम्) and the locative plural
            // the ट्सु pair (राट्सु-राट्त्सु) — the ड/ट flavour the
            // masculine ष्-final nouns have.
            if linga != Linga::M || !stem.ends_with('j') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "q");
            let pause = f(base, "w");
            let mut cells = table(&[
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
            ]);
            // The प्रथमा- and सम्बोधन-एकवचन carry the ड/ट pair; the
            // locative plural carries the ट्सु pair.
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced.clone(), pause]);
            put(&mut cells, 6, 2, vec![f(base, "wsu"), f(base, "wtsu")]);
            cells
        }
        Class::AjjM => {
            // Masculine ज्-final nouns (खन्ज्; site @khanj1): the
            // nominative is the bare stem (खन्), the dual/plural the
            // ञ्ज-forms (खञ्जौ, खञ्जः), the locative plural the pair
            // खन्सु-खन्त्सु.
            if linga != Linga::M || !stem.ends_with("nj") || stem.len() < 4 {
                return Err(NOT_POSSIBLE);
            }
            // The ज्-final nouns replace their न् with the ञ्: खन्ज् →
            // खन् (nominative) and खञ्जौ (dual), so the base drops the
            // न्ज and the ञ्ज-forms build on what is left.
            let base = &stem[..stem.len() - 2];
            let mut cells = table(&[
                (f(base, "n"), f(base, "YjO"), f(base, "YjaH")),
                (f(base, "Yjam"), f(base, "YjO"), f(base, "YjaH")),
                (
                    f(base, "YjA"),
                    f(&format!("{base}n"), "ByAm"),
                    f(&format!("{base}n"), "BiH"),
                ),
                (
                    f(base, "Yje"),
                    f(&format!("{base}n"), "ByAm"),
                    f(&format!("{base}n"), "ByaH"),
                ),
                (
                    f(base, "YjaH"),
                    f(&format!("{base}n"), "ByAm"),
                    f(&format!("{base}n"), "ByaH"),
                ),
                (f(base, "YjaH"), f(base, "YjoH"), f(base, "YjAm")),
                (f(base, "Yji"), f(base, "YjoH"), f(base, "zu")),
                (f(base, "n"), f(base, "YjO"), f(base, "YjaH")),
            ]);
            // The locative plural carries both alternants (खन्सु-खन्त्सु).
            put(
                &mut cells,
                6,
                2,
                vec![f(&format!("{base}n"), "su"), f(base, "ntsu")],
            );
            cells
        }
        Class::AsAnM => {
            // The anusvāra-स्वादि (अणीयस्; site @aNIyas1): the nominal
            // forms take the anusvāra (अणीयान्, अणीयांसौ, अणीयांसः), the
            // rest the स-forms (अणीयसा, अणीयसे, अणीयसः).
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            let mut cells = table(&[
                (f(q, "An"), f(q, "AMsO"), f(q, "AMsaH")),
                (f(q, "AMsam"), f(q, "AMsO"), f(q, "asaH")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(q, "an"), f(q, "AMsO"), f(q, "AMsaH")),
            ]);
            // The locative plural carries both alternants (अनःसु-अनस्सु).
            put(&mut cells, 6, 2, vec![f(q, "aHsu"), f(&w, "su")]);
            cells
        }
        Class::AsEM => {
            // The e-form of the अस्-स्वादि (अनेहस्; site @anehas1): the
            // nominative is the bare stem (अनेहा).
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            let mut cells = table(&[
                (f(q, "A"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(q, "aH"), f(&w, "O"), f(&w, "aH")),
            ]);
            // The locative plural carries both alternants (अनेहःसु-अनेहस्सु).
            put(&mut cells, 6, 2, vec![f(q, "aHsu"), f(&w, "su")]);
            cells
        }
        Class::TaN2(flavour) => {
            // The neuter of a त-स्वादि stem in the site's mixed flavours
            // (तुदत्; site @tudat1/@dadat2/@mahat2): the प्रथमा-/
            // द्वितीया-एकवचन carry the द/त pair as always, but the dual
            // or the plural adds its own alternant — the न्ती-pair
            // (तुदन्ती-तुदती), the न्ति-pair (ददन्ति-ददति) or the
            // आ-form (महान्ति).
            if linga != Linga::N || !stem.ends_with('t') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let voiced = f(base, "d");
            let plural = match *flavour {
                'P' => vec![f(base, "nti"), f(stem, "i")],
                'A' => vec![f(stem_before_vowel(base), "Anti")],
                _ => vec![f(base, "nti")],
            };
            let mut cells = table(&[
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
                (f(stem, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(stem, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(stem, "I"), f(stem, "I"), f(base, "nti")),
            ]);
            put(&mut cells, 0, 0, vec![voiced.clone(), stem.to_string()]);
            put(&mut cells, 1, 0, vec![voiced.clone(), stem.to_string()]);
            put(&mut cells, 7, 0, vec![voiced, stem.to_string()]);
            put(&mut cells, 0, 2, plural.clone());
            put(&mut cells, 1, 2, plural.clone());
            put(&mut cells, 7, 2, plural);
            if *flavour == 'D' {
                // The न्ती-pair in the dual (तुदन्ती-तुदती).
                let pair = vec![f(base, "ntI"), f(stem, "I")];
                put(&mut cells, 0, 1, pair.clone());
                put(&mut cells, 1, 1, pair.clone());
                put(&mut cells, 7, 1, pair);
            }
            cells
        }
        Class::AsM | Class::AsF => {
            // The अस्-स्वादि in the masculine (सुमनसः, site @sumanas1) and
            // the feminine (सदाः, site @sadas2) — the same shape: the
            // प्रथमा-एकवचन has the आ-form, the भ-endings the ओ-form
            // (सदोभ्याम्), and the vocative singular the स-form (हे सदः).
            let ok_linga = match class {
                Class::AsM => linga == Linga::M,
                _ => linga == Linga::F,
            };
            if !ok_linga || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2 {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            let mut cells = table(&[
                (f(q, "AH"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                // The vocative singular takes the स-form (हे अङ्गिरः) where
                // the nominative has the आ-form (अङ्गिराः) — site @aGgiras1.
                (f(q, "aH"), f(&w, "O"), f(&w, "aH")),
            ]);
            // The locative plural carries both alternants (अनःसु-अनस्सु,
            // site @anas1): the visarga form and the doubled स् form.
            put(&mut cells, 6, 2, vec![f(q, "aHsu"), f(&w, "su")]);
            cells
        }
        Class::AsMV => {
            // The अस्-स्वादि masculine of the plain flavour (पिण्डग्रस्;
            // site @piNDagras1): the nominative is the visarga form
            // (पिण्डग्रः) where the आ-flavour writes the आ-form
            // (पिण्डग्राः), the भ-endings the ओ-form (पिण्डग्रोभ्याम्)
            // and the locative plural both alternants
            // (पिण्डग्रःसु-पिण्डग्रस्सु).
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            let q = &w[..w.len() - 2];
            let o = format!("{q}o");
            let mut cells = table(&[
                (f(q, "aH"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&o, "ByAm"), f(&o, "BiH")),
                (f(&w, "e"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&o, "ByAm"), f(&o, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(q, "aH"), f(&w, "O"), f(&w, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(q, "aHsu"), f(&w, "su")]);
            cells
        }
        Class::AsMP => {
            // The अस्-स्वादि masculine of the pause flavour (ध्वस्; site
            // @dhvas1): the nominative carries the द्/त् pair
            // (ध्वद्-ध्वत्), the भ-endings the द-form (ध्वद्भ्याम्) and
            // the locative plural the त्सु (ध्वत्सु).
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            // The stem without its स् — the inherent vowel kept, so the
            // pause forms keep their halant (ध्वद्).
            let base = &w[..w.len() - 1];
            let voiced = f(base, "d");
            let pause = f(base, "t");
            let mut cells = table(&[
                (f(base, "AH"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&voiced, "ByAm"), f(&voiced, "BiH")),
                (f(&w, "e"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&w, "aH"), f(&voiced, "ByAm"), f(&voiced, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(base, "aH"), f(&w, "O"), f(&w, "aH")),
            ]);
            put(&mut cells, 0, 0, vec![voiced.clone(), pause.clone()]);
            put(&mut cells, 7, 0, vec![voiced, pause]);
            put(&mut cells, 6, 2, vec![f(base, "tsu")]);
            cells
        }
        Class::AsMW => {
            // The अस्-स्वादि masculine of the वृद्धि flavour (सुतुस्; site
            // @sutus1): the nominative takes the वृद्धि-form of the stem
            // (सुतूः), the भ-endings the वृद्धि + the र् (सुतूर्भ्याम्)
            // and the locative plural both alternants (सुतूःषु-सुतूष्षु).
            if linga != Linga::M || !(stem.ends_with('s') || stem.ends_with('H')) || stem.len() < 2
            {
                return Err(NOT_POSSIBLE);
            }
            let w = normalize_visarga(stem);
            // The stem without its स् — the inherent vowel kept, so the
            // वृद्धि-form ends on the stem's own vowel (सुतू).
            let v = vriddhi_stem(&w[..w.len() - 1]);
            let bh = format!("{v}r");
            let mut cells = table(&[
                (f(&v, "H"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "am"), f(&w, "O"), f(&w, "aH")),
                (f(&w, "A"), f(&bh, "ByAm"), f(&bh, "BiH")),
                (f(&w, "e"), f(&bh, "ByAm"), f(&bh, "ByaH")),
                (f(&w, "aH"), f(&bh, "ByAm"), f(&bh, "ByaH")),
                (f(&w, "aH"), f(&w, "oH"), f(&w, "Am")),
                (f(&w, "i"), f(&w, "oH"), f(&w, "su")),
                (f(&v, "H"), f(&w, "O"), f(&w, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(&v, "Hzu"), f(&v, "zzu")]);
            cells
        }
        Class::SasV => {
            // The स्-final masculine nouns of the visarga flavour (ओकास्;
            // site @okAs1): the nominative is the visarga form (ओकाः)
            // where the अस्-स्वादि writes the आ-form, the भ-endings take
            // the stem without the ओ-augment (ओकाभ्याम्) and the
            // locative plural both alternants (ओकाःसु-ओकास्सु).
            if !matches!(linga, Linga::M | Linga::F) || !stem.ends_with('s') || stem.len() < 3 {
                return Err(NOT_POSSIBLE);
            }
            let base = &stem[..stem.len() - 1];
            let mut cells = table(&[
                (f(base, "H"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
                (f(stem, "A"), f(base, "ByAm"), f(base, "BiH")),
                (f(stem, "e"), f(base, "ByAm"), f(base, "ByaH")),
                (f(stem, "aH"), f(base, "ByAm"), f(base, "ByaH")),
                (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
                (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
                (f(base, "H"), f(stem, "O"), f(stem, "aH")),
            ]);
            put(&mut cells, 6, 2, vec![f(base, "Hsu"), f(stem, "su")]);
            cells
        }
        Class::VasM => {
            if linga != Linga::M || !stem.ends_with("vas") || stem.len() < 4 {
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
            if linga != Linga::N || !stem.ends_with("vas") || stem.len() < 4 {
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
        Class::SiteGrid(i) => {
            let (_, _, spec) = SITE_GRIDS[*i];
            return Ok(Paradigm {
                stem: stem.to_string(),
                linga,
                class: class.clone(),
                cells: site_grid_cells(spec),
            });
        }
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
    let instr_in = matches!(
        class,
        Class::AM | Class::AN | Class::IMF | Class::UMF | Class::OPlur(_, _)
    );
    let genpl_nam = matches!(
        class,
        Class::AM
            | Class::AN
            | Class::AAF
            | Class::IIF
            | Class::IIFN
            | Class::IIM
            | Class::IIM2(_)
            | Class::IMF
            | Class::UMF
            | Class::UF
            | Class::UUF
            | Class::UUFN
            | Class::GatiF
            | Class::OPlur(_, _)
    );
    // The neuter agent nouns (धातृ → धातृणी) are not इगन्त: the site
    // keeps the dental (क्षन्तृ → क्षन्तॄणि), so the णत्व trigger that
    // sits inside the क्ष conjunct must not fire on them. The neuter of
    // a त-स्वादि stem (बृहत् → बृहन्ति) is the same: the न belongs to
    // the सर्वधातुकारी, not to an affix of its own. The अस्-स्वादि
    // neuters (अन्तर्मनस् → अन्तर्मनांसि) carry their own न the same
    // way.
    let neuter_n_affix = linga == Linga::N
        && !matches!(
            class,
            Class::AtN(_)
                | Class::RFN
                | Class::VantN
                | Class::DadN(_)
                | Class::TaN
                | Class::TaN2(_)
                | Class::OPlur(_, _)
                | Class::AsN
                | Class::AsAnM
        );
    // The नत्व of an न्-final प्रातिपदिक reaches that प्रातिपदिक's own
    // न् — where the न् drops to a न (the ङ्वनिप् drop family: उक्षन् →
    // उक्ष्णः) the न sits one slot earlier. The weak stem of the इ-स्वादि
    // has dropped the न् altogether, so the root's own न keeps its dental
    // (कर्मन्दिभ्याम्, परिपन्थिभ्याम्).
    let nan_pos: Vec<usize> = if all && stem.ends_with('n') {
        let len = stem.len();
        let mut slots = vec![len - 1];
        if len >= 2 && stem.as_bytes()[len - 2] == b'a' {
            slots.push(len - 2);
        }
        slots
    } else {
        Vec::new()
    };
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
                    } else if nan_pos.is_empty() {
                        site_natva(&s, ru_tva)
                    } else {
                        match s.rfind('n') {
                            Some(q) if nan_pos.contains(&q) => site_natva_at(&s, ru_tva, q),
                            _ => s,
                        }
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
    site_natva_at(s, ru_tva, pos)
}

/// The same णत्व step, but aimed at a known `न्` position: the
/// प्रातिपदिक's own final न् (the site's नत्व reaches that न् alone —
/// the weak stem has dropped it, so an internal न keeps its dental:
/// कर्मन्दिभ्याम्, परिपन्थिभ्याम्).
fn site_natva_at(s: &str, ru_tva: bool, pos: usize) -> String {
    if pos >= s.len() {
        return s.to_string();
    }
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

/// The weak stem of the ञ्च-स्वादि च्-final nouns: the citation minus
/// its च, minus the trailing vowel, minus a trailing semi-vowel, plus
/// the flavour's consonant — उदच् → उद + ईच, न्यच् → न + ईच,
/// अन्वच् → अन + ऊच, तिर्यच् → तिर + श्च.
fn ac_weak(base: &str, flavour: char) -> String {
    let core = base.trim_end_matches(['a', 'A', 'i', 'I', 'u', 'U']);
    match flavour {
        'U' => format!("{}Uc", core.strip_suffix('v').unwrap_or(core)),
        'S' => {
            // The परसवर्ण: the य → the श with the vowel before it, so
            // the र keeps its inherent `a` and takes no halant
            // (तिर्यच् → तिरश्च).
            let head = core.strip_suffix('y').unwrap_or(core);
            format!("{head}aSc")
        }
        _ => {
            let head = core
                .strip_suffix('y')
                .or_else(|| core.strip_suffix('v'))
                .unwrap_or(core);
            format!("{head}Ic")
        }
    }
}

/// The ओ-form of the weak stem of the च्-final nouns: the व + अ → the
/// ओ (the संधि) and the च् → the च — गवाच् → गोच्, गोअच् → गोच्,
/// गोच् → गोच्.
fn ac_weak_o(stem: &str) -> String {
    let head = &stem[..stem.len() - 1];
    let head = match head.strip_suffix("vA") {
        // The संधि: the व + the आ → the ओ, whose matra takes the व's
        // place (गवाच् → गोच्).
        Some(h) => {
            let h = h.strip_suffix('a').unwrap_or(h);
            format!("{h}o")
        }
        None => head.strip_suffix('a').unwrap_or(head).to_string(),
    };
    format!("{head}c")
}

fn f(base: &str, ending: &str) -> String {
    format!("{base}{ending}")
}

/// A stem without the inherent vowel a final consonant carries in SLP1,
/// for an ending that starts with a vowel: the neuter आ-form spells
/// तुदान्ति, not तुदआन्ति.
fn stem_before_vowel(base: &str) -> &str {
    if let Some(stripped) = base.strip_suffix('a') {
        if stripped.ends_with(|c: char| c.is_ascii_alphabetic() && !"aAiIuUfFxXeEoO".contains(c)) {
            return stripped;
        }
    }
    base
}

/// The वृद्धि-form of a stem: its last vowel takes the वृद्धि (the short
/// vowel lengthened) — सुतु → सुतू, सुपि → सुपी.
fn vriddhi_stem(base: &str) -> String {
    let mut out: Vec<char> = Vec::with_capacity(base.len());
    let mut done = false;
    for ch in base.chars().rev() {
        if !done {
            let long = match ch {
                'a' => Some('A'),
                'i' => Some('I'),
                'u' => Some('U'),
                _ => None,
            };
            if let Some(v) = long {
                out.push(v);
                done = true;
                continue;
            }
            if "AIUfFxeEoO".contains(ch) {
                done = true;
            }
        }
        out.push(ch);
    }
    out.reverse();
    out.into_iter().collect()
}

/// The अधिक-compounded युष्मद् (अतियुष्मद्): the base in front of the
/// pronoun's त्व/युव/युष्म-forms, one form per cell. The site keeps the
/// first flavour's forms in the nominative singular (अतित्वम्), the
/// nominative plural (अतियूयम्), the dative singular (अतितुभ्यम्) and
/// the genitive singular (अतितव); the dative plural drops the आ
/// (अतित्वभ्यम्) and the ablative singular and plural carry both
/// alternants (अतित्वद्-अतित्वत्).
fn yuz_compound_cells(base: &str, pstem: &str) -> Vec<Slot> {
    let dual = f(base, &format!("{pstem}Am"));
    let mut cells = table(&[
        // प्रथमा — the singular and the plural keep the first flavour.
        (f(base, "tvam"), dual.clone(), f(base, "yUyam")),
        // द्वितीया — the accusative singular takes the dual form.
        (dual.clone(), dual.clone(), f(base, &format!("{pstem}An"))),
        (
            f(base, &format!("{pstem}ayA")),
            f(base, &format!("{pstem}AByAm")),
            f(base, &format!("{pstem}ABiH")),
        ),
        // चतुर्थी — the singular keeps the first flavour, the plural
        // drops the आ (अतित्वभ्यम्).
        (
            f(base, "tuByam"),
            f(base, &format!("{pstem}AByAm")),
            f(base, &format!("{pstem}aByam")),
        ),
        (
            f(base, &format!("{pstem}ad")),
            f(base, &format!("{pstem}AByAm")),
            f(base, &format!("{pstem}at")),
        ),
        // षष्ठी — the singular keeps the first flavour.
        (
            f(base, "tava"),
            f(base, &format!("{pstem}ayoH")),
            f(base, &format!("{pstem}Akam")),
        ),
        (
            f(base, &format!("{pstem}ayi")),
            f(base, &format!("{pstem}ayoH")),
            f(base, &format!("{pstem}Asu")),
        ),
        // सम्बोधना — the nominative forms again.
        (f(base, "tvam"), dual.clone(), f(base, "yUyam")),
    ]);
    // The ablative singular and plural carry both alternants
    // (अतित्वद्-अतित्वत्).
    let abl = vec![
        f(base, &format!("{pstem}ad")),
        f(base, &format!("{pstem}at")),
    ];
    put(&mut cells, 4, 0, abl.clone());
    put(&mut cells, 4, 2, abl);
    cells
}

/// The अधिक-compounded अस्मद् (अत्यस्मद्): the base in front of the
/// pronoun's म/आव/स्म-forms. The म-forms take the plain अति
/// (अतिमाम्), the आव-forms the sandhi य with the आ matra
/// (अत्यावाम्), the स्म-forms the sandhi य with its inherent written
/// (अत्यस्माम्). The site keeps the nominative singular and plural at
/// the अस्मद् forms (अत्यहम्, अतिवयम्) and the dative and genitive
/// singular at the म-forms (अतिमह्यम्, अतिमम).
fn asm_compound_cells(ati: &str, sandhi: &str, pstem: &str) -> Vec<Slot> {
    let fbase = match pstem {
        "m" => ati.to_string(),
        "Av" => sandhi.to_string(),
        _ => format!("{sandhi}a"),
    };
    // The cells the site keeps at the अस्मद् and the म-forms.
    let nom_sg = f(&format!("{sandhi}a"), "ham");
    let nom_pl = f(ati, "vayam");
    let dat_sg = f(ati, "mahyam");
    let gen_sg = f(ati, "mama");
    let dual = f(&fbase, &format!("{pstem}Am"));
    let mut cells = table(&[
        (nom_sg.clone(), dual.clone(), nom_pl.clone()),
        (dual.clone(), dual.clone(), f(&fbase, &format!("{pstem}An"))),
        (
            f(&fbase, &format!("{pstem}ayA")),
            f(&fbase, &format!("{pstem}AByAm")),
            f(&fbase, &format!("{pstem}ABiH")),
        ),
        (
            dat_sg.clone(),
            f(&fbase, &format!("{pstem}AByAm")),
            f(&fbase, &format!("{pstem}aByam")),
        ),
        (
            f(&fbase, &format!("{pstem}ad")),
            f(&fbase, &format!("{pstem}AByAm")),
            f(&fbase, &format!("{pstem}at")),
        ),
        (
            gen_sg.clone(),
            f(&fbase, &format!("{pstem}ayoH")),
            f(&fbase, &format!("{pstem}Akam")),
        ),
        (
            f(&fbase, &format!("{pstem}ayi")),
            f(&fbase, &format!("{pstem}ayoH")),
            f(&fbase, &format!("{pstem}Asu")),
        ),
        (nom_sg.clone(), dual.clone(), nom_pl.clone()),
    ]);
    let abl = vec![
        f(&fbase, &format!("{pstem}ad")),
        f(&fbase, &format!("{pstem}at")),
    ];
    put(&mut cells, 4, 0, abl.clone());
    put(&mut cells, 4, 2, abl);
    cells
}

/// The pronoun grids the पुरुष compounds (परमयुष्मद्, परमास्मद्) use:
/// the pronoun's own cells with their alternants, the base on every
/// alternative (परमत्वाम्-परमत्वा).
const YUZMAD_PARA_GRID: [&[&str]; 21] = [
    &["tvam"],
    &["yuvAm"],
    &["yUyam"],
    &["tvAm", "tvA"],
    &["yuvAm", "vAm"],
    &["yuzmAn", "vaH"],
    &["tvayA"],
    &["yuvAByAm"],
    &["yuzmABiH"],
    &["tuByam", "te"],
    &["yuvAByAm", "vAm"],
    &["yuzmaByam", "vaH"],
    &["tvad", "tvat"],
    &["yuvAByAm"],
    &["yuzmad", "yuzmat"],
    &["tava", "te"],
    &["yuvayoH", "vAm"],
    &["yuzmAkam", "vaH"],
    &["tvayi"],
    &["yuvayoH"],
    &["yuzmAsu"],
];

/// The अस्मद् grid the पुरुष compound (परमास्मद्) uses.
const ASMAD_PARA_GRID: [&[&str]; 21] = [
    &["aham"],
    &["AvAm"],
    &["vayam"],
    &["mAm", "mA"],
    &["AvAm", "nO"],
    &["asmAn", "naH"],
    &["mayA"],
    &["AvAByAm"],
    &["asmABiH"],
    &["mahyam", "me"],
    &["AvAByAm", "nO"],
    &["asmaByam", "naH"],
    &["mad", "mat"],
    &["AvAByAm"],
    &["asmad", "asmat"],
    &["mama", "me"],
    &["AvayoH", "nO"],
    &["asmAkam", "naH"],
    &["mayi"],
    &["AvayoH"],
    &["asmAsu"],
];

/// The पुरुष-compounded pronouns (परमयुष्मद्, परमास्मद्): the
/// pronoun's own grid with its alternants, the base on every
/// alternative. The सम्बोधना row is empty on the site.
fn pron_para_cells(plain: &str, sandhi: &str, pron: &str) -> Vec<Slot> {
    let grid: &[&[&str]] = if pron == "yuzmad" {
        &YUZMAD_PARA_GRID
    } else {
        &ASMAD_PARA_GRID
    };
    let empty: Vec<(String, String, String)> = (0..8)
        .map(|_| (String::new(), String::new(), String::new()))
        .collect();
    let mut cells = table(&empty);
    for (i, alternatives) in grid.iter().enumerate() {
        let forms: Vec<String> = alternatives
            .iter()
            .map(|alt| para_form(plain, sandhi, alt))
            .collect();
        put(&mut cells, i / 3, i % 3, forms);
    }
    put(&mut cells, 7, 0, Vec::new());
    put(&mut cells, 7, 1, Vec::new());
    put(&mut cells, 7, 2, Vec::new());
    cells
}

/// The bases of a पुरुष-compounded pronoun: the plain base (परम) and
/// the sandhi form (परमा), the one the site takes before a vowel
/// (परम + अहम् → परमाहम्).
fn para_bases(stem: &str) -> Option<(String, String)> {
    // परमयुष्मद् keeps the युष्मद् as it is; परमास्मद् elides the अ
    // of अस्मद् into the आ of परमा.
    let base = stem
        .strip_suffix("yuzmad")
        .or_else(|| stem.strip_suffix("smad"))?;
    // The matra of the sandhi gives the consonant its inherent back.
    let plain = match base.strip_suffix('A') {
        Some(rest) => format!("{rest}a"),
        None => base.to_string(),
    };
    let sandhi = format!("{}A", plain.strip_suffix('a').unwrap_or(&plain));
    Some((plain, sandhi))
}

/// One alternative of a पुरुष-compounded pronoun: the site's sandhi of
/// the base and the pronoun's form — a vowel-initial form takes the
/// matra and elides its vowel (परम + अहम् → परमाहम्), a
/// consonant-initial one the plain base (परम + माम् → परममाम्).
fn para_form(plain: &str, sandhi: &str, alt: &str) -> String {
    let mut chars = alt.chars();
    match chars.next() {
        Some('a' | 'A' | 'i' | 'I' | 'u' | 'U' | 'f' | 'F' | 'x' | 'X' | 'e' | 'E' | 'o' | 'O') => {
            format!("{sandhi}{}", chars.as_str())
        }
        _ => format!("{plain}{alt}"),
    }
}

/// The ह्-final shape: the plain stem everywhere, except the
/// प्रथमा-/सम्बोधन-एकवचन (the `pair`), the भ-endings (the `voiced`
/// consonant) and the locative plural (the `loc` forms), which the
/// caller supplies.
fn ha_final_cells(stem: &str, pair: Vec<String>, voiced: &str, loc: Vec<String>) -> Vec<Slot> {
    let mut cells = table(&[
        (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
        (f(stem, "am"), f(stem, "O"), f(stem, "aH")),
        (f(stem, "A"), f(voiced, "ByAm"), f(voiced, "BiH")),
        (f(stem, "e"), f(voiced, "ByAm"), f(voiced, "ByaH")),
        (f(stem, "aH"), f(voiced, "ByAm"), f(voiced, "ByaH")),
        (f(stem, "aH"), f(stem, "oH"), f(stem, "Am")),
        (f(stem, "i"), f(stem, "oH"), f(stem, "su")),
        (f(stem, "aH"), f(stem, "O"), f(stem, "aH")),
    ]);
    put(&mut cells, 0, 0, pair.clone());
    put(&mut cells, 7, 0, pair);
    put(&mut cells, 6, 2, loc);
    cells
}

/// The retroflex of the consonant cluster that stood before the ह्.
///
/// The site keeps two stem forms for these words and uses the retroflexed
/// one only in the cells where the ह् itself changes: the स before the
/// ह् becomes the ष (8.4.40 — तुरासाह् → तुराषाड्) and the द before it
/// becomes the ध (दुह् → धुग्).
fn retroflex_cluster(base: &str) -> String {
    let bytes = base.as_bytes();
    let is_vowel = |b: u8| {
        matches!(
            b,
            b'a' | b'A' | b'i' | b'I' | b'u' | b'U' | b'e' | b'E' | b'o' | b'O'
        )
    };
    // The last vowel of the base; the cluster before it is the retroflexed
    // one.
    let Some(vowel) = bytes.iter().rposition(|b| is_vowel(*b)) else {
        return base.to_string();
    };
    let mut start = vowel;
    while start > 0 && !is_vowel(bytes[start - 1]) {
        start -= 1;
    }
    if start == vowel {
        return base.to_string();
    }
    // The consonant right before the vowel is the retroflexed one; in a
    // द्र cluster the द is (द्रुह् → ध्रुह्).
    let last = vowel - 1;
    let target = if bytes[last] == b'r' && last > 0 && bytes[last - 1] == b'd' {
        last - 1
    } else {
        last
    };
    let retro = match bytes[target] {
        b's' => Some('z'),
        b'd' => Some('D'),
        _ => None,
    };
    let Some(repl) = retro else {
        return base.to_string();
    };
    let mut out = base[..target].to_string();
    out.push(repl);
    out.push_str(&base[target + 1..]);
    out
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
            ["sarvasmAd/sarvasmAt", "sarvAByAm", "sarveByaH"],
            ["sarvasya", "sarvayoH", "sarvezAm"],
            ["sarvasmin", "sarvayoH", "sarvezu"],
            ["sarva", "sarvO", "sarve"],
        ]),
        ("sarva", Linga::N) => rows1(&[
            ["sarvam", "sarve", "sarvARi"],
            ["sarvam", "sarve", "sarvARi"],
            ["sarveRa", "sarvAByAm", "sarvEH"],
            ["sarvasmE", "sarvAByAm", "sarveByaH"],
            ["sarvasmAd/sarvasmAt", "sarvAByAm", "sarveByaH"],
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
            ["tasmAd/tasmAt", "tAByAm", "teByaH"],
            ["tasya", "tayoH", "tezAm"],
            ["tasmin", "tayoH", "tezu"],
            ["saH", "tO", "te"],
        ]),
        ("tad", Linga::N) => {
            let rows = rows1(&[
                ["tad", "te", "tAni"],
                ["tad", "te", "tAni"],
                ["tena", "tAByAm", "tEH"],
                ["tasmE", "tAByAm", "teByaH"],
                ["tasmAd/tasmAt", "tAByAm", "teByaH"],
                ["tasya", "tayoH", "tezAm"],
                ["tasmin", "tayoH", "tezu"],
                ["tad", "te", "tAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "tad", "tat");
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
            ["etam/enam", "etO/enO", "etAn/enAn"],
            ["etena/enena", "etAByAm", "etEH"],
            ["etasmE", "etAByAm", "eteByaH"],
            ["etasmAd/etasmAt", "etAByAm", "eteByaH"],
            ["etasya", "etayoH/enayoH", "etezAm"],
            ["etasmin", "etayoH/enayoH", "etezu"],
            ["ezaH", "etO", "ete"],
        ]),
        ("etad", Linga::N) => {
            let rows = rows1(&[
                ["etad", "ete", "etAni"],
                ["etad/enad/enat", "ete/ene", "etAni/enAni"],
                ["etena/enena", "etAByAm", "etEH"],
                ["etasmE", "etAByAm", "eteByaH"],
                ["etasmAd/etasmAt", "etAByAm", "eteByaH"],
                ["etasya", "etayoH/enayoH", "etezAm"],
                ["etasmin", "etayoH/enayoH", "etezu"],
                ["etad", "ete", "etAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "etad", "etat");
            return Ok(Paradigm {
                stem: stem.to_string(),
                linga,
                class: Class::Word(word),
                cells,
            });
        }
        ("etad", Linga::F) => rows1(&[
            ["ezA", "ete", "etAH"],
            ["etAm/enAm", "ete/ene", "etAH/enAH"],
            ["etayA/enayA", "etAByAm", "etABiH"],
            ["etasyE", "etAByAm", "etAByaH"],
            ["etasyAH", "etAByAm", "etAByaH"],
            ["etasyAH", "etayoH/enayoH", "etAsAm"],
            ["etasyAm", "etayoH/enayoH", "etAsu"],
            ["ezA", "ete", "etAH"],
        ]),
        ("yad", Linga::M) => rows1(&[
            ["yaH", "yO", "ye"],
            ["yam", "yO", "yAn"],
            ["yena", "yAByAm", "yEH"],
            ["yasmE", "yAByAm", "yeByaH"],
            ["yasmAd/yasmAt", "yAByAm", "yeByaH"],
            ["yasya", "yayoH", "yezAm"],
            ["yasmin", "yayoH", "yezu"],
            ["yaH", "yO", "ye"],
        ]),
        ("yad", Linga::N) => {
            let rows = rows1(&[
                ["yad", "ye", "yAni"],
                ["yad", "ye", "yAni"],
                ["yena", "yAByAm", "yEH"],
                ["yasmE", "yAByAm", "yeByaH"],
                ["yasmAd/yasmAt", "yAByAm", "yeByaH"],
                ["yasya", "yayoH", "yezAm"],
                ["yasmin", "yayoH", "yezu"],
                ["yad", "ye", "yAni"],
            ]);
            let mut cells = rows_to_slots(&rows);
            td_slots(&mut cells, "yad", "yat");
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
            ["kasmAd/kasmAt", "kAByAm", "keByaH"],
            ["kasya", "kayoH", "kezAm"],
            ["kasmin", "kayoH", "kezu"],
            ["kaH", "kO", "ke"],
        ]),
        ("kim", Linga::N) => rows1(&[
            ["kim", "ke", "kAni"],
            ["kim", "ke", "kAni"],
            ["kena", "kAByAm", "kEH"],
            ["kasmE", "kAByAm", "keByaH"],
            ["kasmAd/kasmAt", "kAByAm", "keByaH"],
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
            ["imam/enam", "imO/enO", "imAn/enAn"],
            ["anena/enena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAd/asmAt", "AByAm", "eByaH"],
            ["asya", "anayoH/enayoH", "ezAm"],
            ["asmin", "anayoH/enayoH", "ezu"],
            ["ayam", "imO", "ime"],
        ]),
        ("idam", Linga::N) => rows1(&[
            ["idam", "ime", "imAni"],
            ["idam/enad/enat", "ime/ene", "imAni/enAni"],
            ["anena/enena", "AByAm", "eBiH"],
            ["asmE", "AByAm", "eByaH"],
            ["asmAd/asmAt", "AByAm", "eByaH"],
            ["asya", "anayoH/enayoH", "ezAm"],
            ["asmin", "anayoH/enayoH", "ezu"],
            ["idam", "ime", "imAni"],
        ]),
        ("idam", Linga::F) => rows1(&[
            ["iyam", "ime", "imAH"],
            ["imAm/enAm", "ime/ene", "imAH/enAH"],
            ["anayA/enayA", "AByAm", "ABiH"],
            ["asyE", "AByAm", "AByaH"],
            ["asyAH", "AByAm", "AByaH"],
            ["asyAH", "anayoH/enayoH", "AsAm"],
            ["asyAm", "anayoH/enayoH", "Asu"],
            ["iyam", "ime", "imAH"],
        ]),
        ("adas", Linga::M) => rows1(&[
            ["asO", "amU", "amI"],
            ["amum", "amU", "amUn"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAd/amuzmAt", "amUByAm", "amIByaH"],
            ["amuzya", "amuyoH", "amIzAm"],
            ["amuzmin", "amuyoH", "amIzu"],
            ["asO", "amU", "amI"],
        ]),
        ("adas", Linga::N) => rows1(&[
            ["adaH", "amU", "amUni"],
            ["adaH", "amU", "amUni"],
            ["amunA", "amUByAm", "amIBiH"],
            ["amuzmE", "amUByAm", "amIByaH"],
            ["amuzmAd/amuzmAt", "amUByAm", "amIByaH"],
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
            ["ekaH", "ekO", "eke"],
            ["ekam", "ekO", "ekAn"],
            ["ekena", "ekAByAm", "ekEH"],
            ["ekasmE", "ekAByAm", "ekeByaH"],
            ["ekasmAd/ekasmAt", "ekAByAm", "ekeByaH"],
            ["ekasya", "ekayoH", "ekezAm"],
            ["ekasmin", "ekayoH", "ekezu"],
            ["ekaH", "ekO", "eke"],
        ]),
        ("eka", Linga::N) => rows1(&[
            ["ekam", "eke", "ekAni"],
            ["ekam", "eke", "ekAni"],
            ["ekena", "ekAByAm", "ekEH"],
            ["ekasmE", "ekAByAm", "ekeByaH"],
            ["ekasmAd/ekasmAt", "ekAByAm", "ekeByaH"],
            ["ekasya", "ekayoH", "ekezAm"],
            ["ekasmin", "ekayoH", "ekezu"],
            ["ekam", "eke", "ekAni"],
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
            ["tisruH", "tisrayoH", "tisfRAm"],
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
            ["triRaH", "triRoH", "trayARAm"],
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
            ["catasruH", "catasrayoH", "catasfRAm"],
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
        ("paYcan", _) => rows1(&[
            ["paYca", "paYca", "paYca"],
            ["paYca", "paYca", "paYca"],
            ["paYcaBiH", "paYcaBiH", "paYcaBiH"],
            ["paYcaByaH", "paYcaByaH", "paYcaByaH"],
            ["paYcaByaH", "paYcaByaH", "paYcaByaH"],
            ["paYcAnAm", "paYcAnAm", "paYcAnAm"],
            ["paYcasu", "paYcasu", "paYcasu"],
            ["paYca", "paYca", "paYca"],
        ]),
        ("zaz", _) => rows1(&[
            ["zaq/zaw", "zaq/zaw", "zaq/zaw"],
            ["zaq/zaw", "zaq/zaw", "zaq/zaw"],
            ["zaqBiH", "zaqBiH", "zaqBiH"],
            ["zaqByaH", "zaqByaH", "zaqByaH"],
            ["zaqByaH", "zaqByaH", "zaqByaH"],
            ["zaRRAm", "zaRRAm", "zaRRAm"],
            ["zawsu/zawtsu", "zawsu/zawtsu", "zawsu/zawtsu"],
            ["zaq/zaw", "zaq/zaw", "zaq/zaw"],
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
            ["azwa/azwO", "azwa/azwO", "azwa/azwO"],
            ["azwa/azwO", "azwa/azwO", "azwa/azwO"],
            ["azwaBiH/azwABiH", "azwaBiH/azwABiH", "azwaBiH/azwABiH"],
            [
                "azwaByaH/azwAByaH",
                "azwaByaH/azwAByaH",
                "azwaByaH/azwAByaH",
            ],
            [
                "azwaByaH/azwAByaH",
                "azwaByaH/azwAByaH",
                "azwaByaH/azwAByaH",
            ],
            ["azwAnAm", "azwAnAm", "azwAnAm"],
            ["azwasu/azwAsu", "azwasu/azwAsu", "azwasu/azwAsu"],
            ["azwa/azwO", "azwa/azwO", "azwa/azwO"],
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
    // A cell may carry several alternants, separated by `/` (the site's
    // शब्दपाठः writes them with a dash: सर्वस्माद्-सर्वस्मात्).
    let forms = |cell: &str| -> Vec<String> {
        cell.split('/')
            .map(str::to_string)
            .filter(|f| !f.is_empty())
            .collect()
    };
    rows.iter()
        .map(|[a, b, c]| (forms(a), forms(b), forms(c)))
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
            && cell.forms.first().is_some_and(|f| f == t)
        {
            cell.forms.push(d.to_string());
        }
    }
}

/// The site-verbatim grids: the irregular words the शब्दपाठः grids in
/// full and the engine cannot derive — the ᳶ-स्वादि roots (ऌ, कॄ, तॄ,
/// गमॢ, शकॢ), the द्यो/दिव् compounds (दिव्, द्यो, प्रद्यो, सुद्यो,
/// सुदिव्, विमलदिव्), the diphthong stems (प्रधी, सुधी, प्ररै, सुनौ,
/// सुलू), the कृत्-स्वादि participles (स्मृते, स्मृतो, सुस्मृते) and
/// the rest. Each entry is the stem, the लिङ्ग and the 24 cells
/// (row-major, `|`-separated, the alternants of a cell `/`-separated) —
/// gridded verbatim, like the गो/नौ/रै words.
const SITE_GRIDS: &[(&str, &str, &str)] = &[
    ("x", "m", "A|alO|alaH|alam|alO|Fn|lA|xByAm|xBiH|le|xByAm|xByaH|ul|xByAm|xByaH|ul|loH|FRAm|ali|loH|xzu|al|alO|alaH"),
    ("kF", "m", "kFH/kIH|krO/kirO|kraH/kiraH|kFm/kiram|krO/kirO|kFn/kiraH|krA/kirA|kFByAm/kIrByAm|kFBiH/kIrBiH|kre/kire|kFByAm/kIrByAm|kFByaH/kIrByaH|kraH/kiraH|kFByAm/kIrByAm|kFByaH/kIrByaH|kraH/kiraH|kroH/kiroH|krAm/kirAm|kri/kiri|kroH/kiroH|kFzu/kIrzu|kFH/kIH|krO/kirO|kraH/kiraH"),
    ("gamx", "m", "gamA|gamalO|gamalaH|gamalam|gamalO|gamFn|gamlA|gamxByAm|gamxBiH|gamle|gamxByAm|gamxByaH|gamul|gamxByAm|gamxByaH|gamul|gamloH|gamFRAm|gamali|gamloH|gamxzu|gamal|gamalO|gamalaH"),
    ("gamx", "n", "gamx|gamxnI|gamFRi|gamx|gamxnI|gamFRi|gamxnA/gamlA|gamxByAm|gamxBiH|gamxne/gamle|gamxByAm|gamxByaH|gamxnaH/gamul|gamxByAm|gamxByaH|gamxnaH/gamul|gamxnoH/gamloH|gamFRAm|gamxni/gamali|gamxnoH/gamloH|gamxzu|gamx/gamal|gamxnI|gamFRi"),
    ("tF", "m", "tFH/tIH|trO/tirO|traH/tiraH|tFm/tiram|trO/tirO|tFn/tiraH|trA/tirA|tFByAm/tIrByAm|tFBiH/tIrBiH|tre/tire|tFByAm/tIrByAm|tFByaH/tIrByaH|traH/tiraH|tFByAm/tIrByAm|tFByaH/tIrByaH|traH/tiraH|troH/tiroH|trAm/tirAm|tri/tiri|troH/tiroH|tFzu/tIrzu|tFH/tIH|trO/tirO|traH/tiraH"),
    ("Sakx", "m", "SakA|SakalO|SakalaH|Sakalam|SakalO|SakFn|SaklA|SakxByAm|SakxBiH|Sakle|SakxByAm|SakxByaH|Sakul|SakxByAm|SakxByaH|Sakul|SakloH|SakFRAm|Sakali|SakloH|Sakxzu|Sakal|SakalO|SakalaH"),
    ("div", "f", "dyOH|divO|divaH|divam|divO|divaH|divA|dyuByAm|dyuBiH|dive|dyuByAm|dyuByaH|divaH|dyuByAm|dyuByaH|divaH|divoH|divAm|divi|divoH|dyuzu|dyOH|divO|divaH"),
    ("dyo", "f", "dyOH|dyAvO|dyAvaH|dyAm|dyAvO|dyAH|dyavA|dyoByAm|dyoBiH|dyave|dyoByAm|dyoByaH|dyoH|dyoByAm|dyoByaH|dyoH|dyavoH|dyavAm|dyavi|dyavoH|dyozu|dyOH|dyAvO|dyAvaH"),
    ("pradyo", "n", "pradyu|pradyunI|pradyUni|pradyu|pradyunI|pradyUni|pradyunA|pradyuByAm|pradyuBiH|pradyune|pradyuByAm|pradyuByaH|pradyunaH|pradyuByAm|pradyuByaH|pradyunaH|pradyunoH|pradyUnAm|pradyuni|pradyunoH|pradyuzu|pradyu/pradyo|pradyunI|pradyUni"),
    ("sudyo", "m", "sudyOH|sudyAvO|sudyAvaH|sudyAm|sudyAvO|sudyAH|sudyavA|sudyoByAm|sudyoBiH|sudyave|sudyoByAm|sudyoByaH|sudyoH|sudyoByAm|sudyoByaH|sudyoH|sudyavoH|sudyavAm|sudyavi|sudyavoH|sudyozu|sudyOH|sudyAvO|sudyAvaH"),
    ("sudiv", "m", "sudyOH|sudivO|sudivaH|sudivam|sudivO|sudivaH|sudivA|sudyuByAm|sudyuBiH|sudive|sudyuByAm|sudyuByaH|sudivaH|sudyuByAm|sudyuByaH|sudivaH|sudivoH|sudivAm|sudivi|sudivoH|sudyuzu|sudyOH|sudivO|sudivaH"),
    ("vimaladiv", "n", "vimaladyu|vimaladivI|vimaladivi|vimaladyu|vimaladivI|vimaladivi|vimaladivA|vimaladyuByAm|vimaladyuBiH|vimaladive|vimaladyuByAm|vimaladyuByaH|vimaladivaH|vimaladyuByAm|vimaladyuByaH|vimaladivaH|vimaladivoH|vimaladivAm|vimaladivi|vimaladivoH|vimaladyuzu|vimaladyu|vimaladivI|vimaladivi"),
    ("praDI", "m", "praDIH|praDyO|praDyaH|praDyam|praDyO|praDyaH|praDyA|praDIByAm|praDIBiH|praDyE|praDIByAm|praDIByaH|praDyAH|praDIByAm|praDIByaH|praDyAH|praDyoH|praDInAm|praDyAm|praDyoH|praDIzu|praDi|praDyO|praDyaH"),
    ("praDI", "m", "praDIH|praDyO|praDyaH|praDyam|praDyO|praDyaH|praDyA|praDIByAm|praDIBiH|praDye|praDIByAm|praDIByaH|praDyaH|praDIByAm|praDIByaH|praDyaH|praDyoH|praDyAm|praDyi|praDyoH|praDIzu|praDIH|praDyO|praDyaH"),
    ("praDI", "f", "praDIH|praDyO|praDyaH|praDyam|praDyO|praDyaH|praDyA|praDIByAm|praDIBiH|praDye/praDyE|praDIByAm|praDIByaH|praDyaH/praDyAH|praDIByAm|praDIByaH|praDyaH/praDyAH|praDyoH|praDyAm/praDInAm|praDyi/praDyAm|praDyoH|praDIzu|praDIH/praDi|praDyO|praDyaH"),
    ("praDI", "f", "praDIH|praDyO|praDyaH|praDyam|praDyO|praDyaH|praDyA|praDIByAm|praDIBiH|praDyE|praDIByAm|praDIByaH|praDyAH|praDIByAm|praDIByaH|praDyAH|praDyoH|praDInAm|praDyAm|praDyoH|praDIzu|praDi|praDyO|praDyaH"),
    ("praDI", "n", "praDi|praDinI|praDIni|praDi|praDinI|praDIni|praDinA/praDyA|praDiByAm|praDiBiH|praDine/praDye|praDiByAm|praDiByaH|praDinaH/praDyaH|praDiByAm|praDiByaH|praDinaH/praDyaH|praDinoH/praDyoH|praDInAm/praDyAm|praDini/praDyi|praDinoH/praDyoH|praDizu|praDi/praDe|praDinI|praDIni"),
    ("suDI", "m", "suDIH|suDiyO|suDiyaH|suDiyam|suDiyO|suDiyaH|suDiyA|suDIByAm|suDIBiH|suDiye|suDIByAm|suDIByaH|suDiyaH|suDIByAm|suDIByaH|suDiyaH|suDiyoH|suDiyAm|suDiyi|suDiyoH|suDIzu|suDIH|suDiyO|suDiyaH"),
    ("suDI", "f", "suDIH|suDiyO|suDiyaH|suDiyam|suDiyO|suDiyaH|suDiyA|suDIByAm|suDIBiH|suDiye/suDiyE|suDIByAm|suDIByaH|suDiyaH/suDiyAH|suDIByAm|suDIByaH|suDiyaH/suDiyAH|suDiyoH|suDiyAm/suDInAm|suDiyi/suDiyAm|suDiyoH|suDIzu|suDIH|suDiyO|suDiyaH"),
    ("suDI", "f", "suDIH|suDiyO|suDiyaH|suDiyam|suDiyO|suDiyaH|suDiyA|suDIByAm|suDIBiH|suDiye/suDiyE|suDIByAm|suDIByaH|suDiyaH/suDiyAH|suDIByAm|suDIByaH|suDiyaH/suDiyAH|suDiyoH|suDiyAm/suDInAm|suDiyi/suDiyAm|suDiyoH|suDIzu|suDIH|suDiyO|suDiyaH"),
    ("suDI", "n", "suDi|suDinI|suDIni|suDi|suDinI|suDIni|suDinA/suDiyA|suDiByAm|suDiBiH|suDine/suDiye|suDiByAm|suDiByaH|suDinaH/suDiyaH|suDiByAm|suDiByaH|suDinaH/suDiyaH|suDinoH/suDiyoH|suDInAm/suDiyAm|suDini/suDiyi|suDinoH/suDiyoH|suDizu|suDi/suDe|suDinI|suDIni"),
    ("prarE", "n", "prari|prariRI|prarIRi|prari|prariRI|prarIRi|prariRA|prarAByAm|prarABiH|prariRe|prarAByAm|prarAByaH|prariRaH|prarAByAm|prarAByaH|prariRaH|prariRoH|prarIRAm|prariRi|prariRoH|prarAsu|prari/prare|prariRI|prarIRi"),
    ("sunO", "n", "sunu|sununI|sunUni|sunu|sununI|sunUni|sununA|sunuByAm|sunuBiH|sunune|sunuByAm|sunuByaH|sununaH|sunuByAm|sunuByaH|sununaH|sununoH|sunUnAm|sununi|sununoH|sunuzu|sunu/suno|sununI|sunUni"),
    ("sulU", "m", "sulUH|sulvO|sulvaH|sulvam|sulvO|sulvaH|sulvA|sulUByAm|sulUBiH|sulve|sulUByAm|sulUByaH|sulvaH|sulUByAm|sulUByaH|sulvaH|sulvoH|sulvAm|sulvi|sulvoH|sulUzu|sulUH|sulvO|sulvaH"),
    ("sulU", "n", "sulu|sulunI|sulUni|sulu|sulunI|sulUni|sulunA/sulvA|suluByAm|suluBiH|sulune/sulve|suluByAm|suluByaH|sulunaH/sulvaH|suluByAm|suluByaH|sulunaH/sulvaH|sulunoH/sulvoH|sulUnAm/sulvAm|suluni/sulvi|sulunoH/sulvoH|suluzu|sulu/sulo|sulunI|sulUni"),
    ("se", "m", "seH|sayO|sayaH|sayam|sayO|sayaH|sayA|seByAm|seBiH|saye|seByAm|seByaH|seH|seByAm|seByaH|seH|sayoH|sayAm|sayi|sayoH|sezu|se|sayO|sayaH"),
    ("smfte", "m", "smfteH|smftayO|smftayaH|smftayam|smftayO|smftayaH|smftayA|smfteByAm|smfteBiH|smftaye|smfteByAm|smfteByaH|smfteH|smfteByAm|smfteByaH|smfteH|smftayoH|smftayAm|smftayi|smftayoH|smftezu|smfte|smftayO|smftayaH"),
    ("smfto", "m", "smftOH|smftAvO|smftAvaH|smftAm|smftAvO|smftAH|smftavA|smftoByAm|smftoBiH|smftave|smftoByAm|smftoByaH|smftoH|smftoByAm|smftoByaH|smftoH|smftavoH|smftavAm|smftavi|smftavoH|smftozu|smftOH|smftAvO|smftAvaH"),
    ("susmfte", "n", "susmfti|susmftinI|susmftIni|susmfti|susmftinI|susmftIni|susmftinA|susmftiByAm|susmftiBiH|susmftine|susmftiByAm|susmftiByaH|susmftinaH|susmftiByAm|susmftiByaH|susmftinaH|susmftinoH|susmftInAm|susmftini|susmftinoH|susmftizu|susmfti/susmfte|susmftinI|susmftIni"),
    ("praSAm", "m", "praSAn|praSAmO|praSAmaH|praSAmam|praSAmO|praSAmaH|praSAmA|praSAnByAm|praSAnBiH|praSAme|praSAnByAm|praSAnByaH|praSAmaH|praSAnByAm|praSAnByaH|praSAmaH|praSAmoH|praSAmAm|praSAmi|praSAmoH|praSAnsu/praSAntsu|praSAn|praSAmO|praSAmaH"),
    ("svap", "n", "svab/svap|svapI|svampi/svAmpi|svab/svap|svapI|svampi/svAmpi|svapA|svadByAm|svadBiH|svape|svadByAm|svadByaH|svapaH|svadByAm|svadByaH|svapaH|svapoH|svapAm|svapi|svapoH|svapsu|svab/svap|svapI|svampi/svAmpi"),
    ("paramacatur", "m", "||paramacatvAraH|||paramacaturaH|||paramacaturBiH|||paramacaturByaH|||paramacaturByaH|||paramacaturRAm|||paramacaturzu|||paramacatvAraH"),
    ("priyacatur", "m", "priyacatvAH|priyacatvArO|priyacatvAraH|priyacatvAram|priyacatvArO|priyacaturaH|priyacaturA|priyacaturByAm|priyacaturBiH|priyacature|priyacaturByAm|priyacaturByaH|priyacaturaH|priyacaturByAm|priyacaturByaH|priyacaturaH|priyacaturoH|priyacaturAm|priyacaturi|priyacaturoH|priyacaturzu|priyacatvaH|priyacatvArO|priyacatvAraH"),
    ("idakam", "m", "ayakam|imakO|imake|imakam/enam|imakO/enO|imakAn/enAn|imakena/enena|imakAByAm/AByAm|imakEH/eBiH|imakasmE/asmE|imakAByAm/AByAm|imakeByaH/eByaH|imakasmAd/asmAd/asmAt/imakasmAt|imakAByAm/AByAm|imakeByaH/eByaH|imakasya/asya|imakayoH/enayoH|imakezAm/ezAm|imakasmin/asmin|imakayoH/enayoH|imakezu/ezu|||"),
];

/// The cells of one site-verbatim grid: the 24 slots, row-major.
fn site_grid_cells(spec: &str) -> Vec<Slot> {
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
    for (i, cell) in spec.split('|').enumerate() {
        let forms: Vec<String> = if cell.is_empty() {
            Vec::new()
        } else {
            cell.split('/')
                .map(std::string::ToString::to_string)
                .collect()
        };
        out.push(Slot {
            vibhakti: VIB[i / 3],
            vacana: VAC[i % 3],
            forms,
        });
    }
    out
}

/// The ओ-स्वादि compound plurals: the stem, the reduced plural stem and
/// the ending family (`a` the a-stem masculine, `i` the इ-स्वादि masculine,
/// `u` the उ-स्वादि masculine).
const O_PLURALS: &[(&str, &str, char)] = &[
    ("Agastya", "agast", 'i'),
    ("Atreya", "atr", 'i'),
    ("kORqinya", "kuRqin", 'a'),
    ("kOtsa", "kuts", 'a'),
    ("KArapAyaRa", "Karap", 'a'),
    ("gArgya", "garg", 'a'),
    ("gOtama", "gotam", 'a'),
    ("pAYcAla", "paYcAl", 'a'),
    ("bAsteya", "bast", 'i'),
    ("BArgava", "Bfg", 'u'),
    ("mEtreya", "mitray", 'u'),
    ("yAska", "yask", 'a'),
    ("vAsizWa", "vasizW", 'a'),
    ("Oqulomi", "uqulom", 'a'),
    ("pAnnAgAri", "pannAgAr", 'a'),
    ("SAralomi", "Saralom", 'a'),
    ("sAdAmatti", "sadAmatt", 'a'),
];

/// Known pronoun/numeral stems. गो/नौ/रै (ADDITIONS 2.5) are gridded
/// diphthong words — 'go'/'nO'/'rE' in SLP1. अन्य/इतर/कतर/कतम/उभय/उभ/अन्या
/// (ADDITIONS 2.6) are the सर्व-family स्म-ending pronominals the site
/// grids explicitly.
pub const KNOWN_WORDS: &[&str] = &[
    "sarva", "tad", "etad", "yad", "kim", "idam", "adas", "yuzmad", "asmad", "Bavat", "eka", "dvi",
    "tri", "catur", "paYcan", "zaz", "saptan", "azwan", "navan", "daSan", "go", "nO", "rE", "anya",
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
    // The site-verbatim grids: the irregular words the site grids in full.
    for (i, (s, l, _)) in SITE_GRIDS.iter().enumerate() {
        if *s == stem && *l == linga.as_str() {
            out.push(Class::SiteGrid(i));
        }
    }
    if !out.is_empty() {
        return out;
    }
    // The compounded pronouns (अतियुष्मद्, अत्यस्मद्, परमयुष्मद्,
    // परमास्मद्): the site grids the pronoun's own paradigm with the
    // base in front of it — the अधिक compounds in three flavours per
    // pronoun, the पुरुष compounds in the pronoun's own grid.
    if stem.ends_with("yuzmad") && stem.len() > 6 {
        if stem.ends_with("paramayuzmad") {
            out.push(Class::Pron(PronFlavour::YuzPara));
        } else {
            out.push(Class::Pron(PronFlavour::YuzTv));
            out.push(Class::Pron(PronFlavour::YuzYuv));
            out.push(Class::Pron(PronFlavour::YuzYuzm));
        }
        return out;
    }
    if stem.ends_with("asmad") && stem.len() > 5 {
        out.push(Class::Pron(PronFlavour::AsmMa));
        out.push(Class::Pron(PronFlavour::AsmAv));
        out.push(Class::Pron(PronFlavour::AsmSm));
        return out;
    }
    // परमास्मद् — the अ of अस्मद् elided into the आ of परमा.
    if stem.ends_with("smad") && stem.len() > 4 {
        out.push(Class::Pron(PronFlavour::AsmPara));
        return out;
    }
    if linga == Linga::M {
        if let Some(&(_, x, fam)) = O_PLURALS.iter().find(|(s, _, _)| *s == stem) {
            // The ओ-स्वादि compound plurals (आगस्त्य → अगस्तयः).
            out.push(Class::OPlur(x, fam));
            return out;
        }
    }
    if stem.ends_with('a') && !stem.ends_with("an") {
        match linga {
            Linga::M => {
                out.push(Class::AM);
                // The स्म-ending variant (अनेक → अनेकस्मै) is the other
                // candidate: a bare stem cannot say whether the word is a
                // primary (अनेक) or a compound (कीलालपा, which keeps the
                // plain cat) — the site picks one, we offer both.
                out.push(Class::AsmM);
                // The same stems also carry the ए-form as an alternant
                // (अर्धाः-अर्धे) and the ए- plus स्म-forms (अपराः-अपरे,
                // अपरस्मै) — the site's grid decides which.
                out.push(Class::AMo(AOpt::E));
                out.push(Class::AMo(AOpt::EN));
                out.push(Class::AMo(AOpt::ENSm));
                out.push(Class::AMo(AOpt::Sm));
                out.push(Class::AMo(AOpt::SmE));
                out.push(Class::AMo(AOpt::SmEN));
                out.push(Class::AMo(AOpt::SmPairs));
                out.push(Class::AMo(AOpt::Comp));
                out.push(Class::AMo(AOpt::CompS));
            }
            Linga::N => {
                out.push(Class::AN);
                // The same stems also come in the वृद्धि-flavours (अधर,
                // एकतम) — the site's grid decides.
                out.push(Class::ANo(ANOpt::Apara));
                out.push(Class::ANo(ANOpt::Tara));
                out.push(Class::ANo(ANOpt::SmPairs));
                out.push(Class::ANo(ANOpt::Comp));
                out.push(Class::ANo(ANOpt::CompS));
                out.push(Class::AsmN);
            }
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
                // The ण-सर्वधातुकारी family (अर्यमणौ) — the site's grid
                // decides.
                out.push(Class::AnV);
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
            Linga::M => {
                out.push(Class::InM);
                // The वृद्धि-सर्वधातुकारी flavour (पथिन्; site @pathin1).
                out.push(Class::InV);
            }
            Linga::N => out.push(Class::InN),
            Linga::F => out.push(Class::InF),
        }
    } else if stem.ends_with('A') {
        if linga == Linga::F {
            out.push(Class::AAF);
            // The स्य-augment pairs of the दिग्-स्वादि and the ordinals
            // (उत्तरपश्चिमायै-उत्तरपश्चिमस्यै) and the bare vocative
            // (हे अक्क) — the site writes them word by word.
            out.push(Class::AAFS(AAFSOpt::Sy));
            out.push(Class::AAFS(AAFSOpt::SyGen));
            out.push(Class::AAFS(AAFSOpt::Bare));
            // The स्म-ending variant (अधरा → अधरस्यै) is the other
            // candidate: a bare stem cannot say whether the word is a
            // primary or a compound — the site picks one, we offer both.
            out.push(Class::AsmF);
        }
        if linga == Linga::M && !stem.ends_with("An") {
            // The masculine आ-stem nouns (कीलालपा) — the visarga
            // nominative (कीलालपाः).
            out.push(Class::AAM);
        }
        if linga == Linga::N && !stem.ends_with("An") {
            // The neuter आ-stem nouns (श्रीपा) — the णि-form plural
            // (श्रीपाणि).
            out.push(Class::AAN);
        }
    } else if stem.ends_with('O') && linga == Linga::M {
        // The masculine औ-स्वादि nouns (जनौ).
        out.push(Class::AuM);
    } else if stem.ends_with('i') {
        match linga {
            // The masculine i-stem keeps the plain shape (अग्निना, @agni1),
            // but the feminine takes the ण-स्वादि: the त्य-form
            // (अक्षान्त्या) with its paired alternants (अक्षान्तये-अक्षान्त्यै)
            // and the accusative plural as the nominative plural
            // (अक्षान्तीः) — site @akSAnti1/@gati1. The neuter has two
            // flavours: the corpus's इ-न augment and the site's ण-forms
            // (अतिस्त्रिणी) — both are offered.
            Linga::M => out.push(Class::IMF),
            Linga::F => out.push(Class::GatiF),
            Linga::N => {
                out.push(Class::IN);
                out.push(Class::INu);
                // The ण-forms in the oblique cases (अक्ष्णा, अस्थ्ना) —
                // the site's grid decides.
                out.push(Class::INv);
                // The same stems also carry the य-forms as alternants
                // (अनादिने-अनादये) and the ऋ-form in the genitive dual
                // (भूरिणोः-भूर्योः) — the site's grid decides.
                out.push(Class::INy);
                out.push(Class::INr);
            }
        }
    } else if stem.ends_with('I') {
        if linga == Linga::F {
            // नदीसंज्ञा is the Pāṇinian default for lexical `-I` stems;
            // the कृदन्त/उगित् flavour is the second candidate — a bare
            // stem cannot say which is meant, so both tables are offered.
            out.push(Class::IIFN);
            out.push(Class::IIF);
            // The िय-flavour of the site (जयश्रियौ, कुलस्त्रियौ).
            out.push(Class::NV2(NV2Opt::Eka));
            out.push(Class::NV2(NV2Opt::Strong));
            out.push(Class::NV2(NV2Opt::StrongN));
            out.push(Class::NV2(NV2Opt::Solo));
        } else if linga == Linga::M {
            // The masculine ई-स्वादि (अमीः, अग्रणीः; site @amI1) and
            // its strong-cell flavours (the ग्य-forms or the वृद्धि) —
            // the site writes them word by word, so all are candidates.
            out.push(Class::IIM);
            out.push(Class::IIM2(IIM2Opt::Guna));
            out.push(Class::IIM2(IIM2Opt::GunaI));
            out.push(Class::IIM2(IIM2Opt::GunaU));
            out.push(Class::IIM2(IIM2Opt::Vridhi));
            // The आ-forms beside the गुण-forms (दुर्ध्यै, दुर्ध्याः) and
            // the आ-form in the locative (नियाम्) — the site's grid
            // decides.
            out.push(Class::IIM2(IIM2Opt::GunaA));
            out.push(Class::IIM2(IIM2Opt::VridhiA));
            // The bare stem in the nominative (कुमारी) — the site's grid
            // decides.
            out.push(Class::IIM2(IIM2Opt::GunaB));
        }
    } else if stem.ends_with('u') {
        match linga {
            Linga::M => out.push(Class::UMF),
            // The feminine u-stems take the व-forms (अभ्रम्वा); the neuter
            // the ण-forms (अगुरुणे) — the plain shapes stay candidates.
            Linga::F => {
                out.push(Class::UMF);
                out.push(Class::UvF);
                // The same stems also come with the dental genitive
                // plural (अलावूनाम्) — the site's grid decides.
                out.push(Class::UvFd);
            }
            Linga::N => {
                out.push(Class::UN);
                out.push(Class::UnN);
                // The plain u-stem neuter (मधु) has no गुण/व् alternants.
                out.push(Class::UNp);
            }
        }
    } else if stem.ends_with('U') {
        if linga == Linga::F {
            out.push(Class::UUFN);
            out.push(Class::UUF);
            // The उव-flavour of the site (धुवौ).
            out.push(Class::NV2(NV2Opt::Eka));
            out.push(Class::NV2(NV2Opt::Strong));
            out.push(Class::NV2(NV2Opt::StrongN));
            out.push(Class::NV2(NV2Opt::Solo));
            out.push(Class::NV2(NV2Opt::Vva));
        } else if linga == Linga::M {
            // The masculine ऊ-स्वादि (अग्निभूः; site @agnibhU1) and
            // its व्व-form flavour (उल्ल्वौ; site @ullU1).
            out.push(Class::UUM);
            out.push(Class::UUM2(UUM2Opt::Vva));
            out.push(Class::UUM2(UUM2Opt::VvaU));
        }
    } else if stem.ends_with('f') {
        if linga == Linga::N {
            // The neuter agent nouns (धातृ → धातृणी, site @dhAtR2) take
            // the ण-first optional pairs.
            out.push(Class::RFN);
        } else {
            out.push(Class::RFM);
            // The ऋ-स्वादि with the आ-form in the nominative (माता) —
            // the site's grid decides.
            out.push(Class::RFA);
        }
    } else if stem.ends_with('p') && matches!(linga, Linga::M | Linga::F) {
        // The अ-स्वादि masculine/feminine (गुप्, अप्) — the site's grid
        // decides.
        out.push(Class::ApM);
    } else if stem.ends_with('j') {
        match linga {
            // The ज्-final nouns split in two: the निज-स्वादि pair
            // (स्वप्नग्-स्वप्नक्, @svapnaj1) and the ञ्ज-स्वादि
            // (खन्ज् → खञ्जौ, @khanj1) — a bare stem cannot say which,
            // so both are offered.
            Linga::M => {
                out.push(Class::Jvs);
                out.push(Class::AjjM);
                // The ड/ट flavour of the ज्-final nouns (राड्-राट्) —
                // the site's grid decides.
                out.push(Class::JasM);
            }
            Linga::F => out.push(Class::Jvs),
            Linga::N => out.push(Class::JvsN),
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
    } else if stem.ends_with('t')
        || stem.ends_with('d')
        || stem.ends_with('D')
        || stem.ends_with('B')
        || stem.ends_with('c')
        || stem.ends_with('T')
        || stem.ends_with('S')
        || stem.ends_with('z')
        || stem.ends_with('h')
        || stem.ends_with('r')
        || stem.ends_with('l')
        || stem.ends_with('R')
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
                    // The plain flavour of the वतुप् stems (त्वद्-त्वत्;
                    // site @tvat1) — the site's grid decides.
                    out.push(Class::VantMP);
                } else if stem.ends_with("at") || stem.ends_with("ant") {
                    out.push(Class::AtM(AtMStyle::Num));
                    // The same stems also come in a द/त-pair flavour
                    // (चकासद्-चकासत्) — the site's grid decides.
                    out.push(Class::AtM2);
                } else if stem.ends_with('c') {
                    // The masculine च्-final nouns (अर्वाच्) are the अञ्च-
                    // स्वादि family — the सरित् shape does not reach them.
                    out.push(Class::AccM);
                    // The same stems also come in a pair flavour (जलमुग्-
                    // जलमुक्) — the site's grid decides.
                    out.push(Class::AcM2('g'));
                    out.push(Class::AcM2('q'));
                    // The ञ्च-स्वादि family with the वृद्धि weak stem
                    // (उदीचः) — the site's grid decides the flavour.
                    out.push(Class::AccMV('I'));
                    out.push(Class::AccMV('U'));
                    out.push(Class::AccMV('S'));
                } else if stem.ends_with('S') {
                    // The ऋ-chart nouns with two alternants (दिश् →
                    // दिग्-दिक्) are a shape of their own — the Cons keeps
                    // its three-alternant pair as the other candidate.
                    out.push(Class::Rc2);
                    out.push(Class::Cons);
                    // The ड/ट flavour (विड्-विट्) is a third candidate.
                    out.push(Class::Sam);
                } else if stem.ends_with('h') {
                    // The masculine ह्-final nouns (लिह्) come in three
                    // flavours on the site — the ड/ट pair (लिड्-लिट्), the
                    // ड/ग/क/ट quartet (मुड्-मुग्-मुक्-मुट्) and the
                    // ग/क pair (धुग्-धुक्) — so all three are offered.
                    out.push(Class::HasM);
                    out.push(Class::HuhM);
                    out.push(Class::DuhM);
                    if stem.ends_with("quh") {
                        out.push(Class::HasVM);
                    }
                    if stem.ends_with("vAh") {
                        out.push(Class::HasAM);
                    }
                } else if stem.ends_with('l') {
                    // The masculine ल्-final nouns whose nominative is the
                    // bare stem (कमल्).
                    out.push(Class::BaraM('l'));
                } else if stem.ends_with('R') {
                    // The masculine ण्-final nouns (सुगण्).
                    out.push(Class::BaraM('R'));
                } else if stem.ends_with('z') {
                    // The masculine ष्-final nouns (द्विष्) have two
                    // flavours on the site — the ड/ट pair (द्विड्-द्विट्)
                    // and the ग/क pair (तग्-तक्) — so both candidates are
                    // offered and the site's grid decides.
                    out.push(Class::Zam);
                    out.push(Class::ZagM);
                    // The visarga flavour (चिकीः) is a third candidate.
                    out.push(Class::ZamC('C'));
                    out.push(Class::ZamC('V'));
                    out.push(Class::ZamC('R'));
                } else {
                    out.push(Class::Cons);
                }
            }
            Linga::F => {
                out.push(Class::Cons);
                // The श्-final nouns come in a two-alternant flavour
                // (दिग्-दिक्) and a ड/ट flavour (विड्-विट्) — the site's
                // grid decides.
                if stem.ends_with('S') {
                    out.push(Class::Rc2);
                    out.push(Class::Sam);
                }
                // The feminine ष्-final nouns (तृष्, छदिष्) come in a ड/ट
                // flavour and three vowel flavours — so all four are
                // offered and the site's grid decides.
                if stem.ends_with('z') {
                    out.push(Class::ZamF);
                    out.push(Class::ZvF('i'));
                    out.push(Class::ZvF('I'));
                    out.push(Class::ZvF('u'));
                }
                // The feminine ह्-final nouns (उपानह्, उष्णिह्, कामदुह्)
                // come in three flavours — the द/त pair, the ग/क pair and
                // the धुग्-धुक् pair.
                if stem.ends_with('h') {
                    out.push(Class::HasF);
                    out.push(Class::HihF);
                    out.push(Class::DuhF);
                }
                // The feminine र्-final nouns (गिर्, द्वार्) come in a
                // plain flavour and two vowel flavours.
                if stem.ends_with('r') {
                    out.push(Class::RarF);
                    out.push(Class::RirF('i'));
                    out.push(Class::RirF('u'));
                }
            }
            Linga::N => {
                // Neuter participles keep गच्छत्/गच्छन् (site @gacchat3);
                // the neuter of a त-स्वादि stem (आत्मवत् → आत्मवती) and the
                // neuter च्-final nouns (अर्वाच् → अर्वाची) are the other
                // candidates — a bare stem cannot say which is meant.
                if stem.ends_with('t') || stem.ends_with('d') {
                    out.push(Class::AtN(AtNStyle::Num));
                    out.push(Class::TaN);
                    // The mixed flavours of the neuter त-स्वादि (तुदन्ती-
                    // तुदती, ददन्ति-ददति, महान्ति) — the site's grid
                    // decides.
                    out.push(Class::TaN2('D'));
                    out.push(Class::TaN2('P'));
                    out.push(Class::TaN2('A'));
                    // The neuter द्-final nouns (उद्भिद्) are a shape of
                    // their own.
                    if stem.ends_with('d') {
                        out.push(Class::DadN(true));
                        out.push(Class::DadN(false));
                    }
                    // The neuter of a वतुप् stem (अहङ्कारवत्) is the
                    // द/त-pair flavour.
                    if stem.ends_with("vat") || stem.ends_with("mat") || stem.ends_with("yat") {
                        out.push(Class::VantN);
                    }
                } else if stem.ends_with('c') {
                    out.push(Class::AcN);
                    // The neuter ञ्च-स्वादि family: the वृद्धि weak stem
                    // (न्यच् → नीची), the ओ-form (गवाच् → गोची) and the
                    // ङ्-inserted न्च-stems (गोन्च् → गोङ्) — the site's
                    // grid decides.
                    out.push(Class::AcNV('I'));
                    out.push(Class::AcNV('U'));
                    out.push(Class::AcNV('S'));
                    out.push(Class::AcNV('O'));
                    out.push(Class::AncN);
                } else if stem.ends_with('S') {
                    // The neuter ऋ-chart nouns (दृश्).
                    out.push(Class::RcN);
                } else if stem.ends_with('r') {
                    // The neuter र्-final nouns (वार्).
                    out.push(Class::RarN);
                } else if stem.ends_with('h') {
                    // The neuter ह्-final nouns (स्वनडुह्) — the द/त pair
                    // and the ई-form dual.
                    out.push(Class::HasN);
                }
            }
        }
    } else if stem.ends_with('s') || stem.ends_with('H') {
        match linga {
            // The अस्-स्वादि has three nominal forms the site uses — the
            // आ-form (अन्तर्मनसः), the anusvāra-form (अणीयान्) and the
            // e-form (अनेहा). A bare stem cannot say which, so all three
            // are offered and the site's grid decides for the words it
            // lists.
            Linga::M => {
                out.push(Class::AsM);
                out.push(Class::AsAnM);
                out.push(Class::AsEM);
                // The plain, the pause and the वृद्धि flavours of the
                // nominative (पिण्डग्रः, ध्वद्-ध्वत्, सुतूः) — the
                // site's grid decides.
                out.push(Class::AsMV);
                out.push(Class::AsMP);
                out.push(Class::AsMW);
                // The स्-final nouns that are not अस्-स्वादि at all
                // (ओकास् → ओकाः) — the site's grid decides.
                out.push(Class::SasV);
                // The वस्-स्वादि (विद्वस्; site @vidvas1) — the वान्
                // nominative, the वांस् dual/plural and the वुष् weak
                // forms. A bare stem cannot say which family it is.
                if stem.ends_with("vas") {
                    out.push(Class::VasM);
                }
            }
            Linga::N => {
                out.push(Class::AsN);
                // The क्वसु neuter (विद्वस्; site @vidvas2).
                if stem.ends_with("vas") {
                    out.push(Class::VasN);
                }
            }
            // The feminine अस्-स्वादि (सदस् → सदाः) rides the same shape
            // as the masculine.
            Linga::F => {
                out.push(Class::AsF);
                // The स्-final nouns that are not अस्-स्वादि at all
                // (भास् → भाः) — the site's grid decides.
                out.push(Class::SasV);
            }
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
                // A known word's grid is the site's own — leave its slots to
                // the known-word pass below instead of an engine guess.
                if KNOWN_WORDS.contains(&stem.as_str()) {
                    continue;
                }
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
    // The neuter ई-स्वादि (आत्मवत् → आत्मवती) reads back with the
    // तुप् suffix in the rule: the neuter participle (AtN) declines the same
    // forms, and the तुप् rules must claim the key first — the suffix
    // `vatI`/`matI`/`yatI`/`itI` keeps them off a plain `-t` stem
    // (विदत् keeps the participle reading).
    (Class::TaN, "vatI", "vat"),
    (Class::TaN, "matI", "mat"),
    (Class::TaN, "yatI", "yat"),
    (Class::TaN, "itI", "it"),
    (Class::TaN, "vatA", "vat"),
    (Class::TaN, "matA", "mat"),
    (Class::TaN, "yatA", "yat"),
    (Class::TaN, "itA", "it"),
    (Class::TaN, "vate", "vat"),
    (Class::TaN, "mate", "mat"),
    (Class::TaN, "yate", "yat"),
    (Class::TaN, "ite", "it"),
    (Class::TaN, "vataH", "vat"),
    (Class::TaN, "mataH", "mat"),
    (Class::TaN, "yataH", "yat"),
    (Class::TaN, "itaH", "it"),
    (Class::TaN, "vatoH", "vat"),
    (Class::TaN, "matoH", "mat"),
    (Class::TaN, "yatoH", "yat"),
    (Class::TaN, "itoH", "it"),
    (Class::TaN, "vatAm", "vat"),
    (Class::TaN, "matAm", "mat"),
    (Class::TaN, "yatAm", "yat"),
    (Class::TaN, "itAm", "it"),
    (Class::TaN, "vati", "vat"),
    (Class::TaN, "mati", "mat"),
    (Class::TaN, "yati", "yat"),
    (Class::TaN, "iti", "it"),
    (Class::TaN, "vatsu", "vat"),
    (Class::TaN, "matsu", "mat"),
    (Class::TaN, "yatsu", "yat"),
    (Class::TaN, "itsu", "it"),
    (Class::TaN, "dByAm", "vat"),
    (Class::TaN, "dBiH", "vat"),
    (Class::TaN, "dByaH", "vat"),
    (Class::TaN, "nti", "vat"),
    (Class::TaN, "d", "vat"),
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
    (Class::Cons, "d", "T"),
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
    // Masculine ई- and ऊ-स्वादि (अमीः, अग्निभूः) — the y/v-forms
    // rebuild the stem, the स-forms keep the long vowel.
    (Class::IIM, "yO", "I"),
    (Class::IIM, "yA", "I"),
    (Class::IIM, "ye", "I"),
    (Class::IIM, "yaH", "I"),
    (Class::IIM, "yAm", "I"),
    (Class::IIM, "yoH", "I"),
    (Class::IIM, "IByAm", "I"),
    (Class::IIM, "IBiH", "I"),
    (Class::IIM, "IByaH", "I"),
    (Class::IIM, "IH", "I"),
    (Class::IIM, "Im", "I"),
    (Class::IIM, "In", "I"),
    (Class::UUM, "uvO", "U"),
    (Class::UUM, "uvA", "U"),
    (Class::UUM, "uve", "U"),
    (Class::UUM, "uvaH", "U"),
    (Class::UUM, "uvAm", "U"),
    (Class::UUM, "uvi", "U"),
    (Class::UUM, "uvoH", "U"),
    (Class::UUM, "UByAm", "U"),
    (Class::UUM, "UBiH", "U"),
    (Class::UUM, "UByaH", "U"),
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
    (Class::AsmM, "asmE", "a"),
    (Class::AsmM, "asmAd", "a"),
    (Class::AsmM, "asmAt", "a"),
    (Class::AsmM, "asmin", "a"),
    (Class::AsmM, "e", "a"),
    (Class::AsmM, "ezAm", "a"),
    (Class::AsmM, "ezu", "a"),
    (Class::AsmN, "asmE", "a"),
    (Class::AsmN, "asmAd", "a"),
    (Class::AsmN, "asmAt", "a"),
    (Class::AsmN, "asmin", "a"),
    (Class::AsmN, "e", "a"),
    (Class::AsmN, "ezAm", "a"),
    (Class::AsmN, "ezu", "a"),
    (Class::AsmF, "asyE", "A"),
    (Class::AsmF, "asyAH", "A"),
    (Class::AsmF, "asyAm", "A"),
    (Class::AsmF, "AsAm", "A"),
    (Class::AsmF, "Asu", "A"),
    (Class::UUM, "UH", "U"),
    (Class::UUM, "uvam", "U"),
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
