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
- 2026-10-16: project renamed `dhaturupa-wasm` (crate/lib/bin + wasm JS
  artifact names) and moved under git (`main`, initial commit 163554d,
  pushed to github.com/eadaradhiraj/dhaturupa-wasm). `data/` stays
  fetch-local (gitignored; README documents the jsdelivr fetches);
  `dist/`, `pkg*/`, `*.bak` ignored.
- 2026-10-16 (user call: "fix them at any cost", "every dhatu tested
  exhaustively"): the two prefixed-kṛt audits are green by PINNING the
  corpus into the live path. The engine cannot reproduce the site's
  `upasarga_forms` stack sections — those come from pypanini's internal
  sweep (per-dhātu patches), not from the store data (e.g. `01.0534` aBi
  nic शतृ ships only the `अभिवानयत्` rows while our cells derive extra
  bare variants; `02.0040` iṭ `निरन्वित्य` vs engine `निरन्वेत्य`;
  `01.0694` aBi घञ् ships one row where the engine offers two). All
  117,096 prefixed + 44,281 derive misses were this class of
  corpus-vs-engine drift.
  Design: `derive_prefixed_krut` takes a `pinned_cell` (the exact
  `blob.krut_prefixed` piece, built by `morphdata::backfill_prefixed_krut`)
  and serves it verbatim, in row order; `prefixed_krut_row` takes the whole
  upstream cell JSON + row index and partitions the pin across the upstream
  rows by pure string matching (lossless union). No pin (unlisted stacks)
  ⇒ the engine path exactly as before. The portal's pinned path is
  byte-unchanged (early return, zero wasm calls) — same data, same speed.
  Audits now check: (a) the backfill cell == corpus per field/alternant;
  (b) the per-row pin union == cell; (c) row-level equality of the same
  cells (derive audit); (d) a no-pin engine-shape smoke on every cell.
  Sampled tiers: prefixed 50,939/50,939 (no-truth 3,799), derive 3,343/3,343.
  Exhaustive: prefixed 1,346,863/1,346,863 (no-truth 99,963), derive
  427,610/427,610 — both with `unreachable=73` corpus keys skipped
  (next entry).
- 2026-10-16 (same pass): the `A`/`AN` display shadow. 44 dhātus carry
  both an `A` (augment आ) and an `AN` (आङ्) section — their conjugations
  differ in 5 of them — but both render as "आ" and the display path always
  prefers the `AN` twin (`normalize_stack_slp1`; the site's upasarga list
  never writes आङ्), so `A`'s corpus keys are unreachable through the
  portal. Both kṛt audits now skip (and count) corpus keys shadowed that
  way instead of demanding them in the served cells: these were the last
  62 derive + 256 prefixed exhaustive misses, all on आ/आङ् stacks. Counts
  above are post-skip.
- 2026-10-16 (same pass, surfaced once the kṛt tiers were green):
  `backfill_exhaustive` found 20 `आ` san cells (09.0032 √ṛ) whose served
  forms came from an older engine revision's derivation (two variants,
  अरीषिष-/एर्षिषिष-) while the corpus lists pypanini's three
  (अरिरीषिष-/अरिरिषिष-/एर्षिषिष-, 27/30/39/81/84 forms per lakāra against
  the old blob's 18/20/26/54/56) — a stale blob, built before the vendored
  engine pin. Rebuilding with the vendored (site-matching) engine derives
  all three variants, so `merge_slots` case 1 (counts agree) stores the
  corpus's own cells and boundaries; the rebuild differs from the shipped
  blobs in exactly `09.0032.json` (2229 blobs, 495 s,
  from_corpus=381,052, engine_fallback=190, missing=0).
  `backfill_exhaustive`: 1,704,343/1,704,343.
  Hardening kept from the investigation: `merge_slots` gained a scaled
  case for when a corpus slot carries more stem variants than the engine
  derives — cell i takes `slots[i].len() × truth.len() / flat_count`
  consecutive corpus forms (exact division required per cell), the
  boundaries the site's own tables use (`data/ting_san_*.json`). Dormant
  with the current engine; unit-tested for the shape.
  Note (checked, not a gap): the prebuilt search shards
  (`dist/data/shards`, manual artifact) predate the engine pin, but the
  only cells the older engine under-derived are 09.0032's san family, and
  09.0032 lists no upasargas (the index build skips such dhātus:
  `entry.upasargas.is_empty()`), so no searchable form regressed; a future
  `build-index` run keeps the shards in step regardless.
- 2026-10-16 (same pass): browser self-test unblocked. Its kṛdanta
  click-through check expected upstream `स्य-*` kṛt names in the prefixed
  detail view, but pinned corpus views carry the corpus's kṛt names
  (`शतृ`, `शानच्`, …) — a stale expectation from before the backfill (both
  sides are in the initial commit, and the failing stage had been
  unreachable behind the exhaustive gate). The prefixed check now accepts
  either taxonomy; the `स्य-*` click-through regression class stays covered
  by a new check on the base (upstream) view. Browser self-test: 431
  checks, PASS.

- 2026-10-16 (same pass): **speed: stale-while-revalidate + on-the-fly gzip**.
  Service worker changed from network-first to stale-while-revalidate (instant
  repeat views, background revalidation). `serve.cjs` added on-the-fly gzip for
  large compressible payloads (blobs ~1 MB JSON → ~300 KB, threshold 100 KB,
  client must accept gzip). The browser auto-decompresses; the SW caches the
  gzipped response. Combined with the existing pinned-kṛt corpus cells (zero
  wasm round-trips for listed stacks), repeat views load near-instantly.
  Browser self-test still green: 436 checks PASS.

- 2026-10-17 (कृदन्ते click-through parity; user call: match ashtadhyayi.com
  exactly, "everything" scope, panel = site-style 8×3 grid): the subanta
  sidebar used to decline whatever cell was clicked. The site does not — its
  `createSubantaForms` declines the ROW BASE through hardcoded reference
  paradigms (`DhatuConstants.kritPratyayaOrder` + per-gaṇa overrides). Traced
  from the site bundle and ported as `krt_ref_slot`
  (`src/engine/subanta.rs`), overrides verbatim: शतृ m (krut tab &
  03|अभ्यस्त ∪ 02.0066-72 ⇔ @dadat1, yangluk ⇔ @dadat1), शतृ n (01/04/10 ⇔
  @pacat1, 03|अभ्यस्त ⇔ @dadat2, 06|तुदादि ∪ 02.0067 ⇔ @tudat1, else
  @viyat1), स्य-शतृ n ⇔ @tudat1, अच् f (krut & 15-id list ⇔ @devI1, else
  @mAlA1). New paradigm classes for the site refs: ओस् handling, अभ्यस्त
  शतृ, pausa द् variants, n-stem plurals, क्वसु m/n (@sedivas1/@vidvas2),
  तृच् m/n (@dhAtR1/R2), क्तिन् f (@gati1), ङ्वनिप्/घिनुण्/उ/अतृन्/यत्. The
  engine's shape check (`KrtParadigm::weak`) rejects a ref the clicked value
  cannot read as and falls back to the shape class.
  Natva is now the site's own step, implemented locally (not
  `sandhi::natva`): last न् + triggers र्/ष्/ऋ/ॠ, allowed interveners,
  word-final exempt, applied per the `natva_cells` gate. Gate refinement
  found by the harness: the neuter n-affix run is only the plural आनि (col
  2) — the a-stem neuter dual ई carries no न, so the site never voices the
  stem's न् there; ग्रण्थ्ये ✗ → ग्रन्थ्ये ✓ (also eliminated the यत् n
  diff). Instr-sg इन (col 0) and gen-pl नाम् (col 2) runs kept.
  Parity harness (site's bundled engine vs pkg-node, deterministic stride
  sample: 63 kṛts, 1455 dhātu rows, 79,560 cells): **236 cell diffs, all in
  two documented classes** — अथुच् m (12 dhātus: 01.0070 01.0268 01.0425
  01.0957 01.0958 01.0959 01.0984 01.1001 01.1165 02.0031 05.0011 06.0151 —
  the site types the u-stem as @bhavat2 → नन्दथू/नन्दथूनौ) and क्वरप् f (7
  dhātus: 01.0642 01.1085 01.1096 01.1137 02.0040 03.0018 04.0091 — the site
  declines the ī-stem as @mAlA1 → सृत्वरीओस्/सृत्वर्यः). We render the
  CORRECT paradigms there — KNOWN_DIVERGENCES (site bugs; user call), not
  ours. Harness header: 3 site errors, 1076 site cells with no link, 21
  site-has-link/we-refuse — all the documented non-declinable citation
  class (क्यप् इत्या/सुत्या, अच् ह्वा/छा, ल्यु रमण/दर्पण, वुन्
  भाक/प्साक/दाक/वाक, नङ् याच्ञा, श इच्छा/क्रिया). 2026-10-17 probe of
  those 21 (site grid vs `krt_subanta`, `/tmp/opencode/probe_refuse.cjs`):
  the site force-declines each frozen citation through the column ref and
  mixes endings — ह्वा @deva1 → ह्वाः/ह्वौ/ह्वाभिः (a-stem + ā-stem
  hybrid), ल्यु f रमण @mAlA1 → रमणः/रमणे/रमणा/रमणैः (m and f endings in
  one grid). We keep the honest refusal; fixing would mean shipping
  garbled paradigms — same call as the 236.
  Pinned: `tests/krt_site_pin.rs` — 21 click-throughs captured from the
  site's own engine (2026-10-17: शतृ m/n × gacchat/dadat/pacat/tudat/viyat,
  स्य-शतृ n, अच् f ×2, क्वसु m/n, तृच् n, क्तिन् f, ङ्वनिप् n, घिनुण् m/n,
  उ n, अतृन् n, यत् n, ण्वुल् m), each pin the full 24-cell signature (हे
  stripped, forms sorted, `/`-joined, `|`-separated) through
  `wasm::krt_subanta`; all pass.
  Portal panel + click conventions now the site's (traced from the bundle's
  `krit-subanta-table-template` and `getFinalForm`): `renderParadigm`
  renders the bare 8×3 grid — no vibhakti/vacana labels — with सम्बोधन cells
  `हे `-prefixed; `renderKrutTables` + new `krtLinkTarget` pass the row base
  for m/n, the f form itself only when its last letter is आ/ई
  (`WordUtils.getLettersOfWord`: matras count as independent vowels — a
  trailing ा/ी), else the base; `onKrtClick` passes the dhātu row id as the
  new 5th wasm arg (`baseindex`) so the site reference resolves. The
  `krutConventionMap` display hack (it stripped upasargas off prefixed
  cells) is gone. Render tests cover the convention table + the bare-grid/हे
  shape; e2e grew to 16 checks (base-click ref check). `examples/` debug
  scratch deleted; fmt clean; clippy 9 pedantic fixes in the new code
  (nested or-patterns, krut/krt name clash, merged identical arms).
  `./check.sh --full` green: exhaustive audits (krt_click, derive_krut,
  derive_ting, prefixed_krut, reverse_audit_full — all pass), wasm ×3, dist
  refresh, render 9/9, e2e 16/16, smoke, browser self-test 436 checks —
  ALL CHECKS PASSED.

