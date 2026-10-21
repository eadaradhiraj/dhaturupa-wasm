# Additions & Roadmap for dhaturupa-wasm

This document outlines the confirmed additions and enhancements for `dhaturupa-wasm`.

The project is a **fast, standalone, offline-first Web/WASM clone of `ashtadhyayi.com/dhatu`** with high-speed reverse search and morphological analysis. The CLI is strictly a developer harness, test driver, and artifact builder (`build-site`, `build-blobs`, `build-index`, `audit`).

---

## Status (2026-10-20)

Every item below is **delivered** — each with its own commit and a green
`./check.sh` (browser self-test, e2e, smoke) plus the exhaustive audits
(`./check.sh --full`).

| # | Addition | Delivered as |
| :-: | :--- | :--- |
| 1.1 | Traditional 3×3 tiṅanta matrix | `renderTingTables` (वचनम् columns, पुरुषः rows) |
| 1.2 | Subanta grid labels | विभक्तिः row heads + वचनम् column heads |
| 2.1–2.3 | Consonant-final nouns | `Class::Cons` (सरित्/वाच्/ककुभ् families, ऋ-chart, जश्) |
| 2.4 | उस्/इस् neuters | `Class::UsN` (आयुष् family) |
| 2.5 | गो/नौ/रै | word route (verbatim site grids) |
| 2.6 | प्रणाम-स्वादि | word route (अन्य/इतर/कतर/कतम/उभय/उभ) |
| 1.3 | Copy Table (Markdown/TSV) | `tablePayload` + one-click copy on every grid |
| 1.4 | URL hash deep linking | `#search?q=…`, `#word?q=…`, `#dhatu/<id>?fam=&stack=` |
| 3.1 | English gloss search | `#list-q` falls back to `artha_english` |
| 3.2 | Seṭ/Karma filters | `#list-settva`, `#list-karma` dropdowns |
| 4.1 | WASM size | release profile + tuned wasm-opt: 1.61MB → 1.03MB |

Beyond the roadmap, the subanta engine now reproduces **8,820 of the
9,007** grids the site lists (`node web/audit-site-grids.cjs`), and the
new paradigms read back as well as decline.

The paradigms added after the roadmap:

| family | classes |
| :--- | :--- |
| the neuter च्-final nouns | `AcN` |
| the masculine ष्-final nouns (ड/ट, ग/क) | `Zam`, `ZagM` |
| the feminine ष्-final nouns (ड/ट, the vowel flavours) | `ZamF`, `ZvF` |
| the masculine/feminine/neuter ह्-final nouns | `HasM`, `HuhM`, `DuhM`, `HasVM`, `HasAM`, `HasF`, `HihF`, `DuhF`, `HasN` |
| the कृत agent nouns (the neuter) | `RFN` |
| the feminine अस्-स्वादि | `AsF` |
| the वस्-स्वादि | `VasM`, `VasN` |
| the feminine र्-final nouns | `RirF`, `RarF`, `RarN` |
| the bare-nominative nouns | `BaraM` |
| the neuter ऋ-chart nouns | `RcN` |
| the masculine आ- and औ-stems | `AAM`, `AuM` |
| the neuter आ-stem nouns | `AAN` |
| the a-stems with the optional वृद्धि-forms | `AMo` |
| the u-stem neuter without the गुण-forms | `UNp` |
| the i-stem neuters with the य- and ऋ-forms | `INy`, `INr` |
| the neuter of the वतुप् stems | `VantN` |
| the neuter द्-final nouns | `DadN` |
| the ऋ-chart nouns with two alternants | `Rc2` |
| the masculine च्-final nouns of the pair flavour | `AcM2` |
| the masculine at-participles with the द/त pair | `AtM2` |
| the श्-final nouns of the ड/ट flavour | `Sam` |
| the neuter a-stems with the optional वृद्धि-forms | `ANo` |
| the व-स्वादि feminine u-stems with the dental genitive plural | `UvFd` |
| the compounded pronouns (अतियुष्मद्, अत्यस्मद्, परमयुष्मद्, परमास्मद्) | `Pron` |
| the masculine ई-स्वादि in the गुण-/वृद्धि-flavours | `IIM2` |
| the masculine ऊ-स्वादि in the व्व-flavour | `UUM2` |
| the feminine a-stems with the स्य-pairs and the bare vocative | `AAFS` |
| the नदीसंज्ञक feminine ई-/ऊ-स्वादि in the िय-/उव-flavours | `NV2` |
| the इ-स्वादि with the वृद्धि-सर्वधातुकारी | `InV` |
| the ञ्च-स्वादि masculine with the वृद्धि weak stem | `AccMV` |
| the अस्-स्वादि masculine (the plain, the pause and the वृद्धि flavours) | `AsMV`, `AsMP`, `AsMW` |
| the स्-final masculine of the visarga flavour | `SasV` |
| the masculine ष्-final of the visarga flavour | `ZamC` |
| the masculine ई-स्वादि with the आ-forms beside the गुण-forms | `IIM2` |
| the plain flavour of the masculine वतुप् stems | `VantMP` |
| the mixed flavours of the neuter त-स्वादि | `TaN2` |
| the neuter ञ्च-स्वादि (the वृद्धि, the ओ-form and the ङ्-inserted) | `AcNV`, `AncN` |
| the neuter i-stems with the ण-forms in the oblique cases | `INv` |
| the masculine ज्-final of the ड/ट flavour | `JasM` |
| the ऋ-स्वादि with the आ-form in the nominative | `RFA` |
| the masculine अन्-stems of the ण-सर्वधातुकारी family | `AnV` |

