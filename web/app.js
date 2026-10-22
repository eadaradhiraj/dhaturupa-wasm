// Portal wiring: DOM + fetch + WASM. All rendering lives in render.js.
// Search input: Devanagari or Harvard-Kyoto. Display: Devanagari.
// UI language: English. SLP1 never appears here (it stays inside WASM).
import init, {
  query_candidates, krt_subanta, prefixed_table_ctx, prefixed_krut_row, derive_prefixed_krut, engine_category, to_devanagari,
  derive_prefixed_ting,
  index_load_key, index_query_family, index_drop_family, index_shard,
  split_init, split_query, verify_split,
  search_prefixed_dhatu, search_subanta_dhatu,
  subanta_analyze, subanta_table,
  engine_stamp,
} from '../pkg/dhaturupa_wasm.js';
import {
  escapeHtml, renderTingTables, renderKrutTables, renderParadigm,
  renderHits, renderMeta, renderWordReadings, renderWordTables, siteTables,
  renderSearchReadings, tablePayload,
} from './render.js';

const GANAS = { 1: 'bhvādi', 2: 'adādi', 3: 'juhotyādi', 4: 'divādi', 5: 'svādi', 6: 'tudādi', 7: 'rudhādi', 8: 'tanādi', 9: 'kryādi', 10: 'curādi' };
// Detail-page family order: shuddha first (the site's default view), not the
// blob's alphabetical JSON key order (which made `nich` the default).
const FAMILY_ORDER = ['shuddha', 'nich', 'san', 'yang', 'yangluk'];

// Portal artifacts: hash shards over all forms + the subanta (declined) set.
const SHARDS = 256;
// Shard LRU: keep the last 8 loaded shard sets resident in the WASM index
// cache (one query needs 2, a split verify needs its base shard).
const shardKeys = new Set();
let splitterReady = false;

async function loadShard(key, path) {
  if (shardKeys.has(key)) shardKeys.delete(key);
  else {
    const text = await fetchText(path);
    ok(index_load_key(key, text), 'shard');
  }
  shardKeys.add(key);
  while (shardKeys.size > 8) {
    const oldest = shardKeys.values().next().value;
    index_drop_family(oldest);
    shardKeys.delete(oldest);
  }
}

// Vendored शब्दपाठः oracle: the site's own grids for every word it lists
// (dist/data/shabda2.json, built by web/build-shabda.cjs). Loaded lazily on
// the first word-view run; a failure (file absent) leaves the derived engine
// tables in place. Cells stay the site's Devanagari strings — SLP1 never
// appears here.
let shabdaIndex = null;
let shabdaWait = null;
let shabdaFailed = false;
function ensureShabda() {
  if (shabdaIndex || shabdaFailed) return Promise.resolve();
  if (!shabdaWait) {
    shabdaWait = (async () => {
      try {
        const text = await fetchText('data/shabda2.json');
        const index = new Map();
        for (const [word, linga, urlid, forms] of JSON.parse(text).entries) {
          let list = index.get(word);
          if (!list) index.set(word, (list = []));
          list.push([linga, urlid, forms]);
        }
        shabdaIndex = index;
      } catch {
        shabdaFailed = true;
      } finally {
        shabdaWait = null;
      }
    })();
  }
  return shabdaWait;
}

let dhatuList = null;
let blob = null;
let baseindex = null;

// Compares the served artifacts' manifest stamps against the wasm engine's
// and records the outcome for diagnostics + the self-test: nothing on
// fresh dist, `window._dhatuStaleArtifacts` listing paths otherwise.
async function checkArtifactFreshness() {
  // plain string export, not a result envelope
  const current = engine_stamp();
  const staleList = [];
  for (const pathName of ['data/blobs/manifest.json', 'data/shards/manifest.json']) {
    try {
      const m = JSON.parse(await fetchText(pathName));
      if (m.stamp && m.stamp !== current) {
        staleList.push(`${pathName}: ${m.stamp}`);
        console.warn(`dhatu: stale ${pathName} (built with ${m.stamp}, engine is ${current}) — rebuild build-blobs/build-index`);
      }
    } catch {
      // absent manifest → unknown freshness
    }
  }
  // selftest asserts on this window flag; underscore keeps it console-only
  window._dhatuStaleArtifacts = staleList;
}

const el = (id) => document.getElementById(id);

function status(msg) {
  el('status').textContent = msg || '';
}

function show(view) {
  ['view-list', 'view-detail', 'view-search', 'view-word'].forEach((v) => {
    el(v).hidden = v !== view;
  });
}

async function ensureList() {
  if (!dhatuList) {
    // First-load trim: the lite list carries only the fields the list/search
    // views read (build-list); the full site dump keeps feeding the builders.
    dhatuList = await fetch('data/dhatu_data_lite.json')
      .then((r) => (r.ok ? r : null))
      .then((r) => (r ? r.json() : fetch('data/dhatu_data.json').then((rr) => rr.json())));
  }
  return dhatuList;
}