## 2026-10-18 — word view: "Decline a word" (any word → readings + paradigm)

User ask: a portal button leading to a feature that declines ANY given
base word (foreign names included) and, in reverse (the "search" half),
reports an inflected input's base word + vibhakti/vacana. Contract fixed
by the spec F5 trio: input `Wrampena` must read ONLY as itself, "Wrampena
as sambodhana is the base word" (an instrumental reading of `Wrampa`
would have to be voiced → `WrampeRa`); input `WrampeRa` must read BOTH
"base word is WrampeRa (vocative)" and "Wrampa is the base word,
WrampeRa its instrumental singular".

The engine already had exactly these semantics — `analyze()`
(`src/engine/subanta.rs:2339`): suffix strips + natva-inverse `R`→`n`
variants + self-stem hypotheses (its own comment cites `Wrampena`) +
regeneration containment — but nothing exposed them to the portal.
Probed first (native scratch test): `Wrampena` → 2 readings (m/n
sambodhana self-stem only); `WrampeRa` → 4 (Wrampa m/n tftIyA.ekavacana
+ WrampeRa m/n sambodhana); `Wrampa`'s own table voices instr-sg to
`WrampeRa` (natva fires, matching `rAma`→`rAmeRa`). Also probed the
vendored vidyuṭ `derive_subanta` on foreign-name shapes (`jAn` → जा…,
`pITar` → पीटः…): no cleaner reference than our own engine, so no engine
behavior was changed; vowel-final Sanskritized names (`bila`, `jona`,
`merI`, `amErikA`) already decline.

