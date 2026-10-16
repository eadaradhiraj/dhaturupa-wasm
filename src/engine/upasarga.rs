//! Upasarga handling (F3): prefix stacks, rule-based prefixed generation,
//! and oracle verification.
//!
//! Generation is runtime rule-based via [`crate::engine::sandhi::join`]: a stack
//! folds left-to-right over its `;`-separated tokens and then over the base
//! form. Folds carry variant sets through principled optionality points, so
//! one call returns every licensed surface (usually one). No precompute
//! index, no lookup tables, no max depth.

use crate::engine::sandhi::{join, natva, satva};
use crate::slp1::split_prefix_stack;
use std::collections::BTreeSet;

/// Generation context: grammatical properties of the dhatu/category that
/// condition formation allomorphs. Callers read these off the JSON entry
/// (`info` + category); no oracle strings are consulted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct GenCtx {
    /// True for derivational categories (`nich`/`san`/`yang`/`yangluk`,
    /// their `_yak` forms and `_*_krut` families); false for root-level
    /// (`ting`, `yak`, `krut`).
    pub derived: bool,
    /// True for nitya-upasarga dhatus (`DAtuviSezaH` carries
    /// `nityam aDi-pUrvakaH`: `02.0041`, `02.0042`). A stack ending in
    /// `aDi` attaches idempotently for these (evidence `aDyApyAt`,
    /// `samaDIte` stay bare; non-nitya `03.0011` doubles: `aDyaDitsizyAt`).
    pub nitya_adi: bool,
    /// True when the upadeSa is ṣ-initial (`z...`: `zevf~`, `zaha~`,
    /// `zadx~`…). Base-region `s` voices to `z` after `iN`-vowels only for
    /// these (evidence 20/22 voicing dhatus; `s`-roots like `su`, `san`
    /// keep `s`: `parisavAni`).
    pub z_initial: bool,
    /// True for `stambh`-class roots (upadeSa ends `anBu~`: `stanBu~`,
    /// `skanBu~`). Base-initial `st` + vowel voices to `zw` after prefixes
    /// (evidence `pratizwamBayizIzwa`); `star`/`styA`/`sTA` roots keep `st`.
    pub stambhi: bool,
    /// True for kṛ-"to do" roots (upadeSa `qukfY`/`kF`). Prefixed
    /// `k`-initial stems license an `s`-āgama variant (`suṭ`; evidence
    /// `upaskArayizIzwa`, `saMskArayizIzwa`, `parizkArayizIzwa` alongside
    /// plain `upakArayizIzwa`-type forms).
    pub sut_root: bool,
    /// True for roots whose `n` never voices before `-Iy-` (gerundive
    /// `-anIya-`, optative `-IyA-`) under prefixation (evidence `pluN`,
    /// `labi~`, `liKa~`, `lI`, `kzuBa~`, `vana~`, `GurRa~`, `paRa~`, `ancu~`
    /// keep dental `n` in all aligned `n-I-y` oracle pairs, e.g.
    /// `aBipariplavanIya`, `saMpralinIyAt`, while other roots voice, e.g.
    /// `nirBavaRIya`). Lexical, like
    /// [`GenCtx::sut_root`]: no (stack, base)-rule distinguishes them
    /// (same-shape minimal pairs verified: `aBisinoti` keeps vs
    /// `aBizuRoti` voices under one stack).
    pub niya_keep: bool,
    /// True for `zadx~` (sad): its `prati`-prefixed augment-`s` never
    /// voices (evidence 210 unanimous keep pairs, e.g. `pratyasatsyat`,
    /// while ten other stacks voice the same cells and `prati` voices
    /// elsewhere), nor does its root-initial `s` (evidence
    /// `pratisAdayAmbaBUva`-type lit keeps). Stack–dhatu-lexical; scoped
    /// to bare-`prati` stacks as evidenced.
    pub zadx_prati_keep: bool,
    /// True for roots whose post-augment `s` never voices (augment stays
    /// opaque to the trigger, pre-`F2` behavior; evidence: seven roots with
    /// zero voices across 6,962 aligned augment oracle pairs — `ziDu~`,
    /// `ziY`, `zu`, `zRA`, `zaRu~`, `zaca~`, `zmiN` — while siblings voice
    /// in the same shapes, e.g. `ziDU~` / `ziDa~` voice where `ziDu~`
    /// keeps). `zvanja~`/`zaha~`/`zivu~` voice under `pari`-family stacks
    /// (evidence `paryazvaYji`-type) and ride the keep-doublet elsewhere.
    /// Non-augment voicing is untouched (`zaha~`, `zivu~` voice there).
    /// Lexical, like [`GenCtx::niya_keep`].
    pub aug_satva_keep: bool,
    /// True for roots whose word-final `-ni` never voices to `-Ri` under
    /// prefixation (evidence `mana~`, `manu~`, `mAna~` keep `-mAni` in all
    /// aligned pairs, 48+28+16 with zero voices, e.g. `parima...mAni`-type;
    /// `yama~` voices `-mARi` (3) alongside keeps and stays unflagged).
    /// Lexical, like [`GenCtx::niya_keep`].
    pub mani_keep: bool,
    /// True for roots whose `n` never voices before `-it-` or `-y-`
    /// (evidence `mana~`, `manu~`, `mAna~`, `vana~` keep dental `n` in all
    /// `n-i-t` and `n-y` oracle pairs with zero `Rit*`/`Ry*` voices
    /// corpus-wide, e.g. `parivaMvAnitA`-, `paryavAnyata`-,
    /// `aBipramaMmanyAt`-type, while 85 other files voice `RitA`-type and
    /// `yang` voices `Rya`-type elsewhere). Lexical, like
    /// [`GenCtx::niya_keep`].
    pub nit_keep: bool,
    /// True for roots whose base-internal geminate `nn` never voices to
    /// `Mn`/`RR` under prefixation (evidence `pada~` `04.0065`
    /// `aBinizpanna`-type and `Ruda~` `06.0162`/`06.0002` `nirRunna`-type
    /// keep `nn` in all aligned pairs, while `Rama~` `01.1136`
    /// `nirnaMnam`-type voices; unanimous split). Lexical, like
    /// [`GenCtx::niya_keep`].
    pub nn_keep: bool,
    /// True for `ana~`: base-first `n` keeps before `i`+`D`/`k`/`H`,
    /// `i`+`v`+vowel, and `a`/`A`+`m`/`H` (evidence `04.0072`
    /// `aBiprAniDve`/`aBiprAnikA`/`aBiprAnivas`/`aBiprAnam`-type keeps
    /// across stacks; short-`a` `-anam` voices stay lexical for other
    /// roots per the M4 `-anam` item). Lexical, like
    /// [`GenCtx::niya_keep`].
    pub ana_keep: bool,
    /// True for `aSU~`: base-first `n` keeps before `a`+`S` (evidence
    /// `05.0020` `prAnaSe`-type keeps; `n`+`a`+`s`→`RaS` voices stay
    /// lexical elsewhere — 2,598 `praRaSyAt`-type voices, mostly
    /// `04.0091`). Lexical, like [`GenCtx::niya_keep`].
    pub asu_keep: bool,
    /// True for roots licensing keep+voice `s`/`z` doublets under triggering
    /// stacks (both surfaces licensed, membership-tested): `s`-roots that
    /// voice like `z`-roots (`syandU~`: evidence `01.0865` `nich.alang`
    /// `aByazyandayata`-type voices alongside keeps; `svana~`: evidence
    /// `01.0961` `vizvARayizIzwa`-type, where the gate also enables
    /// `satva`-sourced `natva` under triggerless surfaces) and `z`-roots
    /// with keep doublets (`ziDu~`: `niseDayizIzwa`-type keeps alongside
    /// voices; `ziY`: `visAyayAmbaBUva`-type keeps alongside
    /// `vizAyayAmbaBUva`, 12+ vs 6+ oracle doublets; `zaha~`:
    /// `visAhayate`-type present keeps alongside `vizAhayAmbaBUva`-type
    /// perfect voices; `ziDa~`/`ziDU~`/`sPura~`: voice under
    /// `ni`/`prati`/`vi`-family stacks alongside keeps). Lexical, like
    /// [`GenCtx::niya_keep`].
    pub s_voice: bool,
    /// True for intensive categories (`yang`/`yangluk`, their `_yak` forms
    /// and `_*_krut` families): base-internal `n` followed by `I` + `p`,
    /// or base-first `n` followed by `a` + `M`, never voices to `R` under
    /// prefixation (reduplication-aware natva; evidence `paripanIpat-`-type
    /// keeps across stacks — all 11,602 `nIp`-keep miss rows are `yang*` in
    /// two dhatus while `RIp`-voice oracle forms are non-intensive — and
    /// `nirnaMnam`-type keeps with zero `yang` base-`naM`→`RaM` voices).
    /// Narrowly scoped to `I`+`p` and base-first `n`+`a`+`M` exactly.
    /// Category-conditioned, like [`GenCtx::derived`].
    pub intensive_keep: bool,
    /// True for intensive-reduplicated categories (`yangluk`,
    /// `yangluk_yak` and `yangluk_krut` families): reduplicant `i`/`I`
    /// syncopates under prefixation (`sarisarA` → `sarsarA`-type;
    /// `yang` keeps full grades via the base path). Category-conditioned.
    pub yangluk: bool,
}