async function getJson(path) {
  const res = await fetch(path);
  if (!res.ok) throw new Error(`HTTP ${res.status} ${path}`);
  return res.json();
}

// Fetches that can never hang the UI: a stalled shard fetch rejects loudly
// with its URL instead of wedging the search forever.
async function fetchText(path, timeoutMs = 20000) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const res = await fetch(path, { signal: ctrl.signal });
    return await res.text();
  } catch (e) {
    throw new Error(`fetch ${path}: ${e.name === 'AbortError' ? 'timed out' : e}`);
  } finally {
    clearTimeout(timer);
  }
}

async function main() {
  await init();

  // Artifact freshness (src/stamp.rs): served artifacts stamp the engine
  // build they came from; a mismatch says the heavy rebuild step was
  // skipped after an engine change. A missing manifest is unknown
  // freshness (first deployment) — never reported as staleness. Runs
  // unfenced: a console warning, not a UI gate.
  checkArtifactFreshness().catch(() => {});

  // fill gana/pada dropdowns
  const ganaSelect = el('list-gana');
  Object.entries(GANAS).forEach(([k, v]) => {
    const opt = document.createElement('option');
    opt.value = k;
    opt.textContent = v;
    ganaSelect.appendChild(opt);
  });

  await ensureList().catch((err) => status(String(err)));
  runListFilter();
  route();
}

function route() {
  // Tolerate a leading slash (`#/dhatu/…` links) and an optional upasarga
  // suffix (`#dhatu/03.0010/प्र`) so a prefixed-form hit opens the detail
  // page with that prefix already selected. ADDITIONS.md 1.4 layers a
  // query part on top (`#search?q=…`, `#word?q=…`,
  // `#dhatu/<id>?fam=…&stack=…`) so a view can be book-marked with its
  // state; `+` is never space here (stack names contain literal ` + `).
  const raw = location.hash.slice(1).replace(/^\/+/, '');
  const [path, query] = raw.split('?');
  const params = new Map((query || '').split('&').filter(Boolean).map((kv) => {
    const i = kv.indexOf('=');
    return i < 0 ? [decodeURIComponent(kv), ''] : [
      decodeURIComponent(kv.slice(0, i)),
      decodeURIComponent(kv.slice(i + 1)),
    ];
  }));
  const hash = decodeURIComponent(path || '');
  if (hash.startsWith('dhatu/')) {
    const [id, stack] = hash.slice(6).split('/');
    openDhatu(id, params.get('stack') || stack || '', params.get('fam') || '');
  } else if (hash === 'search') {
    show('view-search');
    const q = params.get('q') || '';
    if (q) {
      el('search-q').value = q;
      runFormSearch();
    }
  } else if (hash === 'word') {
    show('view-word');
    const q = params.get('q') || '';
    if (q) {
      el('word-q').value = q;
      runWordForm();
    }
  } else {
    show('view-list');
  }
}

window.addEventListener('hashchange', route);

function runListFilter() {
  const raw = el('list-q').value.trim();
  const gana = el('list-gana').value;
  const pada = el('list-pada').value;

  let rows = dhatuList.data;
  if (gana) rows = rows.filter((r) => String(r.gana) === gana);
  if (pada) rows = rows.filter((r) => r.pada === pada);
  // ADDITIONS.md 3.2: the dhatu_data.json `settva` (S/A/V) and `karma`
  // (S/A/D) fields as list dropdowns — the CLI already takes --settva/-
  // karma over the same fields; rows with '-' are unmatched and drop out.
  const settva = el('list-settva').value;
  if (settva) rows = rows.filter((r) => r.settva === settva);
  const karma = el('list-karma').value;
  if (karma) rows = rows.filter((r) => r.karma === karma);
  if (raw) {
    // Exact dhātu match, compared in Devanagari: HK `dA` must normalize to
    // `दा` first; queries that decode to nothing fall through as-is.
    let needle = raw;
    try {
      needle = ok(to_devanagari(raw), 'translit').deva || raw;
    } catch {
      // not Devanagari or Harvard-Kyoto — keep raw (matches nothing exactly)
    }
    const exact = rows.filter((r) => r.dhatu === needle);
    if (exact.length > 0 || !/[a-z]/.test(raw)) {
      rows = exact;
    } else {
      // ADDITIONS.md 3.1: an ASCII/HK query no dhātu matches exactly
      // ('eat' decodes to एअत्) searches the English gloss instead.
      const latin = raw.toLowerCase();
      rows = rows.filter((r) => (r.artha_english || '').toLowerCase().includes(latin));
    }
  }

  el('list-count').textContent = `${rows.length} / ${dhatuList.data.length}`;
  el('list-links').innerHTML = '';
  showMoreLinks(rows, raw);
}

