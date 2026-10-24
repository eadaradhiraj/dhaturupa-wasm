# dhaturupa-wasm

Standalone Rust clone of `ashtadhyayi.com/dhatu` — list + info/upasarga +
tinanta/krdanta tables + SLP1 generation (subanta, krdanta click-through, upasarga prefixing).
Detached workspace (`[workspace]` in `Cargo.toml`), safe to move outside `panini-rt`.

Out of scope: prakriya prose, commentary (`sutraani/*`), kosha search (`kosha/*`).

## Data

```bash
mkdir -p data
curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/data.txt -o data/dhatu_data.json
curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/vidyut/vidyut_dhatupatha_5.tsv -o data/vidyut_dhatupatha_5.tsv
curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/kosha/upasargartha_chandrika.json -o data/upasargartha_chandrika.json
curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/shabda/data2.txt -o data/shabda_data2.json
for fam in shuddha nich san yang yangluk; do \
  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${fam}_kartari.txt -o data/ting_${fam}_kartari.json; \
  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${fam}_karmani.txt -o data/ting_${fam}_karmani.json; \
  curl -sL https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${fam}_krut.txt -o data/krut_${fam}.json; \
done
```

15 form files: 10 ting (`{shuddha,nich,san,yang,yangluk} × {kartari,karmani}`,
`nich_kartari` carries both `p*`+`a*` keys) + 5 krut. `shuddha/nich/san`
cover 2229 baseindexes, `yang/yangluk` cover 1782 (no intensives for the
rest). Upstream ships no precomputed prefixed tables — prefixing is
runtime-generated (see bridge below). Legacy `ting_kartari.json` /
`krut.json` names are gone (byte-dupes of the `shuddha` files).

16th file: `shabda/data2.txt` (10 MB) — the site's शब्दपाठः dump, 9,007
entries; ashtadhyayi.com renders its `/shabda/<urlid>` pages from exactly
this. The word view serves it trimmed via `node web/build-shabda.cjs` →
`dist/data/shabda2.json` (2.4 MB): the site's own 24 cells per listed word
(the word-view display oracle), while unlisted words keep the derived
engine tables. `check.sh` refreshes the trimmed file whenever the dump is
present.

