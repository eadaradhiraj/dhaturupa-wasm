//! Forward prefix sandhi (F3): `stack + base form -> surface forms`.
//!
//! Runtime rule-based generation. No precompute index, no lookup tables:
//! every output is computed by applying ordered sandhi rules to the inputs.
//! Rules were scoped from the `upasarga_forms` oracle (boundary behaviour of
//! all prefix tokens) and each rule below cites its oracle evidence.
//!
//! Conventions: all strings SLP1, byte-exact. `join` is total: one input
//! pair -> one or two surface strings. Two outputs arise only at principled
//! optionality points (Pāṇinian `vā`-rules with oracle-attested doublets):
//! `sam` + stop/nasal (`saMpra`/`sampra`), `nir` + sibilant/stop
//! (`nissat`/`niHsat`), `ud` + `S` (`ucCvasiti`/`ucSvasiti`). Word-level
//! post-rules ([`satva`], [`natva`]) stay deterministic.

/// SLP1 high vowels triggering `sT -> zW` (evidence: `aDi+sTAsIzwa`).
const HIGH_VOWELS: &[u8] = b"iIuU";

/// Returns true for SLP1 vowel bytes.
#[must_use]
pub const fn is_vowel_byte(b: u8) -> bool {
    matches!(
        b,
        b'a' | b'A'
            | b'i'
            | b'I'
            | b'u'
            | b'U'
            | b'f'
            | b'F'
            | b'x'
            | b'X'
            | b'e'
            | b'E'
            | b'o'
            | b'O'
    )
}

/// Returns true for voiced sounds (vowels + voiced consonants + nasals etc.).
/// The voiceless set is `{k,K,c,C,w,W,t,T,p,P,s,S,z,Z}`; `M`/`H` pattern as
/// voiced here.
#[must_use]
pub const fn is_voiced_byte(b: u8) -> bool {
    !matches!(
        b,
        b'k' | b'K'
            | b'c'
            | b'C'
            | b'w'
            | b'W'
            | b't'
            | b'T'
            | b'p'
            | b'P'
            | b's'
            | b'S'
            | b'z'
            | b'Z'
    )
}