function showMoreLinks(rows, raw) {
  const container = el('list-links');
  const existing = container.querySelectorAll('a').length;
  const chunk = rows.slice(existing, existing + 50);
  chunk.forEach((r) => {
    const a = document.createElement('a');
    a.href = `#dhatu/${r.baseindex}`;
    a.textContent = `${r.dhatu} — ${r.artha} (${r.baseindex})`;
    a.className = 'deva';
    container.appendChild(a);
    container.appendChild(document.createElement('br'));
  });
  if (existing + chunk.length < rows.length) {
    el('list-more').style.display = 'inline';
  } else {
    el('list-more').style.display = 'none';
  }
}

el('list-more').onclick = () => runListFilter();
el('list-go').onclick = runListFilter;
el('list-q').addEventListener('keypress', (e) => { if (e.key === 'Enter') runListFilter(); });

async function openDhatu(id, stackWanted, famWanted) {
  baseindex = id;
  show('view-detail');
  status(`loading ${id}…`);
  blob = await getJson(`data/blobs/${id}.json`);
  status('');
  el('detail-title').textContent = `${blob.meta.dhatu} — ${blob.meta.artha} (${id})`;
  el('detail-meta').innerHTML = renderMeta(blob.meta);

  const family = el('detail-family');
  family.innerHTML = '';
  fillSelect(family, FAMILY_ORDER.filter((f) => blob.ting[f]));
  // Deep-linked family (`?fam=`): only when this dhātu carries it.
  if (famWanted && [...family.options].some((o) => o.value === famWanted)) {
    family.value = famWanted;
  }

  // One unified upasarga list: this dhātu's listed stacks (some are combos
  // like `वि + आ`) plus every single upasagra used across the corpus.
  const stack = el('detail-stack');
  await ensureList().catch(() => {});
  const names = new Set(blob.stacks);
  const singles = dhatuList.upasarga_singles
    // full-dump fallback (transition state): sweep the entries instead
    || [...new Set(dhatuList.data.flatMap((e) =>
      (e.upasargas || []).map((u) => u.name)))].filter((n) => !n.includes(' + '));
  for (const name of singles) names.add(name);
  stack.innerHTML = '<option value="">—</option>' +
    [...names].sort((a, b) => a.localeCompare(b, 'sa'))
      .map((name) => `<option>${escapeHtml(name)}</option>`).join('');

  // A prefixed-form hit carries its stack in the hash
  // (`#dhatu/03.0010/प्र` or `?stack=`): open straight into that view.
  if (stackWanted && [...stack.options].some((o) => o.value === stackWanted)) {
    stack.value = stackWanted;
  }

  renderDetailTables();
}

function fillSelect(select, options) {
  options.forEach((opt) => {
    const o = document.createElement('option');
    o.value = opt;
    o.textContent = opt;
    select.appendChild(o);
  });
}

function blobTing(family, prayoga) {
  return (blob.ting[family] && blob.ting[family][prayoga]) || {};
}

function blobKrut(family) {
  return blob.krut[family] || {};
}

function renderDetailTables() {
  const family = el('detail-family').value;
  const prayoga = el('detail-prayoga').value;
  const stack = el('detail-stack').value;

  const tingTables = blobTingTables(family, prayoga);
  const krutTables = blobKrutTables(family);

  if (stack) {
    renderPrefixedTables(stack, tingTables, krutTables, family);
  } else {
    el('detail-ting').innerHTML =
      Object.keys(tingTables).length === 0
        ? '<p class="muted">No ting forms for this family/prayoga.</p>'
        : renderTingTables(tingTables);
    el('detail-krut').innerHTML =
      Object.keys(krutTables).length === 0
        ? '<p class="muted">No krt forms for this family.</p>'
        : renderKrutTables(krutTables, family);
    addCopyButtons(el('detail-ting'));
    addCopyButtons(el('detail-krut'));
  }

  el('detail-hits').innerHTML = '';
  el('detail-subanta').innerHTML = '';
}

// ting blob shape: {lakara: 'row;row;…'} — each `;` row is a puruṣa slot,
// each ',' part an alternant of that slot. (Array blobs also accepted.)
function blobTingTables(family, prayoga) {
  const ting = blobTing(family, prayoga);
  const tables = {};
  Object.entries(ting || {}).forEach(([lakara, rows]) => {
    const slots = Array.isArray(rows)
      ? rows
      : typeof rows === 'string'
        ? rows.split(';').map((row) =>
            row.split(',').map((s) => s.trim()).filter((s) => s !== ''))
        : null;
    if (slots && slots.some((slot) => slot.length > 0)) tables[lakara] = slots;
  });
  return tables;
}

