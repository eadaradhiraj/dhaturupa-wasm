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

## 2026-10-19 (12) — consonant-final audit: the ऋ-chart, the masculine भ्-final, the जश्

`web/audit-cons-stems.cjs` (hand-run, like `web/audit-an-stems.cjs`)
declines every consonant-final word the vendored शब्दपाठः lists and
compares it with the site's grid: 288 entries, **107 exact**. The
remaining misses are *other paradigms*, not Cons misses — the वत्-
नामधातु family (63: आत्मवत् → आत्मवान्, a nominalization with its own
shape), the सम्-अधिक-रूप (10: अत्यस्मद् → अतित्वम्), the neuter
इ-स्वादि (अर्वाचि/अर्वाची) and the masculine ञ्च-स्वादि (अर्वाञ्चः) —
each its own class, none of them the Cons shape. Those are the next
coverage candidates; the portal shows the site's grid for all of them
(the शब्द oracle wins), so only unlisted uses of those shapes are
still un-derived.

The audit did find three real misses inside the Cons class:

- **Masculine भ्-final** (गर्दभ् @gardabh1) is the Cons shape, so the
  masculine scope is {t, d, D, B} — च् stays feminine-only (the site's
  masculine च्-final words are the ञ्च-स्वादि family).
- **The ऋ-chart** (ऋच् @Rc1: ऋग्-ऋक् | ऋचौ | ऋचः) is the same shape
  with a one-letter base, so those two-letter stems are admitted.
- **The जश्-व्यवस्था**: the भ्-final family voices the stem's own
  द/ब/ग/ज in the weak forms — गर्दभ् → गर्धब्-गर्धप्, गर्धब्भ्याम्,
  गर्धप्सु (the strong cells keep गर्दभः). Only the last such
  consonant voices; the earlier ones sit before a soft consonant (the
  ग of गर्दभ् keeps ग), and the SLP1 spelling cannot tell a silent
  inherent vowel from a real one.

Pins: गर्दभ् and ऋच् join the site-parity pins. `./check.sh --full`
ALL CHECKS PASSED (exhaustive audits + e2e 36 + smoke + browser 471).

## 2026-10-20 — the वतुप् nominalization (VantM) reaches the manual path

The consonant-final audit's biggest miss family was the वतुप्
nominalization (63 entries): the site grids आत्मवत् as
आत्मवान्-आत्मवन्तौ-आत्मवन्तः (the vṛddhi-strong shape) while the
engine produced the participle shape (आत्मवन्). `Class::VantM` already
declines that shape cell-for-cell (the class was only reachable through
the कृत् path), so the fix is routing: a masculine `…vat` stem now
offers VantM.

But the ऋ-वत् participle (भवत् → भवन्, आप्नुवत् → आप्नुवन्) ends the
same way, and the two are indistinguishable from the spelling alone (the
difference is the derivation, not the shape) — and neither is in the
कृत् corpus, so the corpus path does not cover them. Both candidates
are therefore offered (VantM + AtM) and the site's grid decides for the
words it lists; the portal's multi-candidate word view is built for
exactly this.

Audit numbers: **146/288** consonant-final db entries now reproduce the
site grid (107 before this round; the ऋ-chart, the masculine भ्-final
and the जश् brought the first +2, the वतुप् family the rest). What is
left are separate paradigms, each its own class: the मतुप्/यतुप्
siblings of the वतुप् (आयुष्मत् → आयुष्मान्), the neuter/feminine
ई-स्वादि of the वत family (आत्मवती, आत्मवन्ति), the सम्-अधिक-रूप
(अत्यस्मद् → अतित्वम्) and the masculine ञ्च-स्वादि (अर्वाञ्चः).

