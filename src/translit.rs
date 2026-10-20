//! Devanagari ↔ SLP1 transliteration (display bridge).
//!
//! The `dhatuforms_*` tables and `dhatu/data.txt` upasarga names are
//! Devanagari; the morphology engine (`engine::{subanta, upasarga, search}`)
//! is SLP1-only and byte-exact. This module is the boundary layer:
//!
//! - [`deva_to_slp1`] — one Devanagari form/token → SLP1 (`भवति` → `Bavati`).
//! - [`slp1_to_deva`] — SLP1 surface → Devanagari (`Bavati` → `भवति`).
//! - [`deva_stack_to_slp1`] — site stack (`वि + अति`) → engine stack (`vi;ati`).
//!
//! Scope: classical Sanskrit aksharas observed in the dhatu tables
//! (vowels, matras, consonants, virama clusters, anusvara/visarga,
//! avagraha). Separators (`;`, `,`, `/`, `+`, space) are handled by the
//! callers, not here — every function below rejects them. Unknown
//! characters (nukta, Vedic accents, Latin, digits) yield `None`.

const VIRAMA: char = '\u{094D}';
const AVAGRAHA: char = '\u{093D}';
const ANUSVARA: char = '\u{0902}';
const CANDRABINDU: char = '\u{0901}';
const VISARGA: char = '\u{0903}';

/// True when `s` contains any Devanagari codepoint (U+0900–U+097F).
#[must_use]
pub fn contains_deva(s: &str) -> bool {
    s.chars().any(|c| ('\u{900}'..='\u{97F}').contains(&c))
}

/// Independent vowel → SLP1.
fn independent_vowel(ch: char) -> Option<&'static str> {
    match ch {
        'अ' => Some("a"),
        'आ' => Some("A"),
        'इ' => Some("i"),
        'ई' => Some("I"),
        'उ' => Some("u"),
        'ऊ' => Some("U"),
        'ऋ' => Some("f"),
        'ॠ' => Some("F"),
        'ऌ' => Some("x"),
        'ॡ' => Some("X"),
        // Candra/short e-o (foreign and Dravidian spellings: जॉन, पॅट). SLP1
        // has no glyphs for them and no paradigm keys off the distinction,
        // so they fold onto the nearest plain vowel.
        'ए' | 'ऍ' | 'ऎ' => Some("e"),
        'ऐ' => Some("E"),
        'ओ' | 'ऑ' | 'ऒ' => Some("o"),
        'औ' => Some("O"),
        _ => None,
    }
}

/// Dependent vowel sign (matra) → SLP1.
fn matra(sign: char) -> Option<&'static str> {
    match sign {
        'ा' => Some("A"),
        'ि' => Some("i"),
        'ी' => Some("I"),
        'ु' => Some("u"),
        'ू' => Some("U"),
        'ृ' => Some("f"),
        'ॄ' => Some("F"),
        'ॢ' => Some("x"),
        'ॣ' => Some("X"),
        'े' | 'ॅ' | 'ॆ' => Some("e"),
        'ै' => Some("E"),
        'ो' | 'ॉ' | 'ॊ' => Some("o"),
        'ौ' => Some("O"),
        _ => None,
    }
}

/// Consonant → SLP1.
fn consonant(ch: char) -> Option<&'static str> {
    match ch {
        'क' => Some("k"),
        'ख' => Some("K"),
        'ग' => Some("g"),
        'घ' => Some("G"),
        'ङ' => Some("N"),
        'च' => Some("c"),
        'छ' => Some("C"),
        'ज' => Some("j"),
        'झ' => Some("J"),
        'ञ' => Some("Y"),
        'ट' => Some("w"),
        'ठ' => Some("W"),
        'ड' => Some("q"),
        'ढ' => Some("Q"),
        'ण' => Some("R"),
        'त' => Some("t"),
        'थ' => Some("T"),
        'द' => Some("d"),
        'ध' => Some("D"),
        'न' => Some("n"),
        'प' => Some("p"),
        'फ' => Some("P"),
        'ब' => Some("b"),
        'भ' => Some("B"),
        'म' => Some("m"),
        'य' => Some("y"),
        'र' => Some("r"),
        'ल' => Some("l"),
        'ळ' => Some("L"),
        'व' => Some("v"),
        'श' => Some("S"),
        'ष' => Some("z"),
        'स' => Some("s"),
        'ह' => Some("h"),
        _ => None,
    }
}