/// Forward join of `left + right`: one or two surface strings.
///
/// Either side may be empty (pure concatenation). Otherwise exactly one
/// boundary rule fires, in order:
/// 1. `sT -> zW` after high vowels (evidence `aDizWAsIzwa`).
/// 2. Vowel + vowel sandhi (guna/vrddhi/yan/ayadi/dirgha).
/// 3. `m`-final (`sam`): `m` before vowels; `M` before `y/r/l/v`,
///    sibilants and `h` (evidence `saMyatizIzwa`, `saMrakzyAt`); two
///    variants (`M` + parasavarṇa) before stops/nasals (evidence
///    `saMcaNKanti`/`saYcaNKanti`, `saMpra`/`sampra`).
/// 4. `d`-final (`ud`) assimilation/stutva; `d + S` gives two variants
///    (`ucCvasiti`/`ucSvasiti`).
/// 5. `s`-final (`nis`/`dus`): `r` before vowels (evidence `duracArayata`,
///    `viniracArayata`); visarga-class mapping before consonants.
/// 6. `r`-final (`nir`, `dur`): `nir + rV -> nIrrV` (kept `r`, evidence
///    `nIrocayizIzwa`); `n` passes through (word-level [`natva`] decides
///    with full context); `z`/`H` variants before `k/K/p/P`
///    (evidence `nizkfn`/`niHkfn`-type doublets); `S` before `c/C`,
///    `s` before `t/T/s/S`; plain concatenation otherwise.
/// 7. Tuk: vowel + `C` (`ch`) inserts `c` (evidence `vicCedayizIzwa`,
///    `AcCedayizIzwa` — after short and long vowels alike; never before
///    `c`, evidence `AcikrandizyAt`).
/// 8. Fallback: plain concatenation.
#[must_use]
pub fn join(left: &str, right: &str) -> Vec<String> {
    if left.is_empty() {
        return vec![right.to_string()];
    }
    if right.is_empty() {
        return vec![left.to_string()];
    }
    let lb = left.as_bytes();
    let rb = right.as_bytes();
    let lf = lb[lb.len() - 1];
    let ri = rb[0];

    // Opaque tokens: `-` is not SLP1 (sole case: the `aBi-AN` typo-stack in
    // `02.0040.json`, whose oracle forms retain `-` verbatim). Phonetic rules
    // cannot apply; concatenate.
    if left.contains('-') || right.contains('-') {
        let mut out = String::with_capacity(left.len() + right.len());
        out.push_str(left);
        out.push_str(right);
        return vec![out];
    }

    // `N` (= ṅ, solely the āṅ prefix `AN`): lopa at the boundary, then the
    // exposed sound re-joins normally (`AN+aBavat` -> `A+Bavat` -> `ABavat`).
    if lf == b'N' {
        // āṅ + vowel-initial `cam` takes vrddhi (`camat` → `AcAmat`,
        // evidence `01.0540` `pari;AN` plang/plat 30+ rows with zero
        // `camat`-wants under AN stacks; `ni` keeps `nyacamat`). The long
        // `A` from the lopa'd `N`-site fuses with the lengthened vowel,
        // so `AN` + `camat` → `A` + `cAmat` = `AcAmat`. Both licensed
        // (plain concat stays), membership-tested.
        if let Some(rest) = right.strip_prefix("cam") {
            let base = join(&left[..left.len() - 1], right);
            let mut out = base;
            if !rest.is_empty() {
                let v = format!("cAm{rest}");
                let a_form = join(&left[..left.len() - 1], &v);
                for s in a_form {
                    if !out.contains(&s) {
                        out.push(s);
                    }
                }
            }
            return out;
        }
        // Reduplicated `caMcama` → `caMcAma` (evidence `01.0540`
        // `yangluk_krut.GaY` `paryAcaMcAma`-type) and `caYcama` →
        // `caYcAma` (evidence `paryAcaYcAma`-type); keeps ride the
        // base path, membership-tested).
        for (from, to) in [("caMcama", "caMcAma"), ("caYcama", "caYcAma")] {
            if let Some(crest) = right.strip_prefix(from) {
                let mut out = join(&left[..left.len() - 1], right);
                let w = format!("{to}{crest}");
                for s in join(&left[..left.len() - 1], &w) {
                    if !out.contains(&s) {
                        out.push(s);
                    }
                }
                return out;
            }
        }
        return join(&left[..left.len() - 1], right);
    }

    // Rule 1: sT -> zW after high vowels (evidence `aDizWAsIzwa`,
    // `vizWApayizIzwa`-type voices; `A` keeps, evidence `AsTApayizIzwa`).
    // (`a` keeps, evidence `upasTApayati`-type). Retroflex `sT` keeps
    // alongside (`aBisTaH`-type for `aBi`+`sTaH`, evidence `02.0060`
    // `ting.plat`; voice stays canonical `[0]`, membership-tested).
    if HIGH_VOWELS.contains(&lf) && rb.len() >= 2 && rb[0] == b's' && rb[1] == b'T' {
        let mut out = String::with_capacity(left.len() + right.len());
        out.push_str(left);
        out.push('z');
        out.push('W');
        out.push_str(&right[2..]);
        let mut plain = String::with_capacity(left.len() + right.len());
        plain.push_str(left);
        plain.push_str(right);
        return vec![out, plain];
    }

    // Rule 2: vowel + vowel (grades may multiply at a+e / a+o contacts).
    if is_vowel_byte(lf) && is_vowel_byte(ri) {
        return join_vowels(left, lf, right, ri);
    }

    // Rule 7 (tuk) is vowel-triggered; consonant finals fall through to the
    // branches below. A vowel-final `left` before `C` inserts `c`.
    if is_vowel_byte(lf) {
        if ri == b'C' {
            let mut out = String::with_capacity(left.len() + right.len() + 1);
            out.push_str(left);
            out.push('c');
            out.push_str(right);
            return vec![out];
        }
        // `i`/`I` + `r` licenses the `y`-glide alongside plain concat
        // (evidence `aByrAmavAmbaBUva`-type glides vs keeps; both exist,
        // membership-tested; other vowels keep concat).
        if (lf == b'i' || lf == b'I') && ri == b'r' {
            let mut glide = String::with_capacity(left.len() + right.len() + 1);
            glide.push_str(&left[..left.len() - 1]);
            glide.push('y');
            glide.push_str(right);
            let mut plain = String::with_capacity(left.len() + right.len());
            plain.push_str(left);
            plain.push_str(right);
            return vec![plain, glide];
        }
        // `A` + `vu` licenses the shortened `avu` alongside plain concat
        // (evidence `paravupe`-type for `parA`+`vupe`, 104-0 unanimous;
        // other `A`+consonant contacts keep `A`).
        if lf == b'A' && right.starts_with("vu") {
            let mut short = String::with_capacity(left.len() + right.len());
            short.push_str(&left[..left.len() - 1]);
            short.push('a');
            short.push_str(right);
            let mut plain = String::with_capacity(left.len() + right.len());
            plain.push_str(left);
            plain.push_str(right);
            return vec![plain, short];
        }
        let mut out = String::with_capacity(left.len() + right.len());
        out.push_str(left);
        out.push_str(right);
        return vec![out];
    }

    // Consonant-final boundaries.
    if lf == b'm' {
        // Rule 3: sam-final.
        if is_vowel_byte(ri) {
            return vec![concat(left, right)];
        }
        // `sam` + `y`-initial drops the `y` alongside the `M`-form
        // (evidence `01.1031` `samata` for `sam`+`yata` (never `saMyata`),
        // `aBisamata`, `upasamata`; other finals keep `y`: `niyata`,
        // `viyata`, `pariyata`). Both licensed, membership-tested.
        let m_form = replace_final(left, b'M');
        if ri == b'y' {
            return vec![concat(&m_form, right), concat(left, &right[1..])];
        }
        if is_stop_or_nasal(ri) {
            let mut second = String::with_capacity(left.len() + right.len());
            second.push_str(&left[..left.len() - 1]);
            second.push(parasavarna(ri) as char);
            second.push_str(right);
            return vec![concat(&m_form, right), second];
        }
        return vec![concat(&m_form, right)];
    }
    if lf == b'd' {
        // Rule 4: ud-final.
        if is_vowel_byte(ri) {
            return vec![concat(left, right)];
        }
        // `ud` + `y`-initial drops the `y` alongside the plain form
        // (evidence `01.1031` `udata`/`aByudata`/`prodata`/`samudata` for
        // `ud`+`yata`; other finals keep `y`). Both licensed,
        // membership-tested.
        if ri == b'y' {
            return vec![
                concat(&replace_final(left, d_final_map(ri)), right),
                concat(left, &right[1..]),
            ];
        }
        // `ud + hV`: `hA` drops after `ud` (variant with keeping;
        // evidence `aByudDArayizIzwa` alongside `aByudhArayizIzwa`),
        // other vowels take `udh` keeping (evidence `01.0806`
        // `udharzayAmbaBUva`-type) alongside the `h`-dropping `dD` twin
        // (evidence `udDarzayAmbaBUva`-type doublets, 28+28 in `ud`
        // `nich.plit`; all licensed, membership-tested; the older `dDh`
        // surface is retained conservatively).
        if ri == b'h' {
            let stem = &left[..left.len() - 1];
            let rest = &right[1..];
            if rest.starts_with('A') {
                return vec![format!("{stem}dD{rest}"), format!("{stem}dh{rest}")];
            }
            return vec![
                format!("{stem}dDh{rest}"),
                format!("{stem}dh{rest}"),
                format!("{stem}dD{rest}"),
            ];
        }
        // `ud + sT` (retroflex) voices the cluster (`aByutTApayizIzwa`,
        // `aByutTTApayizIzwa`).
        if let Some(rest) = right.strip_prefix("sT") {
            let stem = &left[..left.len() - 1];
            return vec![format!("{stem}tT{rest}"), format!("{stem}tTT{rest}")];
        }
        // `ud` + `stV` (dental): `s` drops with `t` doubling
        // (`uttamBayizIzwa`) alongside the `T`-assimilated twin
        // (`utTtamBayizIzwa`; evidence `09.0007` `ud` pairs, 10+10 per cell;
        // zero `utstam`-wants exist, so the `tst` single is not licensed).
        // Tail keeps `right[1..]` (the dental survives; only `s` drops).
        if right.starts_with("st") && right.as_bytes().get(2).is_some_and(|c| is_vowel_byte(*c)) {
            let stem = &left[..left.len() - 1];
            return vec![
                format!("{stem}t{}", &right[1..]),
                format!("{stem}tT{}", &right[1..]),
            ];
        }
        if ri == b'S' {
            let stem = &left[..left.len() - 1];
            let rest = &right[1..];
            return vec![format!("{stem}cC{rest}"), format!("{stem}cS{rest}")];
        }
        return vec![concat(&replace_final(left, d_final_map(ri)), right)];
    }
    if lf == b's' {
        // Rule 5: s-final.
        if is_vowel_byte(ri) {
            return vec![concat(&replace_final(left, b'r'), right)];
        }
        // `{r, z}` doublets before voiced stops/nasals (evidence
        // `durgA`-type `r` vs `duzg`-type `z` oracle pairs).
        if is_voiced_stop_nasal(ri) {
            return vec![
                concat(&replace_final(left, b'r'), right),
                concat(&replace_final(left, b'z'), right),
            ];
        }
        let mapped = concat(&replace_final(left, s_final_map(ri)), right);
        // Optionality before `s` (evidence `nissyantsIzwa`/`niHsyantsIzwa`,
        // `dussTAsIzwa`/`duHsTAsIzwa` doublets).
        if ri == b's' {
            let stem = &left[..left.len() - 1];
            let mut h = String::with_capacity(left.len() + right.len());
            h.push_str(stem);
            h.push('H');
            h.push_str(right);
            return vec![mapped, h];
        }
        return vec![mapped];
    }
    if lf == b'r' {
        // Rule 6: r-final (vowel-initial: plain concat), except `r + Ay`
        // which voices to `l` (evidence `apanilAyayizIzwa` vs `nirAha`
        // keeping `r` before `Ah`), and `nir` + r-initial which lengthens
        // with r-lopa, classical `ro ri` (evidence `nIrocayizIzwa`-type,
        // 3181 unanimous pairs, zero keeps).
        if left.ends_with("nir") && ri == b'r' {
            let stem = &left[..left.len() - 2];
            return vec![format!("{stem}Ir{}", &right[1..])];
        }
        if is_vowel_byte(ri) {
            // `r` + `Ay` licenses both keep and `l` (evidence `nirAyaMyamitA`
            // -type keeps alongside `nilAyayAmbaBUva`-type `l`, 76+ vs 60+
            // oracle forms; both licensed, membership-tested).
            if right.starts_with("Ay") {
                let stem = &left[..left.len() - 1];
                return vec![concat(left, right), format!("{stem}l{right}")];
            }
            return vec![concat(left, right)];
        }
        return join_r_final(left, right, ri);
    }
    vec![concat(left, right)]
}

