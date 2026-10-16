//! Per-dhatu generation context (portal correctness layer).
//!
//! [`GenCtx`] conditions prefixed-formation allomorphs
//! (`anu;AN→avA`, `nil`-allomorphs, `satva`/`natva` gates, …). The default
//! context is only right for root-level (shuddha) tables; derived families
//! and lexical roots need their own flags. Everything here derives from
//! data the portal already ships:
//!
//! - engine category ← dhatuforms family (`forms::engine_ting_category` /
//!   `engine_krut_family`; `is_derived_category`, `yang*` gates),
//! - lexical flags ← vidyut SLP1 dhatu (same marker convention as the
//!   engine's upadeshas: `mana~`, `stanBu~`, `kF`, `ziDu~`, …),
//! - `nitya_adi` ← `dhatu_note` (`नित्यम् अधि-पूर्वकः`, exactly 02.0041/02.0042).
//!
//! Mirrors `GenCtx::for_entry` (vendored); the vendor stays untouched.

use crate::engine::upasarga::{is_derived_category, GenCtx};
use crate::revindex::LocKind;

/// Engine category for a base-form location (`ting`, `nich`, …, `nich_krut`).
#[must_use]
pub fn category_for_loc(loc: &LocKind) -> Option<&'static str> {
    match loc {
        LocKind::Ting { family, .. } => crate::forms::engine_ting_category(family),
        LocKind::Krt { family, .. } | LocKind::Subanta { family, .. } => {
            crate::forms::engine_krut_family(family)
        }
        LocKind::Prefixed { inner, .. } => category_for_loc(inner),
    }
}

/// Full context for one base form of one dhatu.
///
/// `vidyut_dhatu` is the raw TSV SLP1 dhatu (markers kept: `mana~`);
/// `dhatu_note` is the `DhatuEntry` note (Devanagari).
#[must_use]
pub fn ctx_for(vidyut_dhatu: &str, engine_category: &str, dhatu_note: &str) -> GenCtx {
    let upadesha = vidyut_dhatu.trim();
    GenCtx {
        derived: is_derived_category(engine_category),
        nitya_adi: dhatu_note.contains("नित्य") && dhatu_note.contains("अधि"),
        z_initial: upadesha.starts_with('z'),
        stambhi: upadesha.ends_with("anBu~"),
        sut_root: upadesha == "qukfY" || upadesha == "kF",
        niya_keep: matches!(
            upadesha,
            "pluN"
                | "labi~"
                | "liKa~"
                | "lI"
                | "kzuBa~"
                | "vana~"
                | "GurRa~"
                | "paRa~"
                | "ancu~"
                | "RaSa~"
        ),
        zadx_prati_keep: upadesha == "zadx~",
        aug_satva_keep: matches!(
            upadesha,
            "zRA" | "zaRu~" | "zaca~" | "ziDu~" | "ziY" | "zmiN" | "zu"
        ),
        intensive_keep: engine_category.starts_with("yang"),
        yangluk: engine_category.starts_with("yangluk"),
        s_voice: matches!(
            upadesha,
            "syandU~"
                | "ziDu~"
                | "ziY"
                | "svana~"
                | "zaha~"
                | "ziDa~"
                | "ziDU~"
                | "sPura~"
                | "zu"
                | "zaca~"
        ),
        mani_keep: matches!(upadesha, "mana~" | "manu~" | "mAna~"),
        nit_keep: matches!(upadesha, "mana~" | "manu~" | "mAna~" | "vana~"),
        nn_keep: matches!(
            upadesha,
            "pada~" | "Ruda~" | "vidx~" | "Bidi~r" | "hada~" | "Kida~" | "vida~" | "klidU~"
        ),
        ana_keep: upadesha == "ana~",
        asu_keep: upadesha == "aSU~",
    }
}

/// Context for a located base form (dhatu + declined). Missing vidyut or
/// unknown category degrades to default.
#[must_use]
pub fn ctx_for_loc(vidyut_dhatu: Option<&str>, dhatu_note: &str, loc: &LocKind) -> GenCtx {
    let (Some(dhatu), Some(category)) = (vidyut_dhatu, category_for_loc(loc)) else {
        return GenCtx::default();
    };
    ctx_for(dhatu, category, dhatu_note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_default_shape() {
        let ctx = ctx_for("BU", "ting", "");
        assert!(!ctx.derived);
        assert!(!ctx.z_initial);
        assert!(!ctx.nitya_adi);
    }

    #[test]
    fn derived_and_intensive_flags() {
        let ctx = ctx_for("BU", "nich", "");
        assert!(ctx.derived);
        assert!(!ctx.intensive_keep);
        let ctx = ctx_for("BU", "yangluk_krut", "");
        assert!(ctx.derived && ctx.intensive_keep && ctx.yangluk);
    }

    #[test]
    fn lexical_flags() {
        let ctx = ctx_for("stanBu~", "ting", "");
        assert!(ctx.stambhi);
        let ctx = ctx_for("mana~", "san", "");
        assert!(ctx.derived && ctx.mani_keep && ctx.nit_keep);
        let ctx = ctx_for("kF", "nich", "");
        assert!(ctx.sut_root);
        let ctx = ctx_for("zaha~", "ting", "");
        assert!(ctx.z_initial);
    }

    #[test]
    fn nitya_adi_note() {
        let note = "अयं नित्यम् अधि-पूर्वकः एव प्रयुक्तः दृश्यते ।";
        assert!(ctx_for("i\\N", "ting", note).nitya_adi);
        assert!(!ctx_for("BU", "ting", "").nitya_adi);
    }
}