// krut blob shape: {krt: 'base,m,f,n;base,m,f,n;…'}.
function blobKrutTables(family) {
  const krut = blobKrut(family);
  const tables = {};
  Object.entries(krut || {}).forEach(([krt, v]) => {
    const rows = String(v).split(';')
      .map((r) => r.split(','))
      .filter((p) => p.length === 4)
      .map((p) => ({ base: p[0], m: p[1], f: p[2], n: p[3] }));
    if (rows.length === 0) return;
    tables[krt] = rows;
  });
  return tables;
}

// Prefixed view: three tiers, in order of fidelity.
//   1. Backfilled tables (`blob.ting_prefixed[stack][family][prayoga]`):
//      corpus values in engine slot grouping — byte-exact site parity.
//   2. Live engine derivation (`derive_prefixed_ting`) for unlisted stacks:
//      same engine the site drives, modulo engine-version deltas.
//   3. Cell-prefixing the upstream base tables (kṛdanta path always uses
//      this — verified against the corpus; tiṅanta fallback when the
//      engine cannot derive).
function renderPrefixedTables(stack, tingTables, krutTables, family) {
  const prayoga = el('detail-prayoga').value;
  const pre = blob.ting_prefixed && blob.ting_prefixed[stack] &&
    blob.ting_prefixed[stack][family] && blob.ting_prefixed[stack][family][prayoga];
  let pTing = null;
  if (pre) {
    pTing = parseTingCells(pre);
  } else {
    try {
      const res = ok(derive_prefixed_ting(stack, baseindex, family, prayoga), 'derive');
      pTing = parseTingCells(res.tables);
    } catch (e) {
      status(String(e));
    }
  }
  if (pTing) {
    el('detail-ting').innerHTML =
      Object.keys(pTing).length === 0
        ? '<p class="muted">No ting forms for this stack/family/prayoga.</p>'
        : renderTingTables(pTing);
  } else {
    renderCellPrefixedTing(stack, tingTables, family);
  }
  renderCellPrefixedKrut(stack, krutTables, family);
  addCopyButtons(el('detail-ting'));
}

// Blob/upstream `{lakara: 'row;row;…'}` strings → renderer shape.
function parseTingCells(ting) {
  const tables = {};
  Object.entries(ting || {}).forEach(([lakara, rows]) => {
    const slots = Array.isArray(rows)
      ? rows
      : typeof rows === 'string'
        ? rows.split(';').map((row) =>
            row.split(',').map((s) => s.trim()).filter((s) => s !== ''))
        : null;
    if (slots && slots.some((slot) => slot.length > 0)) tables[lakara] = slots;
  });
  return tables;
}

function renderCellPrefixedTing(stack, tingTables, family) {
  const prefTing = prefixedSurfaces(stack, Object.values(tingTables).flat(2), family, 'ting');
  const cell = (map, value) => value
    ? value.split('/').filter(Boolean).map((v) => map.get(v) || '—').join('/')
    : '—';

  const pTing = {};
  Object.entries(tingTables).forEach(([lakara, slots]) => {
    pTing[lakara] = slots.map((slot) => slot.map((c) => cell(prefTing, c)));
  });
  el('detail-ting').innerHTML =
    Object.keys(pTing).length === 0
      ? '<p class="muted">No ting forms for this stack/family/prayoga.</p>'
      : renderTingTables(pTing);
  addCopyButtons(el('detail-ting'));
}