/// Plain concatenation.
fn concat(left: &str, right: &str) -> String {
    let mut out = String::with_capacity(left.len() + right.len());
    out.push_str(left);
    out.push_str(right);
    out
}

/// True for stops and nasals (parasavarṇa triggers after `sam`).
const fn is_stop_or_nasal(b: u8) -> bool {
    matches!(
        b,
        b'k' | b'K'
            | b'g'
            | b'G'
            | b'N'
            | b'c'
            | b'C'
            | b'j'
            | b'J'
            | b'Y'
            | b'w'
            | b'W'
            | b'q'
            | b'Q'
            | b'R'
            | b't'
            | b'T'
            | b'd'
            | b'D'
            | b'n'
            | b'p'
            | b'P'
            | b'b'
            | b'B'
            | b'm'
    )
}

/// Homorganic nasal for parasavarṇa after `sam` (`mo'nusvāraḥ` + place
/// assimilation; evidence `saYcaNKanti`, `sannamayatAt`, `samprayAti`).
/// Non-stop input maps to `M` (caller separates those branches).
const fn parasavarna(b: u8) -> u8 {
    match b {
        b'k' | b'K' | b'g' | b'G' | b'N' => b'N',
        b'c' | b'C' | b'j' | b'J' | b'Y' => b'Y',
        b'w' | b'W' | b'q' | b'Q' | b'R' => b'R',
        b't' | b'T' | b'd' | b'D' | b'n' => b'n',
        b'p' | b'P' | b'b' | b'B' | b'm' => b'm',
        _ => b'M',
    }
}

/// Vowel + vowel sandhi, both boundary chars vowels.
fn join_vowels(left: &str, lf: u8, right: &str, ri: u8) -> Vec<String> {
    let stem = &left[..left.len() - 1];
    let tail = &right[1..];
    let mut out = String::with_capacity(left.len() + right.len());
    match lf {
        b'a' | b'A' => {
            // guna/vrddhi grade optionality at `a+e` / `a+o` contacts —
            // evidence `apejizIzwa` (a+e -> e) vs `apEjijizizIzwa`
            // (desiderative abhyasa-vrddhi a+e -> E), and `avoRayizIzwa`
            // (a+o -> o) alongside vrddhi `O` (387 oracle surfaces).
            // `A`-contacts fuse short (`A+i` -> e, evidence `ejyAt`;
            // `A+u` -> o (evidence `ovAya`) alongside vrddhi `O`
            // (evidence `01.0989` `OpaviraMramyata`-type; both licensed);
            // `A+U` takes `u` with an `o`
            // variant, evidence `avuzatuH` + single-dhatu `oyAt`).
            // Grades licensed where attested (membership-tested
            // downstream); `A+e` stays vrddhi-only (886 unanimous pairs).
            let grades: &[&str] = match (lf, ri) {
                (_, b'a' | b'A') => &["A"],
                // `a`/`A` + `i`/`I` licenses guna-`e` alongside `ayi`
                // (evidence `upayIrtsa`-type for `upa`+`Irtsa` and
                // `ayijivas`-type for `AN`+`Ijivas` via `N`-lopa; both
                // licensed, membership-tested).
                (_, b'i' | b'I') => &["e", "ayi"],
                (b'a', b'e') => &["e", "E"],
                (b'A', b'e' | b'E') | (b'a', b'E') => &["E"],
                (b'a', b'u' | b'U') => &["o"],
                // `A`+`u` licenses vrddhi-`O` alongside guna-`o`
                // (evidence `01.0989` `A;upa;vi`
                // `OpaviraMramyata`-type voices vs `ovAya`-type keeps; both
                // licensed, membership-tested).
                (b'A', b'u') => &["o", "O"],
                (b'a', b'o') => &["O", "o"],
                (b'A', b'U') => &["u", "o"],
                (_, b'f' | b'F' | b'x' | b'X') => &["a"],
                _ => &["O"],
            };
            let mut outs = Vec::with_capacity(grades.len());
            for c in grades {
                let mut one = String::with_capacity(left.len() + right.len());
                one.push_str(stem);
                one.push_str(c);
                if ri == b'f' || ri == b'F' {
                    one.push('r');
                } else if ri == b'x' || ri == b'X' {
                    one.push('l');
                }
                one.push_str(tail);
                outs.push(one);
            }
            return outs;
        }
        b'i' | b'I' => {
            out.push_str(stem);
            if ri == b'i' || ri == b'I' {
                // Dirgha + yaN doublet (`i`+`i` → `I` alongside `yi`;
                // evidence `pratyiyyAt`-type yaN for `prati`+`iyyAt`
                // alongside `I`-dirgha forms; both licensed,
                // membership-tested).
                out.push('I');
                out.push_str(tail);
                let mut yan = String::with_capacity(left.len() + right.len() + 1);
                yan.push_str(stem);
                yan.push('y');
                yan.push_str(right);
                return vec![out, yan];
            }
            out.push('y');
            out.push_str(right);
            return vec![out];
        }
        b'u' | b'U' => {
            out.push_str(stem);
            if ri == b'u' || ri == b'U' {
                out.push('U');
            } else {
                out.push('v');
                out.push_str(right);
                return vec![out];
            }
            out.push_str(tail);
        }
        b'f' | b'F' => {
            out.push_str(stem);
            if ri == b'f' || ri == b'F' {
                out.push('F');
                out.push_str(tail);
            } else {
                out.push('r');
                out.push_str(right);
            }
            return vec![out];
        }
        b'x' | b'X' => {
            out.push_str(stem);
            out.push('l');
            out.push_str(right);
            return vec![out];
        }
        b'e' | b'E' | b'o' | b'O' => {
            // ayadi: e -> ay, o -> av, E -> Ay, O -> Av before vowels.
            out.push_str(stem);
            match lf {
                b'e' => out.push_str("ay"),
                b'E' => out.push_str("Ay"),
                b'o' => out.push_str("av"),
                _ => out.push_str("Av"),
            }
            out.push_str(right);
            return vec![out];
        }
        _ => {
            out.push_str(left);
            out.push_str(right);
            return vec![out];
        }
    }
    vec![out]
}