Build-only ground truth (never shipped): skt-morph-data
(`SKT_MORPH_DATA`, default `/home/edhiraj/Documents/projs/skt-morph-data/data`)
pins the blob `ting_prefixed`/`krut_prefixed` backfill and the
derive/backfill audits to the same corpus the site's engine version
produced: corpus cells merge into vidyuṭ's slot grid slot-by-slot, and
where a corpus slot carries more stem variants than the engine derives
(√ṛ's san, 09.0032: three against two) the grid boundaries are scaled so
every corpus variant lands in the site's own person-cell. Absent corpus →
backfill skipped with a warning; portal live-derives instead.

Code dependency: `vidyut-prakriya` is vendored at `vendor/vidyut`
(ambuda-org/vidyut@c0fa416, 2024-12-29 — the revision that built the site's
`vidyut_prakriya_bg_5.wasm`; verified corpus-exact over 1,085,520 prefixed
forms). Fork-diffs, both behavior-neutral: the unused `sha256` dependency
removed (pulls tokio, unbuildable for wasm) and their own
`deny(clippy::unwrap_used)` relaxed (fails on 3 wasm-glue sites under the
current toolchain). Do NOT bump without re-running the derive audit —
adjacent engine versions disagree on optional variants and ṣatva.

Mirrors: `/jsondata/dhatu/data.txt`,
`https://raw.githubusercontent.com/ashtadhyayi-com/data/master/dhatu/data.txt`.
Pinned at `a9f9112`, `VIDYUT_WASM_VERSION=5`.

`data/` is fetch-local, git-ignored.

## SLP1 morphology layer

Engine invariant: SLP1-only, byte-exact (`e != E`, `n != N`).

- `src/slp1.rs` — `eq_exact`, `is_slp1_form`, `split_alternants` (`/`), `split_prefix_stack` (`;`).
  Port of `panini-rt/morph/src/slp1.rs`.
- `src/grammar.rs` — `split_lakara_key`, `tinanta_tags`, 9-slot honesty rule.
  Port of `panini-rt/morph/src/grammar.rs`.
- `src/vidyut.rs` — `baseindex -> SLP1 dhatu` join via `vidyut_dhatupatha_5.tsv`
  (`01.0001 -> BU`). This is the Devanagari→SLP1 bridge for dhatus.
- `src/lib.rs` — `dhatu/data.txt` model (`DhatuEntry`), `normalize_baseindex`
  (`1.1 -> 01.0001`), `find_by_baseindex/sk`, `DhatuFilter`, info/upasarga render.
- `src/forms.rs` — `dhatuforms_*` display tables (ting 9-slot, krut `base,m,f,n`)
  + family resolvers (`ting_filename`/`krut_filename` for all 10+5 files)
  + engine-category bridge (`engine_ting_category`: `shuddha -> ting`, …;
  `engine_krut_family`: `shuddha -> krut`, `nich -> nich_krut`, …).
- `src/translit.rs` — Devanagari↔SLP1 boundary (`deva_to_slp1`,
  `slp1_to_deva`, `deva_stack_to_slp1`: `वि + अति -> vi;ati`). Input side of
  `subanta`/`krttable`/`prefix`/`search`; separators (`;`,`,`,`/`) stay with
  the callers.
- `src/hk.rs` — Harvard-Kyoto↔SLP1 (`hk_to_slp1`, `slp1_to_hk`, longest-match:
  `lRR`, `ai`, `kh`, …). Portal search-box input layer; SLP1 is never
  displayed. `search` tries the union of HK-decoded + raw-SLP1-decoded
  Devanagari for ASCII queries.
- `src/engine/` — vendored SLP1 generation from `panini-rt/morph/src`
  (`data,index,krt,sandhi,search,subanta,upasarga,upasplit`; `api.rs` excluded).
  `crate::slp1`/`crate::grammar` are the root ports, shared by the engine.

Bridge notes (`#3`): base search runs on the Devanagari tables directly
(`search` scans ting slots/`;`-`,`-variants and krut cells/`/`-alternants
for one family). Prefixed kṛt forms are served from the pinned corpus cell
for listed stacks (`backfill_prefixed_krut` → `blob.krut_prefixed`); the
live exports take that same cell and return it verbatim
(`derive_prefixed_krut(… pinned_cell)`, `prefixed_krut_row(… rows_json,
row_index, pinned_cell)` partitions it losslessly across the upstream
rows by string matching). Unlisted stacks fall back to
`translit → prefix_krut → SLP1` (engine); the corpus has no truth for them.
`engine::data::DhatuFile` (`skt-morph-data`
corpus model, SLP1, per-dhatu JSON) is *not* the dhatu-page model and is only
used by the vendored generation/search core, not the CLI.

## CLI

```bash
cargo run -q -- list --limit 3
cargo run -q -- list --gana 1 --pada P --sort az --limit 5
cargo run -q -- show 01.0001     # or: show 1.1, show sk1
cargo run -q -- upasarga 01.0001 # meanings (artha_hindi) per upasarga
cargo run -q -- ting 01.0001     # shuddha kartari 9-slot lakara tables (Devanagari)
cargo run -q -- ting 01.0001 --family nich --prayoga kartari  # 20 keys (p*+a*)
cargo run -q -- krut 01.0001 --family san  # base,m,f,n per krt; empty m/f/n = avyaya
cargo run -q -- search भवति      # exact reverse lookup (HK bhavati also works)
cargo run -q -- slp1 01.0001
cargo run -q -- translit भवति    # -> Bavati (auto-detects script)
cargo run -q -- translit hk bhavati   # explicit directions: hk|slp1|deva
# Generation (vendored engine, SLP1 core; Devanagari in and out, `--slp1` for raw):
cargo run -q -- subanta rAma m               # रामः … रामेण … (Devanagari)
cargo run -q -- subanta भवन्ती f             # same, via transliteration
cargo run -q -- krttable krut Satf f BavantI # भवन्ती … भवन्तीभ्याम् … (Devanagari)
cargo run -q -- krttable shuddha शतृ f भवन्ती # same, Devanagari (`/`-alternants tried each)
cargo run -q -- prefix pra Bavati            # -> प्रभवति (root-level context)
cargo run -q -- prefix pra Bavati --family nich --dhatu 01.0001  # per-dhatu GenCtx
cargo run -q -- prefix 'वि + अति' भवति       # -> व्यतिभवति
```

`ting`/`krut` default to `--family shuddha` (`--prayoga kartari` for ting);
override with `--forms` / `--krut` paths.

## Status (all scope items done 2026-10-13)

1. **Tables** — all 15 `dhatuforms` files wired (`ting --family/--prayoga`,
   `krut --family`) + family resolvers and engine-category bridge.
2. **Boundary** — `translit` (Deva↔SLP1) + `hk` (HK↔SLP1); CLI prints
   Devanagari (`--slp1` escape hatch); portal contract (Deva+HK in, Deva
   out, English chrome, SLP1 hidden).
3. **Per-dhatu `GenCtx`** (`src/genctx.rs`; `prefix --family/--dhatu`,
   audit, WASM paths, portal).
4. **Reverse-search audit** — 23M checks, 0 misses exhaustive; builders
   shared by portal/tests/CLI.
5. **WASM + portal** — `src/wasm.rs` exports, `web/` shell, per-dhatu
   blobs (5.9KB gzipped), prebuilt per-family index artifacts (2.6–5MB
   gzipped), offline-first service worker.

Remaining polish (not scope): first-load UX, broader `GenCtx` validation
against site prefixed tables. (The `tfc.n` click-through policy is done —
the तृच् m/n site refs are classes now, pinned in `tests/krt_site_pin.rs`.)

## Portal blobs

One compact JSON per dhatu for lazy portal fetches (detail page = one fetch):

```bash
cargo run -q --release -- build-blobs --out dist/data/blobs
# 2229 blobs + manifest.json (freshness stamp), ~1s
cargo run -q --release -- build-list --out dist/data/dhatu_data_lite.json
# first-load list: list/search fields + global upasarga names only
cargo run -q --release -- build-index --out dist/shards --shards 256
# 256 hash shards (~4.4MB raw, ~0.5MB gzipped each) + manifest.json
cargo run -q --release -- build-subanta-index --out dist/shards --shards 256
# 256 subanta shards (18,028,702 declined forms) — same dir, one manifest
```

Every `manifest.json` names the engine stamp (`build.rs` hashes
`src/stamp.rs:ENGINE_SOURCES`); the web shell warns (console) and the
`artifact_stamp` test fails when an artifact family was built by a different
engine — rebuild the family before releasing.

Measured: 69KB raw / **5.9KB gzipped average** per blob (13.2MB gzipped for
all; ~280KB on average now with the chandrika section). Shape:
`{baseindex, meta, slp1, stacks, ting: {family: {prayoga: {…}}},
krut: {family: {…}}, chandrika: [{upasarga, artha, examples}]}` — slices
feed the WASM entry params directly (`src/blobs.rs`: `ting_slice`/
`krut_slice`/`stacks_json`; chandrika joins by `baseindex`; contract
pinned in `tests/blob_wasm.rs`).

## Portal shell

Static English-chrome app in `web/` (`app.html` + `app.js` + `render.js` +
offline-first `sw.js`): dhatu list with metadata filters (gaṇa, pada, seṭ/
aniṭ/veṭ, sakarmaka/akarmaka/dvikarmaka) and an English-gloss search
(`eat` → अद्, खाद्, स्वद् …), per-dhatu detail
(info, upasargas, ting/krut tables, prefixed views, krt click-through
declension, in-dhatu search), cross-dhatu form search (Devanagari or
Harvard-Kyoto), and a word view (Decline a word: any word in → case
readings + base word + full declension, kṛt readings linked to their dhātu.
For every word the शब्दपाठः lists, the displayed grid is the site's own —
the vendored `data/shabda2.json` cells verbatim (लक्ष्मी gets लक्ष्मीः …,
नदी the ङ्यन्त bare nominative, गति गत्या, सखि सखा …), credited as
`ashtadhyayi.com @urlid`; unlisted words (foreign names like जॉन) keep the
derived engine tables — the lexical नदीसंज्ञा default and the कृदन्त/उगित्
grid both shown for a fresh ī/ū-stem; candra spellings like जॉन fold to
plain vowels). An inflected form resolves to its base word the way the
site's शब्द search does (सरिद्भ्याम् → सरित् → the @sarit1 grid), and the
engine paradigms read back as well as decline: consonant-final nouns
(सरित्, विद्युत्, संविद्, उपनिषद्, युध्, बुध्, ककुभ्, वाच्, त्वच्,
मरुत्), the उस्/इस्-स्वादि neuters (आयुष्, हविस्, धनुस्) and the
word-route grids (गो, नौ, रै, अन्य, इतर, कतर, कतम, उभय, उभ) each
answer with the base word and the case they declined from. Pure
rendering is node-tested (`web/tests/`).

```bash
cargo run -q --release -- build-site --out dist   # shell + data + blobs
node web/build-shabda.cjs                         # word-view शब्द oracle (needs data/shabda_data2.json)
wasm-pack build --target web --out-dir dist/pkg   # WASM bundle
python3 -m http.server 8080 --directory dist      # open /index.html
node --test web/tests/                            # render tests
node web/e2e-dist.check.cjs                     # 36 portal-flow checks vs dist+pkg-node
```

The release profile is tuned for size (`opt-level = "z"`, LTO, one codegen
unit, stripped symbols) and `wasm-opt` runs with `-Oz
--enable-bulk-memory` — `dist/pkg/dhaturupa_wasm_bg.wasm` is 1.03MB
(1.61MB before). No `panic = "abort"`: the wasm target aborts anyway and
the shared profile must keep `cargo test --release` working.

Layout under `dist/`: `index.html app.js render.js sw.js pkg/`
plus `data/dhatu_data.json`, `data/shabda2.json` (word-view शब्द oracle),
`data/shards/{shard,sub}-*.index.json`
(256 + 256 hash shards over all families: base ~4.4MB raw / ~0.5MB gzipped
each, subanta ~5.6MB raw / ~1.5MB gzipped each — one shard of each per
query, LRU 8 in memory, cached offline after),
`data/blobs/*.json` (~6KB gzipped per dhatu). Raw family table files are a
build-time input only and never ship.

## WASM portal glue

`src/wasm.rs` — `String→String` JSON boundary (SLP1 never crosses it).
Devanagari/HK in, Devanagari + English `{"ok":…}` envelopes out.
Table JSON comes from the portal's fetches; family base indexes live in a
WASM-side cache (`index_ensure_family` once, `index_query_family` per
keystroke); per-dhatu prefixed/subanta searches build-query-drop inside one
call. Tested natively in `tests/wasm_api.rs` (18 tests).

```bash
wasm-pack build --target web --out-dir pkg   # browser bundle
wasm-pack build --target nodejs --out-dir pkg-node  # node smoke test
node -e "const w = require('./pkg-node/dhaturupa_wasm.js');
  console.log(w.query_candidates('bhavati'));"
# browser demo (needs pkg/ built): serve repo root, open web/demo.html
python3 -m http.server 8080
```

## Checks

```bash
./check.sh          # the one gate (below), ~2 min
./check.sh --full   # + exhaustive audits (release, ~5 min): 22.9M reverse-search +
                    # 717k kṛdanta click-through + 1.70M tiṅ/kṛt backfill +
                    # 1.35M pinned prefixed-kṛt + 428k pinned derive-kṛt row
                    # checks, 0 unexplained misses

# gate order: fmt → clippy (pedantic deny) → cargo test → [--full] exhaustive
# → wasm-pack build ×3 (browser, node, served dist/pkg — pkg can't drift) →
# dist shell refresh (incl. the trimmed शब्द oracle) → node --test web/tests/
# → e2e-dist.check.cjs (36 portal
# flows on served artifacts) → smoke.cjs (every wasm export the app calls) →
# browser self-test (headless Chromium via chromedriver, 450+ DOM checks)
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
node --test web/tests/
# reverse-search audits (sampled tiers in `cargo test`; exhaustive in release):
cargo test --release -- --ignored
# same runner as a CLI report:
cargo run -q --release -- audit
cargo run -q --release -- audit --full
# subanta parity hand-runs (the site grid vs the derived tables):
node web/audit-an-stems.cjs        # अन्-stem nouns
node web/audit-site-grids.cjs      # every listed word (9,007/9,007 exact;
                                  #   the site's blank cells carry no data)
node web/audit-site-grids.cjs cons # the consonant-final family (322/322)
```