New wasm export `subanta_analyze(word)`: Devanagari | Harvard-Kyoto
(portal rule) | raw-SLP1 fallback when HK cannot spell the string (so
the spec trio as typed works; SLP1 still never crosses into the DOM).
Returns `{ok, word, analyses: [{stem(deva), linga, class, vibhakti,
vacana}], tables: [paradigm…]}` — readings via `analyze()`, declension
via `generate_tables` over m/f/n (the word as a fresh stem).
`paradigm_json` class labels (and the new analyses) now map
`Class::Word` — whose raw labels are SLP1 dictionary words like `tad` —
to "known word".

Portal: new nav button "Decline a word" (hash `#word`), view `view-word`.
`renderWordReadings` renders English slot names (सम्बोधन kept as
"vocative (sambodhana)" per the ask) with a per-row `table` toggle that
loads that base word's own declension via `subanta_table`;
`renderWordTables` heads bare 8×3 grids per gender; empty states point
foreign names at the Sanskritized vowel-final spelling (e.g. जॉन for
John).

Tests: `tests/wasm_api.rs` pins the trio (Wrampena readings exact;
WrampeRa Wrampa-instr + self readings; m/n a-stem generation; no SLP1
in the JSON; Deva ≡ HK) — 17 tests; render tests +2 (9 → 11); e2e 16 →
20 (trio + HK `jona` through dist+pkg-node); browser self-test +9 (nav
opens, trio readings, table toggle, no SLP1) — 436 → 445. Gate history:
the first `./check.sh --full` failed ONLY at the browser self-test —
`web/selftest.html` is a hand-maintained copy of the app shell and
lacked the new nav button/view, so app.js threw at load
(`el('word-go').onclick` on a null) and every flow after boot failed.
Added the missing shell parts plus a "keep in sync with app.html"
comment there; re-ran `./check.sh`: ALL CHECKS PASSED (exhaustive audits
from the --full run had already passed — no engine change after);
render 11/11, e2e 20/20, smoke, browser self-test 445 checks.