---

## 1. Web Portal UI / Presentation Improvements

### 1.1 Traditional 3×3 Matrix for Tiṅanta Conjugation Tables
- **File**: `web/render.js` (`renderTingTables`), `web/app.html`
- **Current State**:
  Lakāra forms are currently rendered as a single vertical column of 9 rows (`prathama · ekavacana`, `prathama · dvivacana`... = 9 rows per lakāra × 10 lakāras = 90 vertical rows). This makes the detail page long, difficult to scan, and inconsistent with traditional Sanskrit tables.
- **Proposed Addition**:
  Render each lakāra as the traditional **3×3 Sanskrit matrix** (matching `ashtadhyayi.com`):
  | पुरुषः | एकवचनम् | द्विवचनम् | बहुवचनम् |
  | :--- | :--- | :--- | :--- |
  | **प्रथमपुरुषः** | भवति | भवतः | भवन्ति |
  | **मध्यमपुरुषः** | भवसि | भवथः | भवथ |
  | **उत्तमपुरुषः** | भवामि | भवावः | भवामः |

### 1.2 Explicit Headers for Subanta Declension Tables
- **File**: `web/render.js` (`renderParadigm`)
- **Current State**:
  Subanta declension tables currently output a bare 8×3 grid of cells with no column or row headers.
- **Proposed Addition**:
  - Column headers: `विभक्तिः`, `एकवचनम्`, `द्विवचनम्`, `बहुवचनम्`.
  - Row headers: `प्रथमा`, `द्वितीया`, `तृतीया`, `चतुर्थी`, `पञ्चमी`, `षष्ठी`, `सप्तमी`, `सम्बोधनम्`.
  - Makes the "Decline a word" view and kṛdanta click-through grids immediately legible.

### 1.3 One-Click "Copy Table" (Markdown & TSV)
- **Files**: `web/render.js`, `web/app.js`
- **Proposed Addition**:
  Add a small "Copy Table" button to tiṅanta and subanta tables:
  - **Copy as Markdown**: Clean formatted table for pasting into personal notes, GitHub, or Obsidian.
  - **Copy as TSV**: Tab-separated values for direct pasting into spreadsheets or Anki flashcards.

### 1.4 URL Hash Deep Linking
- **File**: `web/app.js` (`route`)
- **Current State**:
  Hash routing only recognizes `#dhatu/<id>`, `#search`, or `#word`. Navigating or refreshing loses query state.
- **Proposed Addition**:
  Support query parameters in the URL hash:
  - `#search?q=bhavati` → opens form search with the query pre-filled and executed.
  - `#word?q=rAma` → opens word view with declension and readings pre-filled.
  - `#dhatu/01.0001?fam=shuddha&stack=pra` → preserves family and prefix selection.
  - Enables bookmarking and browser history back/forward navigation.

---

## 2. Sanskrit Morphology Engine Extensions (`src/engine/subanta.rs`)

The word view ("Decline a word" / `subanta_analyze`) is designed to decline any base word and analyze inflected forms. Extending `classes_for` (lines 1876–1970) completes the remaining classical consonant and diphthong paradigms:

