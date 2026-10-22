//! SLP1 helpers — byte-exact, case-sensitive.
//!
//! Engine invariant (SPEC §3 C1): the morphology core only sees SLP1.
//! `e != E`, `n != N`. No normalization, no case folding.
//! Devanagari / Harvard-Kyoto input is translated at the UI boundary;
//! see `vidyut` for the `baseindex -> SLP1 dhatu` join.

/// True for ASCII letters, the only bytes observed in oracle forms.
///
/// Tinanta forms use `A-Za-z` only; krdanta cells add `/` for alternants
/// (`Bavat/Bavad`); upasarga stacks join with `;` (`vi;ati`).
/// `/` and `;` are separators, not SLP1. The site's compound joint in a
/// word itself (`अतिथीनां_सपर्या`, db id `@atithInAM_saparyA1`) does
/// pass through SLP1 as `_` — it is part of the citation form, and the
/// joint word's grid is keyed with it (see `SITE_GRIDS`).
#[must_use]
pub const fn is_slp1_letter_byte(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

/// Byte-exact equality. No normalization.
#[must_use]
pub fn eq_exact(a: &str, b: &str) -> bool {
    a.as_bytes() == b.as_bytes()
}

/// Non-empty string of SLP1 letters only.
#[must_use]
pub fn is_slp1_form(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(is_slp1_letter_byte)
}

/// Split a krdanta cell on `/` into alternants.
///
/// `Bavat/Bavad` -> `["Bavat", "Bavad"]`; `""` -> `[]`.
#[must_use]
pub fn split_alternants(cell: &str) -> Vec<&str> {
    if cell.is_empty() {
        return Vec::new();
    }
    cell.split('/').filter(|p| !p.is_empty()).collect()
}

/// Split a stacked upasarga key on `;`: `vi;ati` -> `["vi", "ati"]`.
#[must_use]
pub fn split_prefix_stack(stack: &str) -> Vec<&str> {
    if stack.is_empty() {
        return Vec::new();
    }
    stack.split(';').filter(|p| !p.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_is_byte_sensitive() {
        assert!(eq_exact("BavataH", "BavataH"));
        assert!(!eq_exact("BavataH", "bavataH"));
        assert!(!eq_exact("WrampeRa", "WrampeNa"));
    }

    #[test]
    fn forms_and_splits() {
        assert!(is_slp1_form("BavataH"));
        assert!(!is_slp1_form("BavatiH "));
        assert_eq!(split_alternants("Bavat/Bavad"), vec!["Bavat", "Bavad"]);
        assert_eq!(split_prefix_stack("vi;ati"), vec!["vi", "ati"]);
    }
}