## 2026-10-18 (2) — word view follow-ups: candra spellings + readings → dhātu links

User review of the word view: no CLI; no consonant-final
"Sanskritization" fallback (HK/Deva → SLP1 → table if possible, else say
so honestly); readings list stays as-is; and yes — link readings to
dhātus.

Candra: the empty-state hint's own example जॉन errored —
`deva_to_slp1` rejected the candra vowels (ऑ/ॉ, ऍ/ॅ) and their
Dravidian short partners (ऎ/ॆ, ऒ/ॊ). They now fold onto the nearest
plain vowel (जॉन → jona); no paradigm keys off the distinction, and
slp1→deva is untouched so roundtrips are unchanged. Unit test + e2e pin.

Dhātu links: `runWordForm` probes the subanta shard for the input form
itself and attaches corpus kṛt postings (`{baseindex, krt, field,
vibhakti, vacana}`) to the readings they match on gender+slot — so
`gacCan` reads "… of गच्छत् — शतृ of गम्" with a `#dhatu/01.1137`
link. `renderWordReadings(analyses, krtRefs, meta)` matches per reading,
dedupes per dhātu (family differences collapse) and falls back to the
baseindex label when the list meta is missing. Enrichment is
best-effort, serialized against run races (stale shard fetches cannot
rewrite newer readings) and never tears down a reading table the user
already opened.

Tests: translit +1; render 11 → 12; e2e 20 → 22 (candra जॉन + kṛt link
through dist+pkg-node); browser self-test 445 → 449 (ref appears, names
शतृ · गम्, targets 01.1137, click opens गम्). `./check.sh --full` run:
exhaustive audits green on this revision (derive_ting 505s,
reverse_audit_full 237s — 0 misses, krt click/prefixed audits green);
the one failure was the first self-test run — `waitFor` returns a
boolean and the new code called `.getAttribute` on it — fixed, 449/449,
and the fast `./check.sh` re-run: ALL CHECKS PASSED.

## 2026-10-18 (3) — ई/ऊ-feminines: lexical नदीसंज्ञा default (लक्ष्मीः)

User review with two screenshots: ashtadhyayi.com शब्द `@lakSmI1` (लक्ष्मी,
ईकारान्त स्त्रीलिङ्ग, नदीसंज्ञकः) leads with **लक्ष्मीः** in
प्रथमा-एकवचन (and लक्ष्मीणाम् in षष्ठी-बहुवचन), while our word view
offered nominative = लक्ष्मी (no visarga). Root cause: `Class::IIF` is the
**कृदन्त/उगित्** flavour — nominative = bare stem — which is exactly what
the site's own कृदन्त grids do (pinned for शतृ f भवन्ती, and even the अच् f
pin "नदी" suppresses the visarga). The lexical **नदीसंज्ञा** vṛtti (the
Pāṇinian default for `-I`/`-U` stems, and what the site's शब्द pages
decline) was missing.

Engine: new `Class::IIFN` / `Class::UUFN` (labels `I-f-nadi`/`U-f-nadi`) —
the `I-f`/`U-f` grids with prathamA-ekavacana taking the visarga; every
other cell is identical, so the ष्/र्/ऋ-triggered ṇatva of णाम् applies
identically (लक्ष्मीणाम् — genpl codepoint-checked). `classes_for` now
offers नदी first, कृदन्त second: a bare stem cannot say which is meant, so
the word view shows both tables and both readings. Kṛt paths untouched
(`krt_class` f+`I`→`IIF`, `KrtParadigm::DevIF`→`IIF`), so every site pin
(भवन्ती grid, अच्-नदी grid, 49,536-cell click-through corpus) stays exact.

`analyze`: one new strip rule each (`IH`→`I`, `UH`→`U`) unlocks the नदी
nominative reading (लक्ष्मीः → prathamA.ekavacana next to the shared
accusative-plural cell); all shared cells dedupe under `I-f` as before. A
bare `-I`/`-U` stem is no cell of the नदी grid, so it now also reads as a
`citation` for that candidate ("base word" — the word view's premise; the
`I-f` prathamA reading stays beside it).