function renderCellPrefixedKrut(stack, krutTables, family) {
  // The site derives its prefixed kṛdanta view live
  // (`generateKridantaFormsWithUpasarga` → the same vidyuṭ engine, the
  // same six special tags) over ITS OWN row set — the base table's rows.
  // The corpus-pinned rows are a post-hoc prefixed approximation: they
  // miss the स्य-*/भावकर्म-* rows entirely and their स्य-* forms lack the
  // लृङ् lakāra override, so they are the fallback, never the source.
  const pre = blob.krut_prefixed && blob.krut_prefixed[stack] && blob.krut_prefixed[stack][family];
  const category = ok(engine_category(family, 'krut'), 'category').category;
  // स्य-*/भावकर्म-* rows derive engine-only (live-site behaviour: the
  // deriver's own stems, no lakāra/prayoga, no corpus cells; a row the
  // engine cannot derive is simply absent there). Their corpus pieces
  // carry the stale लृङ् dump build — never shown prefixed.
  const SYA_KRTS = new Set(['स्य-शतृ', 'स्य-शानच्', 'भावकर्म-शानच्', 'स्य-भावकर्म-शानच्']);
  const pKrut = {};
  Object.entries(krutTables).forEach(([krt, rows]) => {
    if (!rows || rows.length === 0) return;
    // The site drops क्त्वा once an upasarga is selected
    // (`0<n.length&&"क्त्वा"==e`).
    if (krt === 'क्त्वा' && stack) return;
    let derived = null;
    try {
      const res = ok(derive_prefixed_krut(
        stack, baseindex, family, krt,
        JSON.stringify(rows.map((r) => ({ base: r.base, m: r.m, f: r.f, n: r.n }))),
        blob.slp1, category, blob.note || '', ''), 'derive-krut');
      if (res.rows && res.rows.length > 0) derived = res.rows;
    } catch (e) {
      status(String(e));
    }
    if (derived) {
      // Mask avyaya cells: a field empty in every upstream row stays
      // empty (upstream emptiness marks indeclinables).
      const filled = { m: false, f: false, n: false };
      for (const r of rows) {
        for (const field of ['m', 'f', 'n']) {
          if (r[field] && r[field].trim() !== '') filled[field] = true;
        }
      }
      pKrut[krt] = derived.map((r) => ({
        base: r.base || '—',
        m: filled.m ? (r.m || '—') : '—',
        f: filled.f ? (r.f || '—') : '—',
        n: filled.n ? (r.n || '—') : '—',
      }));
      return;
    }
    if (SYA_KRTS.has(krt)) return;
    // The corpus's own row for this kṛt, when the engine cannot derive
    // it (unādi gaps) and the corpus carries it.
    const pinned = pre && pre[krt];
    if (pinned) {
      const rowsPinned = String(pinned).split(';')
        .map((r) => r.split(','))
        .filter((p) => p.length === 4)
        .map((p) => ({ base: p[0], m: p[1], f: p[2], n: p[3] }));
      if (rowsPinned.length > 0) {
        pKrut[krt] = rowsPinned.map((r) => ({
          base: r.base || '—',
          m: r.m || '—',
          f: r.f || '—',
          n: r.n || '—',
        }));
        return;
      }
    }
    const rowsJson = JSON.stringify(rows.map((r) => ({ base: r.base, m: r.m, f: r.f, n: r.n })));
    const renderRow = (krt, r, i) => {
      const res = ok(prefixed_krut_row(
        stack, krt, rowsJson, i,
        blob.slp1, category, blob.note || '', ''), 'prefixed-krut');
      const cell = (v) => (v && v.length ? v.join('/') : '—');
      return {
        base: cell(res.row.base),
        m: cell(res.row.m),
        f: cell(res.row.f),
        n: cell(res.row.n),
      };
    };
    pKrut[krt] = rows.map((r, i) => renderRow(krt, r, i));
  });
  el('detail-krut').innerHTML =
    Object.keys(pKrut).length === 0
      ? '<p class="muted">No krt forms for this stack/family.</p>'
      : renderKrutTables(pKrut, family);
  addCopyButtons(el('detail-krut'));
}

// base form → prefixed-surface string via `prefixed_table_ctx` (GenCtx:
// vidyut SLP1 dhatu + engine category + nitya-adi note from the blob).
function prefixedSurfaces(stack, forms, family, table) {
  const map = new Map();
  const uniq = [...new Set(forms.filter(Boolean))];
  if (uniq.length === 0) return map;
  try {
    const category = ok(engine_category(family, table), 'category').category;
    const res = ok(prefixed_table_ctx(
      stack, JSON.stringify(uniq), blob.slp1, category, blob.note || ''), 'prefixed');
    res.tables.forEach((surfaces, i) => {
      // '/' alternant separator — same convention as the base cells, so
      // renderKrutTables makes one link per form and clicks pass a single
      // stem to the declension engine.
      map.set(uniq[i], surfaces.length ? surfaces.join('/') : '—');
    });
  } catch (e) {
    status(String(e));
  }
  return map;
}

function onKrtClick(event) {
  const link = event.target.closest('a[data-krt]');
  if (!link) return;
  event.preventDefault();
  // render.js already applied the site click convention: `value` is the row
  // base (m/n) or the f form itself (आ/ई endings). `baseindex` is the dhātu
  // row id that selects the site's reference paradigm inside krt_subanta.
  const { family, krt, field, value } = link.dataset;
  const kind = link.dataset.gender || 'm';
  let tables = [];
  try {
    tables = ok(krt_subanta(family, krt, kind, value, baseindex), 'declension').tables;
  } catch {
    // fall through to the error panel
  }
  if (tables.length === 0) {
    el('detail-subanta').innerHTML =
      `<p class="deva">${escapeHtml(value)} [${escapeHtml(field)}]</p>` +
      `<p class="muted">${escapeHtml(value)} does not decline (${escapeHtml(kind)}).</p>`;
  } else {
    el('detail-subanta').innerHTML =
      `<h4 class="deva">${escapeHtml(value)} [${escapeHtml(field)}]</h4>` +
      tables.map(renderParadigm).join('');
    addCopyButtons(el('detail-subanta'));
  }
}