/// One Devanagari token (no separators) → SLP1.
///
/// A consonant takes the inherent `a` unless followed by virama (explicit
/// halant, e.g. `सम्` → `sam`) or a matra (e.g. `भू` → `BU`). Clusters
/// fold compositionally (`प्र` → `pra`, `न्ति` → `nti`).
#[must_use]
pub fn deva_to_slp1(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut idx = 0;
    while idx < chars.len() {
        let ch = chars[idx];
        if let Some(vowel) = independent_vowel(ch) {
            out.push_str(vowel);
            idx += 1;
        } else if let Some(cons) = consonant(ch) {
            let next = chars.get(idx + 1);
            if next == Some(&VIRAMA) {
                out.push_str(cons);
                idx += 2;
            } else if let Some(sign) = next.and_then(|following| matra(*following)) {
                out.push_str(cons);
                out.push_str(sign);
                idx += 2;
            } else {
                out.push_str(cons);
                out.push('a');
                idx += 1;
            }
        } else if ch == ANUSVARA || ch == CANDRABINDU {
            out.push('M');
            idx += 1;
        } else if ch == VISARGA {
            out.push('H');
            idx += 1;
        } else if ch == AVAGRAHA {
            out.push('\'');
            idx += 1;
        } else {
            // Stray virama/matra, separator, or unmapped character.
            return None;
        }
    }
    Some(out)
}

/// SLP1 vowel → (independent, matra).
fn slp1_vowel(ch: char) -> Option<(char, &'static str, &'static str)> {
    match ch {
        'a' => Some(('a', "अ", "")),
        'A' => Some(('A', "आ", "ा")),
        'i' => Some(('i', "इ", "ि")),
        'I' => Some(('I', "ई", "ी")),
        'u' => Some(('u', "उ", "ु")),
        'U' => Some(('U', "ऊ", "ू")),
        'f' => Some(('f', "ऋ", "ृ")),
        'F' => Some(('F', "ॠ", "ॄ")),
        'x' => Some(('x', "ऌ", "ॢ")),
        'X' => Some(('X', "ॡ", "ॣ")),
        'e' => Some(('e', "ए", "े")),
        'E' => Some(('E', "ऐ", "ै")),
        'o' => Some(('o', "ओ", "ो")),
        'O' => Some(('O', "औ", "ौ")),
        _ => None,
    }
}

/// SLP1 consonant → Devanagari.
fn slp1_consonant(ch: char) -> Option<char> {
    match ch {
        'k' => Some('क'),
        'K' => Some('ख'),
        'g' => Some('ग'),
        'G' => Some('घ'),
        'N' => Some('ङ'),
        'c' => Some('च'),
        'C' => Some('छ'),
        'j' => Some('ज'),
        'J' => Some('झ'),
        'Y' => Some('ञ'),
        'w' => Some('ट'),
        'W' => Some('ठ'),
        'q' => Some('ड'),
        'Q' => Some('ढ'),
        'R' => Some('ण'),
        't' => Some('त'),
        'T' => Some('थ'),
        'd' => Some('द'),
        'D' => Some('ध'),
        'n' => Some('न'),
        'p' => Some('प'),
        'P' => Some('फ'),
        'b' => Some('ब'),
        'B' => Some('भ'),
        'm' => Some('म'),
        'y' => Some('य'),
        'r' => Some('र'),
        'l' => Some('ल'),
        'L' => Some('ळ'),
        'v' => Some('व'),
        'S' => Some('श'),
        'z' => Some('ष'),
        's' => Some('स'),
        'h' => Some('ह'),
        _ => None,
    }
}

/// One SLP1 surface (letters + `'` only, no `;`/`/`) → Devanagari.
///
/// A consonant takes the inherent `a` when followed by `a`, a matra for
/// other vowels, and a virama otherwise (cluster or word-final halant:
/// `sam` → `सम्`, `pra` → `प्र`).
#[must_use]
pub fn slp1_to_deva(text: &str) -> Option<String> {
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'\'')
    {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if let Some(deva) = slp1_consonant(ch) {
            out.push(deva);
            match bytes.get(idx + 1).map(|following| *following as char) {
                Some('a') => {
                    idx += 2;
                }
                Some(following) if slp1_vowel(following).is_some() => {
                    let (_, _, sign) = slp1_vowel(following).expect("checked vowel");
                    out.push_str(sign);
                    idx += 2;
                }
                // The anusvāra takes the consonant's place in the syllable:
                // `kM` is कं, not क्ं.
                Some('M') => {
                    idx += 1;
                }
                // The visarga replaces the consonant's inherent vowel:
                // `kulH` is कुलः, not कुल्:.
                Some('H') => {
                    out.push(VISARGA);
                    idx += 1;
                }
                _ => {
                    out.push(VIRAMA);
                    idx += 1;
                }
            }
        } else if ch == 'a' {
            out.push('अ');
            idx += 1;
        } else if let Some((_, indep, _)) = slp1_vowel(ch) {
            out.push_str(indep);
            idx += 1;
        } else if ch == 'M' {
            out.push(ANUSVARA);
            idx += 1;
        } else if ch == 'H' {
            out.push(VISARGA);
            idx += 1;
        } else if ch == '\'' {
            out.push(AVAGRAHA);
            idx += 1;
        } else {
            return None;
        }
    }
    Some(out)
}

/// Site upasarga stack → engine stack: `वि + अति` → `vi;ati`,
/// `प्र` → `pra`. Splits on `+` and `;`, trims spaces.
#[must_use]
pub fn deva_stack_to_slp1(stack: &str) -> Option<String> {
    let mut toks = Vec::new();
    for part in stack.split(['+', ';']) {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        toks.push(deva_to_slp1(t)?);
    }
    if toks.is_empty() {
        return None;
    }
    Some(toks.join(";"))
}

