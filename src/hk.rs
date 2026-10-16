//! Harvard-Kyoto ↔ SLP1 (search-box input layer).
//!
//! The portal search box accepts Devanagari (see [`crate::translit`]) or
//! Harvard-Kyoto; the morphology core stays SLP1-only and SLP1 is never
//! shown in the portal. HK is tokenized longest-match first (`lRR`, `ai`,
//! `kh`, …). Strict ASCII-letters input; separators and unknown tokens
//! yield `None`.

/// Decode the HK token at the head of `rest` (longest match first).
fn hk_token(rest: &[u8]) -> Option<(&'static str, usize)> {
    if rest.starts_with(b"lRR") {
        return Some(("X", 3));
    }
    if rest.len() >= 2 {
        if rest.starts_with(b"lR") {
            return Some(("x", 2));
        } else if rest.starts_with(b"RR") {
            return Some(("F", 2));
        } else if rest.starts_with(b"ai") {
            return Some(("E", 2));
        } else if rest.starts_with(b"au") {
            return Some(("O", 2));
        } else if rest.starts_with(b"kh") {
            return Some(("K", 2));
        } else if rest.starts_with(b"gh") {
            return Some(("G", 2));
        } else if rest.starts_with(b"ch") {
            return Some(("C", 2));
        } else if rest.starts_with(b"jh") {
            return Some(("J", 2));
        } else if rest.starts_with(b"Th") {
            return Some(("W", 2));
        } else if rest.starts_with(b"Dh") {
            return Some(("Q", 2));
        } else if rest.starts_with(b"th") {
            return Some(("T", 2));
        } else if rest.starts_with(b"dh") {
            return Some(("D", 2));
        } else if rest.starts_with(b"ph") {
            return Some(("P", 2));
        } else if rest.starts_with(b"bh") {
            return Some(("B", 2));
        }
    }
    let slp = match rest[0] as char {
        'a' => "a",
        'A' => "A",
        'i' => "i",
        'I' => "I",
        'u' => "u",
        'U' => "U",
        'R' => "f",
        'e' => "e",
        'o' => "o",
        'M' => "M",
        'H' => "H",
        'k' => "k",
        'g' => "g",
        'G' => "N",
        'c' => "c",
        'j' => "j",
        'J' => "Y",
        'T' => "w",
        'D' => "q",
        'N' => "R",
        't' => "t",
        'd' => "d",
        'n' => "n",
        'p' => "p",
        'b' => "b",
        'm' => "m",
        'y' => "y",
        'r' => "r",
        'l' => "l",
        'L' => "L",
        'v' => "v",
        'z' => "S",
        'S' => "z",
        's' => "s",
        'h' => "h",
        _ => return None,
    };
    Some((slp, 1))
}

/// Harvard-Kyoto → SLP1 (`bhavati` → `Bavati`, `kRSNa` → `kfzRa`).
#[must_use]
pub fn hk_to_slp1(text: &str) -> Option<String> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut idx = 0;
    while idx < bytes.len() {
        let (slp, len) = hk_token(&bytes[idx..])?;
        out.push_str(slp);
        idx += len;
    }
    Some(out)
}

/// SLP1 → Harvard-Kyoto (`Bavati` → `bhavati`). Kept for roundtrips and the
/// CLI; the portal never displays either encoding.
#[must_use]
pub fn slp1_to_hk(text: &str) -> Option<String> {
    if text.is_empty()
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_alphabetic() || byte == b'\'')
    {
        return None;
    }
    let mut out = String::new();
    for byte in text.bytes() {
        let hk = match byte as char {
            'a' => "a",
            'A' => "A",
            'i' => "i",
            'I' => "I",
            'u' => "u",
            'U' => "U",
            'f' => "R",
            'F' => "RR",
            'x' => "lR",
            'X' => "lRR",
            'e' => "e",
            'E' => "ai",
            'o' => "o",
            'O' => "au",
            'M' => "M",
            'H' => "H",
            '\'' => "'",
            'k' => "k",
            'K' => "kh",
            'g' => "g",
            'G' => "gh",
            'N' => "G",
            'c' => "c",
            'C' => "ch",
            'j' => "j",
            'J' => "jh",
            'Y' => "J",
            'w' => "T",
            'W' => "Th",
            'q' => "D",
            'Q' => "Dh",
            'R' => "N",
            't' => "t",
            'T' => "th",
            'd' => "d",
            'D' => "dh",
            'n' => "n",
            'p' => "p",
            'P' => "ph",
            'b' => "b",
            'B' => "bh",
            'm' => "m",
            'y' => "y",
            'r' => "r",
            'l' => "l",
            'L' => "L",
            'v' => "v",
            'S' => "z",
            'z' => "S",
            's' => "s",
            'h' => "h",
            _ => return None,
        };
        out.push_str(hk);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_words() {
        assert_eq!(hk_to_slp1("bhavati").as_deref(), Some("Bavati"));
        assert_eq!(hk_to_slp1("kRSNa").as_deref(), Some("kfzRa"));
        assert_eq!(hk_to_slp1("saMskRta").as_deref(), Some("saMskfta"));
        assert_eq!(hk_to_slp1("lRta").as_deref(), Some("xta"));
    }

    #[test]
    fn fuses_diphthongs_and_aspirates() {
        // `gacchantau` must surface छ and औ, never च्ह … अ + उ.
        assert_eq!(hk_to_slp1("gacchantau").as_deref(), Some("gacCantO"));
        assert_eq!(hk_to_slp1("vai").as_deref(), Some("vE"));
        assert_eq!(hk_to_slp1("gauH").as_deref(), Some("gOH"));
        assert_eq!(slp1_to_hk("gacCantO").as_deref(), Some("gacchantau"));
    }

    #[test]
    fn encodes_words() {
        assert_eq!(slp1_to_hk("Bavati").as_deref(), Some("bhavati"));
        assert_eq!(slp1_to_hk("kfzRa").as_deref(), Some("kRSNa"));
    }

    #[test]
    fn roundtrips() {
        for word in ["bhavati", "kRSNa", "saMskRta", "rAma", "Gacchati", "lRR"] {
            let slp = hk_to_slp1(word).expect("decodes");
            assert_eq!(slp1_to_hk(&slp).as_deref(), Some(word), "roundtrip {word}");
        }
    }

    #[test]
    fn rejects_non_hk() {
        assert_eq!(hk_to_slp1(""), None);
        assert_eq!(hk_to_slp1("Bavati"), None);
        assert_eq!(hk_to_slp1("bhava;ti"), None);
        assert_eq!(hk_to_slp1("123"), None);
        assert_eq!(slp1_to_hk(""), None);
        assert_eq!(slp1_to_hk("Bava/ti"), None);
    }
}