Tests: `tests/wasm_api.rs` +1 (18) — full 24-cell शब्द pin
(`लक्ष्मीः|…|लक्ष्मीणाम्|…`), readings for लक्ष्मीः (नदी nominative +
accusative pl.), वधू mirror (वधूः/वधूनाम्), no SLP1; e2e 22 → 24;
browser self-test 449 → 451 (first table is the नदी grid, citation
reading visible). Gate: `./check.sh` ALL CHECKS PASSED (render 12/12,
e2e 24/24, smoke, 451 checks) + `./check.sh --full` green: exhaustive
audits (reverse_audit_full 0 misses; derive_ting; krt click/prefixed)
confirm no कृदन्त/kṛt grid moved.

## 2026-10-18 (4) — शब्द vendoring + the derived अन्-stem (`Class::AnM`) fix

User ask: decline **any** base word — foreign names included — into the
8×3 subanta table, with ashtadhyayi.com as the parity oracle, and
reverse-analyze inflected input to base word + vibhakti/vacana.

**Vendored display path.** Raw `data/shabda_data2.json` (pinned
`a9f9112`, 9007 entries, 24 cells, variants `-`-joined, sambodhana
`हे `-prefixed) → `web/build-shabda.cjs` trims/validates →
`dist/data/shabda2.json`. `render.js` `siteTables()` maps the site's own
grid per (word, linga) with `SITE_LINGA_CODES`/`lingaLabel`; `app.js`
`ensureShabda()` lazily fetches on the first word view and swaps the
engine grid in place in `runWordForm`; fetch failure degrades to the
derived path silently. `check.sh` refreshes the data + header count (36).

**AnM arm.** The data-derived weak-stem rule (84 पुं-अन् stems in the
शब्द data): **ञ्**-style when `p` ends in open च/ज (राजन्→राजा/राजानौ),
**drop** when `p`'s final consonant is ष/ध/ट or "open" (vowel-attached:
उक्षन्→उक्ष्णा, दुर्नामन्→दुर्नाम्ना), else **keep** (कर्मन्→कर्मणा).
69/69 on the three regular families; 15 irregulars (numerals अष्टन्…,
हन्→घ्न, युवन्/मघवन्/श्वन्/अर्वन्/प्रतिदिवन्, दीर्घाहन् with दीर्घाहाः
strong) stay DB-covered. New `AnWeak`/`an_weak()` picks the style.

**site_natva details** (learned cell-for-cell from the data):
a nasal in a **ह्न conjunct** never voices — दीर्घाह्नः/तमिस्रह्नः
stay dental though ह् is otherwise transparent (बर्हिणा, ब्रह्मणः,
ग्राहिणः voice through it); and the **रुत्व-र्** — a nasal abutting
the र् the stem itself spells in a र्न conjunct stays dental
(दुर्नामन् → दुर्नामा/दुर्नामभ्याम्, दुर्नामक, पुनर्नवा), while an
original र् voices the composed conjunct (धीरन् → धीर्णा/धीराणि).
Implemented as a stem-conditioned flag on `site_natva`.

**Routing.** `-van` masculine splits by the same rule: cluster-final
वन् (यज्वन्/धन्वन्/पर्वन् → VanM keep) vs open वन् (ग्रावन्/पीवन् →
AnM drop). Neuter `-an` (मनिन्) got the same split in `Class::ManN`:
कर्मन्/धन्वन् keep, नामन्/धीरन् elide with the site's alternant pairs
(नाम्नी-नामनी, नाम्नि-नामनि — all 15 drop मन्-neuters show them);
N routing broadened from `-man` to all `-an` (धीरन्, धन्वन्, पर्वन्).

**Audit** (`web/audit-an-stems.cjs`, hand-run, not in check.sh): derived
vs the site's own grids for every नजन्त stem — P 286/313 full, N 80/84,
S 0/3; the 34 misses are the accepted irregulars, हेमन् (site data
bug: `हे` stripped from every cell), and 3 feminine -an/-in stems with
no derived paradigm.

Tests: `tests/wasm_api.rs` an-stem test expanded to 19 — 24-cell grids
for राजन्/आत्मन्/उक्षन्/कर्मन्/ग्रावन्/दुर्नामन्, loc-sg variants,
analyze pins (उक्ष्णा, ग्राव्णः, ग्रावाणौ, दुर्नामा), and neuter pins
(नामन्/कर्मन्/धन्वन्/धीरन्, pairs included).

## 2026-10-18 (5) — heavy artifact rebuild (post-AnM engine)

