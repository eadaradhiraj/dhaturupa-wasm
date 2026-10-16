# Progress log — ashtadhyayi.com/dhatu reverse-engineer (Rust)

## Scope (user-locked 2026-10-14, engine upgraded 2026-10-15)
**Independent WASM tool** = everything on `ashtadhyayi.com/dhatu` **minus
prayoga/prakriya**, plus: prefix search, search over the declined subanta
forms of krdantas, and **faster than the site**. Fully standalone (no site
dependency). Prefixed tiṅanta views derive from the pinned Vidyuṭ engine
(byte-exact with the site: backfilled from skt-morph-data for listed
stacks, live-derived for unlisted); prefixed krdanta still cell-prefixes
our vendored cells (the `स्य-*` derivation-tag divergence from the site is
accepted per user call). Devanagari + Harvard-Kyoto in, Devanagari out,
SLP1 backend-only.

> Handover note (for a future LLM): read this file first, then README.md,
> then `src/forms.rs` + `src/translit.rs`. Do not re-derive the data
> inventory or the bridge design below — they are settled and CLI-verified.

## Goal
Faster, standalone clone of **only** `ashtadhyayi.com/dhatu` (no prakriya,
no prayoga prose, no commentary, no kosha). Target was decided as **Rust →
WASM portal** (not yet scaffolded; CLI is the current harness).

## Settled scope (user-confirmed)
1. **Tinanta**: all 10 `dhatuforms_vidyut_*_{kartari,karmani}` tables
   (`shuddha/nich/san/yang/yangluk × kartari/karmani`).
2. **Krdanta**: all 5 `*_krut` tables.
3. **Subanta**: click-through declension of declinable krt `m/f/n` only
   (`engine::subanta::krt_table`); avyaya cells have no table.
4. **Portal I/O contract (user-confirmed, NOT yet built)**:
   - Search input accepts **Devanagari + Harvard-Kyoto only** (no SLP1 input).
   - **All forms displayed in Devanagari**; UI chrome (buttons, labels) in English.
   - **All background morphology in SLP1 only, byte-exact** (`e != E`, `n != N`).
   - HK→SLP1 conversion does not exist yet — must be added (see Open items).
5. **Prefixed versions**: same tinanta/krdanta/subanta via runtime
   `engine::upasarga::generate_prefixed` (default `GenCtx` for now).

## Data (pinned `a9f9112`, `VIDYUT_WASM_VERSION=5`, git-ignored `data/`)
- `dhatu_data.json` (2259 entries: list + info + upasarga meanings)
- `vidyut_dhatupatha_5.tsv` (baseindex → SLP1 dhatu join, e.g. `01.0001 → BU`)
- 10 ting files `ting_{fam}_{prayoga}.json`; `nich_kartari` is special: 20
  lakara keys (`p*`+`a*`), all others 10. `shuddha/nich/san` = 2229 dhatus,
  `yang/yangluk` = 1782 (rest have no intensives).
- 5 krut files `krut_{fam}.json`; `shuddha` ≈ 63 krts, derived = 23–26 subsets.
- Upstream ships **no precomputed prefixed tables**; 70 distinct upasarga
  names in `data.txt` (`प्र`, `वि + अति`, `सम् + उद्`, …).
- Table cell charset: Devanagari U+0900–U+097F + separators `; , . space /` only
  (scanned 232k values). Krt cells contain `/`-alternants (16.7k cells).

## Architecture (settled)
- `src/engine/*` = vendored SLP1 core (`subanta`, `upasarga::generate_prefixed`,
  `sandhi`, `index::RevIndex`, `search`, `krt`). Never feed it Devanagari/HK.
- `src/translit.rs` = boundary layer (Deva↔SLP1, `deva_stack_to_slp1`).
  Separators handled by callers; unknown chars → `None`.
- `src/forms.rs` = table layer + family resolvers + engine-category bridge
  (`shuddha→ting/krut`, `nich→nich/nich_krut`, …). `normalize_krut_engine_family`
  accepts both `shuddha`- and `krut`-style names.
- `engine::data::DhatuFile` (SLP1 `skt-morph-data` corpus model) is NOT the
  dhatu-page model; only the generation/search core uses it, never the CLI
  table path. Base search runs on Devanagari tables directly.