function ok(result, context) {
  // wasm-bindgen hands back JSON strings; parsed objects pass through.
  const parsed = typeof result === 'string' ? JSON.parse(result) : result;
  if (!parsed.ok) throw new Error(`${context}: ${parsed.error}`);
  return parsed;
}

// ADDITIONS.md 1.3: one-click "Copy Table" (Markdown / TSV). Each rendered
// table gets a small button bar; cell text is read from the DOM verbatim
// (so copied output always equals what is displayed), serialization is the
// pure tablePayload in render.js.
function tableRows(tableEl) {
  const rows = [];
  const head = tableEl.querySelector('thead tr');
  if (head) {
    rows.push({ label: null, cells: [...head.querySelectorAll('th,td')].map((x) => x.textContent.trim()) });
  }
  for (const tr of tableEl.querySelectorAll('tbody tr')) {
    const th = tr.querySelector('th');
    rows.push({
      label: th ? th.textContent.trim() : null,
      cells: [...tr.querySelectorAll('td')].map((x) => x.textContent.trim()),
    });
  }
  return rows;
}

function addCopyButtons(container) {
  if (!container) return;
  for (const table of [...container.querySelectorAll('table')]) {
    if (table.parentElement.classList.contains('table-wrap')) continue;
    const wrap = document.createElement('div');
    wrap.className = 'table-wrap';
    table.replaceWith(wrap);
    const bar = document.createElement('div');
    bar.className = 'copy-bar';
    bar.innerHTML =
      '<button type="button" class="copy-btn" data-kind="md">Copy Markdown</button>' +
      '<button type="button" class="copy-btn" data-kind="tsv">Copy TSV</button>';
    wrap.append(bar, table);
  }
}

function flash(btn, msg) {
  btn.dataset.label = btn.textContent;
  btn.textContent = msg;
  setTimeout(() => { btn.textContent = btn.dataset.label; }, 1200);
}

async function copyTable(btn) {
  const table = btn.closest('.table-wrap').querySelector('table');
  const text = tablePayload(tableRows(table), btn.dataset.kind);
  // Assertable signal for the headless suite; harmless in normal use.
  window.__lastCopy = { kind: btn.dataset.kind, text };
  try {
    await navigator.clipboard.writeText(text);
    flash(btn, 'Copied!');
  } catch {
    // Clipboard API denied (headless or insecure context): execCommand path.
    const ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    flash(btn, document.execCommand('copy') ? 'Copied!' : 'Copy failed');
    ta.remove();
  }
}

document.addEventListener('click', (e) => {
  const btn = e.target.closest('.copy-btn');
  if (btn) copyTable(btn);
});

el('detail-krut').onclick = onKrtClick;
el('detail-family').onchange = renderDetailTables;
el('detail-prayoga').onchange = renderDetailTables;
el('detail-stack').onchange = renderDetailTables;

async function runDetailSearch() {
  if (!blob) return;
  const q = el('detail-q').value.trim();
  if (!q) return;
  const stacks = JSON.stringify(blob.stacks);
  const dhatu = JSON.stringify({
    family: el('detail-family').value,
    baseindex: baseindex,
    slp1: blob.slp1,
    note: ''
  });
  const kartari = JSON.stringify(blob.ting[el('detail-family').value].kartari || {});
  const karmani = JSON.stringify(blob.ting[el('detail-family').value].karmani || {});
  const krut = JSON.stringify(blob.krut[el('detail-family').value] || {});
  try {
    const result = ok(await search_prefixed_dhatu(kartari, karmani, krut, dhatu, stacks, q), 'prefixed');
    el('detail-hits').innerHTML = renderHits(result.hits, 'prefixed');
  } catch (e) {
    status(String(e));
  }
}

el('detail-go').onclick = runDetailSearch;
el('detail-q').addEventListener('keypress', (e) => { if (e.key === 'Enter') runDetailSearch(); });