/// `ud`-final mapping before a consonant `ri`.
const fn d_final_map(ri: u8) -> u8 {
    match ri {
        b'm' | b'n' | b'N' | b'Y' | b'R' => b'n',
        b'l' => b'l',
        b'j' => b'j',
        b'c' | b'C' => b'c',
        b'q' | b'Q' => b'q',
        _ if is_voiced_byte(ri) => b'd',
        _ => b't',
    }
}

/// `s`-final (`nis`/`dus`) mapping before a consonant `ri`.
///
/// Voiceless stops voice to `z` (evidence `duzkArayizIzwa`,
/// `nizkarzayizIzwa`); `c/C` to `S` (evidence `duScaritra`-type);
/// `t/T/s/S` keep `s`; voiced sounds take `r` in [`join`] (`durgA`-type),
/// with a `z` variant for voiced stops/nasals (oracle doublets).
const fn s_final_map(ri: u8) -> u8 {
    match ri {
        b'k' | b'K' | b'p' | b'P' => b'z',
        b'c' | b'C' => b'S',
        b't' | b'T' | b's' | b'S' => b's',
        _ if is_voiced_byte(ri) => b'r',
        _ => b's',
    }
}

/// True for voiced stops/nasals (the `{r, z}` doublet context after
/// `s`-final prefixes). Dental `t`/`T` are voiceless and excluded: `dus`/
/// `nis` + `t`-initial stems keep `s` (evidence `dustArayizIzwa`-type,
/// 2,948 `dust`-wants and 58 `nist`-wants with zero `durt`/`duzt`/`nurt`
/// oracle forms; `duzw` 49 keeps the `w` doublet, `durb`/`durd`-type keep
/// theirs — only `t`/`T` move to [`s_final_map`], which maps them to `s`).
const fn is_voiced_stop_nasal(b: u8) -> bool {
    matches!(
        b,
        b'g' | b'G'
            | b'j'
            | b'J'
            | b'w'
            | b'W'
            | b'q'
            | b'Q'
            | b'R'
            | b'd'
            | b'D'
            | b'n'
            | b'b'
            | b'B'
            | b'm'
    )
}

/// `r`-final join (consonant-initial `right`).
///
/// `r + rV -> nIrrV` (kept `r`, evidence `nIrocayizIzwa`);
/// `n` passes through untouched — word-level [`natva`] voices it with full
/// left context (3rd-consonant block included). `z`/`H` doublets before
/// `k/K/p/P` (evidence `nizkfn`/`niHkfn`-type pairs), `z` before `w`/`W`
/// (evidence `nizwaNkay`-type, no keeps), `S` before `c/C`,
/// `s` before `t/T/s/S`; plain concatenation otherwise.
fn join_r_final(left: &str, right: &str, ri: u8) -> Vec<String> {
    let stem = &left[..left.len() - 1];
    match ri {
        b'r' => {
            vec![format!("{stem}Ir{}", &right[1..])]
        }
        b'k' | b'K' | b'p' | b'P' => {
            vec![format!("{stem}z{right}"), format!("{stem}H{right}")]
        }
        // `r` + `w`/`W` (retroflex stops) voices to `z` (evidence
        // `nizwaNkayizIzwa`-type, 80+ oracle forms with zero `nirw`-keeps).
        b'w' | b'W' => {
            vec![format!("{stem}z{right}")]
        }
        b'c' | b'C' => {
            vec![format!("{stem}S{right}")]
        }
        b't' | b'T' => {
            vec![format!("{stem}s{right}")]
        }
        b's' => {
            // Optionality (evidence `nissiDyAt`/`niHsiDyAt` doublets),
            // each alongside the `s`-voiced twins with `r`→`z`/`H`
            // (evidence `seh`-grades voice both: `nizzehatuH`/`niHzehatuH`
            // alongside `nissasAha`-type keeps).
            let mut voiced = String::with_capacity(right.len());
            voiced.push('z');
            voiced.push_str(&right[1..]);
            vec![
                format!("{stem}s{right}"),
                format!("{stem}H{right}"),
                format!("{stem}z{voiced}"),
                format!("{stem}H{voiced}"),
            ]
        }
        b'S' => {
            // Optionality (evidence `niSSasyAt`/`niHSasyAt` doublets).
            vec![format!("{stem}S{right}"), format!("{stem}H{right}")]
        }
        _ => {
            vec![concat(left, right)]
        }
    }
}

/// Replace the final byte of `s` with `c` (both ASCII).
fn replace_final(s: &str, c: u8) -> String {
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..s.len() - 1]);
    out.push(c as char);
    out
}

/// `iN`-vowel triggers for [`satva`] (classical `iN` vowels; `a`/`A`
/// excluded — evidence `anusam`, `upasTAp` keep `s`).
const SATVA_PREV: &[u8] = b"iIuUfFxXeEoOz";

/// Base-third consonants that block [`satva`] voicing (evidence
/// `nisasnO`, `nirnanna`-type keeps vs `nizeDati`-type voices; voiced
/// `sasAda`-type exceptions are `(stack, dhatu)`-lexical, tracked as
/// residuals).
const SATVA_BLOCK_THIRD: &[u8] = b"sSzZMrcCR";

/// Surface penultimate `r` blocks [`satva`] before retroflex/liquid/labial
/// thirds (dissimilation; evidence `pariseDayate`-type keeps vs
/// `pratizeDayate`-type voices with the same base).
const SATVA_PENULT_R_BLOCK: &[u8] = b"DvrRwWqQ";

/// True for high-vowel surface finals that license [`satva`] (evidence:
/// `ni`/`vi`/`su`/`anu`-type surfaces voice, `pra`/`apa`/`AN`/`sam`/`ud`
/// never do).
const fn satva_surface_ok(final_byte: u8) -> bool {
    matches!(final_byte, b'i' | b'I' | b'u' | b'U')
}