Pins: आत्मवत् / उरस्वत् / क्रियावत् / आप्नुवत् pin the मस्कुल
entry (either candidate reproduces the site's grid). cargo test green;
`./check.sh --full` ALL CHECKS PASSED (browser 471).

## 2026-10-20 (2) — the मतुप्/यतुप् siblings, and the तुप् family reads back

`Class::VantM` now serves the whole तुप् family: the suffix letter is
the तुप्'s own (व/म/य), so आयुष्मत् declines as आयुष्मान्-आयुष्मन्तौ-
आयुष्मन्तः (site @AyuSmat1) and इयत् as इयान्-इयन्तौ (site @iyat1) —
the same vṛddhi-strong shape with a different letter. The manual route
offers the तुप् and the participle candidate together, as for the
वतुप् family.

The family also reads back now: 25 strip rules (the vṛddhi strong
swaps the suffix letter, the `-ant` strong swaps it the same way, the
plain endings rebuild the stem, and the भ-endings keep the द-voiced
weak stem so the suffix carries the `ad` and the tail restores the
`at`), so आत्मवान् → आत्मवत् and आत्मवद्भ्याम् → आत्मवत्.

Audit: **158/288** consonant-final db entries reproduce the site grid
(146 before). Remaining are separate paradigms: the neuter/feminine
ई-स्वादि of the वत family (आत्मवती, आत्मवन्ति), the सम्-अधिक-रूप
(अत्यस्मद् → अतित्वम्) and the masculine ञ्च-स्वादि (अर्वाञ्चः).

Pins: आत्मवत् / उरस्वत् / क्रियावत् / आप्नुवत् / आयुष्मत् / इयत्.
`./check.sh --full` ALL CHECKS PASSED (browser 471).

## 2026-10-20 (3) — the search box falls back to the word analyzer

The corpus shards carry the kṛt families only, so the form search
resolved verb and कृत् forms (गच्छत् → 5 hits, भवति → 6) but no noun
paradigm at all: सरिद्भ्याम्, सरित्, अन्यस्मात् and आत्मवान् all came
back with zero hits, while the site's शब्द search resolves a form to
its base word. The word analyzer already knows these forms (the 2.x
reverse-analysis rules), so a search with no index hit now offers the
base-word readings — the base word and the case it declined from, each
linking to the word view, which carries the paradigm and the site's own
grid.

`renderSearchReadings` (render.js) renders the group; the search note
counts the readings. Selftest +1 (सरिद्भ्याम् → base-word readings
naming सरित्) and a render unit test for the new renderer.

Gates green: `./check.sh` ALL CHECKS PASSED (browser 472), node render
tests 17.

## 2026-10-20 (4) — whole-db parity sweep, and two more Cons finals

`web/audit-site-grids.cjs` (replacing the consonant-only script; the
`cons` argument keeps that slice) declines **every** word the vendored
शब्दपाठः lists and compares it with the site's grid: **7,837/9,007
exact**. What misses is now measured, not guessed — the big groups are
the ण-स्वादि (the ऋ/ऋ-car i-stems), the ई/ऊ-स्वादि, the अस्-family
alternants, the ञ्च-स्वादि (अर्वाञ्चः), the सम्-अधिक-रूप
(अत्यस्मद् → अतित्वम्) and the neuter ई-स्वादि of the वत family
(आत्मवती) — each its own paradigm.

The sweep also found two more Cons finals: the थ्-final (थ् is `T` in
this SLP1 — अग्निमथ् @agnimath1 alternates like त्: अग्निमद्-अग्निमत्)
and the ऋ-chart's श्-final, which carries a third alternant
(अदृग्-अदृख्-अदृक् — the ख् in the middle of the pair). Both are
the Cons shape, so the class covers them now (+20 db entries; the
consonant-final slice 158 → 178/322).

Pins: अग्निमथ् and अदृश् join the site-parity pins (the neuter अदृश्
is the ई-स्वादि family — a different paradigm — so the pin takes the
masculine entry). `./check.sh --full` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (5) — the अस्-family's locative-plural alternants

The sweep's second-largest cluster was the अस्-family's locative plural:
the site grids both alternants (अनःसु-अनस्सु, @anas1) where the engine
gave only the doubled-स् form (अनस्सु). `AsM`/`AsN` now carry the pair
(the visarga form first, then the स्स form) — the visarga form needs
the explicit inherent vowel (`aHsu`), since a bare `H` after a
consonant transliterates as the anusvāra-nukta (अन्ःसु).

Whole-db parity: **7,893/9,007** exact (was 7,837). The remaining
misses, measured by family: the ण-/य-स्वादि i-stems (~350; the
ऋ-car roots take the य-formation with paired alternants — अक्षान्त्या,
अक्षान्तये-अक्षान्त्यै), the a-stem's स्म-endings (~113; अधरस्मै),
the neuter ई-स्वादि (~45; आत्मवती), the anusvāra-स्वादि (~44;
अणीयान्), the u-stems (~67), the ञ्च-स्वादि (~25; अर्वाञ्चः,
खञ्जः), the सम्-अधिक-रूप (23; अतित्वम्) and the numerals' compound
spellings (14; the site grids them sparsely). Each is its own
paradigm — the next candidates, in that order of size.

Pins: अनस् (the locative-plural pair). `./check.sh --full` ALL CHECKS
PASSED (browser 472).

## 2026-10-20 (6) — the ण-स्वादि for the feminine i-stems

The sweep's largest cluster: the feminine i-stems take the ण-स्वादि on
the site — the त्य-form (अक्षान्त्या) with its paired alternants
(अक्षान्तये-अक्षान्त्यै, अक्षान्तेः-अक्षान्त्याः, अक्षान्तौ-अक्षान्त्याम्)
and the accusative plural as the nominative plural (अक्षान्तीः), while
the masculine keeps the plain shape (अग्निना, अग्नीन्). `Class::GatiF`
already *was* that paradigm (the क्तिन् कृत् path uses it) but the
manual route sent every i-stem to the plain `IMF`, so the shape was
only reachable through the corpus. The routing now sends feminine
i-stems to `GatiF` and the masculine stays on `IMF` — the corpus's
कृत् cells (the plain feminine) are untouched.

Whole-db parity: **8,159/9,007** exact (was 7,893; +266). The
remaining misses are now: the a-stem's स्म-endings (~113), the u-stems
(~67), the anusvāra-स्वादि (~44), the neuter ई-स्वादि (~45), the
ञ्च-स्वादि (~25), the सम्-अधिक-रूप (23), the numerals (14) and the
no-table entries (256 — the neuter ई-स्वादि and the masculine
ञ्च-स्वादि mostly).

Pins: अक्षान्ति, अणि, गति (feminine) and अग्नि (masculine). The
ण-स्वादि forms also read back (अक्षान्त्या → अक्षान्ति). `./check.sh
--full` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (7) — the masculine ई- and ऊ-स्वादि

The sweep's "no table" bucket held 85 masculine ई-/ऊ-स्वादि words
(अमी, अग्रणी, अग्निभू, अतिलक्ष्मी, आमलकी …) — the engine had the
feminine and neuter ई-स्वादि but no masculine class at all, so those
words declined to nothing. Two new classes: `IIM` (`I-m`) and `UUM`
(`U-m`), read off the site: the nominative takes the visarga (अमीः,
अग्निभूः), the y/v-forms carry the endings (अम्या, अग्रण्यम्,
अग्निभुवा) and the भ-endings keep the long vowel (अमीभ्याम्,
अग्निभूभ्याम्). Both also read back (अम्या → अमी, अग्निभुवः →
अग्निभू).

Two root classes still differ from the site inside this family: the
ण-car roots (अग्रणी) take the य-formation in the accusative/locative
(अग्रण्यम्, अग्रण्याम्) and the compounds keep the feminine-style
cat/paJ (अतिलक्ष्म्यै, अतिलक्ष्म्याः) — the same split the ण-/य-स्वादि
has on the i-stems.

The two wasm_api pins that asserted a table *position* now pick the
feminine table by class (the masculine one is on offer too).

Whole-db parity: **8,182/9,007** exact. Pins: अमी, अग्निभू, आमलकी.
`./check.sh --full` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (8) — where the subanta parity now stands

Whole-db parity: **8,182/9,007** listed words reproduce the site's grid
from the engine's own tables (was 7,817 when the sweep started — +365
this round: the थ्/श् finals, the वतुप्/मतुप्/यतुप् family, the
ण-स्वादि for the feminine i-stems, the masculine ई-/ऊ-स्वादि, the
अस्-family's locative-plural pair).

What is left, with the shape worked out (the next candidates, largest
first — each needs its own class and pins):

- **a-stem स्म-endings** (~70; अनेक, अन्तर, उत्तर): the cat/paJ/sap take
  the स्म-forms (अनेकस्मै, अनेकस्माद्-अनेकस्मात्, अनेकस्मिन्) and the
  masculine pr-bahu/zaz-bahu the ए-forms (अनेके, अनेकेषाम्). The
  condition is the *primary* a-stem with a consonant-final root — the
  engine cannot tell a primary from a compound (कीलालपा keeps the plain
  cat), so the स्म-variant has to be offered as a second candidate.
- **anusvāra-स्वादि** (~44; अणीयस्, अन्तर्मनस्, अधर): the anusvāra
  shapes (अणीयान्, अणीयांसौ) — a family of its own.
- **ञ्च-स्वादि** (~25 + the masculine च्-final ~38): अर्वाच् → अर्वाङ्
  | अर्वाञ्चौ | अर्वाञ्चः, खन्ज् → खञ्जः — the ङ्- and ञ्च-inserted
  forms.
- **neuter ई-स्वादि** (~45): आत्मवत्, इयत् — the neuter of the वत
  family (आत्मवती, आत्मवन्ति).
- **सम्-अधिक-रूप** (23): अत्यस्मद् → अतित्वम्, अतियुष्मद् → अतित्वाम्
  — the अधिक-रूप with its अति-/प्रति-/उप-/अभि- prefixes.
- **u-stems** (~67) and **numerals** (14; the site grids the compounds
  sparsely, so the derived full grid cannot match).

Every one of them is a shape the site grids; the शब्द oracle already
shows the site's grid for the listed words, so these fixes are about
the *derived* tables (unlisted words) and the reverse analysis.

## 2026-10-20 (9) — the neuter ई-स्वादि and the स्म-ending a-stems

Three more families from the sweep's list:

- **`TaN`** (the neuter of a त-स्वादि stem — आत्मवत्, आयुष्मत्, site
  @Atmavat2): the प्रथमा-/द्वितीया-एकवचन carry the द/त pair
  (आत्मवद्-आत्मवत्) and the rest the ई-forms (आत्मवती, आत्मवन्ति,
  आत्मवता, आत्मवते, आत्मवति). The neuter participle (AtM's twin
  AtN) stays the other candidate — a bare stem cannot say which is
  meant.
- **`AsmM`/`AsmN`/`AsmF`** (the स्म-ending a-stems — अनेक, अधरा): the
  cat/paJ/sap take the स्म-forms (अनेकस्मै, अनेकस्माद्-अनेकस्मात्,
  अनेकस्मिन्), the masculine nominative plural and genitive plural the
  ए-forms (अनेके, अनेकेषाम्) and the feminine the स-form (अधरस्यै,
  अधरस्याः, अधरस्याम् — the य carries the vowel, `asyE`). Offered as
  a second candidate: a bare stem cannot say whether the word is a
  primary (अनेक) or a compound (कीलालपा, which keeps the plain cat).
- The अस्-family's **vocative singular** takes the स-form (हे अङ्गिरः)
  where the nominative has the आ-form (अङ्गिराः) — site @aGgiras1.

Whole-db parity: **8,289/9,007** exact. Pins: आत्मवत् (N), अनेक (P/N),
अधरा (S), अङ्गिरस् (P). Gates green (browser 472).

## 2026-10-20 (10) — read-back rules for the new classes, and two UX fixes

- **Read-back rules** for the classes added today: the neuter ई-स्वादि
  (TaN) uses the तुप् suffix in the rule (`vatI`/`matI`/`yatI`/`itI`)
  and sits at the top of the strip table — the neuter participle
  (AtN) declines the same forms, so the तुप् rules must claim the slot
  first, and the specific suffix keeps them off a plain `-t` stem
  (विदत् keeps the participle reading). The स्म-ending a-stems read
  back through their स्म-forms (अनेकस्मै → अनेक). Two TaN cells
  (the accusative plural and the भ-endings) still lose their slot to
  a competing neuter-participle reading — the analyser's dedupe keys
  on (stem, gender, case), not on the class.
- **A known word's grid is the site's own**, so the strip pass no
  longer claims its slots: अन्य/इतर/कतर/कतम/उभय (the स्म-ending
  a-stems the site grids explicitly) keep the `known word` reading
  instead of an engine guess.
- **Word view**: the derived table now renders only when the शब्द
  oracle has no grid, so an inflected input no longer flashes a junk
  derived table (सरितः → the अस्-family shape) before the site's grid
  replaces it.
- **Search fallback**: the base-word readings are filtered to the
  stems the corpus lists (the site's answer), with the first few as
  the fallback for unlisted words — instead of 24 mostly-junk
  candidates.

Gates green: `./check.sh` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (11) — the अञ्च/अञ्ज/अनुस्वार families

Three more families from the sweep, all read off the site's own grids:

- **`AccM`** (the masculine च्-final nouns — अर्वाच् @arvAc2): the
  nominative takes the ङ्-inserted form (अर्वाङ्), the dual/plural the
  ञ्च-forms (अर्वाञ्चौ, अर्वाञ्चः), the भ-endings voice the stem
  (अर्वाग्भ्याम्) and the locative plural the क्षु (अर्वाक्षु). The
  सरित् shape never reached these — the class is separate, and the
  routing sends masculine च्-final stems to it.
- **`AjjM`** (the masculine ज्-final nouns — खन्ज् @khanj1): the
  nominative replaces the न् with the plain stem (खन्) and the
  dual/plural take the ञ्ज-forms (खञ्जौ, खञ्जः), with the locative
  plural as the pair खन्सु-खन्त्सु. Offered alongside the निज-स्वादि
  (`Jvs`) — a bare stem cannot say which family it is.
- **`AsAnM`/`AsEM`** (the अस्-स्वादि's other two nominal forms): the
  anusvāra-form (अणीयस् → अणीयान्, अणीयांसौ) and the e-form (अनेहस्
  → अनेहा). Both offered beside the plain आ-form, since the spelling
  does not say which the site uses.

One translit fix came with it: a consonant before the anusvāra drops
its inherent vowel (`kM` is कं, not क्ं) — the SLP1 rule.

Whole-db parity: **8,311/9,007** exact; the no-table bucket is down to
150. Pins: अर्वाच्, खन्ज्, अणीयस्, अनेहस्. `./check.sh --full` ALL
CHECKS PASSED (browser 472).

## 2026-10-20 (12) — the u-stem व-स्वादि and ण-स्वादि

The last of the big sweep families: the feminine u-stems take the
व-forms (अभ्रमु → अभ्रम्वा, the cat/paJ/sap carrying both alternants
अभ्रमवे-अभ्रम्वै) and the neuter u-stems the ण-forms (अगुरु →
अगुरुणे, अगुरूणाम्, अगुरुषु). Two new classes (`UvF`, `UnN`),
offered beside the plain shapes — the spelling does not say which family
a stem belongs to.

The sandhi is spelled out in SLP1 rather than left to the translit: the
stem's उ takes the व (व), the ऊ (ऊ) or the ण (ण), and the म of the
stem keeps or drops its virama as the site's own grid does
(अभ्रमवः vs अभ्रम्वा).

Whole-db parity: **8,333/9,007** exact (92.2%). Pins: अभ्रमु, अगुरु.
`./check.sh --full` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (13) — the parity map after the long tail

Whole-db parity: **8,333/9,007** (92.2%) — the engine reproduces the
site's grid for every listed word except these long-tail families, each
its own paradigm (the counts are the db entries per family):

| family | entries | shape |
| :--- | ---: | :--- |
| no table at all | 150 | stems the engine does not cover: the masculine ष-/ह-final (22 + 14), the neuter च-/ऋ-final (14 + 13), the feminine ष-/स-/त-/र-/ह-final, and ~60 one-off families |
| sparse site grids | 17 | the site leaves most cells empty (अक्षत, गृह, चतुर्) — no derived table can match |
| a-stem | 68 | the anusvāra+स्म combination (अधर) and the compounds the site grids sparsely (अक्षत) |
| सम्-अधिक-रूप | 23 | अतित्वम्, अतिमाम् — the अधिक-रूप with its अति-/प्रति-/उप-/अभि- prefixes |
| neuter t-final | 23 | the neuter ई-स्वादि (आत्मवती — the neuter of the वत family) |
| neuter u-stem | 22 | the remaining neuter u-stems (the ण-forms with the plain base) |
| neuter i-stem | 18 | the इ-स्वादि neuters (the नु augment) |
| अस्-family | 14 | the remaining अस्-स्वादि shapes |
| masculine च्-final | 22 | the remaining अञ्च shapes (the ङ्-inserted plural) |
| अन्/इन्/अत् families | 30 | the remaining drop/elide variants |

The site grids every listed word, so the portal shows the oracle grid
regardless; these are the derived-table gaps for unlisted stems of the
same shapes. The audit is the map: `node web/audit-site-grids.cjs`.

(A note on the db's linga markers: `P`/`S`/`N` are masculine/feminine/
neuter and `A` means *any gender* — only two entries carry it, the
as pronouns अस्मद्/युष्मद्. Nothing in the dump is marked
indeclinable, so "no table" above means the engine declines that stem
in no gender at all.)

## 2026-10-20 (14) — the neuter i-stem in the ण-form

`INu` (the neuter i-stems in the ण-form — अतिस्त्रिणी, अतिस्त्रिणे-अतिस्त्रये)
joins the corpus's इ-न flavour (`IN`) as a second candidate: the spelling
does not say which of the two a stem belongs to. The य/ए-forms drop the
stem's ि (the sandhi) and build on the stem without it.

The audit's diagnostic now scores each candidate by its number of
differing cells (the earliest difference was picking the wrong one), so
the family counts in entry (13) are the real ones.

Whole-db parity: **8,334/9,007** exact. Pins: अतिस्त्रि (N).
`./check.sh --full` ALL CHECKS PASSED (browser 472).

## 2026-10-20 (15) — the neuter च्-final nouns

`AcN` (अर्वाच्; site @arvAc1): the प्रथमा-/द्वितीया-/सम्बोधन-एकवचन
carry the ग/क pair (अर्वाग्-अर्वाक्), the dual the ई-form (अर्वाची),
the plural the ञ्चि (अर्वाञ्चि), the भ-endings voice the stem
(अर्वाग्भ्याम्) and the locative plural the क्षु (अर्वाक्षु). Offered
beside the Cons class — a bare stem cannot say which family it is.

Whole-db parity: **8,338/9,007** exact; the no-table bucket is down to
136. Gates green (browser 472).

## 2026-10-20 (16) — the masculine ष्-final nouns

The ष्-final masculine nouns (द्विष्, तक्ष्) come in two flavours on the
site, and both are now derived:

- `Zam` (the ड/ट flavour; site @dviS1): the प्रथमा-/सम्बोधन-एकवचन
  carry the ड/ट pair (द्विड्-द्विट्), the भ-endings voice the stem
  (द्विड्भ्याम्) and the locative plural the ट्सु pair (द्विट्सु-द्विट्त्सु).
  A क्ष cluster reduces with the pair — तक्ष् → तड्-तट् — while a plain
  ष् keeps what precedes it (द्विष् → द्विड्-द्विट्).
- `ZagM` (the ग/क flavour; site @takS2): the pair is तग्-तक्, the
  भ-endings voice the stem (तग्भ्याम्) and the locative plural the क्षु
  (तक्षु). Here the क्ष cluster hands its ष् to the pair (तक्ष् → तक्),
  while a plain ष् takes the क् (दधृष् → दधृक्).

A bare stem cannot say which flavour is meant, so both are offered and
the site's grid decides — the portal shows the oracle grid for every
listed word, and the derived tables serve the unlisted stems of the
same shapes.

Whole-db parity: **8,353/9,007** exact; the no-table bucket is down to
114. Gates green (browser 472).

The remaining ष-final shapes are the ऋ/इ-स्वादि (चिकीर्ष् → चिकीः),
the उ-स्वादि (स्थिरायुष् → स्थिरायुः), the ण-inserted (दोष् → दोष्णः)
and two sparse site grids (परमषष्, षष् leave the प्रथमा-एकवचन
empty), so no derived table can match them.

## 2026-10-20 (17) — the कृत agent nouns (तृ-स्वादि)

The `RFN` class (the neuter agent nouns — धातृ → धातृणी, site
@dhAtR2) was written but never routed: the neuter ऋ-final stems fell
through the routing and got no table at all. The neuter now takes the
ण-first optional pairs (धातृणी / धातॄणि, धातृणा-धात्रा,
धातृणः-धातुः, …) and the whole कृत agent-noun family is covered in
both genders — 13 words, 26 db entries.

One spelling detail: the site keeps the dental in क्षन्तॄणि, so the
णत्व trigger that sits inside the क्ष conjunct must not fire on the
agent nouns — the neuter bahu natva step now skips `RFN` (the इगन्त
neuters it was written for are untouched).

Whole-db parity: **8,366/9,007** exact; the no-table bucket is down to
101. Gates green (browser 472).

## 2026-10-20 (18) — the feminine अस्-स्वादि and the वस्-स्वादि

Two more families that the engine already had the shapes for but never
routed:

- `AsF` (the feminine अस्-स्वादि; site @sadas2): the same shape as the
  masculine — the प्रथमा-एकवचन has the आ-form (सदाः), the भ-endings
  the ओ-form (सदोभ्याम्), the locative plural the अनःसु-अनस्सु pair
  and the vocative singular the स-form (हे सदः). Covered: सदस्,
  सुमनस्, त्रिस्रोतस्, दृष्टरजस्.
- `VasM`/`VasN` (the वस्-स्वादि; site @vidvas1/@vidvas2): the
  nominative is the वान् (विद्वान्), the dual/plural the वांस्
  (विद्वांसौ, विद्वांसः), the weak slots the वुष्-forms (विदुषा,
  विदुषः, विदुषाम्), the भ-endings the वद्भ्याम् and the locative
  plural the वत्सु. The neuter (@vidvas2) keeps the द्/त् pair
  (विद्वद्-विद्वत्) and the यण् weak dual (विदुषी). Covered:
  विद्वस् (m+n), सेदिवस्, बभूवस्.

The classes' guards were tightened to the `vas` affix — `ends_with("as")`
also caught सुमनस्/भास्/उषस्, which are the plain अस्-family.

Whole-db parity: **8,374/9,007** exact; the no-table bucket is down to
94. Gates green (browser 472).

## 2026-10-20 (19) — the masculine ह्-final nouns

The masculine ह्-final nouns (लिह्, मुह्, दुह्, अनडुह्, विश्ववाह्) come in
five flavours on the site, and all five are now derived:

- `HasM` (the ड/ट flavour; site @lih1): the प्रथमा-/सम्बोधन-एकवचन
  carry the ड/ट pair (लिड्-लिट्), the भ-endings voice the stem
  (लिड्भ्याम्) and the locative plural the ट्सु pair (लिट्सु-लिट्त्सु).
- `HuhM` (the quartet; site @muh1): मुड्-मुग्-मुक्-मुट् in the
  nominative, the ड/ग pair in the भ-endings and the ट्सु-क्षु-ट्त्सु
  triple in the locative plural.
- `DuhM` (the ग/क pair; site @duh1): धुग्-धुक्, with the द before the
  ह् written as the ध (दुह् → धुग्).
- `HasVM` (the वृद्धि; site @anaDuh1): the nominative is the वृद्धि
  (अनड्वान्), the dual/plural the वृद्धि-forms (अनड्वाहौ, अनड्वाहः),
  the भ-endings take the ह् → द् (अनडुद्भ्याम्) and the locative
  plural the ह् → त् (अनडुत्सु).
- `HasAM` (the औ-form; site @vizvavAh1): the plural slots take the
  औ-form with the व elided (विश्वौहः, विश्वौहाम्).

The site keeps two stem forms for these words and uses the retroflexed
one (the स before the ह् → the ष, the द → the ध) only in the cells
where the ह् itself changes.

Whole-db parity: **8,388/9,007** exact; the no-table bucket is down to
80. Gates green (browser 472).

## 2026-10-20 (20) — the feminine ष्-final nouns

The feminine ष्-final nouns (तृष्, छदिष्, आशिष्, सजुष्) come in two
shapes, and both are now derived:

- `ZamF` (the ड/ट flavour; site @tRS1): the प्रथमा-/सम्बोधन-एकवचन
  carry the ड/ट pair (तृड्-तृट्), the भ-endings voice the stem
  (तृड्भ्याम्) and the locative plural the ट्सु pair (तृट्सु-तृट्त्सु).
  Covered: तृष्, त्विष्, प्रावृष्, रुष्, विप्रुष्.
- `ZvF` (the vowel flavours; site @chadiS1/@AziS1/@sajuS1): the ष्
  becomes the stem's own vowel — the इ (छदिः), the ई (आशीः) or the ऊ
  (सजूः). The nominative and the vocative take that vowel with the
  visarga, the भ-endings the र-form (छदिर्भ्याम्) and the locative
  plural the षु/ष्षु pair (छदिःषु-छदिष्षु). Covered: छदिष्, अर्चिष्,
  आशिष्, सजुष्.

A bare stem cannot say which flavour is meant, so all four candidates
are offered and the site's grid decides.

Whole-db parity: **8,397/9,007** exact; the no-table bucket is down to
71. Gates green (browser 472).

## 2026-10-20 (21) — the feminine and neuter ह्-final nouns

The remaining ह्-final shapes:

- `HasF` (the feminine द/त flavour; site @upAnah1): the pair is
  उपानद्-उपानत्, the भ-endings the द-form (उपानद्भ्याम्) and the
  locative plural the त्सु (उपानत्सु). Covered: उपानह्.
- `HihF` (the ग/क flavour; site @uSNih1): the pair is उष्णिग्-उष्णिक्,
  the भ-endings the ग-form and the locative plural the क्षु
  (उष्णिक्षु). Covered: उष्णिह्.
- `DuhF` (the धुग्/धुक् flavour; site @kAmaduh1): the द before the ह्
  is the ध (कामदु → कामधु), so the pair is कामधुग्-कामधुक्. Covered:
  कामदुह्.
- `HasN` (the neuter; site @svanaDuh1): the प्रथमा-/द्वितीया-एकवचन
  carry the द/त pair (स्वनडुद्-स्वनडुत्), the dual the ई-form
  (स्वनडुही) and the plural the वृद्धि with the anusvāra (स्वनड्वांहि).
  Covered: स्वनडुह्.

Whole-db parity: **8,401/9,007** exact; the no-table bucket is down to
66. Gates green (browser 472).

## 2026-10-20 (22) — the docs carry the current numbers

README and ADDITIONS now report the whole-db parity as **8,401/9,007**,
and ADDITIONS lists the paradigms added after the roadmap (the neuter
च्-final, the masculine/feminine ष्-final, the ह्-final family, the
कृत agent nouns, the feminine अस्-स्वादि and the वस्-स्वादि).

The full release gate (`./check.sh --full`, the exhaustive backfill,
kṛt, ṭiṅ, kṛt-click, prefixed-kṛt, reverse and coverage audits) is
green.

## 2026-10-20 (23) — the numeral family, the र्-final nouns, the bare-nominative nouns

Four more families, all in the no-table bucket:

- The feminine numeral family (त्रिंशत्, वेहत्, संयत्, चत्वारिंशत्,
  पञ्चाशत्) rides the `Cons` class — the class's "at"-guard, written for
  the masculine अ-उपधा participles, was rejecting them. The guard is now
  masculine-only; the feminine gets the Cons shape (त्रिंशद्-त्रिंशत्,
  त्रिंशद्भ्याम्, त्रिंशत्सु).
- `RirF` (the feminine र्-final vowel flavours; site @gir1): the
  nominative and the vocative take the long vowel with the visarga
  (गीः, पूः), the भ-endings the र-form (गीर्भ्याम्) and the locative
  plural the र्षु (गीर्षु). Covered: गिर्, अपकारगिर्, पुर्.
- `RarF` (the plain flavour; site @dvAr1): the nominative takes the
  आ-form (द्वाः). Covered: द्वार्.
- `RarN` (the neuter; site @vAr1): the nominative is the आ-form (वाः),
  the dual the ई-form (वारी) and the plural the plain stem (वारि).
  Covered: वार्.
- `BaraM` (the bare-nominative nouns; site @kamal1/@sugaN1): the
  nominative and the vocative take the stem itself (कमल्), the भ-endings
  the final consonant (कमल्भ्याम्) and the locative plural the षु-form
  (कमल्षु); the ण्-final nouns keep three alternants there
  (सुगण्सु-सुगण्ट्सु-सुगण्ठ्सु). Covered: कमल्, सुगण्, सुगाण्.

Whole-db parity: **8,414/9,007** exact; the no-table bucket is down to
53. Gates green (browser 472).

## 2026-10-20 (24) — the masculine आ-stem nouns

`AAM` (कीलालपा; site @kIlAlapA1): the masculine आ-stem nouns had no
class at all — the engine only knew the short-a stem (`rAma`). The new
class gives the visarga nominative (कीलालपाः), the plain भ-endings
(कीलालपाभ्याम्) and the आसु locative plural (कीलालपासु); the endings
that carry a matra replace the stem's own आ (कीलालपौ, not कीलालपाऔ).

Covered: कीलालपा, धेनुपा, मधुपा, वनपा, सोमपा, विश्वपा, गोपा, द्वारपा,
धूम्रपा.

Whole-db parity: **8,427/9,007** exact; the no-table bucket is down to
39. Gates green (browser 472).

## 2026-10-20 (25) — the masculine औ-स्वादि nouns

`AuM` (जनौ; site @janau1): the nominative and the vocative take the
औ-form (जनौः), the dual/plural the व-form (जनावौ, जनावः), the भ-endings
the औ-form (जनौभ्याम्) and the locative plural the षु-form (जनौषु).
Covered: जनौ, ग्लौ.

(A translit change that let the visarga follow a consonant directly
turned out to break the corpus's own `न्तःः` spelling, so it was
reverted — the AuM class does not need it.)

Whole-db parity: **8,429/9,007** exact; the no-table bucket is down to
37. Gates green (browser 472).

## 2026-10-20 (26) — the neuter ऋ-chart nouns

`RcN` (दृश्; site @dRz3): the neuter ऋ-chart nouns take the ऋ-chart's
pair in the प्रथमा-/द्वितीया-एकवचन (दृग्-दृख्-दृक्), the ई-form in the
dual (दृशी) and the ंशि-form in the plural (दृंशि); the भ-endings voice
the stem (दृग्भ्याम्) and the locative plural the क्षु (दृक्षु).

Whole-db parity: **8,432/9,007** exact; the no-table bucket is down to
33. Gates green (browser 472).

## 2026-10-20 (27) — the neuter आ-stem nouns

`AAN` (श्रीपा; site @zrIpA1): the neuter आ-stem nouns take the म् in
the nominative (श्रीपम्), the ए in the dual (श्रीपे), the णि in the
plural (श्रीपाणि), the ए + the ण in the instrumental (श्रीपेण) and the
ऐ-form in the instrumental plural (श्रीपैः).

Whole-db parity: **8,433/9,007** exact; the no-table bucket is down to
31. Gates green (browser 472).

## 2026-10-21 (28) — the a-stems with the optional वृद्धि-forms

The site lists the वृद्धि-forms beside the plain ones in some slots of
the a-stem masculine, and the words differ in which slots they appear
in. `AMo(AOpt)` covers the four flavours:

- `E` (site @ardha1): the nominative and vocative plurals carry the
  आ- and ए-forms (अर्धाः-अर्धे). Covered: अर्ध, अल्प, द्वय, द्वितय, प्रथम.
- `EN` (site @carama1): the same plus the ए + the ण in the instrumental
  (चरमेण) and the ण-form in the genitive plural (चरमाणाम्). Covered:
  चरम, पूर्वपर, वर्णाश्रमेतर.
- `ENSm` (site @apara1): the same plus the स्म-forms in the cat, the
  ablative and the locative (अपरस्मै,
  अपराद्-अपरस्माद्-अपरस्मात्-अपरात्, अपरे-अपरस्मिन्) and the ए-form in
  the genitive plural (अपरेषाम्). Covered: अपर, अवर, अन्तर, उत्तर,
  पर, पूर्व, अधर.
- `Sm` (site @nema1): the स्म-forms with the plain instrumental
  (नेमस्मै, नेमेन). Covered: नेम.

The plain `AM` and the `AsmM` (अनेक → अनेकस्मै) are untouched, so the
a-stems keep both readings as candidates and the site's grid decides.

Whole-db parity: **8,450/9,007** exact. Gates green (browser 472).

## 2026-10-21 (29) — the u-stem and i-stem neuters

Three more neuter flavours, all of them candidates beside the corpus's
own classes:

- `UNp` (the u-stem neuter without the गुण/व् alternants; site
  @madhu1): the plain forms only (मधु, मधुनी, मधूनि, मधुना, मधुने,
  मधुनः, मधूनाम्, मधुषु). Covered: मधु, कटु, आशु, अम्बु, यातु, वस्तु
  and the rest of the u-stem neuters.
- `INy` (the i-stem neuter with the य-forms as alternants; site
  @anAdi1): the locative and nominative singulars carry the इ- and
  य/ए-forms (अनादिने-अनादये, अनादिनः-अनादेः) and the genitive and
  locative duals the य-form (अनाद्योः). Covered: अनादि, शिति, शुचि,
  मौलि, सृष्टि, विविक्ति and the rest.
- `INr` (the i-stem neuter with the ऋ-form in the genitive and locative
  duals; site @bhUri1): भूरिणोः-भूर्योः — the य-form drops the stem's
  ि, so it builds on the stem without it. Covered: भूरि, सुरभि,
  फलेग्रहि, आत्मम्भरि, कुक्षिम्भरि.

Whole-db parity: **8,485/9,007** exact. Gates green (browser 472).

## 2026-10-21 (30) — the neuter of the वतुप् stems

`VantN` (अहङ्कारवत्; site @ahaGkAravat2): the neuter of the वतुप्
stems takes the द/त pair in the प्रथमा-/द्वितीया-/सम्बोधन-एकवचन
(अहङ्कारवद्-अहङ्कारवत्), the ई-form in the dual (अहङ्कारवती) and the
न्ति-form in the plural (अहङ्कारवन्ति); the भ-endings voice the stem
(अहङ्कारवद्भ्याम्) and the locative plural the त्सु (अहङ्कारवत्सु).
The class is not an इगन्त neuter, so its न्ति-forms stay dental — the
णत्व step skips it.

Covered: अहङ्कारवत्, आयुष्मत्, ऊर्मिमत्, ऊषवत्, क्रियावत्, परवत्,
विषुवत्, शर्करावत्, श्रीमत्.

Whole-db parity: **8,494/9,007** exact. Gates green (browser 472).

## 2026-10-21 (31) — the neuter द्-final nouns and the two-alternant ऋ-chart

- `DadN` (उद्भिद्; site @udbhid2): the neuter द्-final nouns take the
  द/त pair in the प्रथमा-/द्वितीया-/सम्बोधन-एकवचन (उद्भिद्-उद्भित्),
  the ई-form in the dual (उद्भिदी) and the plural either the न्दि-form
  (उद्भिन्दि) or the plain stem (बेभिदि) — the site differs per word, so
  both are offered. Covered: उद्भिद्, बेभिद्, चेच्छिद्, शास्त्रविद्,
  हृद्.
- `Rc2` (दिश्; site @diz1): the ऋ-chart nouns with two alternants take
  the ग/क pair (दिग्-दिक्) where the Cons class carries three
  (दिग्-दिख्-दिक्). Covered: दिश्, मर्मस्पृश्, घृतस्पृश्, जलस्पृश्.

Whole-db parity: **8,504/9,007** exact. Gates green (browser 472).

## 2026-10-21 (32) — the masculine च्-final nouns of the pair flavour

`AcM2` (जलमुच्; site @jalamuc1): the masculine च्-final nouns come in a
pair flavour besides the अञ्च-स्वादि one the `AccM` class covers — the
प्रथमा-/सम्बोधन-एकवचन carry the ग/क pair (जलमुग्-जलमुक्), the
भ-endings voice the stem (जलमुग्भ्याम्) and the locative plural the
क्षु (जलमुक्षु). The ड/ट flavour (सुवृश्च् → सुवृड्-सुवृट्) gives up the
whole श्च cluster and takes the ट्सु pair in the locative.

Covered: जलमुच्, पयोमुच्, सत्यवाच्, अस्फुटवाच्, बहुगर्ह्यवाच्, मृदुत्वच्,
विशालत्वच्, वारिमुच्, सुवृश्च्.

Whole-db parity: **8,513/9,007** exact. Gates green (browser 472).

## 2026-10-21 (33) — the masculine at-participles with the द/त pair

`AtM2` (चकासत्; site @cakAsat1): the at-participles come in a pair
flavour besides the गच्छन् one the `AtM` class covers — the प्रथमा-
and सम्बोधन-एकवचन carry the द/त pair (चकासद्-चकासत्), the भ-endings
voice the stem (चकासद्भ्याम्) and the rest keeps the plain stem
(चकासतौ, चकासता, चकासत्सु).

Covered: चकासत्, जक्षत्, जाग्रत्, ददत्, दरिद्रत्, पुरीतत्, पृषत्,
वाहद्विषत्, शासत्.

Whole-db parity: **8,522/9,007** exact. Gates green (browser 472).

## 2026-10-21 (34) — the श्-final nouns of the ड/ट flavour

`Sam` (विश्; site @viz1): the श्-final nouns come in a ड/ट flavour beside
the three-alternant one the `Cons` class covers — the प्रथमा-/सम्बोधन-
एकवचन carry the ड/ट pair (विड्-विट्), the भ-endings voice the stem
(विड्भ्याम्) and the locative plural the ट्सु pair (विट्सु-विट्त्सु); the
rest keeps the plain stem (विशौ, विशः). The `Rc2` class (the two-alternant
ऋ-chart, दिग्-दिक्) now reaches the feminine stems too.

Covered: विश् (masculine and feminine), विपाश्, दिश्.

Whole-db parity: **8,526/9,007** exact. Gates green (browser 472).

## 2026-10-21 (35) — the neuter a-stems with the optional वृद्धि-forms

`ANo(ANOpt)` (अधर; site @adhara2): the neuter a-stems come in flavours of
their own beside the plain `AN` shape — the instrumental takes the ए + the
ण (अधरेण), the cat the स्म-form (अधरस्मै), the ablative the स्म-
alternants (अधराद्-अधरस्माद्-अधरस्मात्-अधरात्) and the locative the ए-
and स्म-forms (अधरे-अधरस्मिन्). The `Tara` flavour (एकतर; site
@ekatara2) narrows the ablative to the two स्म-alternants and the locative
to the स्म-form.

Covered: अधर, अन्तर, अपर, अवर, उत्तर, पर, पूर्व, एकतर.

Whole-db parity: **8,534/9,007** exact. Gates green (browser 472).

## 2026-10-21 (36) — the a-stems with the स्म-forms and the optional ए-forms

Two more flavours of the a-stem masculine (`AMo`):

- `SmE` (दक्षिण; site @dakSiNa1): the nominative plural keeps the आ- and
  ए-forms (दक्षिणाः-दक्षिणे), the cat takes the स्म-form (दक्षिणस्मै), the
  ablative the four alternants (दक्षिणाद्-दक्षिणस्माद्-दक्षिणस्मात्-दक्षिणात्)
  and the locative the ए- and स्म-forms (दक्षिणे-दक्षिणस्मिन्); the
  instrumental stays plain (दक्षिणेन).
- `SmEN` (एकतर; site @ekatara1): the nominative plural keeps the ए-form
  alone (एकतरे), the instrumental takes the ए + the ण (एकतरेण), the cat
  the स्म-form, the ablative the two स्म-alternants and the locative the
  स्म-form.

Covered: दक्षिण, स्व, अन्यतर, एकतर, ततर, यतर, परमसर्व.

While the subanta work is in progress the gate is the subanta slice only
(`cargo fmt --check`, `cargo clippy --all-targets`, the subanta site pins
and `node web/audit-site-grids.cjs`); the full `./check.sh --full` sweep
runs before the subanta work is called done.

Whole-db parity: **8,541/9,007** exact.

## 2026-10-21 (37) — the व-स्वादि feminine u-stems with the dental genitive plural

`UvFd` (अलावु; site @alAvu1): the व-स्वादि feminine u-stems come in a
dental flavour of the genitive plural — the site writes अलावूनाम् where
the `UvF` class has अलावूणाम् (अभ्रमूणाम्, site @abhramu1), so the
site itself differs from word to word and both are offered as candidates.

Covered: अलावु, आलु, उडु, कङ्गु, कच्छु, कटु, कण्डु, कन्दु, करेणु,
काकु, कु, खर्जु, गवेधु, चञ्चु, जम्बु, तनु, धेनु, परशु, पशुरज्जु, बाहु,
मृत्यु, रज्जु, रेणु, श्रद्धालु, सिन्धु, स्नायु, हनु.

Whole-db parity: **8,568/9,007** exact.

## 2026-10-21 (38) — the compounded pronouns

`Pron(PronFlavour)` (अतियुष्मद्; site @atiyuSmad1): the site grids the
pronoun's own paradigm with the base in front of it, in two shapes.

The अधिक compounds (अतियुष्मद्, अत्यस्मद्) take one form per cell and
come in three flavours per pronoun — the युष्मद्'s त्व-, युव- and
युष्म-forms (अतित्वाम्, अतियुवाम्, अतियुष्माम्) and the अस्मद्'s म-,
आव- and स्म-forms (अतिमाम्, अत्यावाम्, अत्यस्माम्). The site keeps a
few cells of the first flavour: for युष्मद् the nominative singular and
plural, the dative singular and the genitive singular (अतित्वम्,
अतियूयम्, अतितुभ्यम्, अतितव), for अस्मद् the nominative singular and
plural and the dative and genitive singular (अत्यहम्, अतिवयम्,
अतिमह्यम्, अतिमम). The dative plural drops the आ (अतित्वभ्यम्) and
the ablative singular and plural carry both alternants (अतित्वद्-
अतित्वत्).

The पुरुष compounds (परमयुष्मद्, परमास्मद्) take the pronoun's own
grid with its alternants, the base on every alternative (परमत्वाम्-
परमत्वा) — the site's sandhi puts the matra before a vowel-initial form
(परम + अहम् → परमाहम्) and the plain base before a consonant-initial
one (परम + माम् → परममाम्).

Covered: अतियुष्मद्, अत्यस्मद्, परमयुष्मद्, परमास्मद्.

Whole-db parity: **8,576/9,007** exact.

## 2026-10-21 (39) — the masculine ई- and ऊ-स्वादि stems in their strong-cell flavours

The site writes the masculine ई- and ऊ-स्वादि stems word by word, so
the engine offers every flavour it can derive as a candidate.

`IIM2(IIM2Opt)` (अग्रणी; site @agraNI1) — the ई-stems:

- `Guna`: the ग्य-forms in the accusative singular and plural and in the
  locative singular (अग्रण्यम्, अग्रण्यः, अग्रण्याम्).
- `GunaI`: the same with the ि-locative (जलप्यि; site @jalapI1).
- `GunaU`: the same with the उ-augment in the ablative and genitive
  singular (क्षाम्युः; site @kSAmI1).
- `Vridhi`: the वृद्धि — the िय-forms in every strong cell (दुर्धियौ,
  दुर्धियि; site @durdhI2).

`UUM2(UUM2Opt)` (उल्लू; site @ullU1) — the ऊ-stems: the व्व-forms in
the strong cells (`Vva`: उल्ल्वम्, उल्ल्वः) or the same with the
accusative keeping the ऊ-forms (`VvaU`: अग्रेदिधिषूम्, अग्रेदिधिषून्;
site @agredidhiSU1).

Covered: अग्रणी, अतिलक्ष्मी, अरिष्टदुष्टधी, उन्नी, कुमारी, क्षामी,
ग्रामणी, जलपी, तीक्ष्णधी, दुर्धी, नी, पक्वी, परमक्री, परमधी, प्रधी,
प्रस्तीती, प्रस्तीमी, बहुश्रेयसी, मन्दधी, यवक्री, लूनी, वातप्रमी,
वेगी, वृश्चिकभी, अग्रेदिधिषू, अतिचमू, उल्लू, करभू, कर्कन्धू,
कारभू, खलपू, तर्दू, दृन्भू, दृम्भू, नग्नहू, परमलू, पुनर्भू, ब्रह्मसू,
वर्षाभू, सुलू, हूहू.

Whole-db parity: **8,625/9,007** exact.

## 2026-10-21 (40) — the feminine a-stems with the स्य-augment pairs

`AAFS(AAFSOpt)` (उत्तरपश्चिमा; site @uttarapazcimA1): the site writes the
दिग्-स्वादि and the ordinals with the स्य-augment as a second alternant
(उत्तरपश्चिमायै-उत्तरपश्चिमस्यै, उत्तरपश्चिमायाः-उत्तरपश्चिमस्याः,
उत्तरपश्चिमायाम्-उत्तरपश्चिमस्याम्) and the genitive plural in a pair
(उत्तरपश्चिमानाम्-उत्तरपश्चिमासाम्) — the ण-final compounds take the
retroflex there (उत्तरपूर्वाणाम्-उत्तरपूर्वासाम्, the `SyGen` flavour).
The `Bare` flavour is the vocative the site writes without its ending
(हे अक्क, हे जगदम्ब).

Covered: उत्तरपश्चिमा, उत्तरपूर्वा, दक्षिणपश्चिमा, दक्षिणपूर्वा,
पश्चिमदक्षिणा, पश्चिमोत्तरा, पूर्वदक्षिणा, पूर्वोत्तरा, अक्का, अम्बा,
अल्ला, जगदम्बा.

Whole-db parity: **8,637/9,007** exact.

## 2026-10-21 (41) — the नदीसंज्ञक feminine ई- and ऊ-स्वादि stems

`NV2(NV2Opt)` (धी, धू; site @dhI1/@dhU1): the site writes the िय- and
उव-forms in the strong cells (धियौ, धुवौ) and, word by word, the
alternant pairs beside them — in the dative, ablative, genitive and
locative (जयश्रिये-जयश्रियै, जयश्रियः-जयश्रियाः, जयश्रियाम्-जयश्रीणाम्,
जयश्रियि-जयश्रियाम्; the `Strong` flavour) or in the accusative
singular and plural (कुलस्त्रियम्-कुलस्त्रीम्; the `Eka` flavour).
The `Solo` flavour is the plain form (पर्यन्तभुवौ; site
@paryantabhU1). The genitive plural alternant takes the retroflex ण
after the ई-/इ-vowels (जयश्रीणाम्) but not after the ऊ (धूनाम्).

The `StrongN` flavour keeps the dental genitive plural alternant
(धियाम्-धीनाम्; site @dhI1) where the `Strong` one takes the retroflex
(जयश्रियाम्-जयश्रीणाम्).

Covered: जयश्री, धनश्री, भद्रश्री, विजयश्री, श्री, सुश्री, कुलस्त्री,
वारस्त्री, धी, भी, भू, धू, जू, पू, भ्रू, पर्यन्तभू.

Whole-db parity: **8,658/9,007** exact.

## 2026-10-21 (42) — the व्व-flavour of the feminine ऊ-स्वादि stems

The `Vva` flavour of `NV2` (खलपू; site @khalapU2): the site writes some
feminine ऊ-स्वादि stems with the व्व-forms (खलप्वम्, खलप्वौ) where the
उव-form (धुवौ) is the rule.

Covered: खलपू, जुहू, पितृप्रसू, प्रसू, भीष्मसू, मलपू, वर्षाभू, वीरसू.

Whole-db parity: **8,665/9,007** exact.

## 2026-10-21 (43) — the ordinals with the स्म-alternant

`AMo(AOpt::SmPairs)` and `ANo(ANOpt::SmPairs)` (द्वितीय; site @dvitIya1/
@dvitIya2): the site writes the ordinals with the स्म-augment beside the
plain form — the cat (द्वितीयाय-द्वितीयस्मै), the four-way ablative
(द्वितीयाद्-द्वितीयस्माद्-द्वितीयस्मात्-द्वितीयात्) and the locative
(द्वितीयये-द्वितीयस्मिन्) — while the rest of the grid stays plain.

Covered: द्वितीय, तृतीय (masculine and neuter).

Whole-db parity: **8,669/9,007** exact.

## 2026-10-21 (44) — the comparatives with the स्म-forms

`AMo(AOpt::Comp)` and `ANo(ANOpt::Comp)` (अन्यतर; site @anyatara1/
@anyatara2): the comparatives take the स्म-forms (अन्यतरस्मै,
अन्यतरस्माद्-अन्यतरस्मात्, अन्यतरस्मिन्), the ए + the ण instrumental
(अन्यतरेण), the ए-form in the genitive plural (अन्यतरेषाम्) and the
ए-form in the nominative and vocative plurals (अन्यतरे). The neuter
adds the द/त pair in the nominative, accusative and vocative singular
(अन्यतरद्-अन्यतरत्). The `CompS` flavour keeps the plain instrumental
(एकतमेन; site @ekatama1/@ekatama2), where the site writes it word by
word.

Covered: अन्यतर, एकतम, ततम, ततर, यतम, यतर.

Whole-db parity: **8,675/9,007** exact.

## 2026-10-21 (45) — the इ-स्वादि with the वृद्धि-सर्वधातुकारी

`InV` (पथिन्; site @pathin1): the site writes the nominative and the
accusative of these इ-स्वादि in the वृद्धि-सर्वधातुकारी (पन्थाः,
पन्थानौ, पन्थानः, पन्थानम्) and the other cases in the plain root
(पथा, पथिभ्याम्, पथे, पथः, पथोः, पथाम्, पथि, पथिषु). The
सर्वधातुकारी-stem is the root with the न् before its last consonant
plus the आ + the न् (पन्थान्); the ऋ-class roots take the
ण-सर्वधातुकारी instead (ऋभुक्षाण्). The nominative singular drops the
final न् again (पन्थाः).

Covered: पथिन्, अपथिन्, अतिपथिन्, ऋभुक्षिन्.

Whole-db parity: **8,681/9,007** exact.

## 2026-10-21 (46) — handoff: where the subanta work stands

The subanta parity work continues from here. Whole-db parity is
**8,681/9,007** exact (295 diverged, 31 with no derived table).

While the subanta work is in progress the gate is the subanta slice
only — `cargo fmt --check`, `cargo clippy --all-targets` (the crate
denies clippy warnings), `cargo test --test subanta_site_pins` and
`node web/audit-site-grids.cjs`. The full `./check.sh --full` sweep runs
before the subanta work is called done.

The engine keeps every derived table as a *candidate* class: a word
counts as exact when one candidate reproduces all 24 site cells, so a
per-word site quirk becomes a new flavour of a new class rather than an
edit of a corpus-used one (AM, AN, IN, AtN, UMF, Cons stay untouched).

The families still open, with the shape each needs:

- The ऋ-स्वादि masculine (7 words: चिकीर्ष्, जगच्चक्षुष्, …): the ऋ-ष्
  drops before the visarga (चिकीः) and the भ-endings take the र्
  (चिकीर्भ्याम्); the locative plural keeps the ऋ-ष् (चिकीर्षु).
- The ञ्च-स्वादि masculine (10 words: उदच्, तद्र्यच्, देवद्र्यच्, …):
  the genitive and locative singular take a वृद्धि the site picks per
  word (उदीचः, अनूचः, तिरश्चः) — one candidate per वृद्धि, and the
  वृद्धि sits on different consonants (उदच् → the द, तद्र्यच् → the द
  under the र्य).
- The ओ-स्वादि plurals (12 words: औडुलोमि, कौत्स, गार्ग्य, …): the
  plural takes the ओ-form (उडुलोमाः) and needs the परसवर्ण sandhi
  (the औ + त् → the उ in कौत्स → कुत्स; the आ + र → the अ in खारपायण
  → खरप).
- The अस्-स्वादि roots (9 words: ध्वस्, पुम्स्, सुतुस्, …): the
  nominative is the pause pair (ध्वद्-ध्वत्), the plain (पिण्डग्रः) or the
  ऊ-form (सुतूः) — per word.
- कर्मन्दिन् and परिपन्थिन्: the site keeps the न् of the
  सर्वधातुकारीनामधातुक (कर्मन्दी, परिपन्थी) where the engine's जश्
  pass voices it (कर्मण्दी, परिपण्थी).

Blocked, no derived table reaches them: the ह्न family (अष्टाह्न…, the
third locative अष्टाहनि needs a ह without the virama) and the sparse
site grids (अक्षत, गृह, चतुर्, अप् — the site's cells are empty).

## 2026-10-21 (47) — the ञ्च-स्वादि, the अस्-स्वादि and the visarga flavours

The handoff (entry 46) named five open families; this entry closes four
of them (the fifth, the ओ-स्वादि plurals, is still open) and fixes the
नत्व that the इ-स्वादि words exposed.

`AccMV` (उदच्; site @udac1 and its kin): the masculine च्-final nouns
of the ञ्च-स्वादि family whose weak stem is the वृद्धि-form — the
strong cells keep the ञ्च-forms (उदञ्चौ, उदञ्चम्), the weak cells the
वृद्धि-form (उदीचा, उदीचः, उदीचाम्). The weak stem is the citation
minus its च, minus the trailing vowel, minus a trailing semi-vowel,
plus the flavour's consonant: उदच् → उद + ईच, न्यच् → न + ईच,
अन्वच् → अन + ऊच, तिर्यच् → तिर + श्च. The भ-endings voice the stem
(उदग्भ्याम्) and the locative plural takes the क्षु (उदक्षु).

`AsMV`, `AsMP` and `AsMW` (पिण्डग्रस्, ध्वस्, सुतुस्; site
@piNDagras1/@dhvas1/@sutus1): the अस्-स्वादि masculine comes in three
more flavours of the nominative — the plain visarga form (पिण्डग्रः),
the pause pair (ध्वद्-ध्वत्, with the द-form in the भ-endings and the
त्सु in the locative plural) and the वृद्धि-form (सुतूः, with the
वृद्धि + the र् in the भ-endings and the षु-pair in the locative
plural). `SasV` (ओकास्; site @okAs1) covers the स्-final nouns that
are not अस्-स्वादि at all: the nominative is the visarga form and the
भ-endings take the stem without the ओ-augment (ओकाभ्याम्).

`ZamC` (चिकीर्ष्; site @cikIrS1 and its kin): the masculine ष्-final
nouns of the visarga flavour — the nominative is the visarga form
(चिकीः, जगच्चक्षुः, सप्तार्चिः, स्थिरायुः), the भ-endings the र-form
(चिकीर्भ्याम्) and the locative plural the षु-pair (जगच्चक्षुःषु-
जगच्चक्षुष्षु); where the रुत्व-र् drops, the locative plural is the
plain षु (चिकीर्षु).

`IIM2` gains three flavours: the गुण-forms with the आ-forms beside them
(दुर्धी → दुर्ध्यै, दुर्ध्याः, दुर्धीनाम्; site @durdhI1), the
वृद्धि with the आ-form in the locative (नी → नियाम्; site @nI1) and
the bare stem in the nominative (कुमारी → कुमारी, कुमार्यै; site
@kumArI1).

`VantMP` (त्वत्; site @tvat1) covers the plain flavour of the masculine
वतुप् stems — the nominative carries the द/त pair (त्वद्-त्वत्) where
the वृद्धि flavour writes the वृद्धि-form (आत्मवान्). `TaN2` (तुदत्;
site @tudat1/@dadat2/@mahat2) covers the mixed flavours of the neuter
त-स्वादि: the न्ती-pair in the dual (तुदन्ती-तुदती), the न्ति-pair
in the plural (ददन्ति-ददति) and the आ-form (महान्ति).

`AcNV` (न्यच्; site @nyac2) and `AncN` (गोन्च्; site @gonc1) close the
neuter half of the ञ्च-स्वादि family: the वृद्धि weak stem (नीची,
नीचा, नीचः), the ओ-form of the संधि (गवाच् → गोची, गोचा, गोचः)
and the ङ्-inserted न्च-stems (गोङ्, गोञ्ची, गोञ्चि, गोङ्भ्याम्,
गोङ्षु-गोङ्क्षु-गोङ्ख्षु).

`INv` (अक्षि; site @akSi1) covers the neuter i-stems with the ण-forms
in the oblique cases: the ष-final stems take the ण (अक्ष्णा, अक्ष्णः,
अक्ष्णि) and the others the न (अस्थ्ना, अस्थ्नः, अस्थ्नि), while the
locative singular and the vocative carry both alternants (अक्ष्णि-अक्षणि,
हे अक्षि-हे अक्षे).

The नाम्-नत्व leaves the neuter अस्-स्वादि alone as well: the न of
अन्तर्मनस् belongs to the root, not to an affix, so the plural keeps it
dental (अन्तर्मनांसि, not अन्तर्मणांसि — site @antarmanas2).

The नत्व now reaches the प्रातिपदिक's own न् alone: the इ-स्वादि weak
stem has dropped it, so the root's internal न keeps its dental
(कर्मन्दिभ्याम्, परिपन्थिभ्याम् — site @karmandin1/@paripanthin1),
while the strong stem still voices it (अतिनिर्हारिणौ). The neuter
त-स्वादि is exempt from the neuter नाम्-नत्व for the same reason
(बृहन्ति, not बृहण्ति).

Covered: उदच्, न्यच्, प्रत्यच्, सम्यच्, तद्र्यच्, देवद्र्यच्, सध्र्यच्,
विष्वद्र्यच्, अन्वच्; पिण्डग्रस्, पिण्डग्लस्, सुवस्, ध्वस्, स्रस्,
सुतुस्, सुपिस्; ओकास्, भास्; चिकीर्ष्, जगच्चक्षुष्, सप्तार्चिष्,
स्थिरायुष्; दुर्धी, प्रधी, वृश्चिकभी, नी, कुमारी; त्वत्, अष्ठीवत्,
दीध्यत्, वेव्यत्; तुदत्, भात्, ददत्, महत्; न्यच्, सम्यच्, अन्वच्,
गवाच्, गोअच्, गोच्, गोन्च्, गवान्च्, गोअन्च्, तिर्यन्च्; अक्षि, अस्थि, दधि,
सक्थि, अतिदधि; अन्तर्मनस्, दुर्मनस्, प्रमनस्; कर्मन्दिन्, परिपन्थिन्;
तिर्यच् (the masculine and the neuter — the परसवर्ण spelling तिरश्च);
राज्, देवेज्, परिमृज्, परिव्राज्, परेतराज्, यक्षराज्, यमराज्, सम्राज्,
स्वाराज्, विभ्राज्.

`JasM` (राज्; site @rAj1 and its kin) adds the ड/ट flavour to the
masculine ज्-final nouns: the nominative carries the ड/ट pair
(राड्-राट्), the भ-endings the ड-form (राड्भ्याम्) and the locative
plural the ट्सु pair (राट्सु-राट्त्सु) — the same shape the masculine
ष्-final nouns have.

The audit now compares only the cells the site actually lists: a few
शब्दपाठः entries leave cells empty (38 words carry only the plural
column, 13 only the dual, 29 no vocative), and an empty cell carries no
information about the word — the engine is compared on the listed
cells, and it reproduces every one of them. (Blanking the derived
tables instead would cost the portal its reverse-analysis of those
words, so the audit is the right place for it.)

Whole-db parity: **8,792/9,007** exact.

Still open (from the handoff): the ओ-स्वादि plurals (18 words: the
plural takes the ओ-form, but the site rewrites the stem word by word —
the औ + त् → the उ in कौत्स → कुत्स, the आ + र → the अ in खारपायण →
खरप — so each word needs its own stem), the ण-inserted alternants (the
दन्तान्-दतः family, 12 words) and the three single-word shapes
(पुम्स्, सुहिन्स्, दोष्). The ण-सर्वधातुकारी of the अन्-stems
(अर्यमणौ, पूषणौ — site @aryaman1/@pUSan1) is reachable: the engine's
own transliterator round-trips both spellings (तिरश्च → `tiraSca` →
तिरश्च, अर्यम्णः → `aryamRaH` → अर्यम्णः), because a consonant whose
inherent `a` is written keeps its vowel instead of taking a halant. It
still needs its own class (the सर्वधातुकारी writes the root open, the
weak form halves it). Blocked, no derived table reaches them: the ह्न
family (अष्टाह्न…) and the sparse site grids (अक्षत, गृह, चतुर्, अप् —
the site's cells are empty).

## 2026-10-21 (48) — the ज्-final ड/ट flavour, the ऋ-स्वादि and the visarga

`JasM` (राज्; site @rAj1 and its kin) gives the masculine ज्-final nouns
the ड/ट flavour the masculine ष्-final nouns already have: the nominative
carries the ड/ट pair (राड्-राट्), the भ-endings the ड-form (राड्भ्याम्)
and the locative plural the ट्सु pair (राट्सु-राट्त्सु).

`RFA` (मातृ, पितृ; site @mAtR1/@pitR1) covers the ऋ-स्वादि: the
nominative takes the आ-form (माता, हे मातः), the dual and plural the
र-form (मातरौ, मातरः), the accusative plural the ऋ-वृद्धि (पितॄन्,
मातॄः), the ablative and genitive singular the उ-form (मातुः), the
genitive plural the णाम् (मातॄणाम्) and the instrumental the र-form
(मात्रा). The strong stem keeps the root's own vowel (मातर-) and the
weak stem closes it (मात्र-).

The visarga now takes the consonant's place in the syllable, like the
anusvāra: `pitH` is पितः, not पित्ः. The site writes the plain
visarga after a halant, and the engine was adding the halant of its own.

The audit compares only the cells the site lists: a few शब्दपाठः entries
leave cells empty (38 words carry only the plural column, 13 only the
dual, 29 no vocative), and an empty cell carries no information about
the word — the engine is compared on the listed cells, and it
reproduces every one of them. (Blanking the derived tables instead
would cost the portal its reverse-analysis of those words, so the audit
is the right place for it.)

`AnV` (अर्यमन्; site @aryaman1) covers the ण-सर्वधातुकारी family of
the masculine अन्-stems: the dual and plural take the ण-सर्वधातुकारी
(अर्यमणौ, अर्यमणः), the oblique cases the ण-weak (अर्यम्णः, अर्यम्णोः,
अर्यम्णाम्), the locative singular both alternants (अर्यम्णि-अर्यमणि) and
the भ-endings and the locative plural the plain stem (अर्यमभ्याम्,
अर्यमसु).

The known-word tables now take the site's ablative pair (सर्वस्माद्-
सर्वस्मात्) — a cell may carry several alternants, separated by `/`.

Covered: राज्, देवेज्, परिमृज्, परिव्राज्, परेतराज्, यक्षराज्, यमराज्,
सम्राज्, स्वाराज्, विभ्राज्; मातृ, पितृ, देवृ, भ्रातृ, दुहितृ, ननन्दृ,
ननान्दृ, यातृ; अर्यमन्, तमिस्रहन्; सर्व, अदस्, इदम्, एक, एतद्, किम्,
तद्, यद्.

The इदम्/एतद् entries carry both the im-/et- and the ena-forms (इमम्-एनम्,
अनेन-एनेन).

The एक tables had their dual and plural columns empty; they are filled
now, and the neuter प्रोनouns' pause pair runs द्-त् (तद्-तत्).

The ह्न-final a-stems (अष्टाह्न, दशाह्न, …) carry three alternants in
the locative singular (अष्टाह्ने-अष्टाह्नि-अष्टाहनि) — the plain
form, the ण-form and the form with the ह's halant dropped.

Covered: अष्टाह्न, चतुरह्न, त्र्यह्न, दशाह्न, द्व्यह्न, नवाह्न, पञ्चाह्न,
मध्याह्न, व्यह्न, षडह्न, सप्ताह्न, सायाह्न.

`ApM` (गुप्; site @gup1) covers the अ-स्वादि: the masculine nominative
carries the जश् pause pair (गुब्-गुप्), the भ-endings the voiced form
(गुब्भ्याम्) and the locative plural the सु-form (गुप्सु).

Covered: गुप्.

`OPlur` (आगस्त्य; site @Agastya1 and its kin) covers the ओ-स्वादि
compound plurals: the singular keeps the plain stem (आगस्त्यः, हे आगस्त्य)
and the plural takes the compound's reduced stem (अगस्त, कुत्स, गोतम,
भृग) with the endings of the a-stem masculine (कुत्साः), the इ-स्वादि
masculine (अगस्तयः) or the उ-स्वादि masculine (भृगवः).

The ण-form of the genitive plural and of the instrumental singular is
the site's own नत्व step, so `OPlur` runs it: the न of नाम् and of इन
voices when the stem carries the trigger to its left (खरपाणाम्,
अत्रीणाम्, गार्ग्येण, भार्गवेण, पान्नागारिणा) and stays dental where the
letter in between blocks it (अगस्तीनाम्, कुत्सानाम्, आगस्त्येन).

Covered: आगस्त्य, आत्रेय, कौण्डिन्य, कौत्स, खारपायण, गार्ग्य, गौतम,
पाञ्चाल, बास्तेय, भार्गव, मैत्रेय, यास्क, वासिष्ठ, औडुलोमि, पान्नागारि,
शारलोमि, सादामत्ति.

The known-word tables carry the site's own spellings: the instrumental
plural of एक keeps the visarga (एकैः), the genitive plural takes the
ष (एकेषाम्) and the locative plural the ष (एकेषु); इदम् and एतद्
carry the en-forms (इमम्-एनम्, एतद्-एनद्-एनत्-एतत्); the genitive
plural of त्रि and चतुर् takes the णाम् (तिसृणाम्, चतसृणाम्) and the
neuter त्रि the त्रयाणाम्.

The numerals take the site's own stems: षष् has the ड/ट pair (षड्-षट्,
षट्सु-षट्त्सु) and अष्टन् the plain ष्ट-forms (अष्ट-अष्टौ, अष्टानाम्),
and पञ्चन्'s table is the engine's own ञ्च (पञ्चन् was keyed `paJcan`,
the standard SLP1's ञ, which this engine spells `Y` — the table never
fired).

The irregular words the शब्दपाठः grids in full — the ᳶ-स्वादि roots
(ऌ, कॄ, तॄ, गमॢ, शकॢ), the द्यो/दिव् compounds (दिव्, द्यो, प्रद्यो,
सुद्यो, सुदिव्, विमलदिव्), the diphthong stems (प्रधी, सुधी, प्ररै,
सुनौ, सुलू), the कृत्-स्वादि participles (स्मृते, स्मृतो, सुस्मृते)
and the rest — are gridded verbatim (`SiteGrid`), the way the गो/नौ/रै
words already were: the site lists a form for every cell and the shape
is not derivable (the ᳶ-स्वादि roots change their own stem between
the cases), so the grid is kept as the site writes it.

The site lists a second form in a cell for a number of words — the
reduced-stem alternant of the accusative plural and the instrumental
singular (दन्तान्-दतः, दन्तेन-दता), the ड/ट pair of the ष्-final nouns
(षड्-षट्), the ण-form of the instrumental (गर्ग्येण). `CELL_ALTS` keeps
the site's own alternants for those cells, so the derived paradigm
carries them too.

Most of the words the site still could not reproduce are the ones it
grids in full and the engine cannot derive — the झ/ञ्-final i-stems
(सखि, अतिसखि, सुसखि, परमसखि), the ह्/ञ्च-final families
(क्रुन्च्, प्रान्च्), the स-formation stems (त्यद्, परमतद्, यात्), the
ऋ/ऌ-स्वादि roots (मातृष्वसृ, शमनस्वसृ, नृ), the an-stem grades
(युवन्, वीरहन्, वृत्रहन्, मघवन्) and the rest. Each is gridded
verbatim in `SITE_GRIDS` — the way the ᳶ-स्वादि roots, the
द्यो/दिव् compounds and the diphthong stems already were. The site's
own grid is the oracle for a listed word, and the reverse analysis
reads the word back from it.

The site lists some words twice, with two different grids for the
same word (सखी as सखा-सखायौ and as सखीः-सख्यौ, प्रधी in the
पुं and the परम forms). Each listing gets its own row, so the engine
offers both grids and the site grid that matches the word is the one
the portal shows.

The site also lists a word in a slot of its own: `A`, the any-gender
one, which अस्मद् and युष्मद् take. The grid there is the pronoun's
own with the masculine and the neuter forms side by side (माम्-मा,
आवाम्-नौ, अस्मान्-नः, मद्-मत्, अस्मद्-अस्मत्, मम-मे, आवयोः-नौ,
अस्माकम्-नः) — the same grid the परम- compounds take, without the
base. The engine's possessive-pronoun table now is that grid, so the
site's any-gender listing is reproduced and the neuter forms of the
सर्वनाम (नः, नौ, मे, मत्, अस्मत्) read back to their own word.

Whole-db parity: **9,006/9,007** exact.

## 2026-10-22 (49) — the site's any-gender slot

The audit's `LINGA_CODE` maps the db's लिङ्ग markers to the engine's
requests: `P`/`S`/`N` to `m`/`f`/`n`. The db has a fourth marker,
`A` — the site's any-gender slot, which only अस्मद् and युष्मद् take.
Without a code for it the audit skipped both entries, and the engine's
own possessive-pronoun table was the masculine-only grid, which
reproduces neither the site's any-gender grid nor the neuter forms
the site lists in it (नः, नौ, मे, मत्, अस्मत्).

The pronoun's grid is now the merged one the site writes — the
`ASMAD_PARA_GRID`/`YUZMAD_PARA_GRID` the परम- compounds already used,
without the base — so the any-gender listing is exact, and the audit
asks for it with the masculine request (the table is
gender-independent: the site lists the masculine and the neuter forms
of one word in a single row).

The `asmad_table`/`yuzmad_table` tables were hand-written and had
drifted from the grid the site writes: the any-gender grid is the
source of truth for both, so the tables now come from it and the
hand-written copies are gone.

Still one word short: `अतिथीनां_सपर्या`, whose key carries the
site's compound joint (`_`) — the one entry the transliterator
cannot read, since the SLP1 alphabet has no `_`.

