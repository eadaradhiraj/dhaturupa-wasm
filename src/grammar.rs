//! Grammatical tags for SLP1 result cards.
//!
//! Derives display tags from a reverse-index location path such as
//! `conjugations.ting.plat`, `participles.krut.Satf.f` or
//! `upasarga_forms.pra.conjugations.ting.plat`.
//!
//! Honesty rule: `purusha`/`vacana` are reported only for 9-slot paradigms
//! (pos 0–8, order prathama/madhyama/uttama × eka/dvi/bahu) or multiples of
//! 9 (`pos % 9`). Other array shapes carry variant forms whose slot mapping
//! is not positional.

/// Pada decoded from a lakara-key initial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pada {
    /// Parasmaipada (`p*` keys).
    Parasmai,
    /// Atmanepada (`a*` keys).
    Atmane,
}

impl Pada {
    /// SLP1 display, e.g. `parasmEpadam`.
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            Self::Parasmai => "parasmEpadam",
            Self::Atmane => "AtmanEpadam",
        }
    }
}

/// Split a lakara key (`plat`, `alrut`, `pvidhiling`) into pada + lakara stem.
#[must_use]
pub fn split_lakara_key(key: &str) -> Option<(Pada, &str)> {
    let (initial, rest) = key.split_at_checked(1)?;
    let pada = match initial {
        "p" => Pada::Parasmai,
        "a" => Pada::Atmane,
        _ => return None,
    };
    match rest {
        "ashirling" | "lang" | "lat" | "lit" | "lot" | "lrung" | "lrut" | "lung" | "lut"
        | "vidhiling" => Some((pada, rest)),
        _ => None,
    }
}

/// Display name for a lakara stem, e.g. `lat` -> `latlakAraH`.
#[must_use]
pub fn lakara_display(lakara: &str) -> String {
    format!("{lakara}lakAraH")
}

/// Purusha for a 0–8 slot index.
#[must_use]
pub fn purusha_of(pos: usize) -> &'static str {
    match pos / 3 {
        0 => "praTamapuruzaH",
        1 => "maDyamapuruzaH",
        _ => "uttamapuruzaH",
    }
}

/// Vacana for a 0–8 slot index.
#[must_use]
pub fn vacana_of(pos: usize) -> &'static str {
    match pos % 3 {
        0 => "ekavacanam",
        1 => "dvivacanam",
        _ => "bahuvacanam",
    }
}

/// Whether a positional purusha/vacana reading is sound for an array of
/// length `len` at position `pos`: true for 9-slots and multiples of 9.
#[must_use]
pub fn has_positional_slot(len: usize, pos: usize) -> bool {
    len.is_multiple_of(9) && pos < len
}

/// Card tags for one tinanta occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TinantaTags {
    /// Top category, e.g. `ting`.
    pub category: String,
    /// Lakara display, e.g. `latlakAraH`.
    pub lakara: String,
    /// Pada display, e.g. `parasmEpadam`.
    pub pada: String,
    /// Present only when [`has_positional_slot`] holds.
    pub purusha: Option<String>,
    /// Present only when [`has_positional_slot`] holds.
    pub vacana: Option<String>,
    /// Array index (always present; variant slot reference).
    pub index: usize,
}

/// Build tags for `(category, lakara_key, pos, array_len)`.
///
/// Returns `None` when the lakara key is unknown.
#[must_use]
pub fn tinanta_tags(
    category: &str,
    lakara_key: &str,
    pos: usize,
    array_len: usize,
) -> Option<TinantaTags> {
    let (pada, lakara) = split_lakara_key(lakara_key)?;
    let positional = has_positional_slot(array_len, pos);
    Some(TinantaTags {
        category: category.to_string(),
        lakara: lakara_display(lakara),
        pada: pada.display().to_string(),
        purusha: positional.then(|| purusha_of(pos % 9).to_string()),
        vacana: positional.then(|| vacana_of(pos % 9).to_string()),
        index: pos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lakara_keys() {
        assert_eq!(split_lakara_key("plat"), Some((Pada::Parasmai, "lat")));
        assert_eq!(split_lakara_key("alang"), Some((Pada::Atmane, "lang")));
        assert_eq!(split_lakara_key("xlat"), None);
    }

    #[test]
    fn tags_nine_slot() {
        let t = tinanta_tags("ting", "plat", 1, 9).expect("known key");
        assert_eq!(t.lakara, "latlakAraH");
        assert_eq!(t.purusha.as_deref(), Some("praTamapuruzaH"));
        assert_eq!(t.vacana.as_deref(), Some("dvivacanam"));
    }

    #[test]
    fn tags_non_nine_have_no_slot() {
        let t = tinanta_tags("ting", "plat", 0, 5).expect("known key");
        assert_eq!(t.purusha, None);
        assert_eq!(t.vacana, None);
    }
}