/// Apply Paninian Natva (`n -> R`) left-to-right over a joined form.
///
/// Classical scope (`razaByAM no RaH samAnapade` + `aw-kupvAN-num`
/// interveners): an `n` voices iff (a) it is not word-final, (b) it is
/// followed by a vowel/`y`/`v`, (c) an `r`/`z` stands to its left with
/// only vowels, velars, labials, `y/v/r/l/h`, anusvara/`M`, or `R`/`m`
/// between (dentals, palatals, cerebrals, sibilants block — evidence
/// `pratiBAvayamAna` keeping `n` via `t`), (d) the *next-next* char is
/// not `n`/`r` (evidence `praninadizyAt`, `parinAnakzizIzwa` keep `n` while
/// `praROti`, `paryAnAyayizIzwa` voice), and (e) past the base-first char,
/// the next-next char is not a semivowel `y`/`v` (evidence `pramAnayizIzwa`,
/// `parivanIvacizIzwa` keep `n` while base-first `praNayati`-type voices),
/// nor palatal `Y` (evidence `parivaMvanAYcakre`-type keeps), nor sibilants
/// `s`/`S`/`z`/`Z` (evidence `parivaMvanizIzwa`-type keeps across 23k
/// aligned pairs; 37 `han`-aorist `-iz-` voices handled separately), nor
/// non-final `m` (evidence `parivaMvanAmbaBUva`-type keeps; word-final
/// `-am` voices, evidence `aBipraBavaRam`-type). Zone rule: base-first
/// `n` uses the short next-next list (`n`/`r`); every other position
/// (stack-internal and base-internal) uses the full list — evidence
/// `parinivartayAmbaBUva`-type keeps the prefix `n`.
///
/// `pre_len` is the stack-surface length: rule (e) applies only past it.
///
/// Known characterization gap (logged, not fitted): reduplicated perfect
/// stems (`prananAda`) and a few abhyasa formations keep `n` where the
/// surface rule voices. These are morphological, listed as M3 residuals.
///
/// `niya_keep` restores the internal `y`-blocker before long `I` for roots
/// that never voice `n` before `-Iy-` (see [`crate::engine::upasarga::GenCtx`]).
/// `intensive_keep` keeps base-internal `n`+`I`+`p` for intensive categories
/// (see [`crate::engine::upasarga::GenCtx`]); `mani_keep` keeps base-internal `n`
/// before word-final `i` for `mana~`/`manu~`/`mAna~`, and `nit_keep` keeps
/// base-internal `n` before `i`+`t` for `mana~`/`manu~`/`mAna~`/`vana~`
/// (see [`GenCtx`]); `fusion_keep` keeps stack-region `n` before
/// fusion-created `y`/`v` (caller-computed: stack surface ends in `ni`/`nu`
/// fused with a vowel-initial base; base-initial `y`/`v` still voices);
/// `surface_triggered` tells whether the trigger came from the stack
/// surface itself (next-next `y` blocks only then; `satva`-created triggers
/// bypass it — see above).
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
pub fn natva(
    s: &str,
    pre_len: usize,
    niya_keep: bool,
    intensive_keep: bool,
    mani_keep: bool,
    nit_keep: bool,
    fusion_keep: bool,
    surface_triggered: bool,
    geminate_keep: bool,
) -> String {
    const FOLLOW: &[u8] = b"aAiIuUeEoOfFxXyv";
    const BLOCK_NEXT: &[u8] = b"nr";
    // Next-next `N` (retroflex nasal) blocks too (evidence `04.0091`
    // `niranaNkzyat`-type keeps vs `niranaSizyat`-type voices; zero
    // `RaN`-voice oracle forms exist).
    const BLOCK_NEXT_INTERNAL: &[u8] = b"nryvYsSzZN";
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        // Geminate `nn` after a trigger voices as a unit to `RR` on
        // high-vowel surfaces (evidence `AnizaRRa`), but only to `Mn`
        // (first nasal) otherwise (evidence `nirnaMnamizIzwa`; vs
        // separated `nVn` (`praninadizyAt`) which keeps via the next-next
        // block below). Only base-internal geminates voice: a geminate
        // straddling the stack-base boundary (first `n` from an assimilated
        // stack-final, evidence `pronnadya` for `pra;ud`+`nadya`) keeps.
        if b[i] == b'n'
            && i + 1 < b.len()
            && b[i + 1] == b'n'
            && i >= pre_len
            && triggered_left(b, i)
        {
            // Lexical keep: `pada~`/`Ruda~` geminates never voice
            // (evidence `aBinizpanna`/`nirRunna`-type; `Rama~` voices).
            if geminate_keep {
                out.push(b'n');
                out.push(b'n');
                i += 2;
                continue;
            }
            let high_surface = pre_len > 0
                && pre_len <= b.len()
                && matches!(b[pre_len - 1], b'i' | b'I' | b'u' | b'U');
            if high_surface {
                out.push(b'R');
                out.push(b'R');
            } else {
                out.push(b'M');
                out.push(b'n');
            }
            i += 2;
            continue;
        }
        if b[i] == b'n' && i + 1 < b.len() && FOLLOW.contains(&b[i + 1]) {
            // Internal `y` blocks except after long `I` (gerundive `-Iya-`
            // voices, evidence `nirBavaRIya`-type, while `pramAnayizIzwa`
            // (next `a`) and `parivanIvacizIzwa` (next-next `v`) keep;
            // never-voice roots opt out via `niya_keep`) and except
            // post-augment (evidence `paryAnAyayata`-type voices, while
            // `-mi` `anAmyata`-type keeps via `m` below). Internal `m`
            // blocks except word-finally (neuter `-am` voices, evidence
            // `aBipraBavaRam`-type, while `parivaMvanAmbaBUva`, 1sg `-mi`
            // `prakaromi`-type keep). The internal list also governs
            // stack-internal positions (evidence `parinivartayAmbaBUva`
            // keeps the prefix `n`); only base-first `n` uses the short
            // list (evidence base-first `praNayati`-type voices).
            let internal = i != pre_len;
            let augmented = i == pre_len + 1 && b.get(pre_len) == Some(&b'a');
            // Intensive keeps (see `intensive_keep` docs): base-internal `n` +
            // `I` + `p`, or base-first `n` + `a` + `M`. Mani
            // `-ni` keep (see `mani_keep` docs): base-internal `n` before
            // word-final `i` keeps even when triggered.
            let intensive_block = intensive_keep
                && ((i > pre_len && b.get(i + 1) == Some(&b'I') && b.get(i + 2) == Some(&b'p'))
                    || (i == pre_len
                        && b.get(i + 1) == Some(&b'a')
                        && b.get(i + 2) == Some(&b'M')));
            let mani_block =
                mani_keep && i > pre_len && b.get(i + 1) == Some(&b'i') && i + 2 >= b.len();
            // `-mAna` keep: `-mAna` + word-final `-a`/`-aH`/neuter `-am`
            // keeps under surface-sourced triggers *when a classical
            // blocker stands between the trigger and the `n`* (evidence
            // `parilambamAna`-type keeps via `l`, `prabADamAna` via `D`;
            // triggerless voices ride [`mana_voice_variants`]). The keep is
            // gated on [`triggered_left_strict`]: with only aṭ/kup
            // interveners the nous voices (`parABavamARa`, `prakampamARa`,
            // `nirIkzamARa`, `pracaramARa`). Short-`a` `-anam` needs lexical
            // scoping (`BU`/`hana~` voice) — separate M4 item).
            let participle_block = surface_triggered
                && b.get(i.saturating_sub(2)) == Some(&b'm')
                && b.get(i.saturating_sub(1)) == Some(&b'A')
                && b.get(i + 1) == Some(&b'a')
                && (i + 2 >= b.len()
                    || (i + 3 >= b.len() && matches!(b.get(i + 2), Some(b'H' | b'm'))))
                && !triggered_left_strict(b, i);
            // Stack-internal `nit` keep: stack-region `n` + `i` + `t`
            // keeps even when triggered (evidence `01.1077` `pari;ni`
            // `parinitizWAsizIzwa`-type; zero `niRit`-voice oracle forms
            // exist — parallels base-internal `nit_keep` without the flag).
            let stack_nit_block =
                i < pre_len && b.get(i + 1) == Some(&b'i') && b.get(i + 2) == Some(&b't');
            // `anu` keep: stack-internal `n` before `u`/`v` (evidence `duranu`-,
            // `paryanu`-, `duranvagam`-type keeps across stacks; the `u` may
            // surface as `v` before vowels. `y`-extension tried+reverted
            // (4,470 row regressions, all `R`+`y` voices) — fusion-`y` needs
            // pre-fusion M4 morphology.
            let anu_block = i < pre_len && matches!(b.get(i + 1), Some(b'u' | b'v'));
            // `nit`/`ny` keep (see `nit_keep` docs): base-internal `n` + `i` +
            // `t`, or base-internal `n` + `y`.
            let nit_block = nit_keep
                && i > pre_len
                && ((b.get(i + 1) == Some(&b'i') && b.get(i + 2) == Some(&b't'))
                    || b.get(i + 1) == Some(&b'y'));
            // Fusion-`y`/`v` keep (see `fusion_keep` docs): stack-region `n`
            // before fusion-created `y`/`v`. Position-scoped to the stack
            // (`i < pre_len`); base `y`/`v` followers still voice.
            let fusion_block =
                fusion_keep && i < pre_len && matches!(b.get(i + 1), Some(b'y' | b'v'));
            // `nft` keep: `n` + vocalic-`f` + `t` keeps even when triggered
            // (evidence `04.0010` `anfti`/`narnfttAt`-type and base-first
            // `nftyAt`-type keeps; zero `Rft`-voice oracle forms exist — the
            // sole `Rft` string corpus-wide is `info` prose — so no flag
            // needed, and base-first is included). `d` included
            // (evidence `04.0010` `parinarnfdDi`-type; zero `Rfd`
            // voices corpus-wide).
            let vocalic_r_block =
                b.get(i + 1) == Some(&b'f') && matches!(b.get(i + 2), Some(b't' | b'd'));
            let block = if i + 2 < b.len() && internal {
                // Next-next `y` blocks only for surface-sourced triggers:
                // `satva`-created triggers bypass it (evidence `01.0961`
                // `vizvARayizIzwa`-type voices under triggerless `vi`
                // surfaces once `satva` voices, while surface-`r`
                // `parivanIvacizIzwa`-type keeps; the `I`-`y` and augmented
                // unblocks below are unaffected). `v` stays blocked except
                // at the augmented position (evidence `paryaRAvi`-type
                // voices for `pari`+`anAvi`, `paryaRay`-unanimous with zero
                // keeps; triggered gating still applies). `S` stays blocked
                // except at the augmented position (evidence `niraRaSyat`-
                // type voices for `nir`+`anaSyat`; keeps ride the
                // `aug_voice_config` doublet).
                (BLOCK_NEXT_INTERNAL.contains(&b[i + 2])
                    && !(b[i + 2] == b'y' && (b[i + 1] == b'I' && !niya_keep || augmented))
                    && (b[i + 2] != b'y' || surface_triggered)
                    && !(b[i + 2] == b'v' && augmented)
                    && !(b[i + 2] == b'S' && augmented)
                    && !(b[i + 2] == b'z' && augmented))
                    || (b[i + 2] == b'm' && i + 3 < b.len())
            } else {
                i + 2 < b.len() && BLOCK_NEXT.contains(&b[i + 2])
            };
            let ok = !block
                && !intensive_block
                && !mani_block
                && !participle_block
                && !stack_nit_block
                && !anu_block
                && !nit_block
                && !vocalic_r_block
                && !fusion_block
                && triggered_left(b, i);
            out.push(if ok { b'R' } else { b'n' });
        } else {
            out.push(b[i]);
        }
        i += 1;
    }
    // ASCII-only transformation: byte ops preserve UTF-8 boundaries.
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// Left-trigger scan for [`natva`]: `r`/`z` with only allowed interveners.
fn triggered_left(b: &[u8], i: usize) -> bool {
    const ALLOWED: &[u8] = b"aAiIuUeEoOfFxXkKgGNpPbBmyvrlhMR";
    let mut j = i;
    while j > 0 {
        j -= 1;
        if b[j] == b'r' || b[j] == b'z' {
            return true;
        }
        if !ALLOWED.contains(&b[j]) {
            return false;
        }
    }
    false
}