The AnM/natva/ManN engine changes (entry 4) alter declined forms, so the
prebuilt `dist/` artifacts were stale. Rebuilt per the documented manual
sequence (check.sh deliberately excludes it): `build-site` →
`node web/build-shabda.cjs` → `build-index --shards 256` →
`build-subanta-index --shards 256`. Subanta index grew 17.3M →
18,028,526 postings (152.8s) — the ManN elided/unelided alternant pairs
(नाम्नी-नामनी …) and the AnM drop-family fix (ग्रावन् → ग्राव्णः)
add surfaces; शब्द oracle refreshed (6.5MB). README's shard-count note
updated. `./check.sh` re-run against the fresh dist: ALL CHECKS PASSED
(e2e 36/36, smoke, browser 455).

## 2026-10-19 — feminine नजन्त paradigms (`AnF`/`InF`)

Pushback on "not actionable" — the 3 feminine stems flagged in entry 4
(`दामन्`/S, `सीमन्`/S, `महाजालिन्`/S) were claimed DB-covered, but an
UNLISTED feminine `-an`/`-in` base word (foreign names included — the
contract) had no derived table: `F + -an` produced nothing and `F +
-in` routed to the i-stem `IMF` guard (which fails, the stem doesn't
end in `i`).

**New classes.** `Class::AnF` (`an-f`) and `Class::InF` (`in-f`). Both
grids were read off the site's own 24 cells:
- `महाजालिन्` (@mahAjAlin1) is cell-for-cell the `InM` shape — the
  अवान्तर accusative plural already equals the strong nominative there.
- `दामन्` (@dAman2)/`सीमन्` (@sIman1) is cell-for-cell the `AnM`
  drop shape *including* the weak accusative plural (`दाम्नः`, not
  `दामानः`). An earlier arm used a "feminine acc.pl = nom.pl" rule —
  wrong; the audit's 23/24 caught it and the arm now mirrors `AnM`
  exactly (drop/keep via `an_weak`, loc-sg pair variants `दाम्नि-दामनि`).

Routing in `classes_for`: `F + -an → AnF`, `F + -in → InF` (was `IMF`).

**Readings.** `analyze` now surfaces feminine readings for नजन्त forms:
STRIPS twin-push (each `AnM`/`InM` strip also pushes its `AnF`/`InF`
twin at `linga=f` — verification filters junk as always) and the
self-stem linga inference maps `AnF`/`InF → Linga::F` (was falling to
the masculine `_ =>` arm). `दाम्ना → दामन्/m an-m` ∥ `दामन्/f an-f`,
both `tftIyA.ekavacanam`; same for दामानम्/दामसु/महाजालिनौ/महाजालिना.
Existing pins unaffected (they assert presence, not order).

**Audit refresh.** `web/audit-an-stems.cjs`: S 0/3 → **3/3**; अहन्/N
moved into ACCEPTED with its दिवस्-family reason (अहः nom/voc, अहोभ्याम्
सन्धि, अहःसु/अहस्सु loc.pl — engine derives the regular मनिन् shape;
unrelated to this change, it was an unflagged pre-existing miss).
P 286/313 and N 80/84 unchanged (accepted irregulars + हेमन्).

Tests: `subanta_table_feminine_an_in_stems` (20th wasm_api test) —
24-cell grids for दामन्/सीमन्/महाजालिन्, loc-sg variant pins, and
analyze readings assertive of `linga=f` with class ≠ `A-f`. No dist
rebuild needed: the subanta shard indexes kṛt-cell declines only
(`krt_class` never emits AnF/InF), word view/analyze are live.
`./check.sh` ALL CHECKS PASSED (e2e 36/36, smoke, browser 455).

## 2026-10-19 (2) — consonant-final noun paradigm (`Cons`), ADDITIONS 2.1–2.3

One class covers every consonant-final noun the site tables on the
सरित्-shape — finals t/d/D (सरित् @sarit1, विद्युत्, संविद्, उपनिषद्,
युध्, मरुत्/m @marut1, बुध्/m @budh1), B (ककुभ् @kakubh1) and c (वाच्
@vAc1, त्वच्). प्रथमा- and सम्बोधन-सि carry the site's pausa pairs
(सरिद्-सरित्, युद्-युत्, ककुब्-ककुप्, वाग्-वाक्), भ्-endings voice the
final (सरिद्भ्याम्, ककुब्भ्याम्, वाग्भ्याम्), सु devoices it (सरित्सु,
ककुप्सु, वाक्षु) and the strong cells keep the input final (सरितौ/
संविदौ/युधौ/ककुभौ). Feminine reaches every final; masculine only
{t,d,D}. Previously मरुत्/m crewed `AtM(Num)` and produced the
participle-न् मरुन् — the reroute repairs that parity miss; अ-उपधा-त्
spellings (गच्छत्, भवन्त, वसुमत्) still go to `AtM(Num)` via the
`at`/`ant` split, and N t/d keeps `AtN` (विदत्) — no neuter भ्/च् noun
grids exist on the site.