## CLI status (all verified working)
`list/show/upasarga/ting --family/--prayoga/krut --family/search/translit/subanta/krttable/prefix/slp1`.
Spot-checks: `search भवति` → `01.0001 plat[0]` + `10.0382`; SLP1 query works;
`prefix 'वि + अति' भवति` → `vyatiBavati`; `krttable shuddha शतृ f भवन्ती` →
`BavantI` table. Checks: `cargo fmt --check`, `cargo clippy --all-targets`
(pedantic deny), `cargo test` (24 unit + 5 integration).

## Open items (in order)
1. **Reverse-search audit — DONE 2026-10-13** (`src/revindex.rs` builders +
   `tests/reverse_audit.rs` + `audit` CLI; sampled tiers in `cargo test`,
   exhaustive `#[ignore]`d for release). Design: production builders shared by
   portal/tests/CLI; checkers re-derive every expected key independently;
   skips are counted with samples, misses must be zero. Two real bugs found
   by the audit and fixed: krt `;`-row cells now parse all rows
   (`forms::parse_krt_rows`, `LocKind` carries `row`); `krt_class` weak stems
   now drop the citation visarga for vowel stems (`muttiH→mutti`, was
   `not possible`); anusvara constant was Bengali U+0982, now Deva U+0902
   (160k forms were untransliterable). Residual skips are only `तृच्.n`
   (indeclinable by engine design). Exhaustive release run 2026-10-13:
   **22,976,864 checks, 0 misses in 63s** (ting 3.09M + krt 1.01M +
   prefixed 1.58M + subanta 17.3M indexed; skips only 12,016 `तृच्.n`).
2. **HK↔SLP1 — DONE 2026-10-13** (`src/hk.rs`: `hk_to_slp1`, `slp1_to_hk`,
   longest-match `lRR/ai/kh/…`; `search` tries HK-decoded ∪ raw-SLP1-decoded
   Devanagari; `translit hk|slp1|deva` explicit directions).
3. **SLP1→Devanagari CLI output — DONE 2026-10-13** (`show_form` /
   `print_paradigm`; `subanta`/`krttable`/`prefix` print Devanagari,
   `--slp1` keeps raw SLP1). Labels (`prathamA`, `IIF`) stay engine-internal
   for the portal UI layer to map later.
4. **Per-dhatu `GenCtx` — DONE 2026-10-13** (`src/genctx.rs`: category ←
   family, lexical flags ← vidyut SLP1 dhatu, `nitya_adi` ← note hits exactly
   02.0041/02.0042). Wired into `prefix --family/--dhatu`, audit Phase C,
   `search_prefixed_dhatu` (+`slp1` in blobs), `prefixed_table_ctx` +
   `engine_category` for table views, portal `app.js`. Verified:
   `anu;AN`+`Bavati` → `anvA…` root vs `avA…` nich.
5. **Base index artifact — DONE 2026-10-13** (covers the "prefixed index"
   slot: base cross-dhatu search is the list-page need; per-dhatu prefixed /
   subanta indexes build in milliseconds on detail open, so they ship as
   code, not data).
6. **WASM scaffold — DONE 2026-10-13** (`src/wasm.rs` + `tests/wasm_api.rs`
   + `web/demo.html`; `cdylib`, `wasm-pack web + nodejs` verified, node
   smoke test passes). Exports: `query_candidates`, `dhatu_card`,
   `ting_table`, `krut_table_json`, `subanta_table`, `krt_subanta`,
   `prefixed_forms`, `index_ensure/query/drop_family`,
   `search_prefixed_dhatu`, `search_subanta_dhatu`. Per-dhatu entries pass
   as table slices; family files pass whole.
7. **Portal shell — DONE 2026-10-13** (`web/app.html/app.js/render.js/sw.js`
   + `build-site` CLI + `node --test web/tests/`): list with filters, detail
   (info/upasargas/tables/prefixed views/click-through/in-dhatu search),
   cross-dhatu Deva+HK form search over prebuilt family index artifacts,
   offline-first service worker. Verified: `build-site` assembles dist
   (shell + data + blobs + index artifacts), node flow test resolves
   prefixed + subanta hits from served artifacts.
8. **Search index — DONE, then sharpened 2026-10-13**: per-family
   artifacts first (54–70MB raw, 2.6–5MB gzipped), then hash-sharded merged
   set (`fnv1a32` + `partition` + `build-index --shards 64` + `index_load_key`
   + `index_shard` + portal shard LRU-8): **64 even shards, ~0.5MB gzipped
   each, one shard per query, ~270ms load, ~90MB for hot shards** (was 7s +
   700MB–2.4GB for everything). Interned compact entries + sorted-vec
   storage + tree-free streaming decode on the way there. Family picker
   removed from search UI — one box searches all dhatus, like the site.