/// Classical 8.4.2 (`aṭkupvāṅnum`) intervener scan for the `-mAna` keep:
/// only aṭ (vowels + `h y v r`) and kup (velars/labials, incl. their
/// nasals) plus dental `n` license the voice. Unlike [`triggered_left`]
/// the consonant classes outside aṭ/kup block (`l`, `R`, `M`, dentals,
/// palatals, retroflex stops/sibilants) — evidence `-mAna`: `parilambamAna`
/// (`l`), `prabADamAna` (`D`), `pariGUrRamAna` (`R`) keep, while
/// `parABavamARa`, `prakampamARa`, `nirIkzamARa` (closer `z` trigger),
/// `pracaramARa` voice.
fn triggered_left_strict(b: &[u8], i: usize) -> bool {
    const ALLOWED: &[u8] = b"aAiIuUeEoOfFxXhyvrkKgGNpPbBmn";
    let mut j = i;
    while j > 0 {
        j -= 1;
        if b[j] == b'r' || b[j] == b'z' {
            return true;
        }
        if !ALLOWED.contains(&b[j]) {
            return false;
        }
    }
    false
}

/// Apply ṣatva (`s -> z`) over the base region of a joined prefixed form.
///
/// `acc` is the stack surface: `pre_len` (its byte length) locates the base
/// region, while the trigger, surface gate, and penult read off `acc`
/// pre-fusion — boundary yaN fusion (`ni`+`aseDat` → `nyaseDat`) rewrites
/// the joined string's boundary chars but not the morphological trigger.
/// Base-region `s` voices iff all of: (a) the dhatu is ṣ-initial (`z_gate`,
/// evidence: 20/22 voicing dhatus carry `z`-upadeSa — `zevf~`, `zaha~`,
/// `zadx~`… — while `s`-roots like `su`, `san` keep `s`); (b) the preceding
/// char is an `iN`-vowel or feeding `z`, where a base-initial short `a`
/// (laṅ/luṅ/lṛṅ augment) is transparent and evaluates against the
/// stack-final char (evidence `nyazeDat` for `ni`+`aseDat`; ṣ-root stems
/// are consonant-initial, so base-initial `a`+consonant is always the
/// augment); (c) the following char is a vowel, `v`/`y`, `k`, or `P`
/// (evidence `nizeDati`, `nizayUti`, `aBizvaYjayizIzwa`,
/// `parizkandayate`-, `vyazPArizyata`-type voices alongside keeps; `st/sp` keep,
/// evidence `nistyA`, `nispa`); (d) cascade: past the base-first char,
/// an `s` voices only if the nearest base-region sibilant to its left
/// voiced (reduplication copies the voiced head — evidence `niziziDizyAt`
/// vs `apasisiDizyAt`, `AsisiDizyAt`, `nissisiDizyAt`).
///
/// `st -> zw` for `stambh`-class roots is handled separately (base
/// pre-transform in [`crate::engine::upasarga`]), as is `sT -> zW` (Rule 1).
///
/// `aug_keep` restores the opaque augment for roots whose post-augment `s`
/// never voices (see [`crate::engine::upasarga::GenCtx`]).
pub fn satva(s: &str, acc: &str, z_gate: bool, aug_keep: bool) -> String {
    let b = s.as_bytes();
    let ab = acc.as_bytes();
    let pre_len = ab.len();
    if !z_gate || pre_len == 0 || pre_len > b.len() {
        return s.to_string();
    }
    // Trigger morphology off the stack surface (see doc above).
    let surface_final = ab[ab.len() - 1];
    // Surface gate: only high-vowel-final surfaces voice (see
    // [`satva_surface_ok`]), plus `r`-final surfaces (solely `nir`)
    // over an unfused base-initial augment (evidence `nirazeDayata`-type
    // for `nir`+`aseDayata` while `nisseDayate`-type unaugmented keeps;
    // keeps ride the doublet in `generate_prefixed`).
    let r_aug = surface_final == b'r' && b.get(pre_len) == Some(&b'a');
    if !satva_surface_ok(surface_final) && !r_aug {
        return s.to_string();
    }
    let penult_r = ab.len() >= 2 && ab[ab.len() - 2] == b'r';
    // Root-agreement bypass for the penult-`r` block: past the base-first
    // char, a base whose root already voices licenses voicing despite a
    // blocking third (evidence `pariziziDizAmbaBUva`-type voices vs
    // `pariseDayate`-type keeps with unvoiced roots). Augment-initial
    // bases are excluded (their reduplicants keep, evidence thousands of
    // `asisiDizyAt`-type keeps); head-`s` is covered by reduplicant
    // twins instead.
    let base_has_z = b.get(pre_len..).is_some_and(|t| t.contains(&b'z'));
    let aug_first = b.get(pre_len) == Some(&b'a');
    // Nearest base-region sibilant voiced-ness (`None` = none seen yet).
    // Pre-seed with base `z` states below as encountered. `voiced_any`
    // tracks in-pass `s`→`z` voicings cumulatively (for the broad cascade
    // below; `last_voiced` keeps nearest-semantics for chaining).
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut last_voiced: Option<bool> = None;
    let mut voiced_any = false;
    for (i, c) in b.iter().enumerate() {
        if *c == b'z' && i >= pre_len {
            last_voiced = Some(true);
            out.push(*c);
            continue;
        }
        if *c != b's' || i < pre_len || i == 0 {
            out.push(*c);
            continue;
        }
        // Base-initial `s` may voice: `zu`-root base `sotA`-shapes keep
        // (evidence `01.1091` `parisotA`), but `su`-roots voice base-initial
        // `s` under `i`/`u`-surfaces (evidence `05.0001` `aBizuRvAnam`,
        // `01.0961`/`06.0163` svan-voice families). Tried+reverted blanket
        // keep (+589 new rows); root-scoping deferred to M4.
        // Base-first `s` consults the stack-final trigger (identical to the
        // joined char when the boundary is intact; correct under yaN
        // fusion); a base-initial augment `a` is transparent (see doc),
        // except for `aug_keep` roots where it stays opaque. Fusion hides
        // the augment (`pari`+`aseDayata` → `paryaseDayata`,
        // `parA`+`aseDayata` → `parAseDayata`): a `y`/`v` trace after an
        // `i`/`I`/`u`/`U`-final surface, or an `A` trace after an
        // `A`-final surface, consults the stack-final too (evidence
        // `paryazeDayata`-type voices; unaugmented `y`/`v`-stems ride the
        // keep-doublet in `generate_prefixed`).
        let augmented = i == pre_len + 1 && b[pre_len] == b'a';
        let fused = (matches!(surface_final, b'i' | b'I' | b'u' | b'U')
            && matches!(b.get(pre_len), Some(b'y' | b'v')))
            || (surface_final == b'A' && b.get(pre_len) == Some(&b'A'));
        let transparent = (augmented || fused) && !aug_keep;
        let prev = if i == pre_len || transparent {
            surface_final
        } else {
            b[i - 1]
        };
        let next_ok = i + 1 < b.len()
            && (is_vowel_byte(b[i + 1])
                || b[i + 1] == b'v'
                || b[i + 1] == b'y'
                || b[i + 1] == b'k'
                || b[i + 1] == b'P');
        let chained = i == pre_len || transparent || last_voiced == Some(true);
        // Cascade scope: an in-pass-voiced `s` voices later `s` broadly
        // (evidence `niziziDizyAt`-type cascades), but a stem-owned `z`
        // voices a later `s` only before a vowel (evidence `vizizvaRizyAH`
        // alongside `zisvaNkz`-type keeps with consonant following).
        // Base-first and transparent positions are unaffected (broad).
        let cascade_broad = i == pre_len || transparent || voiced_any;
        // Third-`n` blocks except after `u` (evidence `aBizuRoti`,
        // `aBizuRute`, `aByazuRuta` for `su`-stems) and before geminate
        // `nn` (evidence `aBinizaRRa` for `sanna`, whose `nn` voices via
        // natva once `s` voices); elsewhere it keeps (evidence
        // `vyasinuta`, `nisinoti`, `parisanizIzwa`-type keeps).
        let third_n_blocked = i + 2 < b.len()
            && b[i + 2] == b'n'
            && b[i + 1] != b'u'
            && !(i + 3 < b.len() && b[i + 3] == b'n');
        let third_ok = i + 2 >= b.len()
            || (!SATVA_BLOCK_THIRD.contains(&b[i + 2]) && !third_n_blocked)
            // Third-`c` voices after `e`/`i` (evidence `pratizece`-type
            // voices unanimously) but keeps after `a`/`A` (evidence
            // `asac`-type keeps unanimously).
            || (b[i + 2] == b'c' && matches!(b[i + 1], b'e' | b'i'));
        // Reduplicant/root `-sats-` keep: past the augmented position, an `s`
        // followed by `a` + `t` keeps even when the head voiced (evidence
        // `01.0990`/`06.0163` `san.plit` `nizisats-`-type across
        // `ni`/`vi`/`pari`/`anu`; `ziDa~` `sisiDiz-` voices with next
        // `i`/third `D`, so scoped to `a`/`t` exactly). The augmented
        // position itself (`i == pre_len+1` with augment `a` at `pre_len`)
        // still voices (evidence `01.0990` `ting.plrung` `asatsyat→Anyazatsyat`
        // -type; the augment is transparent there, stack trigger applies).
        // (`-sAda-` `A`/`d` tried+reverted: voices `s-A-d` elsewhere, +2,018
        // with 49 new — needs M4 inventory, not a block.)
        let sats_keep =
            i > pre_len + 1 && b.get(i + 1) == Some(&b'a') && b.get(i + 2) == Some(&b't');
        // `s-A-h` keep with long-`I` downstream: an `s` followed by `A` + `h`
        // keeps when a long `I` (not itself voicing-bound) occurs later in
        // the base region (evidence `10.0341` `visAhayizIzwa`-type keeps
        // across benedictive `-Izwa`-/`-IyA`-grades; `vizAhayAmbaBUva`-type
        // (no downstream `I`) voices; zero other `zAhayI`-voice oracle forms
        // exist) — unless an `nI`/`RI` pair occurs downstream, which voices
        // together with the `s` (evidence `vizAhaRIyA`-type gerundive
        // `-nIya`-/`-RIyA` voices both; row audit caught it).
        // Scoped to `A`+`h` exactly (`sAyay`-type keeps ride the `s_voice`
        // doublet instead).
        let sah_keep = b.get(i + 1) == Some(&b'A')
            && b.get(i + 2) == Some(&b'h')
            && b.get(i + 3..).is_some_and(|tail| {
                tail.contains(&b'I') && !tail.windows(2).any(|w| w == b"RI" || w == b"nI")
            });
        let third_ok = third_ok && !sats_keep && !sah_keep;
        let penult_ok = !penult_r
            || i + 2 >= b.len()
            || !SATVA_PENULT_R_BLOCK.contains(&b[i + 2])
            || (i > pre_len && base_has_z && !aug_first);
        // A transparent augment forces voicing past the trigger and
        // penult-`r` checks (evidence `paryazeDayata` with penult `r` +
        // `D`-third, `nirazeDayata` with `r`-final surface; keeps ride the
        // doublet in `generate_prefixed`), and past third-`S`
        // (evidence `niraRaSyat`-type; other thirds still block).
        let aug_voice =
            transparent && next_ok && chained && (third_ok || b.get(i + 2) == Some(&b'S'));
        // `next_ok` narrows to vowels for stem-`z`-seeded cascade positions
        // (see `cascade_broad` above); base-first, transparent, and
        // in-pass-voiced positions keep the broad gate.
        let effective_next =
            next_ok && (cascade_broad || (i + 1 < b.len() && is_vowel_byte(b[i + 1])));
        if (SATVA_PREV.contains(&prev) && effective_next && chained && third_ok && penult_ok)
            || aug_voice
        {
            out.push(b'z');
            last_voiced = Some(true);
            voiced_any = true;
        } else {
            out.push(*c);
            last_voiced = Some(false);
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// True when a joined prefixed form carries a `satva`-transparent augment:
/// unfused base-initial `a` at the stack boundary, a yaN/`v`-fusion trace
/// after an `i`/`I`/`u`/`U`-final stack surface, or an `A`-fusion trace
/// after an `A`-final surface (evidence `ni`+`aseDat`, `paryazeDayata`,
/// `parAseDayata`-type). Used for the keep-doublet alongside
/// augment-voicing in [`crate::engine::upasarga::generate_prefixed`].
#[must_use]
pub fn aug_voice_config(acc: &str, joined: &str) -> bool {
    let ab = acc.as_bytes();
    let b = joined.as_bytes();
    let (Some(&surface_final), Some(&boundary)) = (ab.last(), b.get(ab.len())) else {
        return false;
    };
    boundary == b'a'
        || (matches!(surface_final, b'i' | b'I' | b'u' | b'U') && matches!(boundary, b'y' | b'v'))
        || (surface_final == b'A' && boundary == b'A')
}

/// True when a joined prefixed form has an `s`+`k`/`P` cluster at or after
/// the stack boundary (evidence `parizkandayate` + `pariskandayate`,
/// `vyazPArizyata` + `vyasPArizyata` doublets).
/// Used for the keep-doublet alongside `s`+`k`/`P` voicing in
/// [`crate::engine::upasarga::generate_prefixed`].
#[must_use]
pub fn sk_voice_config(acc: &str, joined: &str) -> bool {
    let ab = acc.as_bytes();
    let b = joined.as_bytes();
    if ab.len() > b.len() {
        return false;
    }
    b[ab.len()..].windows(2).any(|w| w == b"sk" || w == b"sP")
}

/// True when a joined prefixed form has an `s`+`U`/`I`/`A` cluster at or
/// after the stack boundary (evidence `parisUyate`-, `parisAyate`-type
/// keeps alongside voices). Used for the keep-doublet alongside voicing
/// in [`crate::engine::upasarga::generate_prefixed`].
#[must_use]
pub fn su_voice_config(acc: &str, joined: &str) -> bool {
    let ab = acc.as_bytes();
    let b = joined.as_bytes();
    if ab.len() + 1 > b.len() {
        return false;
    }
    b[ab.len()..]
        .windows(2)
        .any(|w| w == b"sU" || w == b"sI" || w == b"sA")
}