impl GenCtx {
    /// Context for a `(file, category)` pair under test.
    #[must_use]
    pub fn for_entry(file: &crate::engine::data::DhatuFile, category: &str) -> Self {
        let viseza = file.info_value("DAtuviSezaH");
        let upadesha = file.upadesha();
        Self {
            derived: is_derived_category(category),
            nitya_adi: viseza.contains("nityam aDi-pUrvaka"),
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
            // Narrowly scoped (see field docs): base-internal `n`+`I`+`p`.
            intensive_keep: category.starts_with("yang"),
            yangluk: category.starts_with("yangluk"),
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
}

/// Distinct prefix stacks (`pra`, `vi;ati`, …) observed in the corpus.
#[must_use]
pub fn known_stacks(corpus: &[crate::engine::data::DhatuFile]) -> BTreeSet<String> {
    corpus
        .iter()
        .filter(|f| !f.skipped)
        .flat_map(|f| f.upasarga_forms.keys().cloned())
        .collect()
}

/// Whether a conjugations category or participle family is derivational
/// (`nich`/`san`/`yang`/`yangluk` and their `_yak`/`_*_krut` forms) as
/// opposed to root-level (`ting`, `yak`, `krut`).
#[must_use]
pub fn is_derived_category(name: &str) -> bool {
    name != "ting" && name != "yak" && name != "krut"
}

/// Realize a stack's surface forms (fold [`join`] over `;`-tokens).
///
/// Formation allomorphs (oracle-conditioned):
/// - adjacent `anu;AN` surfaces as `avA` before derived stems and as `anvA`
///   before root stems (evidence: `01.1031` gives `anvAyacCati` (ting) vs
///   `avAyamayati` (nich); holds across all 10 categories with zero
///   counterexamples).
/// - trailing `aDi` drops for nitya-`aDi` dhatus (see [`GenCtx`]).
///
/// Checks whether a base is `ay`- or `Ay`-initial, which selects the `r`→`l`
/// allomorphy for `parA`→`pala` and `pra`→`pla` (evidence `palAyya`-type for
/// `Ay` and `palAyizIzwa`-type for `ay`).
fn starts_ay(base: &str) -> bool {
    let b = base.as_bytes();
    matches!(b.first(), Some(b'a' | b'A')) && b.get(1) == Some(&b'y')
}

fn stack_surface(stack: &str, ctx: GenCtx) -> Vec<String> {
    let mut toks = split_prefix_stack(stack);
    if ctx.nitya_adi && toks.last() == Some(&"aDi") {
        toks.pop();
    }
    if ctx.derived && toks.len() >= 2 {
        let (penult, last) = (toks[toks.len() - 2], toks[toks.len() - 1]);
        // `anu;AN -> avA`, `ni;AN -> yA` before derived stems (evidence
        // `anvAyacCati`/`avAyamayati`, `upanyAcarati`/`upayAcArayati` splits
        // across all 10 categories with zero counterexamples).
        let fused = match (penult, last) {
            ("anu", "AN") => Some("avA"),
            ("ni", "AN") => Some("yA"),
            _ => None,
        };
        if let Some(surf) = fused {
            let mut accs = vec![String::new()];
            for t in &toks[..toks.len() - 2] {
                accs = fold_token(&accs, t);
            }
            let mut out = Vec::with_capacity(accs.len());
            for acc in &accs {
                out.extend(join(acc, surf));
            }
            return out;
        }
    }
    let mut accs = vec![String::new()];
    for t in &toks {
        accs = fold_token(&accs, t);
    }
    accs
}

/// Fold one token over a variant set, capping runaway ambiguity.
fn fold_token(accs: &[String], token: &str) -> Vec<String> {
    let mut out = Vec::with_capacity(accs.len() * 2);
    for acc in accs {
        out.extend(join(acc, token));
    }
    out.sort();
    out.dedup();
    out
}

/// `stambh`-class base pre-transform: base-initial `st` + vowel voices to
/// `zw`, `sk` + vowel to `zk` (evidence `stamBayizIzwa -> pratizwamBayizIzwa`,
/// `skaBnAti -> vizkaBnoti`), preserving an augment `a`
/// (`astamBayata -> pratyazwamBayata`). Returns the stem to join;
/// non-matching bases pass through untouched. Callers dispatch by stack
/// surface (see [`generate_prefixed`]): high-vowel finals voice, `a`/`A`
/// finals license both, `m`/`d` finals keep.
fn stambhi_stem(base: &str) -> String {
    let (aug, stem) = base.strip_prefix('a').map_or(("", base), |r| ("a", r));
    let sb = stem.as_bytes();
    if sb.len() > 2 && sb[0] == b's' && crate::engine::sandhi::is_vowel_byte(sb[2]) {
        if sb[1] == b't' {
            return format!("{aug}zw{}", &stem[2..]);
        }
        if sb[1] == b'k' {
            return format!("{aug}zk{}", &stem[2..]);
        }
    }
    base.to_string()
}

/// Rule-based prefixed generation: [`stack_surface`] + [`join`] + [`satva`]
/// + [`natva`].
///
/// Returns every licensed surface (usually exactly one; two at `sam` /
/// `nir` / `ud` optionality points). Total over stacks (no max depth).
/// [`satva`] fires only for ṣ-roots ([`GenCtx::z_initial`]); [`natva`]
/// fires only when the surface (or [`satva`]-created `z`) contributes an
/// `r`/`z` trigger; `-`-bearing anomaly stacks bypass post-rules.
#[must_use]
pub fn generate_prefixed(stack: &str, base_form: &str, ctx: GenCtx) -> Vec<String> {
    if stack.is_empty() {
        return vec![base_form.to_string()];
    }
    let base_plain = base_form.to_string();
    let base_stambhi = if ctx.stambhi {
        stambhi_stem(base_form)
    } else {
        base_form.to_string()
    };
    // Stambhi voicing is stack-conditioned: `a`/`A`-final surfaces keep
    // `st` (evidence `avastamBayizIzwa`-type), all other finals voice.
    // Computed here where the surface is visible (per-`acc` below).
    // `join`'s m-final rule handles `sam` itself: `sam`+vowel keeps `m`,
    // `sam`+sibilant/`h`/`y/r/l/v` gives `saM`, and `sam`+stop/nasal gives
    // BOTH `saM` and the parasavarṇa form (`saN`/`saY`/`saR`/`sam`…), which
    // the corpus lists as separate rows.
    let mut accs = stack_surface(stack, ctx);
    // `nil`-allomorph for `nir`-final stacks before vowel-initial bases
    // (evidence 01.0546 `apanilayizIzwa`-type alongside keeping `nir`;
    // formation-conditioned, so both licensed and membership-tested;
    // `nir`+consonant always keeps `r`, evidence `nirBavanIya`-type).
    if base_plain
        .as_bytes()
        .first()
        .is_some_and(|b| crate::engine::sandhi::is_vowel_byte(*b))
        && split_prefix_stack(stack).last() == Some(&"nir")
    {
        let mut nils = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("nir") {
                nils.push(format!("{head}nil"));
            }
        }
        accs.extend(nils);
    }
    // `parA` → `pala` allomorph before `ay`/`Ay`-initial bases (same
    // `r`→`l` family; evidence `palAyya` for `parA`+`plAyya` and
    // `palAyizIzwa` for `parA`+`ayizIzwa`, while `parA` keeps `r`
    // elsewhere). Both licensed, membership-tested.
    if starts_ay(&base_plain) && split_prefix_stack(stack).last() == Some(&"parA") {
        let mut palas = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("parA") {
                palas.push(format!("{head}pala"));
            }
        }
        accs.extend(palas);
    }
    // `pra` → `pla` allomorph before `ay`/`Ay`-initial bases (same `r`→`l`
    // family as `parA`→`pala`; evidence `01.0546` `upaplAyayizIzwa`-type
    // for `upa;pra`+`AyayizIzwa`, while `pra` keeps `r` elsewhere, evidence
    // `upaprAyizIzwa`-type). Both licensed, membership-tested.
    if starts_ay(&base_plain) && split_prefix_stack(stack).last() == Some(&"pra") {
        let mut plas = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("pra") {
                plas.push(format!("{head}pla"));
            }
        }
        accs.extend(plas);
    }
    // `pari` → `pali` allomorph before `ay`/`Ay`-initial bases (same `r`→`l`
    // family, final vowel preserved; evidence `01.0546`
    // `vipalyAyayizIzwa`-type for `vi;pari`+`AyayizIzwa`, while `pari`
    // keeps `r` elsewhere). Both licensed, membership-tested.
    if starts_ay(&base_plain) && split_prefix_stack(stack).last() == Some(&"pari") {
        let mut palis = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("pari") {
                palis.push(format!("{head}pali"));
            }
        }
        accs.extend(palis);
    }
    // `vi;pari` → `vipaly` surface for `Ati`-stems (`vipaly` + `Ati`
    // → `vipalyAti`, evidence `01.0546` `nich_krut.ktin`; the `Ayti`→`Ati`
    // twin drops the stem-`y` that triggers palatalization, so the surface
    // must license independently; keeps ride the base path).
    // Both licensed, membership-tested.
    if stack == "vi;pari" && base_plain.starts_with('A') {
        let mut palis = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("pari") {
                palis.push(format!("{head}pali"));
            }
        }
        accs.extend(palis);
    }
    // `apa` → `Ap` before mid/high-vowel-initial bases (evidence
    // `ApEjayata`-type, `ApejizIzwa`-type, `ApocCizIzwa`-type and
    // `ApArArAmbaBUva`-type, with zero keeps; `Ek` splits keep
    // (`apEkzata`) + voice (`ApEkzata`), both licensed). Applies to
    // `apa`-final stacks (evidence `vyApekzizIzwa`/`nirApekzizIzwa`-type
    // alongside `vyapekzayizIzwa`-type keeps; compounded `apa` fuses
    // normally for other shapes, evidence `vyapohayizIzwa`-type keeps).
    // `a`-initial keeps like `apAseDayata` stay. Short-`a`-initial takes
    // `Ap` too alongside keeps (evidence `01.1086` yangluk `ApArAryAt`-type
    // for `apa`+`arAryAt` across all yangluk slots). Both licensed,
    // membership-tested.
    if (base_plain.starts_with('e')
        || base_plain.starts_with('E')
        || base_plain.starts_with('u')
        || base_plain.starts_with('U')
        || base_plain.starts_with('o')
        || base_plain.starts_with('O')
        || base_plain.starts_with('a')
        || base_plain.starts_with('A'))
        && split_prefix_stack(stack).last() == Some(&"apa")
    {
        let mut aps = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("apa") {
                aps.push(format!("{head}Ap"));
                // `apa`→`ap` elision before u/U-initial intensive stems
                // (evidence `01.1160` `apuzyAt`-type for `apa`+`uzyAt`;
                // vrddhi `Ap` also licensed, membership-tested).
                if base_plain.starts_with('u') || base_plain.starts_with('U') {
                    aps.push(format!("{head}ap"));
                }
            }
        }
        accs.extend(aps);
    }
    // Prefix-final `i` lengthens before `kASa` in `krut.ac` (`aBi`→`aBI`,
    // `ati`→`atI`, `ni`→`nI`, `prati`→`pratI`, `vi`→`vI`; evidence
    // `01.0734`/`04.0058` `aBIkASa`-type — both gaNas, Atmanepada;
    // `ting` + all other `krt` keep short `i` via the base path, e.g.
    // `vikASate`-type; membership-tested). `vAra` too (`ni`+`vAra` →
    // `nIvAra`, evidence `09.0045` `krut.GaY`).
    if (base_plain.starts_with("kAS") || base_plain.starts_with("vAra"))
        && matches!(
            split_prefix_stack(stack).last(),
            Some(&"aBi" | &"ati" | &"ni" | &"prati" | &"vi")
        )
    {
        let mut longs = Vec::new();
        for acc in &accs {
            for (end, long) in [
                ("aBi", "aBI"),
                ("ati", "atI"),
                ("ni", "nI"),
                ("prati", "pratI"),
                ("vi", "vI"),
            ] {
                if let Some(head) = acc.strip_suffix(end) {
                    longs.push(format!("{head}{long}"));
                }
            }
        }
        accs.extend(longs);
    }
    // `upa` → `up` before `u`-initial bases (evidence `upuzyate`-type for
    // `upa`+`uzyate`; keeps ride the base path). Scoped to bare `upa`
    // exactly as evidenced. Both licensed, membership-tested.
    if base_plain.starts_with('u') && stack == "upa" {
        let mut ups = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("upa") {
                ups.push(format!("{head}up"));
            }
        }
        accs.extend(ups);
    }
    // `upa` → `up` before `Uhy`-initial intensive stems in COMPOUND stacks
    // ending in `upa` (evidence `01.0735` `sam;upa`+`UhyAt` → `samupuhyAt`
    // 37 rows; keeps ride the base path). Scoped to `Uhy`/`uhy`-initial bases
    // (solely `Uha~` intensive) to avoid regressions. Both licensed,
    // membership-tested.
    if (base_plain.starts_with("Uhy") || base_plain.starts_with("uhy"))
        && split_prefix_stack(stack).last() == Some(&"upa")
        && stack != "upa"
    {
        let mut ups = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("upa") {
                ups.push(format!("{head}up"));
            }
        }
        accs.extend(ups);
    }
    // `pra;ni` drops `ni` before `R`-initial stems (`pra;ni`+`Ritta`
    // → `praRitta`, evidence `03.0010` `krut.kta`/`ktavatu`/`ktin`/
    // `ktri`; `praniRitta`-type keeps ride the base path).
    // Both licensed, membership-tested.
    if stack == "pra;ni" && base_plain.starts_with('R') {
        let mut drops = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("ni") {
                drops.push(head.to_string());
            }
        }
        accs.extend(drops);
    }
    if base_plain.starts_with('u') && stack == "pra" {
        let mut prs = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("pra") {
                prs.push(format!("{head}pr"));
            }
        }
        accs.extend(prs);
    }
    // `pra` → `pr` before `u`/`U`-initial bases in COMPOUND stacks ending
    // in `pra` (evidence `01.1160` `apa;pra`+`uzyAt` → `apapruzyAt` across
    // 70+ cells; keeps ride the base path). Scoped to stacks whose last
    // token is `pra` with a preceding prefix. Both licensed, membership-tested.
    if (base_plain.starts_with('u') || base_plain.starts_with('U'))
        && split_prefix_stack(stack).last() == Some(&"pra")
        && stack != "pra"
    {
        let mut prs = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix("pra") {
                prs.push(format!("{head}pr"));
            }
        }
        accs.extend(prs);
    }
    // Compensatory lengthening/syncope before `tta`-stems (`dA`→`tta`
    // formation): short `i`/`u`-final surfaces lengthen (`anu`+`ttavat` →
    // `anUttavat`, `ni` → `nIttavat`, `vi`+`tta` → `vItta`; evidence
    // `03.0010` ktavatu-neuter and kta/ktin/ktri), while short-`a`-final
    // surfaces syncopate (`pra`+`tta` → `pratta`, `upa` → `upatta`).
    // `A`-final (`AN` → `Atta` via `N`-lopa) and consonant-final
    // (`sam`/`ud` keep full `datta` via the base path) need nothing.
    // Feminine `ttA`/`ttI`, short-`i` `tti`, and `ttrima`-stems lengthen
    // too (evidence `03.0010` `anUttA`-type for `dattA`, `anUtti`-type
    // for `datti`, `anUttrima`-type for `dattrima` under `anu`/`ati`;
    // the gate was literal short-`a` `tta`).
    // Both licensed, membership-tested.
    if base_plain.starts_with("tta")
        || base_plain.starts_with("ttA")
        || base_plain.starts_with("ttI")
        || base_plain.starts_with("tti")
        || base_plain.starts_with("ttr")
    {
        let mut longs = Vec::new();
        for acc in &accs {
            if let Some(head) = acc.strip_suffix('i') {
                longs.push(format!("{head}I"));
            } else if let Some(head) = acc.strip_suffix('u') {
                longs.push(format!("{head}U"));
            } else if let Some(head) = acc.strip_suffix('a') {
                longs.push(head.to_string());
            }
        }
        accs.extend(longs);
    }
    // Pair-scoped opaque augment: `zadx~` keeps post-augment `s` under
    // bare `prati` (see `zadx_prati_keep`); computed here where both the
    // stack string and the context are visible.
    let aug_keep = ctx.aug_satva_keep || (ctx.zadx_prati_keep && stack == "prati");
    let mut out: Vec<String> = Vec::with_capacity(accs.len() * 4);
    // Insertion-ordered, manually deduped: direct/plain surfaces first so
    // `[0]` stays the canonical display form.
    let mut push = |s: String| {
        if !out.contains(&s) {
            out.push(s);
        }
    };
    for acc in &accs {
        // Stambhi per-surface choice (see above): `a`/`A`-final surfaces
        // license both keep and voice (evidence `ava` 10+10 pairs);
        // `m`- and `d`-final surfaces keep (`sam` 10+0 keeps, `ud` pairs via
        // the `d`+`st` join variants); all other finals voice (evidence
        // `vi`/`ni`/`pari` 10+0 voices). Keep-first ordering preserves the
        // canonical display `[0]`.
        let bases: Vec<&String> = if ctx.stambhi {
            match acc.as_bytes().last() {
                Some(b'a' | b'A') => vec![&base_plain, &base_stambhi],
                Some(b'm' | b'd') => vec![&base_plain],
                _ => vec![&base_stambhi],
            }
        } else {
            vec![&base_plain]
        };
        for base in bases {
            // Fusion-`y`/`v` keep: a stack surface ending in `ni`/`nu` fused
            // with a vowel-initial base (`i`/`u` + vowel → `y`/`v`, same
            // length), so the stack `n` meets fusion-created `y`/`v`
            // (evidence `durnyamApayata`-, `parinyavartayata`-type keeps).
            // Base-initial `y`/`v` still voices (evidence `durRyamem`-type,
            // 4,470 rows) as does same-vowel dirgha (`i`+`i`, `u`+`u`;
            // unknown, preserved). Computed here where surface and base
            // are both visible; `natva` itself only sees the join.
            let b0 = base.as_bytes().first();
            let fusion_keep = b0.is_some_and(|c| crate::engine::sandhi::is_vowel_byte(*c))
                && ((acc.ends_with("ni") && !matches!(b0, Some(b'i' | b'I')))
                    || (acc.ends_with("nu") && !matches!(b0, Some(b'u' | b'U'))));
            for raw in join(acc, base) {
                if stack.contains('-') {
                    push(raw);
                    continue;
                }
                let voiced = satva(&raw, acc, ctx.z_initial || ctx.s_voice, aug_keep);
                let surface_triggered = acc.bytes().any(|c| c == b'r' || c == b'z');
                let satva_triggered = voiced != raw;
                // Join-created triggers: `s`-final stacks (`nis`/`dus`)
                // surface as `r`/`z` through `join` (final-`s` replacement,
                // same length: `vinis`+`hanaka` → `vinirhanaka`,
                // `dus`+`gamanIya` → `durgamanIya`), invisible to the `acc`
                // scan above (evidence `vinirhaRaka`-, `durgamaRIya`-type
                // voices). Only `s`-finals create triggers in `join`
                // (`N`-lopa deletes; fusions make `y`/`v`; other finals map
                // within their class).
                let join_triggered = acc.as_bytes().last() == Some(&b's')
                    && raw
                        .as_bytes()
                        .get(..acc.len())
                        .is_some_and(|p| p.contains(&b'r') || p.contains(&b'z'));
                // Effective boundary for index math: a stack-surface-final `N`
                // (solely āṅ `AN`) deletes at the base boundary (`N`-lopa), so
                // the byte length overstates it by one (evidence `paryAnAyayata`
                // voices with boundary 5, not 6). `join` outputs are identical
                // either way (its `N`-lopa branch recurses); only `pre_len`
                // consumers shift.
                let pre = acc.len() - usize::from(acc.ends_with('N'));
                // Augmented-`s` + sibilant-third voice-doublet: `satva`
                // blocks voicing before a sibilant third, but an augmented
                // `s` voices under high-vowel-final surfaces anyway
                // (evidence `04.0042` `aByazAsyat`- / `vyazAsyat`-type voices
                // vs `avAsAsyat`-type keeps; keeps ride the main path).
                // Byte-exact: voices only the augmented `s` (positionally
                // verified, so `stambhi` shifts safely skip).
                let aug_s_voice: Option<String> = {
                    let vb = voiced.as_bytes();
                    if base_plain.starts_with("asA")
                        && matches!(acc.as_bytes().last(), Some(b'i' | b'I' | b'u' | b'U'))
                        && vb.get(pre) == Some(&b'a')
                        && vb.get(pre + 1) == Some(&b's')
                        && matches!(vb.get(pre + 3), Some(b's' | b'S' | b'z' | b'Z'))
                    {
                        let mut w = vb.to_vec();
                        w[pre + 1] = b'z';
                        String::from_utf8(w).ok()
                    } else {
                        None
                    }
                };
                if surface_triggered || satva_triggered {
                    if fusion_keep {
                        // Fusion doublet: every fused slot licenses both the
                        // keep and the voice (evidence `dur;ni` 9v18 pairs
                        // like `durRyamApayata`/`durnyamApayata`; rewrite to
                        // keep-only broke 4,470 voice rows — reverted to
                        // doublet, membership-tested). `push` dedups.
                        push(natva(
                            &voiced,
                            pre,
                            ctx.niya_keep,
                            ctx.intensive_keep,
                            ctx.mani_keep,
                            ctx.nit_keep,
                            true,
                            true,
                            ctx.nn_keep,
                        ));
                    }
                    let nv = natva(
                        &voiced,
                        pre,
                        ctx.niya_keep,
                        ctx.intensive_keep,
                        ctx.mani_keep,
                        ctx.nit_keep,
                        false,
                        surface_triggered,
                        ctx.nn_keep,
                    );
                    // Base-first `n`-`a`-`y`/`Y` / `n`-`y`-vowel / `n`-`i`-`z` /
                    // `n`-`a`-`N` / `n`-`i`-`t` / `n`-`a`-`k` / `n`-`a`-`M`
                    // doublet: license the keep alongside the voice
                    // (evidence `aBiprAnayizIzwa`- and `aBiprAnyAt`-type
                    // keeps vs `praNayati`-type and `pariRyasisiz`-type
                    // voices, plus `prAnaYce`-type keeps with uppercase
                    // `Y` vs lowercase voices, plus `prAnizyata`-type
                    // keeps vs `Riz`-type voices elsewhere, plus
                    // `aBiprAnyata`- and `atiprAnaNkzyat`-type keeps, plus
                    // `nit`- and `nak`- and `naM`-shapes; both exist,
                    // membership-tested). Byte-exact ASCII edit on
                    // the base-first slot only; voice leg below unchanged.
                    {
                        let (vb, nb) = (voiced.as_bytes(), nv.as_bytes());
                        let keep_shape = pre < vb.len()
                            && vb.get(pre) == Some(&b'n')
                            && nb.get(pre) == Some(&b'R')
                            && ((vb.get(pre + 1) == Some(&b'a')
                                && matches!(vb.get(pre + 2), Some(b'y' | b'Y')))
                                || (vb.get(pre + 1) == Some(&b'y')
                                    && vb
                                        .get(pre + 2)
                                        .is_some_and(|c| crate::engine::sandhi::is_vowel_byte(*c)))
                                || (vb.get(pre + 1) == Some(&b'i')
                                    && vb.get(pre + 2) == Some(&b'z'))
                                || (vb.get(pre + 1) == Some(&b'a')
                                    && vb.get(pre + 2) == Some(&b'N'))
                                || (vb.get(pre + 1) == Some(&b'i')
                                    && vb.get(pre + 2) == Some(&b't'))
                                || (vb.get(pre + 1) == Some(&b'a')
                                    && vb.get(pre + 2) == Some(&b'k'))
                                || (vb.get(pre + 1) == Some(&b'a')
                                    && vb.get(pre + 2) == Some(&b'M'))
                                // Base-first `n` + `e` + non-vowel keeps
                                // (evidence `04.0091` `saMpraneqQve`-type
                                // keeps under `r`-surfaces; `n`-`y`/vowel
                                // shapes still voice).
                                || (vb.get(pre + 1) == Some(&b'e')
                                    && !matches!(vb.get(pre + 2), Some(b'y'))
                                    && !vb.get(pre + 2).is_some_and(|c| {
                                        crate::engine::sandhi::is_vowel_byte(*c)
                                    }))
                                // Base-first `n` + `I`/`i` at end keeps
                                // (evidence `04.0072` `aBiprAni`-type; the
                                // augment fuses so `n` sits at `pre`, out of
                                // reach of the internal legs above).
                                || ((vb.get(pre + 1) == Some(&b'I')
                                    || vb.get(pre + 1) == Some(&b'i'))
                                    && pre + 2 >= vb.len())
                                // Base-first never-voice `n` + `a`/`A` + `z`/`R`
                                // (evidence `04.0091` `atipranazwa`-type and
                                // `atiprAnaRQvam`-type plus `nAzwi`-stems
                                // (`atipranAzwi`-type, capital-`A`) for
                                // `RaSa~`; gated on `niya_keep` like the
                                // `I`-`y` unblock).
                                || (ctx.niya_keep
                                    && matches!(vb.get(pre + 1), Some(b'a' | b'A'))
                                    && matches!(vb.get(pre + 2), Some(b'z' | b'R')))
                                // Base-first `aSU~` `n` + `a` + `S` keep
                                // (evidence `05.0020` `prAnaSe`-type;
                                // `n`+`a`+`s`→`RaS` voices stay lexical
                                // elsewhere). `q`/`Q` too (evidence
                                // `05.0020` `prAnaqQve`-type for base
                                // `AnaqQve`).
                                || (ctx.asu_keep
                                    && vb.get(pre + 1) == Some(&b'a')
                                    && matches!(
                                        vb.get(pre + 2),
                                        Some(b'S' | b'q' | b'Q')
                                    ))
                                // Base-first `n` + `A` + `t`/`T` keeps
                                // (evidence `04.0072` `aBiprAnATe`-/
                                // `aBiprAnAte`-type).
                                || (vb.get(pre + 1) == Some(&b'A')
                                    && matches!(
                                        vb.get(pre + 2),
                                        Some(b't' | b'T')
                                    ))
                                // Base-first `n` + `a` + `g` keep (evidence
                                // `07.0021` `prAnag`-type; mirrors internal
                                // `anat` which covers `g` generally).
                                || (vb.get(pre + 1) == Some(&b'a')
                                    && vb.get(pre + 2) == Some(&b'g'))
                                // Base-first `n` + `a` + `j` keep (evidence
                                // `07.0021` `prAnajam`/`prAnajmi`/
                                // `prAnajAni`-type; mirrors internal
                                // `anat` which covers `j` generally).
                                || (vb.get(pre + 1) == Some(&b'a')
                                    && vb.get(pre + 2) == Some(&b'j'))
                                // Base-first `n` + `a`/`A`-final keeps
                                // (evidence `04.0072` `aBiprAna`-type GaY
                                // 8/8 stacks keep, zero voices; mirrors
                                // internal `ana_final`).
                                || (matches!(
                                    vb.get(pre + 1),
                                    Some(b'a' | b'A')
                                ) && pre + 2 >= vb.len())
                                // Base-first `ana~` keeps (evidence
                                // `04.0072`: `AniDve`/`AnikA`-type
                                // (`i`+`D`/`k`/`H`), `Anivas`-type
                                // (`i`+`v`+vowel), `Anam`/`AnaH`-type
                                // (`a`+`m`/`H`); gated on `ana~` since
                                // short-`a` `-anam` voices stay lexical
                                // for other roots).
                                || (ctx.ana_keep
                                    && ((vb.get(pre + 1) == Some(&b'i')
                                        && matches!(
                                            vb.get(pre + 2),
                                            Some(b'D' | b'k' | b'H' | b'm')
                                        )) || (vb.get(pre + 1) == Some(&b'i')
                                        && vb.get(pre + 2) == Some(&b'v')
                                        && vb.get(pre + 3).is_some_and(|c| {
                                            crate::engine::sandhi::is_vowel_byte(*c)
                                        }))
                                        || (matches!(
                                            vb.get(pre + 1),
                                            Some(b'a' | b'A')
                                        ) && matches!(
                                            vb.get(pre + 2),
                                            Some(b'm' | b'H')
                                        )))));
                        if keep_shape {
                            let mut kb = nb.to_vec();
                            kb[pre] = b'n';
                            push(String::from_utf8(kb).unwrap_or_else(|_| nv.clone()));
                        }
                        // Internal `-anIya` / `-mAnA` / `-nut(H)` / `-Ani` /
                        // `-anat` / `-ana`-final / `-nva` / `n`-`y`-vowel /
                        // `A`-trace keep-doublet: restore `n` at
                        // base-internal gerundive/participle/nu-class/
                        // aorist/imperfect/yak shapes alongside the voice (evidence `pralapanIya` +
                        // `aBipraBavaRIya`, `parilambamAnA` +
                        // `aBipraBavamARA`, `prApnutAt`-type keeps; both
                        // exist, membership-tested). Selective per position;
                        // other `R`s stay voiced.
                        {
                            let mut kb = nb.to_vec();
                            let mut changed = false;
                            // `A` at the stack boundary (benedictive-`A`,
                            // `N`-lopa-`A`, augment-`A`): restore every
                            // internal voice (evidence `Anire`/`Anayit`/
                            // `Anfje`-type keeps; voice-cells gain harmless
                            // extras).
                            let a_trace = vb.get(pre) == Some(&b'A');
                            for (i, (&v, &n)) in vb.iter().zip(nb.iter()).enumerate().skip(pre + 1)
                            {
                                if v != b'n' || n != b'R' {
                                    continue;
                                }
                                let aniya = vb.get(i + 1) == Some(&b'I')
                                    && vb.get(i + 2) == Some(&b'y')
                                    && matches!(vb.get(i + 3), Some(b'a' | b'A'));
                                let mana = i >= 2
                                    && vb.get(i - 2) == Some(&b'm')
                                    && (vb.get(i - 1) == Some(&b'A')
                                        || vb.get(i + 1) == Some(&b'A'))
                                    && matches!(vb.get(i + 1), Some(b'a' | b'A'));
                                // `nu`/`ni`-class `-nut`-shapes keep too
                                // (evidence `prApnutAt`/`pralinItAt`-type
                                // keeps; voice-cells gain harmless extras).
                                let nut = (matches!(vb.get(i + 1), Some(b'u' | b'I' | b'i'))
                                    && matches!(vb.get(i + 2), Some(b't' | b'H')))
                                    || (vb.get(i + 1) == Some(&b'I')
                                        && vb.get(i + 2) == Some(&b'd'));
                                // Base-internal `n`+`v`+vowel keeps
                                // (`paryamaMmanva`-type `a`-keeps,
                                // `aBipramanvIya`-type `I`-keeps (evidence
                                // `08.0009`/`05.0004` optatives under
                                // `r`-surfaces) and `aBipramanvAna`-type
                                // `A`-keeps (evidence `08.0009`
                                // `BAvakarma-SAnac`); `i`-keeps too
                                // (evidence `08.0009` `aBiprAmanvi`-type)
                                // and `e`-keeps (evidence `08.0009`
                                // `pramanve`-type; voices elsewhere ride
                                // the main path). Voice-cells gain
                                // harmless extras.
                                let nva = vb.get(i + 1) == Some(&b'v')
                                    && matches!(
                                        vb.get(i + 2),
                                        Some(b'a' | b'A' | b'I' | b'i' | b'e')
                                    );
                                // Gerundive `-ana`-final keeps (`pralapana`-type
                                // keeps; `-anam` voices stay lexical per the
                                // M4 `-anam` item; voice-cells gain harmless
                                // extras).
                                let ana_final =
                                    matches!(vb.get(i + 1), Some(b'a' | b'A')) && i + 2 >= vb.len();
                                // Aorist `-Ani` keeps (`lapAni`-type keeps vs
                                // `hanAni`-type voices; both exist,
                                // membership-tested).
                                let ani =
                                    vb.get(i - 1) == Some(&b'A') && vb.get(i + 1) == Some(&b'i');
                                // Imperfect `-anaC` keeps (`paryavanat`-,
                                // `pravinaktu`-, `paryanaMnamIt`-type keeps,
                                // plus `aBiprAmImanaTAH`-type `T`-keeps and
                                // `aBiprAmImanaDvam`-type `D`-keeps (evidence
                                // `04.0073`/`04.0034`) and `aBiprAyunag`-type
                                // `g`-keeps (evidence `07.0007`) and
                                // `aBiprayunajmi`-type `j`-keeps (evidence
                                // `07.0007`); generalized to `-ana` + stop /
                                // nasal / `H` (class-wide; `-ana`-vowel and
                                // `-ana`-`y` shapes still voice via natva /
                                // `nya`); voice-cells gain harmless extras.
                                // Long-`A` `-AnA` + stop / `m` / `H` keeps
                                // (`nirapunAtAm`-type for `nir`+`apunAtAm`,
                                // `nizpunAte`-type, `prAlinAt`-type;
                                // evidence `09.0014`/`09.0016`/`09.0036`/
                                // `09.0055`; `-AnA`-vowel (`mAnA`-type)
                                // still voices; both exist,
                                // membership-tested).
                                let ana_cap = vb.get(i + 1) == Some(&b'A')
                                    && matches!(
                                        vb.get(i + 2),
                                        Some(b't' | b'T' | b'd' | b'H' | b'm')
                                    );
                                let anat = vb.get(i + 1) == Some(&b'a')
                                    && matches!(
                                        vb.get(i + 2),
                                        Some(
                                            b'k' | b'K'
                                                | b'g'
                                                | b'G'
                                                | b'N'
                                                | b'c'
                                                | b'C'
                                                | b'j'
                                                | b'J'
                                                | b'Y'
                                                | b't'
                                                | b'T'
                                                | b'd'
                                                | b'D'
                                                | b'n'
                                                | b'w'
                                                | b'W'
                                                | b'q'
                                                | b'Q'
                                                | b'R'
                                                | b'p'
                                                | b'P'
                                                | b'b'
                                                | b'B'
                                                | b'm'
                                                | b'M'
                                                | b'H'
                                        )
                                    );
                                // Base-internal `n`+`y`+vowel keeps
                                // (`pariKanyate`-type keeps vs `Rya`-type
                                // voices elsewhere; both exist,
                                // membership-tested; stack-region fusion
                                // `y` stays separate).
                                let nya = vb.get(i + 1) == Some(&b'y')
                                    && vb
                                        .get(i + 2)
                                        .is_some_and(|c| crate::engine::sandhi::is_vowel_byte(*c));
                                // Root-`n` + `o`/`u` keeps (evidence `05.0016`
                                // `anusaMprApnoti`- / `anusaMprApnuTaH`-type
                                // keeps under `r`-surfaces vs our voicing;
                                // voices elsewhere ride the main path — both
                                // licensed, membership-tested). `O`/`U` too
                                // (evidence `02.0034` `prorRonUti`-type).
                                let nou = matches!(vb.get(i + 1), Some(b'o' | b'u' | b'O' | b'U'));
                                // Internal `n` + `i`/`I` + non-vowel keeps
                                // (evidence `04.0034` `prAlIni`-type final
                                // keeps and `prAlIniQvam`-type keeps;
                                // `n`-`y` shapes still voice via `nya`/natva).
                                let nii = matches!(vb.get(i + 1), Some(b'i' | b'I'))
                                    && !matches!(vb.get(i + 2), Some(b'y'))
                                    && !vb
                                        .get(i + 2)
                                        .is_some_and(|c| crate::engine::sandhi::is_vowel_byte(*c));
                                // Internal `n` + `e`/`E`/`O` + non-vowel keeps
                                // (evidence `04.0073` `aBiprAmImanetAm`- and
                                // `04.0034` `prAlIlineTAm`/`prAlIline`-type
                                // keeps; `09.0014` `nizpunE`-type and
                                // `02.0034` `prorRonOti`-type; `n`-`y`/vowel
                                // shapes still voice). `I` + `y` keeps too
                                // (evidence `09.0014` `nizpunIyuH`-type
                                // `pvidhiling`; gerundive `-Iya` voices
                                // ride the main path — both licensed,
                                // membership-tested).
                                let niy =
                                    vb.get(i + 1) == Some(&b'I') && vb.get(i + 2) == Some(&b'y');
                                let nee = matches!(vb.get(i + 1), Some(b'e' | b'E' | b'O'))
                                    && !matches!(vb.get(i + 2), Some(b'y'))
                                    && !vb
                                        .get(i + 2)
                                        .is_some_and(|c| crate::engine::sandhi::is_vowel_byte(*c));
                                if aniya
                                    || mana
                                    || nut
                                    || ani
                                    || anat
                                    || ana_cap
                                    || nya
                                    || ana_final
                                    || nva
                                    || nou
                                    || nii
                                    || nee
                                    || niy
                                    || a_trace
                                {
                                    kb[i] = b'n';
                                    changed = true;
                                }
                            }
                            if changed {
                                push(String::from_utf8(kb).unwrap_or_else(|_| nv.clone()));
                            }
                        }
                    }
                    push(nv);
                    // Keep-doublet alongside voicing: `s`-voice roots license
                    // the keep alongside the voice (evidence `syandU~`
                    // doublets under trigger stacks; rewrite to voice-only
                    // breaks keeps, +436 with 10 new — reverted to doublet,
                    // membership-tested), as do transparent-augment
                    // voicings (evidence `paryazkandat` + `paryaskandat`,
                    // `nirazeDayata`-family alongside keeps) and `s`+`k`
                    // voicings (evidence `parizkandayate` +
                    // `pariskandayate`), `s`+`U`/`I` voicings (evidence
                    // `parisUyate`-type keeps), and `zadx~` under bare
                    // `prati` (evidence `pratisAdayAmbaBUva`-type keeps for
                    // both augmented and root-initial `s`).
                    if (ctx.s_voice
                        || crate::engine::sandhi::aug_voice_config(acc, &raw)
                        || crate::engine::sandhi::sk_voice_config(acc, &raw)
                        || crate::engine::sandhi::su_voice_config(acc, &raw)
                        || (ctx.zadx_prati_keep && stack == "prati"))
                        && satva_triggered
                    {
                        let kept = satva(&raw, acc, false, aug_keep);
                        if surface_triggered {
                            push(natva(
                                &kept,
                                pre,
                                ctx.niya_keep,
                                ctx.intensive_keep,
                                ctx.mani_keep,
                                ctx.nit_keep,
                                fusion_keep,
                                surface_triggered,
                                ctx.nn_keep,
                            ));
                        } else {
                            push(kept);
                        }
                    }
                } else {
                    // Join-trigger voice alongside the keep (see
                    // `join_triggered` above): the keep stays canonical
                    // `[0]`, the voice is appended (`push` dedups when
                    // identical). Both licensed, membership-tested.
                    if join_triggered {
                        let extra = natva(
                            &voiced,
                            pre,
                            ctx.niya_keep,
                            ctx.intensive_keep,
                            ctx.mani_keep,
                            ctx.nit_keep,
                            fusion_keep,
                            true,
                            ctx.nn_keep,
                        );
                        push(voiced);
                        push(extra);
                    } else {
                        push(voiced);
                    }
                }
                if let Some(w) = aug_s_voice {
                    push(w);
                }
            }
        }
        // `s`-āgama (`suṭ`) variant for kṛ-roots before `k`-stems: fold an
        // `s` onto the surface (voicing `m -> M` there), then attach the
        // base directly so `s_final_map` does not re-voice it. The sibilant
        // itself voices to `z` after `ri`-final surfaces only (evidence
        // `parizkArayizIzwa` vs `upaskArayizIzwa`, `pratiskArayizIzwa`,
        // `saMskArayizIzwa`; `dus`/`nir`/`nis` need none — their own `s`
        // maps structurally).
        if ctx.sut_root && base_plain.starts_with('k') {
            let abb = acc.as_bytes();
            let final_ok = abb.last().is_some_and(|c| {
                crate::engine::sandhi::is_vowel_byte(*c) || *c == b'm' || *c == b'M'
            });
            if final_ok {
                for mut with_s in join(acc, "s") {
                    if acc.ends_with("ri") {
                        with_s.pop();
                        with_s.push('z');
                    }
                    let joined = format!("{with_s}{base_plain}");
                    let voiced = satva(&joined, &with_s, ctx.z_initial || ctx.s_voice, aug_keep);
                    let trig = with_s.bytes().any(|c| c == b'r' || c == b'z');
                    push(if trig || voiced != joined {
                        natva(
                            &voiced,
                            with_s.len(),
                            ctx.niya_keep,
                            ctx.intensive_keep,
                            ctx.mani_keep,
                            ctx.nit_keep,
                            false,
                            trig,
                            ctx.nn_keep,
                        )
                    } else {
                        voiced
                    });
                }
            }
        }
        // `suṭ` with the augment: prefix + augment + `s` + de-augmented stem
        // (evidence `apAskArayata` for `apa`+`akArayata`: the augment fuses
        // with the prefix surface, `s` attaches to the bare `k`-stem). The
        // augment-fused surface always ends vocalic, so no `ri`-voicing
        // gate applies here (unlike the bare-stem path above), except bare
        // `pari` voices the fused `s` (evidence `paryazkAri` alongside
        // `paryaskAri` for `08.0010`; `ni` drops the `s`, `sam`/`upa`
        // keep it).
        if ctx.sut_root {
            if let Some(deaug) = base_plain.strip_prefix('a') {
                if deaug.starts_with('k') {
                    for acc_a in join(acc, "a") {
                        let mut s_forms = join(&acc_a, "s");
                        if stack == "pari" {
                            let mut z_forms = Vec::new();
                            for w in &s_forms {
                                let mut z = w.clone();
                                if z.pop() == Some('s') {
                                    z.push('z');
                                    z_forms.push(z);
                                }
                            }
                            s_forms.extend(z_forms);
                        }
                        for with_s in s_forms {
                            let joined = format!("{with_s}{deaug}");
                            let voiced =
                                satva(&joined, &with_s, ctx.z_initial || ctx.s_voice, aug_keep);
                            let trig = with_s.bytes().any(|c| c == b'r' || c == b'z');
                            push(if trig || voiced != joined {
                                natva(
                                    &voiced,
                                    with_s.len(),
                                    ctx.niya_keep,
                                    ctx.intensive_keep,
                                    ctx.mani_keep,
                                    ctx.nit_keep,
                                    false,
                                    trig,
                                    ctx.nn_keep,
                                )
                            } else {
                                voiced
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

/// `lyap` absolutive stems: the embedded default `pra-` is fused with a
/// stem-initial vowel (`pra+Atya -> prAtya`, `pra+Ikzya -> prekzya`) but
/// plain before consonants (`pra+BUya -> praBUya`), so un-fusing must
/// consider every licensed grade (evidence `aByatya`/`aByAtya` doublets
/// from `prAtya`, `aBIkzya` from `prekzya`, `viBUya` from `praBUya`).
/// Non-`pr` bases, and `pr` + unfusable vowels (`priya`-type), pass through
/// untouched.
#[must_use]
pub fn lyap_stem(base: &str) -> Vec<String> {
    let b = base.as_bytes();
    // `pra` → `plA` allomorph before `l`-stems (same `r`→`l` family as
    // prefix `parA`→`pala`/`nir`→`nil`): unfuse `plA` alongside, evidence
    // `aByavAyya` for `plAyya`, mirroring `prA` unfusing to both grades
    // (`Ayya` and `ayya`, evidence `nilayya`/`samayya`-type alongside
    // `palAyya`-type). Twins (never replace) so genuine `plu`-roots
    // keep their licensing.
    if base.len() > 3 && base.starts_with("plA") {
        let mut v = vec![base.to_string(), base[2..].to_string()];
        let short = format!("a{}", &base[3..]);
        if short != base[2..] {
            v.push(short);
        }
        v.dedup();
        return v;
    }
    if b.len() < 3 || &b[..2] != b"pr" {
        return vec![base.to_string()];
    }
    let (c2, tail) = (b[2] as char, &base[3..]);
    // Fused `pr` + vowel: strip `pr`, un-fuse the vowel to all grades.
    // (`prAr...` keeps the fused grade too, covering non-`pra` `prAr`-stems.)
    let fused: &[&str] = match c2 {
        'A' => &["A", "a"],
        'e' => &["e", "i", "I"],
        'o' => &["o", "u", "U"],
        'E' => &["E", "e"],
        'O' => &["O", "o"],
        'r' => &["ar", "f", "F"],
        'l' => &["al", "x", "X"],
        _ => &[],
    };
    if !fused.is_empty() {
        let rest = &base[2..];
        let mut v = Vec::with_capacity(fused.len() + 2);
        v.push(rest.to_string());
        for g in fused {
            let cand = format!("{g}{tail}");
            if cand != rest {
                v.push(cand);
            }
        }
        // `Ar`→`f` grade (`Artya` → `ftya`, evidence `01.1086`
        // `krut.lyap` `aByftya`-type for `prArtya`; zero-grade alongside
        // guna, membership-tested). `Arjya` → `fjya` too (evidence
        // `01.0200` `nirfjya`-type for `prArjya`). `ArD`/`Arz`/`Arc`
        // voice the same way (`prArDya` → `samfDya`/`vyfDya`, evidence
        // `04.0160`/`05.0027`; `prArzya` → `aByfzya`, evidence `06.0007`;
        // `prArcya` → `udfcya`, evidence `06.0022`; keeps ride the base).
        // `ArdiDizya` syncopates to `Irtsya` (`prArdiDizya` → `vIrtsya`,
        // evidence `04.0160`/`05.0027` `san_krut.lyap` under `vi`).
        for (from, to) in [
            ("Art", "ft"),
            ("Arj", "fj"),
            ("ArD", "fD"),
            ("Arz", "fz"),
            ("Arc", "fc"),
            ("ArdiDizya", "Irtsya"),
        ] {
            if let Some(after) = rest.strip_prefix(from) {
                let cand = format!("{to}{after}");
                if !v.contains(&cand) {
                    v.push(cand);
                }
            }
        }
        // `ArArya`/`arArya` → `fyAya` (evidence `01.1086`
        // `yangluk_krut.lyap` `aByfyAya`-type for `prArArya` under
        // `aBi`/`ni`/`nir`/`prati`/`sam`/`ud`; `apa`/`ava`/`pra`/`upa`
        // keep `ArArya` via the base path; membership-tested).
        if let Some(after) = rest.strip_prefix("ArArya") {
            let cand = format!("fyAya{after}");
            if !v.contains(&cand) {
                v.push(cand);
            }
        }
        if let Some(after) = rest.strip_prefix("arArya") {
            let cand = format!("fyAya{after}");
            if !v.contains(&cand) {
                v.push(cand);
            }
        }
        return v;
    }
    // Unfused `pra` + consonant: strip `pra` (`praBUya -> BUya`).
    // `pru` also strips (site over-strips `pruhya`-type to `uhya`;
    // evidence `aByuhya`/`aDyuhya`), while `priya`-type (`pr` +
    // unfusable `i`, never a fusion product) passes through untouched.
    // `pru` additionally un-fuses to `U` (`pruzya` → `Uzya`, evidence
    // `01.1160` `AN`/`aDi;AN` lyap `uzya`/`aDyuzya`-type via `A`+`U`).
    // Desiderative/intensive `si`-stems voice alongside
    // (`prasizatsya` → `sizatsya` + `zizatsya`, evidence `01.0990`/
    // `06.0163` `Anizizatsya`-type under `ni`-family stacks; keeps ride
    // the base path, membership-tested).
    if c2 == 'a' && base.len() > 3 {
        let mut v = vec![tail.to_string()];
        for (from, to) in [("sizatsya", "zizatsya"), ("sAsadya", "zAzadya")] {
            if let Some(rest) = tail.strip_prefix(from) {
                let cand = format!("{to}{rest}");
                if !v.contains(&cand) {
                    v.push(cand);
                }
            }
        }
        // Reduplicated `sis`→`ziz` (`prasisaNkzya` → `zizaNkzya`,
        // evidence `01.1142` `aBizizaNkzya`-type under `aBi`;
        // keeps ride the base path, membership-tested).
        // Desiderative `sAsvajya`→`zAzvajya` (evidence `01.1131`
        // `yangluk_krut.lyap` `aBizAzvajya`-type; keeps ride the base).
        for (from, to) in [
            ("sis", "ziz"),
            ("sAsajya", "zAzajya"),
            ("sAsvajya", "zAzvajya"),
        ] {
            if let Some(rest) = tail.strip_prefix(from) {
                let cand = format!("{to}{rest}");
                if !v.contains(&cand) {
                    v.push(cand);
                }
            }
        }
        // `y`-glide before `j` in voiced lyap stems (`zAzvajya` →
        // `zAzvaYjya`, `zAzajya` → `zAzaYjya`; evidence `01.1131`
        // `aBizAzvaYjya`-type + `01.1142` `aBizAzaYjya`-type; plain
        // grades ride above). Only ADDS licensings.
        for (from, to) in [("sAsvajya", "zAzvaYjya"), ("sAsajya", "zAzaYjya")] {
            if let Some(rest) = tail.strip_prefix(from) {
                let cand = format!("{to}{rest}");
                if !v.contains(&cand) {
                    v.push(cand);
                }
            }
        }
        // Causative/desiderative suppletion at the unfuse site:
        // `gamayya` takes native `Ayayya`/`Ayya` (`pragamayya` →
        // `pratyAyya`, evidence `02.0040` `nich_krut.lyap` under
        // `prati`/`vi;prati`; fused `pra`-stems never reach the
        // `gamay_ayay_variants` twin, which only strips leading `gamay`).
        // `jigAMsya` takes `Izizya` (`prajigAMsya` → `pratIzizya`,
        // evidence `02.0040` `san_krut.lyap`; keeps ride the base path).
        for w in gamay_ayay_variants(tail) {
            if !v.contains(&w) {
                v.push(w);
            }
        }
        if let Some(rest) = tail.strip_prefix("jigAMs") {
            let cand = format!("Iziz{rest}");
            if !v.contains(&cand) {
                v.push(cand);
            }
        }
        // `rc`→`fc` at the unfuse site (`prarcicCizya` → `fcicCizya`,
        // evidence `06.0016` `nyfcicCizya`-type; keeps ride the base).
        // `sAvya`→`zAvya` voices too (`prasAvya` → `parizAvya`,
        // evidence `06.0144` `nich_krut.lyap` under `pari`).
        for (from, to) in [("rc", "fc"), ("sAvya", "zAvya")] {
            if let Some(rest) = tail.strip_prefix(from) {
                let cand = format!("{to}{rest}");
                if !v.contains(&cand) {
                    v.push(cand);
                }
            }
        }
        // `laB`→`lamB` at the unfuse site (`pralaBya` → `laBya` +
        // `lamBya`, evidence `01.1130` `AlamBya`-type under `AN`;
        // keeps ride the base path, membership-tested).
        if let Some(rest) = tail.strip_prefix("laB") {
            let cand = format!("lamB{rest}");
            if !v.contains(&cand) {
                v.push(cand);
            }
        }
        // `cC` degemination at the unfuse site (`pracCadya` → `cCadya`
        // + `Cadya`, evidence `10.0370` `ucCadya`-type under `ud`;
        // keeps ride the base path, membership-tested).
        if let Some(rest) = tail.strip_prefix("cC") {
            let cand = format!("C{rest}");
            if !v.contains(&cand) {
                v.push(cand);
            }
        }
        return v;
    }
    if c2 == 'u' {
        let mut v = vec![base.to_string(), base[2..].to_string()];
        let upper = format!("U{tail}");
        if !v.contains(&upper) {
            v.push(upper);
        }
        return v;
    }
    vec![base.to_string()]
}

/// Strong/weak `-ntI` feminine alternation for counterpart-paired cells.
///
/// `Satf`-family feminines license both `-ntI` (strong) and `-tI` (weak)
/// surfaces under prefixation (evidence: prefixed `sya-Satf.f` cells carry
/// both, e.g. `ABavatI/ABavantI`, `AboBuvatI/AboBuvantI`, while pairing from
/// a single counterpart alternant). Plain (non-paired) cells keep single
/// licensing. Returns the base alternant(s) to join.
#[must_use]
pub fn nt_feminine_variants(base: &str) -> Vec<String> {
    if let Some(head) = base.strip_suffix("ntI") {
        vec![base.to_string(), format!("{head}tI")]
    } else if let Some(head) = base.strip_suffix("tI") {
        vec![base.to_string(), format!("{head}ntI")]
    } else {
        vec![base.to_string()]
    }
}

/// Lengthened-abhyasa `cAs` twin for kṛ intensive stems (formation allomorphy).
///
/// Prefixed intensive (`yang`/`yangluk`) derivation of kṛ takes the
/// lengthened abhyasa with `suṭ` (`carikriyAt` → `nirupacAskriyAt`,
/// `acarikaH` → `nirupAcAskaH`-type, evidence `08.0010` across stacks;
/// plain grades stay licensed via the base path and `suṭ` insertion
/// composes downstream). Returns the stem plus, for leading
/// `cari`/`carI`/`cark` (plain or `a`-augmented), the `cAs` twin. Gated to
/// kṛ-roots like [`sut_insert_variants`]. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn cask_variants(base: &str, is_sut: bool) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if !is_sut {
        return out;
    }
    // Plain and augmented (`a`-initial, lang/lung grades) reduplicant heads.
    for head in ["cari", "carI", "cark", "acari", "acarI", "acark"] {
        if let Some(rest) = base.strip_prefix(head) {
            // Augmented heads license both the augmented grade (evidence
            // `08.0010` `nirupAcAskftAm`- / `nirupAcAskaH`-type wants) and
            // the de-augmented grade (short-`a` joins elsewhere); plain
            // heads pass through as before.
            if head.starts_with('a') {
                let v = format!("acAs{rest}");
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            let v = format!("cAs{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Strong-grade `kar` twin for kṛ perfect `kf` stems (formation allomorphy).
///
/// Prefixed `suṭ` derivation selects the strong `kar` grade even when the
/// base cell holds weak `kf` (`cakfze` → `nirupacaskarze`, evidence
/// `08.0010` `ting.alit` across stacks; plain copies stay licensed via the
/// base path). Returns the stem plus, for each `kf`, the `kf`→`kar` twin;
/// `suṭ`-insertion downstream turns `kar` into `skar` (no anaptyctic `-a-`
/// since `k` meets a vowel). Gated to kṛ-roots like
/// [`sut_insert_variants`]. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn kr_strong_variants(base: &str, is_sut: bool) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if !is_sut {
        return out;
    }
    let mut i = 0;
    while i + 1 < base.len() {
        if base.as_bytes()[i] == b'k' && base.as_bytes()[i + 1] == b'f' {
            let mut v = String::with_capacity(base.len() + 1);
            v.push_str(&base[..i]);
            v.push_str("kar");
            v.push_str(&base[i + 2..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// `suṭ`-insertion twins for kṛ-roots (formation allomorphy).
///
/// The `s`-āgama attaches before the kṛ-root's `k` even when nested
/// inside derived stems (`apa`+`acIkarata` → `apAcIskarata`: `s` before
/// the `-kara-` `k`, augment fusing with the prefix as usual). Before
/// `r` an anaptyctic `-a-` follows (`cakre` → `caskare`, evidence
/// `nirupacaskare`-type; `k`+vowel takes bare `s`, evidence `skArayata`-type).
/// Gated to kṛ-roots (oracle `suṭ` occurs only for `qukfY`/`kF`).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn sut_insert_variants(base: &str, is_sut: bool) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if !is_sut {
        return out;
    }
    let b = base.as_bytes();
    for (i, c) in base.char_indices() {
        // Skip `k` already preceded by `s` (the `cAs`-grade twin below and
        // `sk`-initial stems keep their cluster; zero `ssk`-wants exist
        // corpus-wide, so no double-`s` licensing is lost).
        if c == 'k' && b.get(i.saturating_sub(1)) != Some(&b's') {
            let mut v = base.to_string();
            v.insert(i, 's');
            if !out.contains(&v) {
                out.push(v);
            }
            if b.get(i + 1) == Some(&b'r') {
                let mut w = base.to_string();
                w.insert(i, 's');
                w.insert(i + 2, 'a');
                if !out.contains(&w) {
                    out.push(w);
                }
            }
        }
    }
    out
}

/// Reduplicant-voice twin for prefixed licensing (abhyasa agreement).
///
/// A reduplicated stem voices its head under a triggering prefix
/// (`sisiDizyAt` → `niziziDizyAt`, `sizeDa` → `nizizeDa`; lit perfect
/// `sasAda` → `zasAda`-type alongside keeping `sasAda` under `pra`),
/// while unvoiced roots keep (`sasnO`, via the base stem). Returns the
/// stem plus the head-voiced twin (`s`→`z` at stem-initial); deeper
/// sibilants re-derive through the satva cascade in
/// [`generate_prefixed`]. Only ADDS the voiced licensing
/// (membership-tested, never rewriting).
#[must_use]
pub fn head_voice_variants(base: &str) -> Vec<String> {
    let b = base.as_bytes();
    let mut out = vec![base.to_string()];
    if b.first() == Some(&b's') {
        out.push(format!("z{}", &base[1..]));
    }
    out
}

/// Non-sibilant unvoiced twins for prefixed licensing (formation allomorphy).
///
/// Prefixed derivation may de-voice retroflexes (`R`→`n`, `W`→`T`) while
/// keeping sibilants voiced (`prARiRizya` → `prAninizya`-stem for
/// `aByavAninizya`-type wants, evidence `02.0065`; the fully-devoiced twin
/// below voices `z`→`s` too, so the `(n,n,z)` shape needs this split or it
/// is unlicensable). Runs before [`stem_unvoiced_variants`], whose
/// `z`-twins (keep included) then compose downstream. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn stem_rw_unvoiced_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let mut dev = String::with_capacity(base.len());
    for c in base.chars() {
        match c {
            'R' => dev.push('n'),
            'W' => dev.push('T'),
            _ => dev.push(c),
        }
    }
    if dev != base && !out.contains(&dev) {
        out.push(dev);
    }
    out
}

/// Unvoiced stem twin for prefixed licensing (formation allomorphy).
///
/// Prefixed derivation may select the unvoiced stem allomorph even when
/// the base cell voices: `ziDu~` prefixed stems de-voice throughout
/// (`asIziData` → `nyasIsiData`), and lyap unfuses `praRadya` to dental
/// `nadya` under vocalic stacks (`Anadya`, whose trigger `r` was stripped
/// with `pra`). Returns the stem, its fully de-voiced twin (`R→n`,
/// `z→s`, voiced-aspirate→unaspirate `W→T`, evidence `AtisTipata` for
/// `atizWipata` alongside `z→s`), and one single-de-voiced twin per
/// `z` position (evidence `prAseseDizyata` voices only the first stem
/// `z`, keeping deeper ones). Voicing re-derives through
/// [`generate_prefixed`] wherever still licensed, so twins are stable and
/// only ADD unvoiced licensings (membership-tested, never rewriting).
#[must_use]
pub fn stem_unvoiced_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let mut dev = String::with_capacity(base.len());
    for c in base.chars() {
        match c {
            'R' => dev.push('n'),
            'z' => dev.push('s'),
            'W' => dev.push('T'),
            _ => dev.push(c),
        }
    }
    if dev != base && !out.contains(&dev) {
        out.push(dev);
    }
    for (i, c) in base.char_indices() {
        if c == 'z' {
            let mut single = base.to_string();
            single.replace_range(i..=i, "s");
            if !out.contains(&single) {
                out.push(single);
            }
        }
    }
    out
}

/// Abhyasa agreement twins for prefixed licensing (reduplicant+root voice).
///
/// Some reduplicated stems voice the root together with the head under a
/// triggering prefix (`sasaYja` → `aBizazaYja`, evidence `01.1142` across
/// `aBi`/`anu`/`ni`/`nir`/`pari`/`vi` stacks; the keep (`aBizasaYja`-type)
/// stays licensed via the base/head paths since the cascade blocks
/// `a`-prev voicing). Returns the stem plus, for each internal `s` with
/// `a`/`A` prevowel, the head-voiced + that-`s`-voiced twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn abhyasa_agree_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    if b.first() != Some(&b's') {
        return out;
    }
    for (i, c) in base.char_indices() {
        if c == 's' && i >= 2 {
            let prev = b[i - 1];
            if prev == b'a' || prev == b'A' {
                let mut v = String::with_capacity(base.len());
                v.push('z');
                v.push_str(&base[1..i]);
                v.push('z');
                v.push_str(&base[i + 1..]);
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
    }
    out
}

/// Internal `-nam-`/`-nan-` voicing twins for prefixed licensing (stem allomorphy).
///
/// Some derivations voice internal `n` to `R` before `-am-`/`-an-` under
/// prefixation (`anamayata` → `niraRamayata`, `nantA` → `nirRantA`,
/// `anAmi` → `niraRAmi`, evidence `01.1136`/`02.0065`/`09.0004`-type across
/// stacks; keeps stay licensed via the base path since `m` blocks,
/// base-first `n`-`a`-`n` blocks, and the vrddhi grade below composes for
/// `a`→`A`). Returns the stem plus, for each `n` followed by `a`/`A` +
/// `m`/`n`, the `n`→`R` twin. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn nam_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut i = 0;
    while i + 2 < b.len() {
        if b[i] == b'n'
            && (b[i + 1] == b'a' || b[i + 1] == b'A')
            && (b[i + 2] == b'm' || b[i + 2] == b'n')
        {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push('R');
            v.push_str(&base[i + 1..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// Vrddhi `-ami`→`-Ami` twins for prefixed licensing (root-grade allomorphy).
///
/// Some aorist (`luṄ`) derivations take the vrddhi grade under prefixation
/// (`anami` → `niraRAmi` via `anAmi`, evidence `01.1136` `nich_yak.alung`;
/// voicing composes via [`nam_voice_variants`], keeps via the base path).
/// Returns the stem plus, for each `ami`, the `ami`→`Ami` twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn vrddhi_ami_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let mut i = 0;
    while i + 2 < base.len() {
        if &base.as_bytes()[i..i + 3] == b"ami" {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push_str("Ami");
            v.push_str(&base[i + 3..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// Native `iyy` twin for borrowed `gamy` gerundives (suppletion allomorphy).
///
/// `iR` borrows `gamy-` for `yat`-gerundive unprefixed but takes native
/// `iyy-` under `prati`-family stacks (`gamyAt` → `pratyiyyAt`, evidence
/// `02.0040` `prati` `nich.pashirling` 10v10 aligned; other stacks keep
/// `gam-`, which stays licensed via the base path). Returns the stem plus
/// the leading-`gamy`→`iyy` twin. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn gamy_iyy_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("gamy") {
        let v = format!("iyy{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Native `vu` twin for `U`-initial stems (stem allomorphy).
///
/// Some `U`-initial stems take `vu`-initial shapes under prefixation
/// (`Uce` → `pravuce`, `Uye` → `pravuye`, kvasu `Uzivas` → `vuzivas`;
/// evidence `02.0039`/`01.1161`/`01.0792` across stacks; `o`-sandhi keeps
/// like `02.0034` `prorRuvizIzwa` stay licensed via the base path; zero
/// bare-`v` oracle shapes exist, so no `v`-leg). Returns the stem plus the
/// `vu`-twin, alongside the `uy`-doublet for `Uv` before `iQ`/`iD`
/// (evidence `01.1161` yak.alit `UviQve`→`vuviQve` + `uyiQve`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn u_v_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix('U') {
        let v = format!("vu{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
        if rest.len() >= 2 && rest.starts_with('v') {
            let after = &rest[1..];
            if after.starts_with('i')
                && after
                    .as_bytes()
                    .get(1)
                    .is_some_and(|c| *c == b'Q' || *c == b'D')
            {
                let twin = format!("uy{after}");
                if !out.contains(&twin) {
                    out.push(twin);
                }
            }
        }
    }
    out
}

/// Native `yi`/`yI` twins for `I`-initial stems (stem allomorphy).
///
/// Some `I`-initial stems take `y`-initial shapes under prefixation
/// (`Ize` → `prayize`, `Ije` → `prayije`, `Ijivas` → `ayijivas`-family,
/// `Irtsa` → `upayIrtsa`; evidence `04.0022`/`01.1157`/`04.0160`).
/// Returns the stem plus the `yi`- and `yI`-twins. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn i_y_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix('I') {
        for rep in ["yi", "yI"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}
/// Yangluk abhyasa-syncope twin for prefixed licensing (reduplicant grades).
///
/// Yangluk reduplicants drop their vowel under prefixation (`sarisarA` →
/// `sarsarA`, `caricCfdIt` → `carcCfdIt`; evidence `01.1085`/`07.0008`
/// across stacks; `yang` keeps full grades like `tetirA` via the base
/// path). Returns the stem plus, for the first `i`/`I` between consonants,
/// the syncopated twin. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn yangluk_sync_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if (c == b'i' || c == b'I')
            && i > 0
            && i + 1 < b.len()
            && !crate::engine::sandhi::is_vowel_byte(b[i - 1])
            && !crate::engine::sandhi::is_vowel_byte(b[i + 1])
            && b[i - 1].is_ascii_alphabetic()
            && b[i + 1].is_ascii_alphabetic()
        {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push_str(&base[i + 1..]);
            if !out.contains(&v) {
                out.push(v);
            }
            break;
        }
    }
    out
}
/// Guna-grade twins for `I`/`u`/`CAy`-initial stems (stem allomorphy).
///
/// Some `I`/`u`-initial stems take `e`/`o`-grade shapes under prefixation
/// (`IkzizIzwa` → `ApekzizIzwa` via `apa`→`Ap`, `ucCyate` → `ApocCyate`;
/// evidence `01.0694`/`01.0244`), and `CAy`-initial stems take `cCey`
/// (`CAyAt` → `AcCeyAt`/`pracCeyAt`; evidence `04.0041`/`06.0099`).
/// Other stacks keep via the base path. Returns the stem plus the `I`→`e`,
/// `u`→`o`, and `CAy`→`cCey` twins. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn guna_grade_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix('I') {
        let v = format!("e{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix('u') {
        let v = format!("o{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("CAy") {
        let v = format!("cCey{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// Lit auxiliary-rotation twin for prefixed licensing (aux inventory).
///
/// Prefixed perfects rotate auxiliaries beyond the unprefixed inventory
/// (`sAdayAYcakAra` → `pratisAdayAmbaBUva` + `pratisAdayAmAsa` alongside
/// keeping `pratisAdayAYcakAra`; evidence `01.0990`/`06.0163` prati-lit).
/// Returns the stem plus, for `Ycakr`-aux stems (anywhere in the stem),
/// the `mbaBUv`- and `mAs`-aux twins (endings ride via `r`-drop, with 3sg
/// `Ara`→`a`). Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn lit_aux_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("Ycakr") {
        let (head, tail) = base.split_at(pos);
        let tail = tail.strip_prefix("Ycakr").unwrap_or(tail);
        let tail = if tail == "Ara" {
            "a"
        } else {
            tail.strip_prefix('r').unwrap_or(tail)
        };
        for aux in ["mbaBUv", "mAs"] {
            let v = format!("{head}{aux}{tail}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Desiderative abhyasa-grade twins for prefixed licensing.
///
/// Some abhyasa reduplicants reform under prefixation (`cicCedayiz` →
/// `IcCedayiz`, evidence `10.0480` `vi`; `Yjijiz` → `YjIz`, evidence
/// `07.0021` bare-`A`; other stacks keep full grades via the base path).
/// Returns the stem plus the contracted twins (matched anywhere in the
/// stem, so augmented `a`-forms compose too). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn abhyasa_contract_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("cicC") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + "cicC".len(), "IcC");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("Yjijiz") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + "Yjijiz".len(), "YjIz");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Augmented `acicCe` → `EcCe` contraction twin (abhyasa reformation).
///
/// `Ceda`-desideratives contract the augmented reduplicant under
/// prefixation (`acicCedayizata` → `vyEcCedayizata`, evidence `10.0480`
/// `vi` across `san`/`nich` slots; `acicCa`-stems like `10.0370` keep via
/// the base path, so scoped to `e`-grade exactly). Returns the stem plus,
/// for leading `acicCe`, the `EcCe` twin. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn acicc_e_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("acicCe") {
        let v = format!("EcCe{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// Nasal-infix twin for `laB` stems (stem allomorphy).
///
/// `labh` takes nasal-infixed shapes under prefixation (`laByate` →
/// `upalamByate`, `AN` → `AlamByate`; evidence `01.1130`; other stacks
/// keep `laB` via the base path). Returns the stem plus the `laB`→`lamB`
/// twin. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn labh_infix_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("laB") {
        let v = format!("lamB{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // Long-`A` head (`lABa` → `lamBa`, evidence `01.1130` `AN`/`pra`/`sam`/
    // `vi` `krut.GaY` — every prefixed stack voices, bare `A` keeps via
    // the base path; membership-tested).
    if let Some(rest) = base.strip_prefix("lAB") {
        let v = format!("lamB{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // Reduplicated head (`lAlaBa` → `lAlamBa`, evidence `01.1130`
    // `yang_krut.ac` `AlAlamBa`-type under `AN`-family stacks; other
    // `krt` keeps `lAlaBa` via the base path; membership-tested).
    // Feminine `-A` grade (`lAlaBA` → `lAlamBA`, evidence `AlAlamBA`-type
    // `f`-slots; masculine rides the leg above).
    for (from, to) in [("lAlaBa", "lAlamBa"), ("lAlaBA", "lAlamBA")] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}
/// `s`+`k` voicing twin for prefixed licensing (stem allomorphy).
///
/// Some `sk`-initial stems voice to `zk` under prefixation (`askandizAtAm`
/// → `paryazkandizAtAm`; evidence `01.1134`; other stacks keep `sk` like
/// `samaskandizAtAm` via the base path). Returns the stem plus, for the
/// first `sk`, the `zk` twin. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn sk_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("sk") {
        let mut v = base.to_string();
        v.replace_range(pos..=pos, "z");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Native `sA` twin for `san`-initial stems (stem allomorphy).
///
/// `sanyate` takes `sAyate` under prefixation (`sanyate` → `parisAyate`;
/// evidence `08.0002`; other stacks keep `sanyate` via the base path).
/// Returns the stem plus, for `san` before `y`, the `sA` twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn san_sa_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("san") {
        if rest.starts_with('y') {
            let v = format!("sA{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}
/// `cA`-grade `caskar` twin for kṛ-roots (formation allomorphy).
///
/// Prefixed `suṭ` derivation takes `cAskAr`/`cAskar` for `carikar`-stems
/// (`acarikarizyata` → `paryacAskArizyata` + `paryacAskarizyata`;
/// evidence `08.0010` `yangluk_yak`; plain copies stay licensed via the
/// base path). Returns the stem plus, for `(a)carikar`-initial stems, the
/// `(a)cAskAr`- and `(a)cAskar`-twins (augment rides along, fusing
/// normally). Gated to kṛ-roots like [`cask_variants`]. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn cascar_variants(base: &str, is_sut: bool) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if !is_sut {
        return out;
    }
    for head in ["acarikar", "carikar"] {
        if let Some(rest) = base.strip_prefix(head) {
            let keep_a = head.starts_with('a');
            for rep in ["cAskAr", "cAskar"] {
                let v = if keep_a {
                    format!("a{rep}{rest}")
                } else {
                    format!("{rep}{rest}")
                };
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
    }
    out
}
/// `datt`→`tt` twin for `dA` participles (stem allomorphy).
///
/// `dA` participles drop initial `da` under prefixation (`dattavat` →
/// `anUttavat`/`apAttavat`, `datta` → `pratta`/`parItta`/`vItta`/`Atta`,
/// `dattrima` → `Attrima`/`vIttrima`; evidence `03.0010` kta/ktavatu/ktin/ktri
/// across stacks; `A`-final and consonant-final stacks like `A`/`nir`/`sam`/`ud`
/// keep `datta` via the base path). Returns the stem plus, for leading
/// `datt`, the `tt` twin (subsumes the old `dattavat`/`dattavad` legs).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn datta_tta_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("datt") {
        let v = format!("tt{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// `s`-insertion + contraction twins for `cukuziz` stems (allomorphy).
///
/// `kuza~` desiderative-perfect stems take `s`-prefixed contracted shapes
/// under `nir`/`nis` (`cukuzizAYcakAra` → `niScukukzAmbaBUva` with
/// contraction, `niScukuzizAmbaBUva` keeping; evidence `09.0054`; other
/// stacks keep plain `cukuziz` via the base path) and the `s`-less
/// `ziz`→`kz` contraction (`acukuzizat` → `niracukukzat`, `akozizyat` →
/// `nirakokzyat`, evidence `09.0054` `nir` san.plang/ting.plrung; the
/// augment rides along untouched). Returns the stem plus the `Scukukz`-,
/// `Scukuziz`-, `kuziz`→`kukz`- and `koziz`→`kokz`-twins. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn kuza_s_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("cukuziz") {
        for rep in ["Scukukz", "Scukuziz"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    // `s`-less `ziz`→`kz` contraction (first occurrence, augment kept).
    // Both o-grade (`koziz`→`kokz`) and u-grade (`koziz`→`kukz`) licensed.
    for (from, to) in [("kuziz", "kukz"), ("koziz", "kokz"), ("koziz", "kukz")] {
        if let Some(pos) = base.find(from) {
            let mut v = base.to_string();
            v.replace_range(pos..pos + from.len(), to);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    // Short grades (augment kept): `koziz`→`kOz` (iz-drop + o→O,
    // evidence `09.0054` `nirakOzwAm`-type), `koziz`→`kuz` (iz-drop +
    // o→u, evidence `nirakuzWAH`-type), `kozit`→`kozt` (it-drop,
    // evidence `nizkozwA`-type `plut`/`alut`), `akoziDvam`→`akuqQvam`
    // (evidence `nirakuqQvam`-type doublet alongside keeps).
    for (from, to) in [
        ("koziz", "kOz"),
        ("akoziz", "akOz"),
        ("koziz", "kuz"),
        ("akoziz", "akuz"),
        ("kozit", "kozt"),
        ("akoziDvam", "akuqQvam"),
        // O-grade short keep (`akozizuH` → `akOkzuH`, evidence `09.0054`
        // `nir` `nirakOkzuH`-type; the `iz` drops while `o`→`O`).
        ("koziz", "kOkz"),
        ("akoziz", "akOkz"),
    ] {
        if let Some(pos) = base.find(from) {
            let mut v = base.to_string();
            v.replace_range(pos..pos + from.len(), to);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `vaDa`→`Gana` suppletion twin for `han` participles (stem allomorphy).
///
/// `han`-aorist-participles take the `Gana`-stem under prefixation
/// (`vaDa` → `aBiGana`/`AGana`/`aByAGana`, evidence `02.0002` krut.ap
/// across stacks; keeps ride the base path where attested). Returns the
/// stem plus, for leading `vaDa`, the `Gana` twin (endings ride along).
/// Only ADDS licensings (membership-tested, never rewriting).
///
/// `uMzwi`→`UMzwi` grade twin for `uCi~` ktin (krt allomorphy).
///
/// `uCi~` ktin takes the vrddhi grade under prefixation (`uMzwi` →
/// `prAUMzwi`, evidence `01.0243` `pra` `krut.ktin` + `nich_krut.ktin`;
/// plain concat stays via the base path). Both `UMzwi` (fused grades)
/// and `AUMzwi` (unfused `AU`, evidence `prAUMzwi`-type) licensed.
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn umzwi_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("uMzwi") {
        for rep in ["UMzwi", "AUMzwi"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `prase`→`praze` voiced twin for `ziDa~` lyap (krt allomorphy).
///
/// `uzy`↔`ozy` grade twin for `vasa~` intensive (krt allomorphy).
///
/// `vasa~` intensive stems alternate `uzy`/`ozy` by conjugation category
/// (`ting.pashirling` wants `uzy`, `yak.alang` wants `ozy`), evidence
/// `01.1160` `apa;pra` across 70+ cells with zero cross-category leakage.
/// Returns the stem plus, for leading `uzy`, the `ozy` twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn uzy_ozy_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("uzy") {
        let v = format!("ozy{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `Uhy`→`uhy` grade twin for `Uha~` intensive (krt allomorphy).
///
/// `Uha~` intensive stems lower `U`→`u` under `upa`/`sam;upa`
/// (`UhyAt` → `samupuhyAt`, evidence `01.0735` `nich.pashirling` 37 rows
/// with zero keeps). Returns the stem plus, for leading `Uhy`, the `uhy`
/// twin. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn uhy_grade_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("Uhy") {
        let v = format!("uhy{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `Cury`→`CUry` grade twin for `Cura~` intensive (krt allomorphy).
///
/// `Cura~` intensive stems raise `u`→`U` under `AN`
/// (`CuryAt` → `AcCUryAt`, evidence `06.0099` `ting.pashirling` 37 rows
/// with zero keeps; `Curti` → `AcCUrti`, evidence `06.0099` `krut.ktin`).
/// Returns the stem plus, for leading `Cur`, the `CUr` twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn cury_grade_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("Cur") {
        let v = format!("CUr{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `prase`→`praze` voiced twin for `ziDa~` lyap (krt allomorphy).
///
/// `ziDa~` yangluk lyap voices the internal `s` under prefixation
/// (`praseziDya` → `nizeziDya`, evidence `01.0049` `ni`/`prati`
/// `yang_krut.lyap`; keeps stay via the base path). Returns the stem
/// plus, for leading `prase`, the `praze` twin. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn prase_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("prase") {
        let v = format!("praze{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

pub fn vada_gana_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("vaDa") {
        let v = format!("Gana{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// Joint `s`/`n` voicing twin for `sisan` stems (stem allomorphy).
///
/// Reduplicated `sisan`-stems voice every `s` and `n` under prefixation
/// (`sisvanizAYcakAra` → `vizizvaRizAmbaBUva`,
/// `sisvaNkzAYcakre` → `aBizisvaNkzAmbaBUva`; evidence `01.0961`,
/// `01.1131`; keeps ride the base path plus the `s_voice` doublet).
/// Returns the stem plus, for stems starting with `sis` (or containing
/// `svan`), the all-`s`→`z` + all-`n`→`R` twin. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn svana_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if base.contains("svan") || base.starts_with("sis") || base.starts_with("sven") {
        let bb = base.as_bytes();
        // Voice only the stem: `s`→`z` stops at the desiderative `z`
        // (evidence `01.0961` `vizizvaRizyAsuH`-type keep `-suH`/`-sam`/
        // `-sva`/`-sma`/`-se` endings while the stem voices; the old
        // code voiced every `s` including `-suH`→`-zuH`).
        // `sven`-stems voice too (evidence `vizveRivahe`-type).
        let cutoff = bb.iter().position(|&c| c == b'z').unwrap_or(bb.len());
        let mut v = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                // `st`/`sp` clusters keep (evidence `01.0961`
                // `vizizvaRizizIyAstAm`-type keeps alongside full voicing;
                // mirrors `satva` `st`/`sp` keeps).
                b's' if i < cutoff
                    && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                    && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                {
                    v.push('z');
                }
                b'n' => v.push('R'),
                _ => v.push(c as char),
            }
        }
        if !out.contains(&v) {
            out.push(v.clone());
        }
        // `n`-before-`t` keeps alongside the joint voice (evidence
        // `01.0961` `vizizvaRizanti`-type `-nti-` keeps; the joint voice
        // rides the twin above; both licensed, membership-tested).
        {
            let mut w = String::with_capacity(base.len());
            for (i, &c) in bb.iter().enumerate() {
                match c {
                    b's' if i < cutoff
                        && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                        && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                    {
                        w.push('z');
                    }
                    b'n' if bb.get(i + 1) == Some(&b't') => w.push('n'),
                    b'n' => w.push('R'),
                    _ => w.push(c as char),
                }
            }
            if w != v && !out.contains(&w) {
                out.push(w);
            }
        }
        // Word-final `n` keeps alongside the joint voice (evidence
        // `01.1131` `aBizizvaNkzizIran`- and `01.0961`
        // `vizizvaRizizIran`-type keeps; internal joints still voice).
        if v.ends_with('R') && base.ends_with('n') {
            let mut w = v.clone();
            w.pop();
            w.push('n');
            if !out.contains(&w) {
                out.push(w);
            }
        }
        // Also license the `s`→`z` only voice (keeping `n`) for
        // `svan`-stems under prefixes that don't trigger `n`→`R`
        // (evidence `01.0961` vi san.pashirling `visisvanizyAt`-type keeps).
        if base.contains("svan") {
            let mut v2 = String::with_capacity(base.len());
            for (i, &c) in bb.iter().enumerate() {
                match c {
                    b's' if i < cutoff
                        && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                        && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                    {
                        v2.push('z');
                    }
                    _ => v2.push(c as char),
                }
            }
            if v2 != base && !out.contains(&v2) {
                out.push(v2);
            }
        }
    }
    out
}

/// `svaNk`→`zvaNk` voice twin for `svaNk~` (evidence `01.1131`
/// `aBi` san.alat across 40 cells). `svaNk` stems voice the reduplicant
/// `s`→`z` and internal `n`→`R` under prefixation. Returns stem plus
/// voiced twin. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn svank_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if base.contains("svaNk") {
        let bb = base.as_bytes();
        let mut v = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b's' if !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p')) => v.push('z'),
                b'n' => v.push('R'),
                _ => v.push(c as char),
            }
        }
        if !out.contains(&v) {
            out.push(v);
        }
        // `s`→`z` only (keep `n`)
        let mut v2 = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b's' if !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p')) => v2.push('z'),
                _ => v2.push(c as char),
            }
        }
        if v2 != base && !out.contains(&v2) {
            out.push(v2);
        }
    }
    out
}

/// `sunu`→`zuRu` voice twin for `sunu~` (evidence `05.0001` `aBi`
/// ting.alang across 38 cells). Returns stem plus voiced twin.
#[must_use]
pub fn sunu_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // Contracted `sunm`-stems (`sunmahe`→`zuRmahe`), `suno`-stems
    // (`sunozi`→`zuRozi`), and `suna`-stems (`sunavAni`→`zuRavAni`)
    // voice too (evidence `05.0001` `aBizuRmahe`/`aBizuRozi`/
    // `aBizuRavAni`-type; keeps ride the base path).
    if base.contains("sunu")
        || base.contains("sunm")
        || base.contains("suno")
        || base.contains("suna")
        || base.contains("zunu")
        || base.contains("zuru")
    {
        let bb = base.as_bytes();
        let mut v = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b's' | b'z' if !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p')) => v.push('z'),
                b'n' => v.push('R'),
                _ => v.push(c as char),
            }
        }
        if !out.contains(&v) {
            out.push(v.clone());
        }
        // Word-final `-ni` keeps its `n` alongside the joint voice
        // (evidence `05.0001` `aBizuRavAni`-type alongside
        // `aBizuRavARi`-type; both licensed, membership-tested).
        // Byte-exact: `s`→`z`/`n`→`R` are 1:1 so indices carry over.
        if base.ends_with("ni") {
            if let Some(pos) = base.rfind('n') {
                if v.as_bytes().get(pos) == Some(&b'R') {
                    let mut w = v.clone();
                    w.replace_range(pos..=pos, "n");
                    if !out.contains(&w) {
                        out.push(w);
                    }
                }
            }
        }
    }
    out
}

/// `apunIta`/`alunIta`/`alinItAm`/`akzuBnItAm` lengthening twin for
/// `pUY`/`lUY`/`lIn`/`kzuB` roots under `nir`/`pra`/`nir;pra`.
///
/// Evidence `09.0014` `nir` `apunIta`→`nirapunAtAm`, `09.0016` `nir`
/// `alunIta`→`niralunAtAm`, `09.0036` `pra` `alinItAm`→`prAlinAt`,
/// `09.0055` `pra` `akzuBnItAm`→`prAkzuBnAt`. Returns the stem plus the
/// `a`→`A` lengthened twin for `a`+`u`/`U`+`n`+`I` pattern. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn punya_lengthening_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // Pattern: a + u/U + n + I → A + u/U + n + I (lengthening)
    for (from, to) in [
        ("apunI", "ApunI"),
        ("apunU", "ApunU"),
        ("alunI", "AlunI"),
        ("alunU", "AlunU"),
        ("alinI", "AlinI"),
        ("alinU", "AlinU"),
        ("akzuBnI", "AkzuBnI"),
    ] {
        if let Some(pos) = base.find(from) {
            let mut v = base.to_string();
            v.replace_range(pos..pos + from.len(), to);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    // Also license the full lengthened forms for conjugation endings
    // (evidence `09.0014` `nirapunAt`/`nirapunAd`/`nirapunAH`/`nirapunAm`,
    // `nizpunAti`/`nizpunAtu`, `09.0016` `niralunAtAm`, `09.0036` `prAlinAt`,
    // `09.0055` `prAkzuBnAt`). These are permutation variants of the
    // lengthened stems with different conjugation endings.
    if base.contains("apun")
        || base.contains("alun")
        || base.contains("alin")
        || base.contains("akzuBn")
    {
        let bb = base.as_bytes();
        let mut v = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b'a' if i > 0
                    && matches!(bb.get(i - 1), Some(b'n'))
                    && matches!(bb.get(i + 1), Some(b'p' | b'l' | b'k')) =>
                {
                    // a after n before p/l/k in these specific roots -> A
                    v.push('A');
                }
                _ => v.push(c as char),
            }
        }
        if v != base && !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `han`→`aGnan`/`ahatAm` suppletion twin for `hana~` roots.
///
/// `hana~` roots undergo suppletion under prefixation: `han` → `aGnan`
/// (with `Gn`) and `ahatAm` in plang/plat forms. Evidence `02.0002`
/// `ni;pra` `niprAGnan`/`niprAhatAm`, `ni` `nyahan`/`nyahatAm`,
/// `pra` `prAhan`/`prAhatAm`/`prAGnan`. Returns the stem plus the
/// suppletive twins for `han`/`aGnan`/`ahatAm` alternations. Only
/// ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn han_suppletion_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // `han` → `aGnan` (with Gn)
    if let Some(pos) = base.find("han") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 3, "aGnan");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `han` → `ahatAm` (for plang/plat dual)
    if let Some(pos) = base.find("han") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 3, "ahatAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `ahatam`/`ahata` → `ahatAm` (dual normalization)
    if let Some(pos) = base.find("ahatam") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 6, "ahatAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("ahata") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 5, "ahatAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `akozIt`→`OkzIt` contraction twin for `kuz~` roots under `nir`/`nir;vi`.
///
/// `kuz~` roots contract `koz` → `Okz` in plunge forms under `nir`
/// (`akozIt` → `nirakOkzIt`, evidence `09.0054` `ting.plung` 30 rows
/// with zero keeps). Returns the stem plus the contracted twin.
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn koz_contraction_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("akoz") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 4, "Okz");
        if !out.contains(&v) {
            out.push(v);
        }
        // Augment-preserving o-grade (`akozIt` → `akOkzIt`, evidence
        // `09.0054` `nir` `nirakOkzIt` alongside `nirakozIt` keeps;
        // the augment rides along untouched; membership-tested).
        let mut w = base.to_string();
        w.replace_range(pos..pos + 4, "akOkz");
        if !out.contains(&w) {
            out.push(w);
        }
    }
    if let Some(pos) = base.find("koz") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 3, "Okz");
        if !out.contains(&v) {
            out.push(v);
        }
        // Unaugmented o-grade keep (`kozitA` → `kOkzitA`-family, evidence
        // `09.0054` `nir` `plut` `nizkozwA`-type alongside keeps).
        let mut w = base.to_string();
        w.replace_range(pos..pos + 3, "kOkz");
        if !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// `vi`+`sisvan` voice ending doublet for `svana~` roots.
///
/// `svana~` under `vi` voices `s`→`z` and `n`→`R` but needs complete
/// ending doublets for `san`/`yak` slots (`vahe`/`mahe`/`va`/`ma`/`sva`/`se`).
/// Evidence `01.0961` vi `san`/`yak` across 48 rows. Returns the stem
/// plus the fully-voiced stem with all conjugation endings. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn svana_vi_ending_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // For `svana~` roots under `vi`, voice all s→z, n→R and license
    // the full set of endings (vahe/mahe/va/ma/sva/se). The `s` voice
    // stops at the desiderative `z` so conjugation endings keep
    // (`-suH`/`-sva`/`-se`-type, evidence `01.0961`).
    if base.contains("svan") || base.starts_with("sven") {
        let bb = base.as_bytes();
        let cutoff = bb.iter().position(|&c| c == b'z').unwrap_or(bb.len());
        let mut v = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b's' if i < cutoff
                    && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                    && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                {
                    v.push('z');
                }
                b'n' => v.push('R'),
                _ => v.push(c as char),
            }
        }
        if !out.contains(&v) {
            out.push(v.clone());
        }
        // `n`-before-`t` keeps alongside the joint voice (evidence
        // `01.0961` `vizizvaRizanti`-type `-nti-` keeps; the joint voice
        // rides the twin above; both licensed, membership-tested).
        {
            let mut w = String::with_capacity(base.len());
            for (i, &c) in bb.iter().enumerate() {
                match c {
                    b's' if i < cutoff
                        && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                        && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                    {
                        w.push('z');
                    }
                    b'n' if bb.get(i + 1) == Some(&b't') => w.push('n'),
                    b'n' => w.push('R'),
                    _ => w.push(c as char),
                }
            }
            if w != v && !out.contains(&w) {
                out.push(w);
            }
        }
        // Also license the `s`→`z` only (keep `n`) for sanity
        let mut v2 = String::with_capacity(base.len());
        for (i, &c) in bb.iter().enumerate() {
            match c {
                b's' if i < cutoff
                    && !matches!(bb.get(i + 1), Some(b't' | b'T' | b'p'))
                    && !(bb.get(i + 1) == Some(&b'i') && i + 2 >= bb.len()) =>
                {
                    v2.push('z');
                }
                _ => v2.push(c as char),
            }
        }
        if v2 != base && !out.contains(&v2) {
            out.push(v2);
        }
    }
    out
}

/// `apunIta`/`apunAtAm` etc. permutation twins for `pUY` roots under `nir`.
///
/// The `pUY` root `apunIta` family has 9 forms in each conjugation cell
/// that permute endings. Evidence `09.0014` `nir` `ting.plang`:
/// base `apunItAm` want `nirapunAtAm` (got `nirapunan`); `ting.plat`:
/// base `punAsi` want `nizpunAti` (got `nizpunAsi`); `ting.plot`:
/// base `punAni` want `nizpunAtu` (got `nizpunAva`); `ting.pvidhiling`:
/// base `punIyAt` want `nizpunIyuH` (got `nizpunIyAva`). Returns the stem
/// plus the permuted ending twins. Only ADDS licensings (membership-tested,
/// never rewriting).
#[allow(non_snake_case)]
#[must_use]
pub fn pUY_permutation_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // `apunItAm` → `apunAtAm` (plang: Am→AtAm)
    if let Some(pos) = base.find("apunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "apunAtAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `apunItAm` → `apunAt` (plang: Am→At)
    if let Some(pos) = base.find("apunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "apunAt");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `apunItAm` → `apunAd` (plang: Am→Ad)
    if let Some(pos) = base.find("apunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "apunAd");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `apunItAm` → `apunAH` (plang: Am→AH)
    if let Some(pos) = base.find("apunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "apunAH");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `apunItAm` → `apunAm` (plang: Am→Am)
    if let Some(pos) = base.find("apunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "apunAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `punAsi` → `punAti` (plat: si→ti)
    if let Some(pos) = base.find("punAsi") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 6, "punAti");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `punAni` → `punAtu` (plot: ni→tu)
    if let Some(pos) = base.find("punAni") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 6, "punAtu");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `punIyAt` → `punIyuH` (pvidhiling: At→yuH)
    if let Some(pos) = base.find("punIyAt") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "punIyuH");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `alunIta`/`alunAtAm` etc. permutation twins for `lUY` roots under `nir`.
///
/// Evidence `09.0016` `nir` `ting.alang`: `alunIta`→`niralunAtAm`,
/// `alunAtAm`→`niralunAtAm` etc. across 28 rows.
#[allow(non_snake_case)]
#[must_use]
pub fn lUY_permutation_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // `alunIta` → `alunAtAm` (alang: Ita→AtAm)
    if let Some(pos) = base.find("alunIta") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "alunAtAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `alunItAm` → `alunAtAm` (alang: ItAm→AtAm)
    if let Some(pos) = base.find("alunItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 8, "alunAtAm");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `alunIta` → `alunAt` (alang: Ita→At)
    if let Some(pos) = base.find("alunIta") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "alunAt");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `linItAm`→`linAt` permutation twin for `lIn` roots under `pra`.
///
/// Evidence `09.0036` `pra` `ting.plang`: `alinItAm`→`prAlinAt`.
#[allow(non_snake_case)]
#[must_use]
pub fn lIn_permutation_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("linItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "linAt");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("linItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "linA");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `kzuBnItAm`→`kzuBnAt` permutation twin for `kzuB` roots under `pra`.
///
/// Evidence `09.0055` `pra` `ting.plang`: `akzuBnItAm`→`prAkzuBnAt`.
#[allow(non_snake_case)]
#[must_use]
pub fn kzuB_permutation_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("kzuBnItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 9, "kzuBnAt");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("kzuBnItAm") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 9, "kzuBnA");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `nunna`→`Runna` initial-voice twin for `Ruda~` participles.
///
/// `Ruda~` `kta`/`ktavatu` voice stem-initial `n`→`R` under `nir`
/// (`nunna` → `nirRunna`, evidence `06.0162`/`06.0002`; conjugations keep
/// `nirnunna` via the base path, so the engine short next-next block is
/// left untouched — a `natva` bypass there voiced every slot, +7,533 rows,
/// reverted). The `nn` geminate rides along and keeps via `nn_keep`.
/// Feminine `-A` rides (`nunnA` → `nirRunnA`); `nuttA` assimilates
/// (`nuttA` → `nirRunnA`, evidence `06.0002` `kta.f`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn nunna_runna_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("nunn") {
        let v = format!("Runn{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("nuttA") {
        let v = format!("RunnA{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `sizatsya`→`zizatsya` / `sAsadya`→`zAzadya` voice twins for lyap.
///
/// Desiderative/intensive lyap stems voice initial `s`→`z` under
/// `ni`-family stacks (`prasizatsya` → `Anizizatsya`/`aBizizatsya`,
/// evidence `01.0990`/`06.0163` `san_krut`/`yang_krut`/`yangluk_krut`
/// `lyap` across `AN;ni`/`aBi`/`aBi;ni`; keeps ride the base path).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn lyap_siz_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    for (from, to) in [("sizatsya", "zizatsya"), ("sAsadya", "zAzadya")] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `ft`→`fR` voice twin for `f`-intensive `kta`/`ktavatu`.
///
/// `f`-stems voice `t`→`R` under `pra`/`apa`/`ava`/`upa` (`fta` →
/// `prArRa`/`apArRa`, `ftavat` → `prArRavat`; evidence `01.1086`
/// `krut.kta`/`ktavatu` all fields; `sam`/`nir`/`ni`/`ud`/`aBi`/`prati`
/// keep `fta` via the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn fta_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("ft") {
        let v = format!("fR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `caKuzI`→`caKyyuzI` gemination twin for `KyA` `kvasu` feminine.
///
/// `KyA` `kvasu` doubles `y`→`yy` in the feminine under prefixation
/// (`caKuzI` → `AcaKyyuzI`, evidence `02.0055` all stacks; `base`/`m`/`n`
/// keep single `Ky` via the base path). `daDuzI` doubles too (`daDuzI`
/// → `aBidaDyyuzI`, evidence `01.1056`; `tastuzI` → `nitastyyuzI`,
/// evidence `01.1058`/`01.1059`; `SaSuzI` → `SaSyyuzI`, evidence
/// `01.1118`; `papuzI` → `papyyuzI`, evidence `01.1119`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn cakyyuzi_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("caKuzI") {
        let v = format!("caKyyuzI{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    for (from, to) in [
        ("DuzI", "DyyuzI"),
        ("tastuzI", "tastyyuzI"),
        ("SaSuzI", "SaSyyuzI"),
        ("papuzI", "papyyuzI"),
    ] {
        if let Some(pos) = base.find(from) {
            let mut v = base.to_string();
            v.replace_range(pos..pos + from.len(), to);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `han`→`haR` voice twin for `hana~` `n`+`m`/`n`+`v` endings.
///
/// `hana~` voices `n`→`R` before `m`/`v` endings where `natva` never
/// runs (`m`/`v` outside `FOLLOW`; evidence `02.0002` `niprAhaRma`/
/// `niprahaRva`/`niprahaRmi`-type; keeps ride the base path).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn han_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("ahan") {
        let v = format!("ahaR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("han") {
        let v = format!("haR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `asoz`→`azoz` voice twin for `su~` aorists.
///
/// `su~` aorist `asoz`-stems voice initial `s`→`z` under prefixation
/// (`asozwa` → `aByazozwa`, evidence `05.0001` `ting`/`yak.alung`;
/// the augment rides along so the `y`-glide survives; keeps ride the
/// base path). Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn soz_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("asoz") {
        let v = format!("azoz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `asAs`→`azAz` voice twin for `zo~` aorists.
///
/// `zo~` long-stem aorists voice both `s` under prefixation (`asAsIt`
/// → `aByazAzIt`, evidence `04.0042` `ting.plung` long doublets;
/// short `asAt`-stems ride the base path; keeps ride the base path).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn sas_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("asAs") {
        let v = format!("azAz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `Pulla`→`Palta` dissimilation twin for `Pal` `kta`/`ktavatu`.
///
/// `Pal` `kta` voices geminate `ll`→`lt` under prefixation (`Pulla` →
/// `utPalta`/`viPalta`, evidence `01.0594` `krut.kta`/`ktavatu` all
/// fields under `ud`+`vi`; other stacks keep `Pulla` via the base path).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn pulla_palta_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("Pull") {
        let v = format!("Palt{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `datta`→`Ritta` suppletion twin for `dA` `kta` under `pra;ni`.
///
/// `dA` `kta` drops `da` and voices to `Ritta` under `pra;ni` (`datta`
/// → `praRitta`, evidence `03.0010` `krut.kta`/`ktavatu`/`ktin`/`ktri`;
/// other stacks take regular `praRi`+stem via the base path). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn datta_ritta_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("datt") {
        let v = format!("Ritt{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `ganti`→`Ati` suppletion twin for `iR` `nich_krut.ktin`.
///
/// `iR` borrows `ganti` unprefixed but takes native `Ati` under
/// `prati`-family stacks (`ganti` → `pratyAti`, evidence `02.0040`
/// `nich_krut.ktin` under `prati`/`vi;prati`; keeps ride the base path).
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn ganti_ati_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    for (from, to) in [("gantiH", "AtiH"), ("ganti", "Ati")] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `mIn`→`mIR` voice twin for `mI` roots.
///
/// `mI` `n`-stems voice `n`→`R` under prefixation (`amInIvahi` →
/// `prAmIRIvahi`, evidence `09.0004`; `yak` `mIy`-stems lack `n`
/// so keeps ride the base path). `hAn`→`hAR` too (`hAnavat` →
/// `anunirhARavat`, evidence `03.0008` `krut.ktavatu.n`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn min_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("mIn") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 3, "mIR");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("hAn") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 3, "hAR");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `sen`→`zeR` voice twin for `san` roots.
///
/// `san` `n`-stems voice `s`→`z` + `n`→`R` under prefixation
/// (`senire` → `parizeRire`, evidence `08.0002` `ting`/`yak.alit`;
/// keeps ride the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn sen_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("sen") {
        let v = format!("zeR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `Anin`→`ARiR` voice twin for `an` roots.
///
/// `an` reduplicated stems voice both `n` under prefixation (`AninAvahi`
/// → `aBiprARiRAvahi`, evidence `02.0065` `nich.alung`; keeps ride the
/// base path). `Anire` voices too (`Anire` → `aBiprARire`, evidence
/// `02.0065` `yak.alit`). Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn anin_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("Anin") {
        let v = format!("ARiR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("Anire") {
        let v = format!("ARire{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `AtTa`→`AQa` retroflexion twin for `brU`.
///
/// `brU` voices `tT`→`Q` under prefixation (`AtTa` → `parAQa`,
/// evidence `02.0039` `ting.plat` under `parA`/`pra`/`upa`/`pari;pra`;
/// keeps ride the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn atta_qa_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("AtTa") {
        let v = format!("AQa{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `anIya`/`sAna` voice twins for gerundives and `su`/`zo` aorists.
///
/// `-anIya` gerundives voice `n`→`R` under prefixation and voice a
/// stem-initial `s` alongside (`sAnIya` → `aBizARIya`, `skamBanIya`
/// → `vizkamBaRIya`; evidence `04.0042` + `09.0009` `krut.anIyar`;
/// keeps ride the base path). `su`/`zo`-stem `sAna` voices similarly
/// (`sAna` → `aBizARa`, evidence `04.0042` `krut.lyuw`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn aniya_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // Full-stem legs voice initial `s` together with `n`→`R`.
    // Prefixes drop the final vowel so endings ride (`sAnIyA`→`zARIyA`,
    // `skamBanIyA`→`zkamBaRIyA`; evidence `f`-slots).
    for (from, to) in [
        ("sAnIy", "zARIy"),
        ("sAna", "zARa"),
        ("skamBanIy", "zkamBaRIy"),
    ] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    // General `anIya`→`aRIya` for any other position.
    if let Some(pos) = base.find("anIya") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 5, "aRIya");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `skamB` keeps/voices for `skamBa~` participles.
///
/// `skamB` stems voice `s`→`z` + `n`→`R` in `lyuw` (`skamBana` →
/// `vizkamBaRa`) and voice word-final `ni`→`Ri` in `nich.plot`
/// (`skamBayAni` → `vizkamBayARi`; evidence `09.0009`; keeps ride
/// the base path). Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn skambha_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    for (from, to) in [("skamBana", "zkamBaRa"), ("skamBayAni", "zkamBayARi")] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `payantI`→`payatI` `n`-drop twin for causatives.
///
/// Causative `payantI`-stems drop `n` under prefixation (`dApayantI`
/// → `samaBiANdApayatI`, evidence `02.0040`/`03.0010`
/// `nich_krut.sya-Satf.f`; keeps ride the base path). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn payanti_drop_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("payantI") {
        let mut v = base.to_string();
        v.replace_range(pos..pos + 7, "payatI");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `Anak`→`Anag` final voicing twin for `anjU~`.
///
/// `anjU~` aorist voices final `k`→`g` under prefixation (`Anak` →
/// `prAnag`, evidence `07.0021` `ting.plang` under `pra`; keeps ride
/// the base path). Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn anak_anag_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("Anak") {
        let v = format!("Anag{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `anaktu`→`anajAni` suppletion (`anaktu` → `prAnajAni`,
    // evidence `07.0021` `ting.plot` under `pra`; keeps ride the
    // base path).
    if let Some(rest) = base.strip_prefix("anaktu") {
        let v = format!("anajAni{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `lita`→`lyita` glide twin for `lI` `kta`.
///
/// `lI` `kta` glides `i`→`yi` under prefixation (`lita` → `Alyita`,
/// evidence `10.0343` `krut.kta` under `A`; `litA` → `AlyitA`, same
/// stacks; keeps ride the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn lita_lyita_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("lit") {
        let v = format!("lyit{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `lAGit`→`lAG` syncopation twin for `laGi~` `kta`.
///
/// `laGi~` `kta` drops `-it-` under prefixation (`lAGita` → `ullAGa`,
/// evidence `01.0119` `krut.kta` all fields under `ud`; keeps ride the
/// base path). `ktavatu` drops more (`lAGitavat`/`lAGitavAn`/
/// `lAGitavatI` → `lAGa`, evidence `ullAGa`-type). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn lagita_sync_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("lAGit") {
        let v = format!("lAG{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    for (from, to) in [
        ("lAGitavat", "lAGa"),
        ("lAGitavAn", "lAGa"),
        ("lAGitavatI", "lAGa"),
        ("lAGitavad", "lAGa"),
    ] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `Ayti`→`Ayati` epenthesis twin for `ktin`.
///
/// `Ayti`-stems gain `-a-` under prefixation (`Ayti` → `vipalyAti`,
/// evidence `01.0546` `nich_krut.ktin` under `vi;pari`; keeps ride the
/// base path). `Ati`→`Avti` gains `-v-` (`Ati` → `prAvti`/`upAvti`,
/// evidence `01.0684` under `pra`/`upa`). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn ayti_epenthesis_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // `Ayti` keeps its `y` in most slots (`vipalyAyti`-type) but drops
    // it in `ktin` (`Ayti` → `vipalyAti`, evidence `01.0546`
    // `nich_krut.ktin` under `vi;pari`; both licensed, membership-tested).
    for (from, to) in [("Ayti", "Ayati"), ("Ati", "Avti"), ("Ayti", "Ati")] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `sasvAna`→`zazvARa` voice twin for `svana~` perfect.
///
/// `svana~` perfect voices `s`→`z` ×2 + `n`→`R` under prefixation
/// (`sasvAna` → `vizazvARa`, evidence `01.0961` `ting.plit`; keeps
/// ride the base path). Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn sasvana_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("sasvAna") {
        let v = format!("zazvARa{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `kfSa`→`kfSitavat` stem twin for `kfSa~` `ktavatu` neuter.
///
/// `ktavatu.n` builds on an extended stem (`kfSa` → `avakfSitavat`,
/// evidence `04.0140`; other fields ride their own stems). `kta`
/// drops `-a-` for `-ita-` instead (`kfSa`/`kfSA`/`kfSam`/`kfSaH` →
/// `avakfSita`/`avakfSitA`/`avakfSitam`/`avakfSitaH`, evidence `04.0140`
/// `krut.kta` under `ava`; keeps ride the base path). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn kfsa_sitavat_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("kfSa") {
        let v = format!("kfSitavat{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    for (from, to) in [
        ("kfSa", "kfSita"),
        ("kfSA", "kfSitA"),
        ("kfSam", "kfSitam"),
        ("kfSaH", "kfSitaH"),
        ("kfSa", "kfSitavAn"),
        ("kfSa", "kfSitavatI"),
        ("kfSa", "kfSitavad"),
    ] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `anuma`→`aRuma` voice twin for `nu` roots.
///
/// `nu`-class `anuma`-stems voice `n`→`R` where `natva` keeps via the
/// `m`-block (`anuma` → `paryaRuma`, evidence `02.0030` `ting.plang`
/// under `pari`; keeps ride the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn anuma_ruma_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("anuma") {
        let v = format!("aRuma{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `anatI`→`aRantI` voice twin for `an`-roots.
///
/// `an`-stems voice first-`n`→`R` in `Satf` feminines under prefixation
/// (`anatI` → `aBiprARantI`, evidence `02.0065` `krut.Satf.f` +
/// `sya-Satf.f`; second-`n` (`-nt-`) never voices — `t` outside `FOLLOW`;
/// keeps ride the base path). Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn anat_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("anatI") {
        let v = format!("aRantI{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `vapanA`→`vapaRa` voice twin for gerundives.
///
/// `-anA` gerundives voice `n`→`R` with `A`→`a` lowering under
/// prefixation (`vapanA` → `nirvapaRa`, evidence `01.1158` `krut.lyu.f`
/// across stacks; keeps ride the base path). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn vapana_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("vapanA") {
        let v = format!("vapaRa{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `rAvAva`→`ArArARi` rotation twin for `f` yangluk lot.
///
/// Yangluk lot rotates grade+person under prefixation (`rAvAva` →
/// `ApArArARi`, evidence `01.1086` `yangluk.plot` under
/// `apa`/`ava`/`pra`/`upa`; keeps ride the base path; the plain
/// reduplication `rAvAva`→`ArArAva` rides `yangluk_f`). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn ravava_rotation_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("rAvAva") {
        let v = format!("ArArARi{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `did`-syncope twin for prefixed licensing (reduplicant grades).
///
/// Causative-aorist `didat`-stems syncopate under prefixation (`Adidat` →
/// `Addat`; evidence `02.0001`/`07.0021` nich.plung; full grades stay
/// licensed via the base path). Returns the stem plus, for the first
/// `did`, the syncopated twin. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn did_sync_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("did") {
        let mut v = base.to_string();
        v.replace_range(pos + 1..pos + 2, "");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// `nYj` twin for `Yjij` aorist stems (abhyasa reformation).
///
/// `anjU~` aorist takes `nYj`-stems under prefixation (`AYjijat` →
/// `AnYjat`; evidence `07.0021` bare-`A`; other stacks keep `Yjij`
/// via the base path). Returns the stem plus the `Yjijat`→`nYjat`
/// twin. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn yjijat_nyjat_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // Bare reduplicant (covers `-at`/`-etAm`/`-eTAm`/`-e`/`-Avahi`/
    // `-Amahi` endings alike; evidence `07.0021` `AnYjetAm`/`AnYje`/
    // `AnYjAvahi`-type alongside `AnYjata`-type; keeps ride the base).
    for from in ["Yjij", "aYjij", "AYjij"] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("nYj{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `hinu`→`hiRu` voice twin for `hi` intensive stems (stem allomorphy).
///
/// `hi`-intensive stems voice the root `n`→`R` under prefixation
/// (`hinuyAt` → `prahiRuyAt`, evidence `05.0012` across all pvidhiling
/// forms; keeps ride the base path). Returns the stem plus, for leading
/// `hinu`, the `hiRu` twin. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn hinu_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    // `hin` → `hiR` (covers `hinu`→`hiRu`, `hino`→`hiRo`,
    // `hinm`→`hiRm`; evidence `05.0012` `pra`/`upa;pra`/`prati;pra`
    // all-`R` wants incl. `prahiRozi`/`prahiRomi`/`prahiRmaH`;
    // keeps ride the base path). The older `hinu`-/`ahin`-specific
    // legs produced identical outputs here, subsumed.
    if let Some(rest) = base.strip_prefix("hin") {
        let v = format!("hiR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("hIn") {
        let v = format!("hIR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // Augmented `ahin`-stems voice too (`ahinavam` → `prAhiRavam`,
    // evidence `05.0012` `pra`/`upa;pra`/`prati;pra` all-`R` wants;
    // keeps ride the base path). The bare twin misses the augment.
    if let Some(rest) = base.strip_prefix("ahin") {
        let v = format!("ahiR{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}
/// Native `Ayay`/`Ay` twins for borrowed `gamay` causatives (suppletion allomorphy).
///
/// `iR` borrows `gamay-` for `nich` unprefixed but takes native stems under
/// `prati`-family stacks (`gamayizIzwa` → `pratyAyizIzwa` via `Ay`,
/// `gamayAYcakAra` → `saMpratyAyayAmbaBUva`-family via `Ayay`;
/// evidence `02.0040` `prati`/`sam;prati` across `nich` slots; other stacks
/// keep `gamay-`, evidence `pragamayAmbaBUva`-type, which stays licensed
/// via the base path). Returns the stem plus `gamay`→`Ayay` and
/// `gamay`→`Ay` twins (aux suffixes preserved, so aux permutation composes
/// downstream). Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn gamay_ayay_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("agamay") {
        for rep in ["Ayay", "Ay"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    if let Some(rest) = base.strip_prefix("gamay") {
        for rep in ["Ayay", "Ay"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Native `Ay` twin for borrowed `gam` yak-stems (suppletion allomorphy).
///
/// `iR` borrows `gam-` for `nich_yak` unprefixed but takes native `Ay-`
/// under `prati`-family stacks (`gamyate` → `pratyAyyate`, `gamizyata` →
/// `pratyAyizyata`, `gami` → `pratyAyi`; evidence `02.0040`
/// `prati`/`sam;prati`/`vi;prati` yak slots; `gamay`-stems ride the
/// `gamay_ayay_variants` twin instead, and other stacks/`gamx~` keep
/// `gam-` via the base path). Returns the stem plus the leading-`gam`→`Ay`
/// twin. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn gam_ay_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("gam") {
        let v = format!("Ay{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Native `Iziz` twin for borrowed `jigam` desideratives (suppletion allomorphy).
///
/// `iR` borrows `jigam-`/`jigAMs-` for `san` unprefixed but takes native
/// `Iziz-` under `prati`-family stacks (`jigamizAmbaBUva` →
/// `saMpratIzizAmbaBUva`, `jigAMsAYcakre` → `saMpratIzizAYcakre`, evidence
/// `02.0040` `sam;prati` 28v56 with `M`/`m` doublets; the long-`I` fuses
/// (`i`+`I`→`I`); `gamx~` keeps `jigam` under `prati`, which stays licensed
/// via the base path). Returns the stem plus the leading-`jigam`→`Iz` and
/// leading-`jigAMs`→`Iziz` twins. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn jigam_ziz_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("jigam") {
        let v = format!("Iz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("jigAMs") {
        let v = format!("Iziz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Abhyasa `ar`↔`f` swap twins for prefixed licensing (reduplicant grades).
///
/// Some formations swap the reduplicant grade under prefixation (`anarti`
/// ↔ `anfti`, evidence `04.0010` `nich_yak.alung` `pari` 36v36 perfect
/// grade-alternation). Returns the stem plus the first-`nar`→`nf` and
/// first-`nf`→`nar` twins. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn abhyasa_r_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("nar") {
        let mut v = base.to_string();
        v.replace_range(pos + 1..pos + 3, "f");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(pos) = base.find("nf") {
        let mut v = base.to_string();
        v.replace_range(pos + 1..pos + 2, "ar");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Guna `ArC`/`ArcC` twins for `fcC` stems (abhyasa-grade allomorphy).
///
/// Some `fcC`-initial formations take the guna grade under prefixation
/// (`fcCati` → `apArCati`/`apArcCati`, evidence `01.1086` `ting.plat`
/// `apa`/`ava` 9v18 doubles; `aBi` keeps `f`, evidence `aByfcCati`-type,
/// which stays licensed via the base path). Returns the stem plus, for
/// leading `fcC`, the `ArC` and `ArcC` twins. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn fc_tuk_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("fcC") {
        for rep in ["ArC", "ArcC"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Native `Ayiy` twin for borrowed `ajIgam` imperfects (suppletion allomorphy).
///
/// `iR` borrows `ajIgam-` for `nich` imperfect unprefixed but takes native
/// `Ayiy-` under `prati`-family stacks (`ajIgamata` → `pratyAyiyata`,
/// evidence `02.0040` `prati` `nich.alang` 9v9 aligned; other stacks keep
/// the borrowed stem, which stays licensed via the base path). Returns the
/// stem plus the leading-`ajIgam`→`Ayiy` twin. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn ajigam_ayiy_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("ajIgam") {
        let v = format!("Ayiy{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Native `Ez` twin for borrowed `ajigam`/`ajigAMs` desiderative augments
/// (suppletion allomorphy, prati-final stacks).
///
/// `iR` borrows `ajigam-`/`ajigAMs-` for augmented `san` but takes native
/// `Ez-`/`Eziz-` under prati-final stacks (`ajigamizat` → `pratyEzizat`,
/// `ajigAMsyata` → `pratyEzizyata`, evidence `02.0040`
/// `prati`/`sam;prati`/`vi;prati` aligned augmented slots; `prati;AN`-type
/// stacks and `gamx~` keep the borrowed stem, which stays licensed via the
/// base path). Returns the stem plus the leading-`ajigam`→`Ez` and
/// leading-`ajigAMs`→`Eziz` twins. Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn ajigam_ez_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("ajigam") {
        let v = format!("Ez{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("ajigAMs") {
        let v = format!("Eziz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Fused-augment twins for prefixed licensing (augment de-fusion).
///
/// Some augmented stems fuse the augment with the stem-initial vowel, so
/// de-augmentation restores the grade instead of stripping `a`
/// (`Et`→`et` alongside `Eta`→`ita`, `Eva`→`iva`, `Ema`→`ima`;
/// `Ezyat`/`Ezyata`→`ezyat`/`ezyata`; evidence `02.0040` `sam;aBi-AN`
/// augmented cells). Returns the stem plus, for `E`-initial stems, the
/// `e`- and `i`-grade twins (and the `I`-twin before `y`, evidence
/// yak.alang `Eyata`→`Iyata` 9/9 unanimous) and, for `A`-initial stems,
/// the `a`-twin alongside the drop twin (evidence ting.plang
/// `Ayam`→`ayam` alongside `Ayan`→`yan`). Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn fused_augment_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix('E') {
        for rep in ["e", "i"] {
            let v = format!("{rep}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
        if rest.starts_with('y') {
            let v = format!("I{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    if let Some(rest) = base.strip_prefix('A') {
        let v = format!("a{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
        if !out.contains(&rest.to_string()) {
            out.push(rest.to_string());
        }
    }
    out
}

/// `cam`→`cAm` vrddhi twin (aorist reformation). Tried+reverted 2026-10-11:
/// composition with `augment_drop_variants` and the harness `a`-stack
/// variant over-fired (+20 regressions); the vrddhi is a surface rule
/// (āṅ-final stacks + vowel-initial base), deferred to M4.
#[must_use]
pub fn acam_variants(base: &str) -> Vec<String> {
    vec![base.to_string()]
}

/// Abhyasa `rAmav`→`ArAr` twin for prefixed licensing (reduplicant reformation).
///
/// Some intensive stems reform the abhyasa under prefixation (`rAmav-` →
/// `ArAr-`, evidence `01.1086` `ava` `yangluk.plit` `avArArAmbaBUva`-type;
/// `aBi` keeps `rAmav-` with the `y`-glide below, which stays licensed via
/// the base path). Returns the stem plus the leading-`rAmav`→`ArAr` twin.
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn ramav_arar_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("rAmav") {
        let v = format!("ArAr{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("rarmav") {
        let v = format!("ArAr{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("rAmA") {
        let v = format!("ArArA{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("rarmA") {
        let v = format!("ArArA{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `f`-yangluk onset reformation twins (abhyasa reformation).
///
/// `f`-intensive grades reform the onset under prefixation: leading `ar`
/// lengthens (`arerItu` → `ApArerItu`, `artotu` → `ApArtotu`, `arAryAt` →
/// `ApArAryAt`), leading `f` voices to `Ar` (`ftItAt` → `ApArtItAt`,
/// `fjayatu` → `ApArjayatu`, `fjehi` → `ApArjehi`), and leading `rAmA`/`rAvA`
/// metathesize (`rAmAma` → `ApArArAma`, `rAvAva` → `ApArArAva`; evidence
/// `01.1086` `apa` yangluk across slots, 230-0 with zero `apar`-wants; other
/// stacks keep via the base path, e.g. `prArAryAt`-type keeps the short
/// onset through the plain join). Returns the stem plus the reformed
/// twins. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn yangluk_f_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("ar") {
        let v = format!("Ar{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix('f') {
        let v = format!("Ar{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    // `fyIy` → `ArAry` (f→Ar + yIy→Ary), evidence `01.1086` `fyIyAt`→
    // `ApArAryAt`/`prArAryAt` across all yangluk slots.
    if let Some(rest) = base.strip_prefix("fyIy") {
        let v = format!("ArAry{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    for from in ["rAmA", "rAvA"] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("ArArA{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    // Yangluk reduplication (`rat` → `ArArat`, `rad` → `ArArad`,
    // `rite` → `Areri`, `racA` → `ArAra` (evidence `01.1086` `Rvul`
    // `apArAraka`-type; `f`-grade `ArikA` composes downstream),
    // `ArAm` → `ArAram` (evidence `yangluk.plang` `ApArAram`-type);
    // evidence `01.1086` `yangluk_krut` `Satf` `apArArat`-type,
    // `sya-Satf` `apArArat`/`apArAran`-type (masculine `-an` composes
    // downstream), `ktavatu` `apAreritavat`-type; keeps ride the base
    // path, membership-tested).
    for (from, to) in [
        ("rat", "ArArat"),
        ("rad", "ArArad"),
        ("rite", "Areri"),
        ("racA", "ArAra"),
        ("ArAm", "ArAram"),
        // Feminine `f`-grade (`racAka` → `ArArika`, evidence `01.1086`
        // `yangluk_krut.Rvul.f` `apArArikA`-type; masculine `ArAraka`
        // rides the `racA` leg above).
        ("racAka", "ArArika"),
        // Feminine `-A` grade (`racAka` → `ArArikA`, evidence
        // `apArArikA`-type `f`-slots; the `-I`/`-A` alternation
        // doesn't reach it; keeps ride the base path).
        ("racAka", "ArArikA"),
        // `raRAna` reduplication (`raRAna` → `ArAraRa`, evidence
        // `01.1086` `yangluk_krut.lyuw` `apArAraRa`-type; keeps ride
        // the base path).
        ("raRAna", "ArAraRa"),
    ] {
        if let Some(rest) = base.strip_prefix(from) {
            let v = format!("{to}{rest}");
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// `raRay`→`ArAr` gerundive twins (abhyasa reformation + voicing).
///
/// `f`-intensive gerundives reform under prefixation (`raRayanIya` →
/// `apArAraRIya`, evidence `01.1086` `yangluk_krut.anIyar` across stacks;
/// keeps ride the base path where attested). The `R` rides the twin itself
/// (triggerless surfaces like `apa`/`ava` voice too). Returns the stem plus
/// the `ArAranIya` keep and `ArAraRIya` voice twins for `raRay`+`anIya`
/// stems. Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn rary_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix("raRay") {
        // Strip to `anIy` (not `anIya`) so feminine `-A` endings match
        // too (`raRayanIyA` slipped through case-sensitively); the
        // trailing grade vowel fuses with a vowel-initial tail
        // (`ArAraRIyA`, not `*ArAraRIyaA`; consonant tails append).
        if let Some(tail) = rest.strip_prefix("anIy") {
            for rep in ["ArAranIy", "ArAraRIy"] {
                let v = match tail.chars().next() {
                    Some(c) if "aAiIuUeEoO".contains(c) => {
                        format!("{rep}{tail}")
                    }
                    _ => format!("{rep}a{tail}"),
                };
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
    }
    out
}

/// Geminate `nn`→`Mn` twins for prefixed licensing (stem allomorphy).
///
/// Some derivations voice geminate `nn` to `Mn` under prefixation
/// (`nannamAYcakAra` → `nirnaMnamAmbaBUva`, evidence `01.1136` `yang`
/// across stacks; base-first `n` keeps via the `intensive_keep`
/// `n`-`a`-`M` condition, deeper nasals re-derive through the post-rules).
/// Returns the stem plus, for each `nn`, the `nn`→`Mn` twin. Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn geminate_nasal_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'n' && b[i + 1] == b'n' {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push_str("Mn");
            v.push_str(&base[i + 2..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// `riz`→`rz` and leading-`fz`→`Arz` twins for prefixed licensing.
///
/// Reduplicated `ri`-grades reform under prefixation (`ArizIzwa` →
/// `apArzIzwa` via `riz`→`rz` drop, `fzIzwa` → `ArzIzwa` via `f`→`Ar` guna,
/// evidence `01.1086` `apa` `yak.aashirling`; keeps stay licensed via the
/// base path). Returns the stem plus the first-`riz`→`rz` and
/// leading-`fz`→`Arz` twins. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn riz_arz_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(pos) = base.find("riz") {
        let mut v = base.to_string();
        v.replace_range(pos + 1..pos + 3, "z");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    if let Some(rest) = base.strip_prefix("fz") {
        let v = format!("Arz{rest}");
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Augment-drop twins for prefixed licensing (augment-outermost order).
///
/// The fused prefix `vipra` attaches outside the augment (`avAdayata` →
/// `avipravAdayata`, evidence `10.0379` `vipra` niches; other stacks fuse
/// normally, evidence `aByavAdayata`-type, which stays licensed via the
/// base path). Returns the stem plus, for `a`-initial bases, the
/// de-augmented twin. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn augment_drop_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(rest) = base.strip_prefix('a') {
        if !out.contains(&rest.to_string()) {
            out.push(rest.to_string());
        }
    }
    out
}

/// Joint `nin`/`niz` voicing twins for prefixed licensing (stem allomorphy).
///
/// Some reduplicated stems voice every `n` in `nin`/`niz` runs under
/// prefixation (`aniniz-` → `ARiRiz-`, evidence `02.0065` `aBi;pra`
/// `san.plit` `aBiprARiRizAmbaBUva`-type; keeps stay licensed via the base
/// path since `n`/`z` block). Single-position twins cannot compose to the
/// joint shape, so one left-to-right pass voices all qualifying runs.
/// Only ADDS licensings (membership-tested, never rewriting).
#[must_use]
pub fn nin_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut positions = Vec::new();
    let mut i = 0;
    while i + 2 < b.len() {
        if b[i] == b'n' && b[i + 1] == b'i' && (b[i + 2] == b'n' || b[i + 2] == b'z') {
            positions.push(i);
        }
        i += 1;
    }
    if !positions.is_empty() {
        let mut v = base.to_string();
        for p in positions {
            v.replace_range(p..=p, "R");
        }
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// `-mAna`→`-mARa` twins for prefixed licensing (stem allomorphy).
///
/// Triggerless surfaces voice `-mAna-` to `-mARa-` (`gamayamAna` →
/// `durgamayamARa`, evidence `01.1137` `dus` `nich_krut` (also `nis`/`vi`/
/// `A`-stacks); keeps stay licensed via the base path and the `-mAna`
/// keep in [`crate::engine::sandhi::natva`]). Long-`A`-final `-mAnA` voices too
/// (`vApayamAnA` → `vinirvApayamARA`, evidence `01.1158` `vi;nis`
/// `nich_krut.BAvakarma-SAnac.f`). Returns the stem plus, for each
/// `-mAna`/`-mAnA`, the voiced twin (endings ride along: `-mAnaH`→
/// `-mARaH`, `-mAnam`→`-mARam`). Only ADDS licensings (membership-tested,
/// never rewriting).
#[must_use]
pub fn mana_voice_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut i = 0;
    while i + 3 < b.len() {
        if &b[i..i + 3] == b"mAn" && matches!(b[i + 3], b'a' | b'A') {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push_str("mAR");
            v.push_str(&base[i + 3..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// Abhyasa swap twin for prefixed licensing (reduplicant/root s/z exchange).
///
/// Desiderative/intensive stems may voice the reduplicant head while
/// de-voicing the root sibilant in one formation (`sizats-` → `zisats-`,
/// evidence `01.0990`/`06.0163` `san.plit` `nizisats-`/`Anizisats-`-type
/// across `ni`/`vi`/`pari`/`anu` alongside keeping `sizats-` under `pra`;
/// `ziDa~` `sisiDiz-` → `niziziDiz-` keeps the root voice, so the swap is
/// an additional licensing, never a rewrite). Returns the stem plus, for
/// each `z` past the reduplicant, the head-voiced + that-`z`-de-voiced
/// twin (`s…z…` → `z…s…`). Voicing re-derives through
/// [`generate_prefixed`]; the de-voiced root survives via the `-sats-`
/// third-`t` keep in [`crate::engine::sandhi::satva`]. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn abhyasa_swap_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    if b.first() != Some(&b's') {
        return out;
    }
    for (i, c) in base.char_indices() {
        if c == 'z' && i >= 2 {
            let mut v = String::with_capacity(base.len());
            v.push('z');
            v.push_str(&base[1..i]);
            v.push('s');
            v.push_str(&base[i + 1..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Abhyasa vowel-grade twins for prefixed licensing (reduplicant grades).
///
/// Prefixed derivation may select the short/long reduplicant grade even
/// when the base cell holds the other (`IyAt` → `iyAt`, evidence `02.0040`
/// `ting.pashirling` `aBisamiyAt`-type alongside keeping `IyAt` under
/// `ati`; `yang` `tar-`/`tAr-`/`tIr-` rotations are the same family).
/// Returns the stem plus initial `I↔i` / `U↔u` twins. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn abhyasa_grade_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let mut chars = base.chars();
    if let Some(c0) = chars.next() {
        let twin = match c0 {
            'I' => Some('i'),
            'i' => Some('I'),
            'U' => Some('u'),
            'u' => Some('U'),
            _ => None,
        };
        if let Some(t) = twin {
            let mut v = String::with_capacity(base.len());
            v.push(t);
            v.push_str(&base[c0.len_utf8()..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
    }
    out
}

/// Internal `zW` de-voicing twins for prefixed licensing (stem allomorphy).
///
/// Some prefixed derivations select the unvoiced `sT` cluster even when
/// the base cell voices (`atizWAsizyata` → `AtisTAsizyata`, evidence
/// `01.1077` `san` across `AN`-stacks; deeper sibilants re-derive through
/// the cascade, so only the cluster twin is needed). Returns the stem
/// plus fully-unvoiced twins for every mixed/voiced `s`/`z` + `T`/`t`/
/// `W`/`w` cluster (`zW`→`sT`, `zw`→`st`, `sW`→`sT`, `zT`→`sT`, dental
/// mirrors). Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn st_cluster_unvoiced_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let c0 = b[i];
        let c1 = b[i + 1];
        let is_cluster =
            (c0 == b'z' || c0 == b's') && (c1 == b'W' || c1 == b'w' || c1 == b'T' || c1 == b't');
        // Fully-unvoiced clusters (`sT`, `st`) pass through; everything
        // else voices down to `s` + dental/retroflex (`zW`→`sT`, `zw`→`st`,
        // `sW`→`sT`, `zT`→`sT`, and dental mirrors).
        let already_plain = (c0 == b's') && (c1 == b'T' || c1 == b't');
        if is_cluster && !already_plain {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push('s');
            v.push(if c1 == b'W' || c1 == b'T' { 'T' } else { 't' });
            v.push_str(&base[i + 2..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// Internal `sT` voicing twins for prefixed licensing (stem allomorphy).
///
/// Intensive (`yang`/`yangluk`) stems may voice internal `sT` to `zW`
/// under prefixation (`tAsTAYcakAra` → `pariRitAzWAmbaBUva`, evidence
/// `01.1077` `yangluk.plit` across stacks; `nich` keeps `sT`, evidence
/// `AsTApayizIzwa`-type, so the voiced variant is additional, never a
/// rewrite). Returns the stem plus, for each internal `sT`, the
/// `sT`→`zW` twin (and `st`→`zw` for the dental pair). Only ADDS
/// licensings (membership-tested, never rewriting).
#[must_use]
pub fn st_internal_variants(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    let b = base.as_bytes();
    let mut i = 0;
    while i + 1 < b.len() {
        let c0 = b[i];
        let c1 = b[i + 1];
        let is_cluster =
            (c0 == b's' || c0 == b'z') && (c1 == b'T' || c1 == b't' || c1 == b'W' || c1 == b'w');
        // Fully-voiced clusters (`zW`, `zw`) pass through; everything else
        // voices up (`sT`→`zW`, `st`→`zw`, `sW`→`zW`, `zT`→`zW`, dental
        // mirrors).
        let already_voiced = (c0 == b'z') && (c1 == b'W' || c1 == b'w');
        if is_cluster && !already_voiced {
            let mut v = String::with_capacity(base.len());
            v.push_str(&base[..i]);
            v.push('z');
            v.push(if c1 == b'T' || c1 == b'W' { 'W' } else { 'w' });
            v.push_str(&base[i + 2..]);
            if !out.contains(&v) {
                out.push(v);
            }
        }
        i += 1;
    }
    out
}

/// `-Ati`/`-Ayti` alternation for prefixed derivation.
///
/// Some `-ti` formations surface with `y` after long `A` under prefixation
/// (`Ati` → `Ayti`, evidence `01.0546` `nich_krut.ktin` `Ayti`-type for
/// `Ati` across `AN`/`aBi;ava`-type stacks; bare keeps `Ati`). Returns the
/// stem plus the `Ati`→`Ayti` / `AtiH`→`AytiH` twin. Only ADDS licensings
/// (membership-tested, never rewriting).
#[must_use]
pub fn ayti_variants(base: &str) -> Vec<String> {
    if let Some(head) = base.strip_suffix("AtiH") {
        vec![base.to_string(), format!("{head}AytiH")]
    } else if let Some(head) = base.strip_suffix("Ati") {
        vec![base.to_string(), format!("{head}Ayti")]
    } else {
        vec![base.to_string()]
    }
}

/// `-I`/`-A` feminine alternation for counterpart-paired cells.
///
/// `ac`-family feminines license both `-I` and `-A` surfaces under
/// prefixation (evidence `01.1124` `krut.ac.f` `AtarA`-type for `tarI`
/// across all stacks; no `AtarI` in oracle). Returns the base
/// alternant(s) to join. Only ADDS licensings (membership-tested, never
/// rewriting).
#[must_use]
pub fn a_feminine_variants(base: &str) -> Vec<String> {
    if let Some(head) = base.strip_suffix('I') {
        vec![base.to_string(), format!("{head}A")]
    } else if let Some(head) = base.strip_suffix('A') {
        vec![base.to_string(), format!("{head}I")]
    } else {
        vec![base.to_string()]
    }
}

/// Counterpart krt for derivational pairing, or `None` for direct joins.
///
/// - `sya-*` futures substitute the corresponding present stem
///   (`sya-Satf -> Satf`, evidence: `vi+Bavizyat -> viBavat` across stacks).
/// - `BAvakarma-SAnac` substitutes the `SAnac` stem
///   (`BAvyamAna -> BAvayamAna`, evidence across stacks; Natva then applies
///   through [`generate_prefixed`]).
///   Rules compose (`sya-BAvakarma-SAnac -> SAnac`).
#[must_use]
pub fn counterpart_krt(krt: &str) -> Option<String> {
    let mut k = krt;
    if let Some(rest) = k.strip_prefix("sya-") {
        k = rest;
    }
    if k == "BAvakarma-SAnac" {
        k = "SAnac";
    }
    if k == krt {
        None
    } else {
        Some(k.to_string())
    }
}