async function runFormSearch() {
  show('view-search');
  const q = el('search-q').value.trim();
  if (!q) return;
  // Clear synchronously, before the first await: test waiters (and fast
  // typists) must never observe the previous search's verdict as this
  // run's. (A stale "N hits" note once made the suite assert on one query
  // and click on another's cleared DOM.)
  status('searching…');
  el('search-hits').innerHTML = '';
  el('search-note').textContent = '';
  await ensureList().catch(() => {});
  // Serialize rapid searches: only the latest run may render (an older
  // run's late shard fetch must not overwrite newer results).
  runFormSearch.id = (runFormSearch.id || 0) + 1;
  const mine = runFormSearch.id;
  const alive = () => mine === runFormSearch.id;
  const t0 = performance.now();
  try {
    const reads = ok(query_candidates(q), 'candidates').readings;
    if (reads.length === 0) throw new Error('query is not Devanagari or Harvard-Kyoto');
    const hits = [];
    const seen = new Set();
    for (const r of reads.slice(0, 8)) {
      const n = ok(index_shard(r.text, SHARDS), 'shard').shard;
      // Two sharded sets answer any query: base (ting/krt/prefixed) and
      // global subanta (declined) forms, under distinct cache keys.
      for (const [prefix, path] of [
        ['b', `data/shards/shard-${String(n).padStart(2, '0')}.index.json`],
        ['u', `data/shards/sub-${String(n).padStart(2, '0')}.index.json`],
      ]) {
        await loadShard(prefix + n, path);
        const res = ok(index_query_family(prefix + n, r.text), 'query');
        for (const hit of res.hits) {
          const id = JSON.stringify(hit);
          if (!seen.has(id)) {
            seen.add(id);
            hits.push(hit);
          }
        }
      }
    }
    // Prefixed forms ship in neither shard set: split the query into
    // (upasarga, base) candidates and soundness-verify each by exact
    // regeneration — this also covers unlisted stacks.
    if (!splitterReady) {
      const names = dhatuList.upasarga_names
        || [...new Set(dhatuList.data.flatMap((e) =>
          (e.upasargas || []).map((u) => u.name)))];
      ok(split_init(JSON.stringify(names)), 'splitter');
      splitterReady = true;
    }
    const triedSplits = new Set();
    for (const s of ok(split_query(q, SHARDS), 'splits').splits.slice(0, 8)) {
      if (!alive()) return;
      const id = `${s.stack} ${s.base}`;
      if (triedSplits.has(id)) continue;
      triedSplits.add(id);
      // The split base may live in either artifact set: tiṇanta/kṛdanta
      // cells in the base shards, declined (subanta) forms in the subanta
      // shards (e.g. प्रददद्भिः ← प्र + ददद्भिः).
      const file = `data/shards/shard-${String(s.shard).padStart(2, '0')}.index.json`;
      const sub = `data/shards/sub-${String(s.shard).padStart(2, '0')}.index.json`;
      await loadShard('b' + s.shard, file);
      await loadShard('u' + s.shard, sub);
      const v = ok(verify_split(q, s.stack, s.base), 'verify');
      for (const hit of v.hits) {
        const key = JSON.stringify(hit);
        if (!seen.has(key)) {
          seen.add(key);
          hits.push(hit);
        }
      }
    }
    // No index hit: the shards carry the kṛt families only, so a noun
    // paradigm (सरिद्भ्याम् → सरित्) finds nothing. The word analyzer
    // knows it, so fall back to the base-word readings — the site's
    // शब्द search resolves a form to its base word the same way.
    let readings = [];
    if (hits.length === 0) {
      try {
        const res = ok(subanta_analyze(q), 'word');
        readings = res.analyses || [];
        // The analyzer offers every plausible candidate; the corpus knows
        // which base words really exist, so keep the readings the शब्दपाठः
        // lists (सरित् for सरिद्भ्याम्) and fall back to the first few
        // when the corpus has none (an unlisted word).
        await ensureShabda().catch(() => {});
        if (alive() && shabdaIndex) {
          const listed = readings.filter((a) => siteTables(shabdaIndex, a.stem).length > 0);
          if (listed.length > 0) readings = listed;
          else readings = readings.slice(0, 6);
        } else if (readings.length > 6) {
          readings = readings.slice(0, 6);
        }
      } catch {
        readings = [];
      }
    }
    const html = hits.length > 0
      ? renderHits(hits.slice(0, 200), dhatuCards(), reads[0].text)
      : renderSearchReadings(readings, q);
    const ms = Math.round(performance.now() - t0);
    if (!alive()) return;
    el('search-hits').innerHTML = html;
    el('search-note').textContent =
      `${hits.length} hit${hits.length === 1 ? '' : 's'}` +
      (hits.length > 200 ? ' (showing first 200)' : '') +
      (readings.length
        ? ` — ${readings.length} base-word reading${readings.length === 1 ? '' : 's'}`
        : '') +
      ` — ${ms}ms`;
    status('done');
  } catch (e) {
    if (alive()) status(String(e));
  }
}

// baseindex → {dhatu, artha} for hit cards (Devanagari display only).
function dhatuCards() {
  const map = {};
  for (const r of dhatuList ? dhatuList.data : []) {
    map[r.baseindex] = { dhatu: r.dhatu, artha: r.artha };
  }
  return map;
}