/// Normalize a stack SLP1 code for portal/backfill use: a bare `A` token
/// is always the आङ् upasarga (`AN`) — `dhatu_data` never writes `आङ्`,
/// so display `आ` is ambiguous and must prefer `AN`. (The corpus keeps
/// distinct `A`/`AN` pairs with different content for 44 dhātus; resolving
/// display `आ` to augment-`A` shows the wrong tables, e.g. 09.0032.)
/// All other tokens pass through untouched — no blanket Ṅ-guessing.
pub fn normalize_stack_slp1(stack_slp1: &str) -> String {
    stack_slp1
        .split(';')
        .map(|tok| {
            if tok == "A" {
                "AN".to_string()
            } else {
                tok.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_deva() {
        assert!(contains_deva("भवति"));
        assert!(contains_deva("praभ"));
        assert!(!contains_deva("Bavati"));
        assert!(!contains_deva("vi;ati"));
    }

    #[test]
    fn converts_forms() {
        assert_eq!(deva_to_slp1("भवति").as_deref(), Some("Bavati"));
        assert_eq!(deva_to_slp1("भवन्ति").as_deref(), Some("Bavanti"));
        assert_eq!(deva_to_slp1("भू").as_deref(), Some("BU"));
        assert_eq!(deva_to_slp1("राम").as_deref(), Some("rAma"));
        assert_eq!(deva_to_slp1("रामेण").as_deref(), Some("rAmeRa"));
        assert_eq!(deva_to_slp1("सम्").as_deref(), Some("sam"));
        assert_eq!(deva_to_slp1("प्र").as_deref(), Some("pra"));
    }

    #[test]
    fn converts_signs() {
        // Anusvara/visarga/candrabindu/avagraha codepoints (regression:
        // U+0902 anusvara was once mistyped as Bengali U+0982).
        assert_eq!(ANUSVARA, 'ं');
        assert_eq!(VISARGA, 'ः');
        assert_eq!(deva_to_slp1("हिं").as_deref(), Some("hiM"));
        assert_eq!(deva_to_slp1("हिंस्यात्").as_deref(), Some("hiMsyAt"));
        assert_eq!(deva_to_slp1("रामः").as_deref(), Some("rAmaH"));
        assert_eq!(deva_to_slp1("तेऽपि").as_deref(), Some("te'pi"));
    }

    #[test]
    fn converts_krt_names() {
        assert_eq!(deva_to_slp1("शतृ").as_deref(), Some("Satf"));
        assert_eq!(deva_to_slp1("क्वसु").as_deref(), Some("kvasu"));
        assert_eq!(deva_to_slp1("तृच्").as_deref(), Some("tfc"));
    }

    #[test]
    fn converts_stacks() {
        assert_eq!(deva_stack_to_slp1("प्र").as_deref(), Some("pra"));
        assert_eq!(deva_stack_to_slp1("वि + अति").as_deref(), Some("vi;ati"));
        assert_eq!(deva_stack_to_slp1("सम् + उद्").as_deref(), Some("sam;ud"));
    }

    #[test]
    fn converts_back() {
        assert_eq!(slp1_to_deva("Bavati").as_deref(), Some("भवति"));
        assert_eq!(slp1_to_deva("BU").as_deref(), Some("भू"));
        assert_eq!(slp1_to_deva("sam").as_deref(), Some("सम्"));
        assert_eq!(slp1_to_deva("pra").as_deref(), Some("प्र"));
        assert_eq!(slp1_to_deva("BavantI").as_deref(), Some("भवन्ती"));
    }

    #[test]
    fn roundtrips() {
        for word in ["भवति", "भवन्ति", "भू", "रामेण", "सम्", "शतृ", "भवन्ती", "एध्"]
        {
            let slp = deva_to_slp1(word).expect("converts");
            assert_eq!(
                slp1_to_deva(&slp).as_deref(),
                Some(word),
                "roundtrip {word}"
            );
        }
    }

    #[test]
    fn rejects_separators_and_unknown() {
        assert_eq!(deva_to_slp1(""), None);
        assert_eq!(deva_to_slp1("भव;ति"), None);
        assert_eq!(deva_to_slp1("Bavati"), None);
        assert_eq!(slp1_to_deva(""), None);
        assert_eq!(slp1_to_deva("praBavati;"), None);
        assert_eq!(slp1_to_deva("Bava/ti"), None);
    }

    #[test]
    fn candra_vowels_fold_to_plain() {
        // Foreign-name spellings decode: candra o/e have no SLP1 glyph and no
        // paradigm keys off the distinction, so they fold onto o/e — `जॉन`
        // must reach the word view as a declinable jona-class stem.
        assert_eq!(deva_to_slp1("जॉन").as_deref(), Some("jona"));
        assert_eq!(deva_to_slp1("जॉर्ज").as_deref(), Some("jorja"));
        assert_eq!(deva_to_slp1("पॅट").as_deref(), Some("pewa"));
        assert_eq!(deva_to_slp1("ऑन").as_deref(), Some("ona"));
    }
}