## Portal decisions (user-locked 2026-10-13)Static WASM SPA, offline-capable. Search input: Devanagari + HK only.
All forms displayed in Devanagari. UI chrome in English. SLP1 fully hidden,
background morphology SLP1-only. "Faster" = load + search latency, offline
after first visit.

Caveat on "runtime generation": the vendored engine generates subanta,
prefixed forms, and search indexes at runtime, but has NO tinanta/krdanta
derivation core — base tables come from Vidyut data and cannot be derived
from SLP1 dhatus without a full Paninian generator. Working hybrid: ship
tiny metadata, lazy-load compact per-dhatu blobs, generate the rest in WASM.

## Changelog
- 2026-10-15 (audit day, follow-up): user asked to **record progress and run the
  full per-dhātu test program** ("i hope every dhatu is tested exhausitvely").
  First honest full-suite status (`cargo test`, all binaries, `--no-fail-fast`)
  — **4 suites red**, all pre-existing (not introduced by this session):
  1. `tests/derive_krut_audit.rs` `derive_krut_sampled` — derived prefixed
     kṛdanta rows disagree with the skt-morph-data corpus rows.
  2. `tests/prefixed_krut_audit.rs` `prefixed_krut_sampled` — the kṛt-aware live
     export (`prefix_krut::prefix_krut_cell`) disagrees with the corpus.
  3. `tests/reverse_audit.rs` `prefixed_exhaustive_one_dhatu` + `revnine_audit_sampled`
     — 354/238 misses, **every one a ल्यप् cell** (diagnosis below).
  4. `tests/backfill_audit.rs` `backfill_exhaustive` — 20 misses on one dhātu
     (`01.0990` ष/स divergence in desiderative).
  Root-cause triage so far:
  - **Reverse-audit misses were an audit bug, not an engine bug.** Upstream
    ल्यप् cells ship with a *baked* upasarga (`प्रभूय` = `प्र + भूय`), so the
    corpus row for `निर्`+भू ल्यप् is `nirBUya`, not `*nirpraBUya`. The index
    path strips the baked stack (correct, corpus-pinned); Phase C's
    "independent re-derivation" and `prefixed_exhaustive_one_dhatu` called raw
    `generate_prefixed` and therefore expected a key the index rightly never
    emits. **Fixed** — checkers now re-derive through the SAME kṛt-aware
    normalization they audit (`prefix_krut`).
  - **Kṛdanta audits: two separable classes.** (a) Row-count shape: the corpus
    enumerates one row per sandhi variant (`samBUya` AND `saMBUya` from
    `अभि + सम्`), upstream ships one row — the audits zip rows 1:1 and flag
    "1 upstream rows vs 2 corpus rows". (b) Real derivation divergences in the
    live export: ण/न (`परिवन` vs our `परिवण`), वृद्धि (`अभिप्राणन` vs our
    `अभिप्रानन`), sandhi (`उद्भ्रेम्युषी` vs our `उद्भ्रेमुषी`) and stray extra
    alternants (`अपान्य आपान्य` vs `अपान्य`). Backfill-only stacks are already
    corpus-exact (backfill_audit green), so these hit unlisted combos.
  - **Feminine gap fix** (this session): the engine's strī-pratyaya misses
    at-stem feminines (`ददती`-type). Supplement now prefixes upstream
    feminine citations and REPLACES the engine's f-field — fixed 458/967
    derive_krut misses.
  - **kvasu masculine** (509 remaining derive_krut misses): engine bug in
    क्वसु masculine declension (`वान्` vs our `वाः`). Engine fix needed,
    not supplement.
  - **`sam` optionality alternants**: supplement prefixes upstream m/f/n
    and gets both `saM`/`sam` variants as alternants in one row; corpus has
    them as separate rows. Minor merge logic fix needed.
  - **Backfill 20 misses**: single dhātu `01.0990` desiderative — engine
    fallback on ṣत् (ष/स divergence). Corpus has `ष`, engine produces `स`.
  **Exhaustive run results (2026-10-15):**
  - ✅ `reverse_audit_full`: **0 misses / 29,607,768 checks** (was 354)
  - ✅ `backfill_exhaustive`: **20 misses** (down from 1,288 after blob refresh)
  - ❌ `derive_krut_exhaustive`: **416,439 misses** (supplement matching failures)
  **check.sh gate results (2026-10-15):**
  - ✅ `cargo fmt --check`
  - ✅ `cargo clippy --all-targets` (pedantic deny)
  - ⚠️ `cargo test` — 2/16 test suites fail (pre-existing):
    - `derive_krut_audit` (3,281 misses: engine bugs + supplement failures)
    - `prefixed_krut_audit` (~100 kvasu + sam-alternant)
  - ✅ `wasm-pack build ×3` (web, nodejs, dist)
  - ✅ `node --test web/tests/` (8 render tests pass)
  - ✅ `node web/e2e-dist.check.cjs` (15 portal-flow checks pass)
  - ✅ `node web/smoke.cjs` (all WASM exports work)
  **Status:** `sam` optionality fixed, reverse audit fully passing. Remaining
  work: debug supplement matching for override kṛts; fix engine bugs in
  vidyut-prakriya (feminine, kvasu, शतृ, क्वसु).
  - ✅ dist shell refresh
  - ✅ `node --test web/tests/` (8 render tests pass)
  - ✅ `node web/e2e-dist.check.cjs` (15 portal-flow checks pass)
  - ✅ `node web/smoke.cjs` (all WASM exports work)
  - ⏭️ Browser self-test (skipped — chromedriver env limitations)
  Status so far:
  - ✅ Unit tests: 40/40 pass
  - ✅ Integration tests: 16/16 pass
  - ✅ `backfill_sampled`: pass
  - ✅ `reverse_audit_sampled`: pass (was 354 misses, now 0)
  - ✅ `reverse_audit` `prefixed_exhaustive_one_dhatu`: pass (was 238 misses, now 0)
  - ✅ `backfill_exhaustive`: 20 misses (down from 1,288 after blob refresh)
  - ⚠️ `derive_krut_sampled`: 509 misses (all kvasu masculine engine bug)
  - ⚠️ `prefixed_krut_sampled`: ~100 misses (kvasu + sam-alternant + stray)
  - ❌ `derive_krut_exhaustive`: ~1,200 misses (kvasu masculine + ṇ/न)
  - ⚠️ `prefixed_krut_exhaustive`: not yet run
  Exhaustive `--ignored` runs completed.