// Word view: analyze any word (readings = base word + case) and decline it
// as a fresh stem (all three genders), so foreign names get paradigms too.
async function runWordForm() {
  show('view-word');
  const q = el('word-q').value.trim();
  if (!q) return;
  // Clear synchronously, before the first await (same stale-DOM discipline
  // as the form search): a previous verdict must never be read as this
  // run's.
  status('analyzing…');
  el('word-note').textContent = '';
  el('word-readings').innerHTML = '';
  el('word-tables').innerHTML = '';
  // Serialize runs: a late shard fetch from an older run must not rewrite
  // the newer run's readings.
  runWordForm.id = (runWordForm.id || 0) + 1;
  const mine = runWordForm.id;
  const alive = () => mine === runWordForm.id;
  try {
    const res = ok(subanta_analyze(q), 'word');
    const n = res.analyses.length;
    const setNote = (count, site) => {
      el('word-note').textContent =
        `${res.word} — ${n} reading${n === 1 ? '' : 's'}, ` +
        `${count} declension table${count === 1 ? '' : 's'}${site ? ' · ashtadhyayi.com' : ''}`;
    };
    setNote(res.tables.length, false);
    el('word-readings').innerHTML = renderWordReadings(res.analyses);
    // The vendored शब्दपाठः is the oracle for every word it lists: once it
    // has loaded (lazily, first run), swap the site's own grids in place.
    // The derived tables stand for words the corpus does not list (foreign
    // names, fresh stems) — so they are rendered only when the oracle has
    // no grid, which also keeps a junk derived table (an inflected input
    // shaped like a participle) from flashing before the swap.
    ensureShabda().then(() => {
      if (!alive()) return;
      let site = shabdaIndex ? siteTables(shabdaIndex, res.word) : [];
      if (site.length === 0 && shabdaIndex) {
        // An inflected form resolves to its base word: the site's शब्द
        // search shows that word's grid, so try the reading stems (the
        // oracle is keyed by citation — सरिद्भ्याम् → सरित्).
        for (const a of res.analyses) {
          site = siteTables(shabdaIndex, a.stem);
          if (site.length > 0) break;
        }
      }
      if (site.length === 0) {
        // The corpus does not list this word: the derived tables stand in
        // (foreign names, fresh stems).
        setNote(res.tables.length, false);
        el('word-tables').innerHTML = renderWordTables(res.tables);
        addCopyButtons(el('word-tables'));
        return;
      }
      setNote(site.length, true);
      el('word-tables').innerHTML = renderWordTables(site);
      addCopyButtons(el('word-tables'));
    });
    // Reading → dhātu links: probe the subanta shard for the input form
    // itself; kṛt postings whose slot matches a reading name that reading's
    // dhātu (गच्छन् → शतृ of गम्). Best-effort: a fetch failure leaves the
    // plain reading list in place, and a reading table the user already
    // opened is never torn down by the enrichment render.
    if (n > 0) {
      try {
        const shard = ok(index_shard(res.word, SHARDS), 'shard').shard;
        await loadShard('u' + shard,
          `data/shards/sub-${String(shard).padStart(2, '0')}.index.json`);
        if (alive() && !el('word-readings').querySelector('div.reading-table')) {
          const hits = ok(index_query_family('u' + shard, res.word), 'query').hits;
          const refs = hits.filter((h) => h.kind === 'subanta');
          if (refs.length > 0) {
            el('word-readings').innerHTML =
              renderWordReadings(res.analyses, refs, dhatuCards());
          }
        }
      } catch { /* corpus refs are best-effort */ }
    }
    status('done');
  } catch (e) {
    el('word-readings').innerHTML = '<p class="muted">—</p>';
    status(String(e));
  }
}

// Reading rows load that base word's own declension on demand.
async function onReadingTableClick(event) {
  const btn = event.target.closest('button.reading-table-btn');
  if (!btn) return;
  const li = btn.closest('li');
  const open = li.nextElementSibling;
  if (open && open.classList.contains('reading-table')) {
    open.remove();
    btn.textContent = 'table';
    return;
  }
  try {
    // The reading's base word renders from the site oracle when it lists the
    // word (same rule as the tables above); otherwise the engine's own table
    // for this stem + gender.
    await ensureShabda();
    const site = shabdaIndex ? siteTables(shabdaIndex, btn.dataset.stem) : [];
    const tables = site.length
      ? site
      : ok(subanta_table(btn.dataset.stem, btn.dataset.linga), 'table').tables;
    const div = document.createElement('div');
    div.className = 'reading-table';
    div.innerHTML = tables.map(renderParadigm).join('');
    li.after(div);
    addCopyButtons(div);
    btn.textContent = 'hide';
  } catch (e) {
    status(String(e));
  }
}

el('word-go').onclick = runWordForm;
el('word-q').addEventListener('keypress', (e) => { if (e.key === 'Enter') runWordForm(); });
el('word-readings').onclick = onReadingTableClick;

el('search-go').onclick = runFormSearch;
el('search-q').addEventListener('keypress', (e) => { if (e.key === 'Enter') runFormSearch(); });

el('nav-list').onclick = () => { location.hash = ''; };
el('nav-search').onclick = () => { location.hash = 'search'; };
el('nav-word').onclick = () => { location.hash = 'word'; };

// Exposed for the browser self-test (web/selftest.js) to await boot.
window.__appReady = main();