### 2.1 Feminine `t`/`d` Stems (`Class::AtF`)
- **Stems**: *sarit* (सरित्), *vidyut* (विद्युत्), *marut* (मरुत् f), *samvid* (संविद्), *upaniṣad* (उपनिषद्).
- **Current Issue**: Line 1960 in `subanta.rs` has `Linga::F => {}` (empty match arm).
- **Declension**: Regular non-num consonant endings (*sarit, saritau, saritaḥ; saritam, saritau, saritaḥ; saridbhyām...*).

### 2.2 Palatal & Velar Stop Stems (`c`/`k`)
- **Stems**: *vāc* (वाच् f: वाक्/वाग्, वाचौ, वाचः... वाग्भ्याम्... वाक्षु), *tvac* (त्वच् f).

### 2.3 Dental & Labial Aspirate Stems (`dh`/`bh`)
- **Stems**: *yudh* (युध् f: युत्/युद्, युधौ, युधः... युद्ध्याम्... युत्सु), *budh* (बुध् m/f), *kakubh* (ककुभ् f: ककुप्/ककुब्, ककुभौ, ककुभः... ककुब्भ्याम्... ककुप्सु).

### 2.4 Neuter `is`/`us` Stems (`Class::IsUsN`)
- **Stems**: *havis* (हविस् n: हविः, हविषी, हवींषि), *dhanus* (धनुस् n: धनुः, धनुषी, धनूंषि), *āyus* (आयुस् n).

### 2.5 Diphthong Stems (`Class::Diphthong`)
- **Stems**: *go* (गो m/f: गौः, गावौ, गावः; गाम्, गावौ, गाः; गवा, गोभ्याम्... गोषु), *nau* (नौ f: नौः, नावौ, नावः...), *rai* (रै m).

### 2.6 Additional Pronominals (Sarvanāma)
- *anya*, *itara*, *katara*, *katama*, *ubha* (dual-only), *ubhaya*.

---

## 3. Dhātu List Filters

### 3.1 Meaning (English Gloss) Search
- **Files**: `web/app.html`, `web/app.js` (`runListFilter`)
- **Current State**: `#list-q` only matches exact Sanskrit dhātu names.
- **Proposed Addition**: Allow searching by English meaning (`r.artha_english`), e.g. typing `"eat"` matches `अद्`, `खाद्`, `भक्ष्`, `अश्`.

### 3.2 Grammatical Filters: Seṭ/Aniṭ and Sakarmaka/Akarmaka
- **Files**: `web/app.html`, `web/app.js`
- **Current State**: Data fields `settva` (`S`/`A`/`V`) and `karma` (`S`/`A`/`D`) exist in `dhatu_data.json` and are supported in CLI `--settva` / `--karma`, but are missing from the web interface.
- **Proposed Addition**: Add dropdowns in `#view-list` for:
  - **Settva**: seṭ (सेट्), aniṭ (अनिट्), veṭ (वेट्).
  - **Karma**: sakarmaka (सकर्मकः), akarmaka (अकर्मकः), dvikarmaka (द्विकर्मकः).

---

## 4. Performance & WASM Binary Optimization

### 4.1 WASM Binary Size Reduction
- **File**: `Cargo.toml`
- **Current Size**: `dist/pkg/dhaturupa_wasm_bg.wasm` is ~1.6MB uncompressed.
- **Proposed Addition**:
  Add release profile optimizations:
  ```toml
  [profile.release]
  opt-level = "z"        # Optimize for binary size
  lto = true             # Full link-time optimization
  codegen-units = 1      # Maximize inlining across crates
  panic = "abort"        # Strip panic unwinding tables
  strip = true           # Strip symbols
  ```
  Reduces binary footprint by **30–45%** (~900KB–1MB), accelerating initial load.

---

## Summary Matrix

| # | Addition | Component | Impact | Complexity |
| :-: | :--- | :--- | :-: | :-: |
| **1** | Traditional 3×3 Matrix for Tiṅanta tables | `web/render.js`, `web/app.html` | High | Low |
| **2** | Subanta table header labels (Vibhakti/Vacana) | `web/render.js` | High | Low |
| **3** | Subanta consonant & diphthong stems (`subanta.rs`) | `src/engine/subanta.rs`, `tests/` | High | Med |
| **4** | Table Copy button (Markdown / TSV) | `web/render.js`, `web/app.js` | Med | Low |
| **5** | URL Hash Deep Linking (`#search?q=...`) | `web/app.js` | Med | Low |
| **6** | English gloss search & Settva/Karma filters | `web/app.html`, `web/app.js` | Med | Low |
| **7** | WASM binary size optimization (`opt-level = "z"`) | `Cargo.toml` | Med | Low |