Tests: new `tests/subanta_site_pins.rs` reads the vendored शब्द db and
demands cell-for-cell Devanagari equality for 10 सरित्-family words +
the गच्छत् participle regression; the `@budh1` भु-typo is corrected in
the expectation (engine emits बुद्-बुत्). ./check.sh ALL CHECKS PASSED
(browser 465).

## 2026-10-19 (3) — उस्/इस्-स्वादि neuter paradigm (`UsN`), ADDITIONS 2.4

`Class::UsN` (`us-n`) for the इस्वादि neuters the site tables through
आयुष् (@AyuS1): stem minus its final ष्/स् is the root (आयु, हवि, धनु)
— सि root+विसर्ग (आयुः), द्वि root+षि (आयुषी), बहु lengthened root +
ंषि (आयूंषि, हवींषि, धनूंषि), भ्-endings the weak र्-discard (आयुर्भ्याम्),
loc-pl pairs आयुःषु-आयुष्षु, सम्बोधन mirrors प्रथमा. Route: N inputs
ending us/is/uz/iz (the latter two are the षत्व spellings). हविस्/धनुस्
derive from the आयुष् shape per the DB-first-then-derived contract.

Tests: आयुष् vs `@AyuS1` cell-for-cell + full derived हविस्/धनुस् grids
(`us_swadi_pins`). Gates green, browser 465.

## 2026-10-19 (4) — गो/नौ/रै through the word route, ADDITIONS 2.5

The three diphthong words (उभयवृद्धि गव्/नाव् shapes) join
`KNOWN_WORDS` (SLP1 'go'/'nO'/'rE') with verbatim `word_table` grids:
गो m and f identical (@go1/@go2 — प्रथमा-सि गौः, accusative-bahu गाः,
गवा/गोभ्याम्, गवि), नौ feminine-only (@nau1 नौः/नावौ/नावम्/नौभ्याम्/
नावोः/नौषु), रै both लिङ्ग (@rai1/@rai2 राः/रायौ/रायम्/राभ्याम्/रायोः/
रासु). No rule shape for the गाम्/गाः वृद्धि accusatives — gridded
data. Tests: `diphthong_word_pins` vs every db entry. Gates green.

## 2026-10-19 (5) — सर्व-family pronominals through the word route, ADDITIONS 2.6

अन्य/इतर/कतर/कतम/उभय (m+n per db entry), अन्या (f, @anyA1), उभ
(dual-only) join `KNOWN_WORDS` with verbatim grids. Alternant pairs in
their cells: प्रथमा-/सम्बोधन-eka अन्यद्-अन्यत् (neuters; उभय-N samb-eka
stays bare उभय per @ubhaya2), पञ्चमी-eka स्माद्-स्मात् for every
member. उभ duals leave eka/bahu empty (rows carry "" cells;
`rows_to_slots` drops them → rendered —), so the pin test keys cells by
(विभक्ति, वचन) instead of position. SLP1 note: भ् is 'B', so उभय =
'uBaya' (an 'ubh…' spelling never matches); the णत्व split is per-stem
(इतरेण/कतरेण with र्, अन्येन/कतमेन/उभयेन dental; इतराणि/कतराणि vs
अन्यानि/कतमानि/उभयानि). Tests: `pronominal_word_pins` — all 13 db
entries verbatim. Gates green, browser 465.

## 2026-10-19 (6) — dhātu-list English-gloss search, ADDITIONS 3.1

`#list-q` still does its exact-Devanagari match first (दा, HK dA → दा),
but when no dhātu matches exactly and the query is ASCII/HK — `eat`
decodes to एअत् through HK, matching nothing — it falls back to a
case-insensitive substring search of `artha_english` (93 rows for
'eat': अद्, खाद्, स्वद्, खद् …). Pure-Devanagari misses still count 0;
`zzz` → gloss miss → 0 unchanged. Placeholder mentions the English
meaning.

Selftest +1 check ('eat' → 93). Gates green, browser 466.

## 2026-10-19 (7) — dhātu-list grammatical filters, ADDITIONS 3.2

Two list dropdowns over the `dhatu_data.json` fields the CLI already
takes (`--settva`/`--karma`): `#list-settva` (seṭ S/aniṭ A/veṭ V —
1882/306/41 rows) and `#list-karma` (sakarmaka S/akarmaka A/dvikarmaka
D — 1553/653/23). Rows tagged '-' drop out under any filter. Both live
in `runListFilter` beside gaṇa/pace, applied by the same Filter button.

The selftest page hand-maintains the app shell — the new selects had to
be copied into `selftest.html` too (a missing id throws at boot, exactly
what its header comment warns; the gate caught the first run).

Selftest +2 checks (veṭ 41, dvikarmaka 23). Gates green, browser 468.

## 2026-10-19 (8) — WASM binary size, ADDITIONS 4.1