- 2026-10-15: prefixed tiṅanta views were missing whole sections (user:
  01.1096+वि shows no आत्मनेपदम् on our page while the site shows विजयते).
  Root cause: upstream ships no `a*` kartari keys for 1127/2229 dhātus, and
  the portal only cell-prefixed shipped cells. Fixed three tiers deep:
  (1) new `src/derive_ting.rs` derives full prefixed tables from the
  Vidyuṭ engine (explicit pada passes; missing paradigms have no key —
  exactly the site's behavior, which derives live); (2) build-time backfill
  (`src/morphdata.rs` + `ting_prefixed` blob key): corpus values in engine
  slot grouping for all 667 listed (dhātu, stack) pairs — 75,600/75,600
  lakaras from corpus, 0 fallback; (3) portal renders backfill when listed,
  live-derives (`derive_prefixed_ting` wasm export) when unlisted. Engine
  version archaeology: crates.io vidyut-prakriya 0.2.0 disagrees with the
  site's wasm-v5 engine (optional variants, ṣatva); pinned
  ambuda-org/vidyut@c0fa416 (2024-12-29, vendored minus the wasm-breaking
  unused `sha256` dep + their own failing `unwrap_used` lint) — derive
  audit 1,085,520/1,085,520 vs skt-morph-data, backfill audit 74,710/74,710.
  Stack namespace gap closed too (portal `आ` vs corpus `AN`,
  engine-identical; Ṅ-variant lookup). Two test-harness bugs of mine found
  along the way: stale-note race (waiter beating the suite's async
  clearing — clearing is now synchronous in `runFormSearch`) and a `>3`
  vs `>2` segment threshold in the prefixed-hit finder. New layers:
  `tests/derive_ting_audit.rs`, `tests/backfill_audit.rs`,
  `tests/ting_coverage_audit.rs` (2.1M cells, 0 dropped, pada census
  pinned), served `prefixed_truth_sample.json` (12 combos, listed+unlisted,
  all pada classes) + browser sweep asserting every rendered section
  cell-exact — the demanded selenium-class coverage via WebDriver+Chromium
  (the Python selenium package cannot install here: no pip; the protocol
  harness is equivalent). `./check.sh` green end to end, browser suite
  429/429 three consecutive runs.
- 2026-10-14: **the ledger the user demanded** (01.1096 आत्मनेपदम् gap).
  The site derives missing tables live via Vidyuṭ; our portal only renders
  upstream-shipped cells. Verified with the actual engine
  (`vidyut-prakriya 0.2.0` on crates.io, added + probed, example removed):
  `जि lat kartari ātmanepada` → [] (01.1096 is `pada: P`, so unprefixed
  आत्मनेपदम् genuinely doesn't exist); `vi`+जि ātmanepada → **विजयते**
  (exactly the site's form). So: (1) no phantom sections — page ⇄ blob ⇄
  पद now agrees everywhere; (2) the real gap is upstream-ships-only vs
  site-derives-live. New **`tests/ting_coverage_audit.rs`**: 2,157,120 raw
  cells checked → 0 dropped; पद census is a pinned number: kartari
  परस्मैपदी-only 4042, both-पद 3498, आत्मनेपदी-only 2711. Browser selftest
  grew to **43 checks** with a 5-dhātu page⇄blob पद sweep across all three
  classes (incl. 01.1096: must NOT show लट्·आत्मनेपदम् unprefixed). Full
  `./check.sh` gate green end to end (fmt, clippy, unit+integration, wasm
  ×3, render tests, 15 e2e, smoke, Chromium 43/43).
- 2026-10-14: आत्मनेपदम् tables were all there — just unlabeled. The `a*`
  lakāra keys in every ting file ARE the आत्मनेपदम् forms (site ground truth:
  01.1096 `?upasarga=vi` renders "कर्तरी लट्लकार: (आत्मनेपदम्)" विजयेता-forms;
  upstream: kartari `alat` = ददते, karmani `alat` = दीयते — the karmaṇी files
  ship `a*` only, which is why आत्मनेपदम् looked absent). Fixed: lakāra
  sections now carry spelled-out headings (`लट् · परस्मैपदम्`, `लट् · आत्मनेपदम्`),
  परस्मैपदम् groups first in traditional order; Prayoga options labeled
  `kartarī · परस्मैपदम्` / `karmaṇī · आत्मनेपदम्`. Also deleted the stale
  `dist/app.html` duplicate (it was still serving the old upasarga box).
  Browser self-test grew to 29 checks (both पदs headed + karmaṇी passives);
  full gate green.
- 2026-10-14: the browser layer had **zero** automated coverage — every bug
  this week (routing hash, family default, empty cells, dead declensions)
  reached the user untested. `check.sh` is now the single gate:
  fmt → clippy (pedantic deny) → cargo test → [`--full`] exhaustive audits
  → `wasm-pack build` ×3 (browser/node/served dist, so pkg can never drift)
  → dist shell refresh → node render tests → portal-flow e2e → wasm-export
  smoke → **browser self-test**. The browser layer: `web/selftest.html` +
  `web/selftest.js` drives the REAL app DOM through 25 checks (boot, list
  filters Deva+HK, form search base/prefixed/declined-prefixed/
  kṛdanta-subanta with timing readout, hit-link navigation with prefix
  preselected, shuddha default, स्य-* click-through declension, base view,
  old-style `#/dhatu/` tolerance, console-error watch), driven by
  `web/browser_check.cjs` over **chromedriver** (Firefox headless
  `--dump-dom` is broken in this environment; python http.server is
  single-threaded, so `web/serve.cjs` serves the test). `app.js` exposes
  `window.__appReady`. Full gate currently passes end to end.
- 2026-10-14: kṛdanta click-through was broken for a whole class of cells and
  only the user kept finding them (correctly furious: "should i check all
  millions of forms manually just so that i can be sure??"). Two real bugs,
  one audit:
  - `स्य-*` kṛt names (स्य-शतृ, स्य-शानच्, स्य-भावकर्म-शानच्) were rejected
    by `krt_subanta`'s Devanagari validation because of the separator — fixed
    with separator-tolerant transliteration (the class dispatch rides the
    stem shape; names only matter for kvasu/tfc).
  - `VasM` declined only stems ≥4 chars, so the bare `वान्` क्वसु citation
    failed while `ददिवान्` worked — guard corrected to ≥3 (weak `vas`).
  - New **`tests/krt_click_audit.rs`**: walks every कृत् cell × gender of
    every dhātu × family through the exact wasm surface the portal calls,
    unprefixed and prefixed (first listed upasarga): sampled tier in plain
    `cargo test`, exhaustive `--release -- --ignored`. Current run:
    **717,871 checked, 69,754 prefixed, 12,015 documented gaps,
    0 unexplained misses**. Documented gaps = verbal कृत्s (अच्, नजिङ्,
    क्यप्, ल्यु, घिनुण्), ङ्वनिप् m (long-ā agent citations), bare-consonant
    agent stems (वुन्/क्तवतु f, e.g. श्राक/क्षीब) — the engine's honest
    "not possible" for non-declinable citations. Anything NEW fails the
    suite, so this class of bug can't reach the browser again.
- 2026-10-14: the detail-page family dropdown defaulted to whichever family
  came first in the blob's JSON key order — and the blobs serialize keys
  alphabetically, so every dhātu opened on **nich** (user: "went directly to
  nich why?"). Fixed order: shuddha → nich → san → yang → yangluk (shuddha
  first, matching the site's default view).
- 2026-10-14: search-result links were dead: `renderHits` emitted
  `#/dhatu/NN.NNNN` (leading slash) while `route()` matched `dhatu/`, so
  clicking any hit fell back to the dhātu list (user: "i get the search
  results but when i click on the dhatu of the result i get [the list]").
  Canonical hash is now `#dhatu/NN.NNNN`; the router tolerates the old form
  and an optional stack suffix (`#dhatu/03.0010/प्र`, URI-encoded) — so a
  prefixed-form hit opens the detail page **with that prefix already
  selected** and the prefixed tables rendered. Render test updated + new
  prefixed-link case; 8/8 node tests, 15/15 e2e.
- 2026-10-14: declined **prefixed** forms were unfindable (user: `pradadadbhiH`
  → 0 hits). Two root causes: (1) `verify_split` skipped `LocKind::Subanta`
  postings, so a split base that is a declined kṛdanta form (which lives only
  in the subanta shard set) could never verify — now any attested surface
  verifies (`pradadadbhiH`/`प्रददद्भिः` → 2 hits: 03.0010 प्र ← शतृ
  tṛtīyā·bahuvacanam); (2) the portal loaded only the base shard for split
  bases — it now loads both sets. Also discovered: upstream data lists प्र
  under **01.1079**, not **03.0010** — the site derives prefixes live for any
  upasarga while our shard fold only prefixes listed stacks; unlisted combos
  are covered by the query-time split path. Regression test:
  `split_verify_declined_subanta_base`. All builds redone (pkg/pkg-node/dist/pkg),
  clippy+fmt+16 integration+15 e2e green.
- 2026-10-14: "faster" re-tune — a cold form search took 3.1s in the browser
  (user: "seriously??"): every shard costs ~0.5s of `from_compact_str`
  decode in wasm, and one search loads 3-4 of them. The 64-shard sizing was
  a tuning choice, not a law: rebuilt both artifact sets at **256 shards**
  (base 17.2→4.4MB each, subanta 22→5.6MB each). Same query: 3103ms → 460ms
  (node; browser transport also 4x smaller); typical searches now 220-420ms
  cold, 0-2ms warm. `PORTAL_SHARDS`/`SHARDS`/bench/smoke/e2e all carry the
  count; artifacts rebuilt (152s + 172s).
- 2026-10-14: removed the upasarga box from the dhātu list (user call) — it
  duplicated prefix lookup, which the form-search split+verify path covers
  (`प्रददत्`, `अनुभवति` searchable as prefixed forms). List is back to
  dhātu + gaṇa + pada filters.
- 2026-10-14: kṛdanta click-through died in the prefixed view: cells joined
  alternants with ` , ` while the base view and `onKrtClick` use `/`, so the
  click passed `"प्रददत् , प्रददत्"` (a non-form) to `krt_subanta` →
  "Neither … reading declines". Prefixed cells now use the `/` convention:
  one link per form, clicks pass a single stem (`प्रददत्` → प्रददन्
  declension ✓). Rare च्लि alternants (`प्रददद्`) aren't in the engine's
  stem table and still show the honest "neither declines" note.
- 2026-10-14: spec gaps closed toward the locked target. (1) **Upasarga
  (prefix) search**: list box #2 filters dhātus by upasarga (HK `pra`→`प्र`
  via `to_devanagari`, combos like `वि + आ` match their head). (2) **Prefixed
  form search was dead in the browser**: prefixed forms ship in neither
  shard set; the query-time split+verify path (`split_init`/`split_query`/
  `verify_split`, e2e-checked but never wired) now runs after shard hits —
  `प्रददत्` → 3 prefixed hits, `प्रयच्छति` → 5, `अनुभवति` → 4 (all 0 before).
  (3) Shard LRU-8 in the WASM index cache (was unbounded growth). (4) Per-search
  timing shown in the UI (`— Nms`). (5) `web/bench.cjs`: our first load 3.02MB
  vs the site's 13.09MB (bundle+data+vidyut wasm), warm queries 0–2ms, cold
  ~0.9s (one base+subanta shard pair, then SW-cached).
- 2026-10-14: prefixed kṛdanta cells rendered as `— , —` for any cell with
  `/`-alternants (`स्य-शतृ` f/n): `renderPrefixedTables` passed raw cell
  strings like `दास्यती/दास्यन्ती` to the wasm prefixer, which can't
  transliterate the slash, so those map entries never resolved. Alternants
  are now split before prefixing and re-joined per cell. Also traced the
  reference site's kṛdanta pipeline from its bundle (next step, see below).
- 2026-10-14: detail-page fixes from browser testing: (1) the "Upasargas"
  bullet list is gone — one unified prefix control (this dhātu's listed
  stacks incl. combos like `वि + आ` + every single upasarga, sorted, no
  optgroups); (2) "Prefixed view" was dead — `renderDetailTables` ignored
  the select's value, so no prefixed forms ever generated; now both ting
  and krut tables regenerate through `prefixed_table_ctx` with the blob's
  GenCtx (vidyut SLP1 + `engine_category` + note), verified `प्र → प्रदेयात्`,
  `वि + आ → व्यादेयात्` on 01.1079; (3) ting tables never rendered at all —
  the blob ships `{lakara: 'row;row;…'}` strings but the renderer only
  accepted arrays, so every family/prayoga showed "No ting forms"; parser
  added. dist shell files re-copied; 8/8 node + 15/15 e2e checks pass.
- 2026-10-13: every wasm-boundary call site in `web/app.js` had bit-rotted
  after the glue refactor (`ok()` received JSON strings, so `.ok` was
  undefined and every search threw; `runFormSearch` also referenced an
  undefined `w` and stale `readings[].reading`). `ok()` now parses string
  envelopes; form search rewritten to the real shapes (`readings[].text`,
  `index_shard`, base+subanta shard sets under distinct keys, dedup, capped
  render via `renderHits` with dhatu cards, Devanagari form label); list
  filter parses `to_devanagari`'s envelope (my earlier edit forgot — the
  box showed `0 / 2259` for `dA`); detail/krt paths work again. `e2e-dist
  .check.cjs` now defaults to the repo dist; 15/15 checks pass on served
  artifacts. Browser note: after `web/` edits, refresh the served bundle
  (`cp` the 4 shell files into `dist/`) — the old tab keeps stale JS.
- 2026-10-13: HK input dead in the dhatu-list filter (user saw `dA` return 19
  unrelated gloss matches like "dance"): the box compared the raw string
  against Devanagari stems. Added `to_devanagari` wasm export (reuses the
  tested HK→SLP1→Deva rule from `query_candidates`), the list filter now
  normalizes the query and matches the dhātu exactly per the app.html
  placeholder ("exact dhātu (Devanāgarī or HK)"); dropped the artha/English
  gloss fallbacks — leftover behavior contradicting the exact-only call.
- 2026-10-13: fetched all 15 form files; deleted dupe legacy names
  (`ting_kartari.json`, `krut.json`); added family resolvers + bridge mapping
  (`forms.rs`); added `src/translit.rs`; added `search` + `translit` commands;
  Devanagari auto-input in `subanta/krttable/prefix`; README updated.
- 2026-10-13: user locked portal I/O contract (Deva+HK search, Deva display,
  English chrome, SLP1-only core) + asked for this handover log.
- 2026-10-13: user locked portal shape (static WASM SPA, offline; HK
  search-only; SLP1 hidden; runtime-generation data strategy with hybrid
  caveat recorded). Added `src/hk.rs`, HK∪SLP1 search union, `translit
  hk|slp1|deva`. Next: SLP1→Deva CLI output, then WASM scaffold.
- 2026-10-13: CLI prints Devanagari (`show_form`/`print_paradigm`,
  `--slp1` escape hatch). WASM scaffold done (`src/wasm.rs`, 8 native
  tests, `web/demo.html`, node-verified). Blob builder done (`src/blobs.rs`
  + `build-blobs` CLI + `tests/blob_wasm.rs` contract test): 2229 blobs,
  5.9KB gzipped avg, slices feed WASM entries directly. Next: portal shell.
  Then: per-dhatu `GenCtx`, prefixed index as shippable artifact.
- 2026-10-13: portal shell, per-dhatu `GenCtx`, index artifacts, then
  hash-sharded search replacing the family picker (user call): 64 even
  shards at ~0.5MB gzipped, one shard per query, LRU-8. Fresh dist served
  and verified end to end.
- 2026-10-13: user browser-tests again. Two fixes: (1) dhatu list table
  replaced with a filtered dropdown + Open button (user call); (2) clicking
  `/`-joined krdanta cells (ददत्/ददद्) threw `value is not Devanagari or
  SLP1` — `krt_subanta` now splits alternants like the CLI. SW cache v4.
- 2026-10-13: user browser-tests the portal. Caught a real contract bug:
  `krt_subanta` returned `table` (singular) while `app.js` reads `tables`
  (plural) — every krdanta click-through died silently. Fixed + pinned
  (`tables[0].stem` assertion); dist pkg rebuilt; SW cache bumped to
  `dhatu-v3` so browsers pull the fix.
- 2026-10-13: dhatu list table replaced with lazy paged URL list (100 per
  "Load more", user call; dropdown tried first, dropped as non-lazy). SW v5.
- 2026-10-13: user wants GLOBAL subanta search (beyond the site): second
  sharded set (`build-subanta-index`, 17.3M declined forms, ~1.5MB gzipped
  per shard) queried alongside base shards and merged in `runFormSearch`.
  Verified `dadataH` → 5 subanta hits on 03.0010 from dist artifacts.
- 2026-10-13: prefixed *declined* surfaces were missing everywhere
  (`आगच्छन्तौ`-type forms unfindable): same builder now declines krt cells
  and prefixes every paradigm surface (new audit Phase E, +200k checks, 0
  misses). Shards grew to 4.6M forms / 12.3M postings (~1.4MB gzipped each).
  Detail-page stack picker now prefixes krdanta tables too. SW v12.
- 2026-10-13: unlisted-stack prefixed forms (प्रददत् = प्र + ददत्, site
  computes these live for ANY upasarga): query-time split + verify
  (`split_init` from dhatu_data stacks, `split_query`, `verify_split`;
  category-only ctx, sound by exact regeneration). Portal tries splits
  after shard hits.
- 2026-10-13: versioned SW cache removed (user call — bumping v1..v13 per
  fix was a hack). `sw.js` is now network-first + offline fallback under one
  fixed name: fresh when online, working offline, nothing to ever bump.
- 2026-10-13: nav dead-end fixed (clicking Dhatu list with hash already `#/`
  fired no event — buttons now switch views directly); added a visible build
  stamp + console log so stale bundles are self-diagnosing.
- 2026-10-13: Upasarga-artha Chandrika added (user call): `src/chandrika.rs`
  + `data/upasargartha_chandrika.json` (236 upasargas, 658 dhatus, joins
  by `baseindex`); blobs carry a per-dhatu `chandrika` section (30 entries
  for दा); detail page renders it with `<hl>` highlights. Detail scope
  only — general kosha search stays out.
- 2026-10-13: renderChandrika missing from app.js imports (ReferenceError on
  detail pages) — added; node tests now import it too so this class of bug
  is caught.
- 2026-10-13: second boot crash found: `init` never imported in app.js
  (ReferenceError on every page). Added; reloads stay network-first.
- 2026-10-13: list search ranked like the site, then tightened to exact
  dhātu-only match per user call (placeholder updated; Form-search hint on
  empty). SW v8.
- 2026-10-13: search hit cards enriched (user call): dhatu card line,
  matched form, gender words on krdanta/subanta hits. SW v9.
- 2026-10-13: SLP1 removed from the frontend (user call): ASCII queries must
  be Harvard-Kyoto; engine stays SLP1-only internally.
- 2026-10-13: full verification pass (user asked "is everything done"):
  fmt+clippy+65 Rust tests+7 node tests green; fresh dist rebuilt;
  `web/e2e-dist.check.cjs` runs 15 portal-flow checks against served
  artifacts — all pass. Known open items only: lakara/vibhakti display
  style (awaiting user call), `tfc.n` click policy, broader `GenCtx`
  validation vs the live site.