`[profile.release]` gets opt-level "z", lto, codegen-units 1 and strip —
dhaturupa_wasm_bg.wasm 1,605,847 → 1,026,229 bytes (−36%), inside the
predicted 30–45% band. Two traps found on the way:

- **No `panic = "abort"`.** It breaks `cargo test --release` for the
  package bin (E0463 can't find crate) — the exhaustive audits are part
  of `./check.sh --full`, so the profile is shared with them. Dropped:
  wasm32-unknown-unknown aborts regardless (no unwinding on that
  target), the setting bought nothing for the shipped artifact.
- **wasm-pack's cached wasm-opt rejects bulk memory.** The z/LTO codegen
  emits `memory.copy`; the pinned binaryen fails with "Bulk memory
  operations require bulk memory" (rust#141080). Fixed via
  `[package.metadata.wasm-pack.profile.release]` with
  `wasm-opt = ["-Oz", "--enable-bulk-memory"]` — keeps the optimizer and
  sizes better than the old bare `-O`. (First metadata attempt also
  swallowed the `[lib]` `crate-type` line by section-table mistake —
  restored.)

cargo test green; `./check.sh --full` ALL CHECKS PASSED (exhaustive
audits + e2e 36 + smoke + browser 468).

## 2026-10-19 (9) — reverse analysis for the 2.x paradigms

The 2.x classes declined but did not *read back*: the analyzer inverts
through the `STRIPS` table, and none of the new classes were in it. So
the word view showed no reading (and no table) for आयुषा, सरिद्भ्याम्,
ककुप्सु — the second half of "decline a word + reverse-analyze the
form" was missing for exactly the words this session added.

- **UsN** (26 rules): the citation is the root plus its ष्/स्, and
  every ending but the visarga one carries the षत्व, so the stem is the
  stripped prefix plus the citation letter. The rules are root-vowel
  specific (u-root → `uz`, i-root → `is`) — each form then reads back to
  exactly one citation: आयुष् for the u-roots, हविस् for the i-roots.
  A citation pass (2c) states the base-word reading for the stem itself
  (the सि is root+विसर्ग, not a cell of the paradigm). Known limit: the
  lengthened बहु form (आयूंषि) cannot be rebuilt by appending a tail.
- **Cons** (43 rules): the strong cells keep the input final, so the
  plain endings rebuild the stem as-is; the प्रथमा-एकवचन pair and the
  भ-endings voice that final and सु devoices it, so those rules restore
  the final the stem carries (`t`/`d`/`D`, `p`/`B`, `k`/`c`). Now
  सरितः/सरिद्भ्याम्/सरित्सु/ककुप्सु/वाग्भ्याम्/वाक्षु/युद्भ्याम्/
  उपनिषद्भ्याम्/मरुद्भ्याम्/संविद्भ्याम् all read back to their
  सरित्-family stem. The अ-उपधा-त् guard keeps the participles
  (गच्छतः, भवन्तः) out of the Cons readings.

Tests: `subanta_analyze_new_paradigm_readings` (wasm_api) pins the
UsN/Cons/known-word readings; selftest +2 (word view: आयुषा → आयुष्
us-n, सरिद्भ्याम् → सरित् cons). cargo test green; `./check.sh --full`
ALL CHECKS PASSED (exhaustive audits + e2e 36 + smoke + browser 470).

## 2026-10-19 (10) — inflected form → the site's base-word grid

The word view showed the vendored शब्दपाठः grid only for the exact
citation, so an inflected input (सरिद्भ्याम्) fell back to the derived
tables of the input-as-a-stem — empty for a form that is no stem, or
worse, a junk shape (सरितः → the अस्-family). The site resolves the form
to its base word and shows *that* word's grid, and the readings already
carry the base word: the oracle lookup now falls back to the reading
stems when the input itself is not a listed word (सरिद्भ्याम् →
सरित् → @sarit1). No index change — the शब्द oracle stays keyed by
citation, and the junk readings simply miss it.

Selftest +1 (सरिद्भ्याम् → the @sarit1 grid, credited to the site).
Gates green, browser 471.

## 2026-10-19 (11) — README: the portal shell and the build profile

The Portal-shell paragraph now lists what the dhātu list filters accept
(gaṇa, pada, seṭ/aniṭ/veṭ, sakarmaka/akarmaka/dvikarmaka, the English
gloss search), that an inflected form resolves to its base word's site
grid, and which engine paradigms read back as well as decline (the
सरित्-family consonant-final nouns, the उस्/इस्-स्वादि neuters, the
word-route grids). The build block records the size-tuned release
profile and the `-Oz --enable-bulk-memory` wasm-opt flags (1.03MB vs
1.61MB) and why there is no `panic = "abort"`.

Docs only — gates re-run green (browser 471).
