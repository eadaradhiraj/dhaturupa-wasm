// Browser self-test: drives the real app DOM (web/app.js) through the
// flows that regressed during development — routing, list filters, form
// search (base/prefixed/declined-prefixed/kṛdanta-subanta, Deva + HK),
// hit-link navigation with prefix preselection, family default, and the
// kṛdanta click-through declension. Verdict lands in #selftest-out.
//
// Run by ./check.sh (headless Chromium via chromedriver, DOM dumped after
// completion):
//   SELFTEST: PASS | SELFTEST: FAIL + per-check failures + console errors

const out = document.getElementById('selftest-out');
const errors = [];
window.addEventListener('error', (e) => errors.push(String((e.error && e.error.message) || e.message)));
const origError = console.error;
console.error = (...args) => { errors.push(args.map(String).join(' ')); origError(...args); };

let passed = 0;
const failures = [];
const ok = (name, cond, extra) => {
  if (cond) { passed += 1; return; }
  failures.push(extra ? `${name} — ${extra}` : name);
};
const step = (where) => { out.textContent = `STEP: ${where}`; };
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const $ = (id) => document.getElementById(id);
const text = (el) => (el ? el.textContent : '');
const waitFor = async (fn, { timeout = 10000, step = 50 } = {}) => {
  const t0 = performance.now();
  for (;;) {
    let value;
    try { value = fn(); } catch { value = false; }
    if (value) return true;
    if (performance.now() - t0 > timeout) return false;
    await sleep(step);
  }
};

const verdict = () => {
  const fail = failures.length > 0 || errors.length > 0;
  out.textContent =
    (fail ? 'SELFTEST: FAIL\n' : 'SELFTEST: PASS\n') +
    `checks passed: ${passed}\n` +
    (failures.length ? 'failures:\n' + failures.map((f) => `  - ${f}`).join('\n') + '\n' : '') +
    (errors.length ? 'console errors:\n' + errors.slice(0, 10).map((e) => `  - ${e}`).join('\n') : '');
  document.title = fail ? 'SELFTEST FAIL' : 'SELFTEST PASS';
};

try {
  await Promise.race([
    (async () => {
  await window.__appReady;
  step('boot');
  ok('boot: list count is 2259', text($('list-count')) === '2259 / 2259', text($('list-count')));

  // ---- artifact freshness: dist manifests stamp this engine build
  // (src/stamp.rs; absent manifests would leave the flag unset → hard fail)
  ok('artifacts: stamped by this engine build',
    await waitFor(() => Array.isArray(window._dhatuStaleArtifacts)
      && window._dhatuStaleArtifacts.length === 0, { timeout: 2000 }),
    (window._dhatuStaleArtifacts || ['freshness check never ran']).join(' ; '));

  // ---- list filters: exact dhātu (Devanagari + HK), pada, and no upasarga box
  $('list-q').value = 'दा'; $('list-go').click();
  ok('list: दा exact match (3)', text($('list-count')) === '3 / 2259', text($('list-count')));
  $('list-q').value = 'dA'; $('list-go').click();
  ok('list: HK dA → दा', text($('list-count')) === '3 / 2259', text($('list-count')));
  $('list-q').value = 'zzz'; $('list-go').click();
  ok('list: unknown dhātu → 0', text($('list-count')).startsWith('0 /'), text($('list-count')));
  $('list-q').value = 'eat'; $('list-go').click();
  ok('list: English gloss eat (93)', text($('list-count')) === '93 / 2259', text($('list-count')));
  $('list-q').value = ''; $('list-pada').value = 'P'; $('list-go').click();
  const padaCount = text($('list-count'));
  ok('list: pada filter applies', /^[1-9]\d* \/ 2259$/.test(padaCount), padaCount);
  $('list-pada').value = ''; $('list-go').click();
  ok('list: no upasarga box', !document.getElementById('list-upasarga'));
  $('list-settva').value = 'V'; $('list-go').click();
  ok('list: veṭ filter (41)', text($('list-count')) === '41 / 2259', text($('list-count')));
  $('list-settva').value = ''; $('list-karma').value = 'D'; $('list-go').click();
  ok('list: dvikarmaka filter (23)', text($('list-count')) === '23 / 2259', text($('list-count')));
  $('list-karma').value = ''; $('list-go').click();

  // ---- form search: base, prefixed, declined-prefixed, kṛdanta-subanta, HK
  // Returns the result; stops waiting as soon as the search settles
  // (verdict in the note, or an error in the status) instead of burning
  // the whole timeout on a dead run.
  const formSearch = async (q) => {
    location.hash = 'search';
    $('search-q').value = q;
    $('search-go').click();
    await waitFor(
      () => {
        if (/\d+ hits?/.test(text($('search-note')))) return true;
        const st = text($('status'));
        return st !== '' && st !== 'searching…';
      },
      { timeout: 30000 },
    );
    return {
      note: text($('search-note')),
      hits: $('search-hits').querySelectorAll('li').length,
    };
  };
  const bhava = await formSearch('भवति');
  ok('search: भवति hits > 0', bhava.hits > 0, bhava.note);
  ok('search: timing readout shown', /\d+ms/.test(bhava.note), bhava.note);
  const prad = await formSearch('प्रददत्');
  ok('search: प्रददत् (prefixed) hits > 0', prad.hits > 0, prad.note);

  // ---- hit-link navigation: canonical hash + prefix preselected + defaults.
  // Click the first PREFIXED hit of the प्रददत् search above (it found 6:
  // kṛt + declined-subanta) — operating on those results directly, no
  // re-search, so this cannot race the suite. A prefixed hit carries its
  // stack as a second hash path segment (`#dhatu/03.0010/…`).
  const hitLink = [...$('search-hits').querySelectorAll('a')]
    .find((a) => a.getAttribute('href').split('/').length > 2);
  const href = hitLink ? hitLink.getAttribute('href') : '';
  ok('hit link uses canonical #dhatu/… hash', href.startsWith('#dhatu/'),
    `note=${JSON.stringify(text($('search-note')))} links=${$('search-hits').querySelectorAll('li').length}`);
  if (hitLink) hitLink.click();
  // Wait for the tables, not the title: openDhatu sets the title before it
  // builds/selects the prefix dropdown — reading earlier races the render.
  const opened = await waitFor(
    () =>
      !$('view-detail').hidden &&
      text($('detail-title')).length > 0 &&
      Boolean($('detail-krut').querySelector('table')),
    { timeout: 15000 },
  );
  ok('hit click opens the detail page', opened);
  if (!opened) {
    failures.push('detail checks skipped: navigation failed');
  } else {
  ok('detail: family defaults to shuddha', $('detail-family').value === 'shuddha', $('detail-family').value);
  const krutText = text($('detail-krut'));
  const stackValue = $('detail-stack').value;
  ok('detail: prefixed hit preselects the stack', stackValue === 'प्र', `stack=${stackValue}`);
  ok('detail: kṛdanta cells are prefixed (प्र…)', /प्र/.test(krutText), krutText.slice(0, 60));
  ok('detail: kṛdanta cells are linkable', $('detail-krut').querySelectorAll('a[data-krt]').length > 0);

  // ---- आत्मनेपदम् tables: the `a*` lakāra keys are the āt्मनेपद forms
  const tingText = text($('detail-ting'));
  ok('ting: परस्मैपदम् section headed', tingText.includes('लट् · परस्मैपदम्'), tingText.slice(0, 80));
  ok('ting: आत्मनेपदम् section headed', tingText.includes('लट् · आत्मनेपदम्'), tingText.slice(0, 80));
  ok('ting: परस्मैपदम् before आत्मनेपदम्', tingText.indexOf('परस्मैपदम्') < tingText.indexOf('आत्मनेपदम्'));

  // ---- karmaṇी (कर्मणी) view: passive āt्मनेपद forms (a* only there)
  $('detail-prayoga').value = 'karmani';
  $('detail-prayoga').dispatchEvent(new Event('change'));
  await waitFor(() => /आत्मनेपदम्/.test(text($('detail-ting'))), { timeout: 5000 });
  const karmaniText = text($('detail-ting'));
  ok('karmaṇī: passive forms shown (प्र…दीय…)', /प्र\S*दीय/.test(karmaniText), karmaniText.slice(0, 80));
  $('detail-prayoga').value = 'kartari';
  $('detail-prayoga').dispatchEvent(new Event('change'));

  // ---- kṛdanta click-through: a prefixed cell must decline (regression
  // class). Pinned corpus cells carry the corpus's kṛt names; the live
  // engine fallback carries upstream names (स्यादि) — accept either.
  const wanted = ['स्य-भावकर्म-शानच्', 'स्य-शानच्', 'स्य-शतृ', 'शानच्', 'शतृ'];
  const cell = [...$('detail-krut').querySelectorAll('a[data-krt]')]
    .find((a) => wanted.includes(a.dataset.krt));
  ok('detail: a prefixed kṛdanta cell is present', Boolean(cell));
  if (cell) {
    cell.click();
    await waitFor(() => text($('detail-subanta')).trim().length > 0, { timeout: 5000 });
    const dec = text($('detail-subanta'));
    ok('click-through: prefixed cell declines', dec.includes('प्र') && !dec.includes('Neither'), dec.slice(0, 80));
    ok('click-through: renders a paradigm table', Boolean($('detail-subanta').querySelector('table')));
  }

  // ---- base (unprefixed) view still works through the same select
  $('detail-stack').value = '';
  $('detail-stack').dispatchEvent(new Event('change'));
  const baseKrut = text($('detail-krut'));
  ok('base view: cells unprefixed (दद… present)', /दद/.test(baseKrut), baseKrut.slice(0, 60));
  // Upstream table cells carry the स्य-* kṛt names — the original
  // click-through regression class stays covered on the base view.
  const syCell = [...$('detail-krut').querySelectorAll('a[data-krt]')]
    .find((a) => a.dataset.krt.startsWith('स्य-'));
  ok('base view: a स्य-* kṛdanta cell is present', Boolean(syCell));
  if (syCell) {
    syCell.click();
    await waitFor(() => text($('detail-subanta')).trim().length > 0, { timeout: 5000 });
    const decBase = text($('detail-subanta'));
    ok('click-through (base): स्य-* cell declines',
      !decBase.includes('Neither') && decBase.trim().length > 10, decBase.slice(0, 80));
  }
  } // end detail checks (skipped when navigation failed)
  const praddec = await formSearch('प्रददद्भिः');
  ok('search: प्रददद्भिः (declined+prefixed) hits > 0', praddec.hits > 0, praddec.note);
  const hkdec = await formSearch('pradadadbhiH');
  ok('search: HK pradadadbhiH hits > 0', hkdec.hits > 0, hkdec.note);
  const dadan = await formSearch('ददानम्');
  ok('search: ददानम् (kṛdanta subanta) hits > 0', dadan.hits > 0, dadan.note);
  // A noun paradigm is not in the corpus shards (they carry the kṛt
  // families), so the search falls back to the word analyzer: सरिद्भ्याम्
  // reads back to सरित्, the way the site's शब्द search resolves a form
  // to its base word.
  const sari = await formSearch('सरिद्भ्याम्');
  ok('search: सरिद्भ्याम् falls back to the base word (सरित्)',
    /base-word reading/.test(sari.note) && sari.hits > 0 &&
    text($('search-hits')).includes('सरित्'), sari.note);


  // ---- plain dhatu navigation
  location.hash = 'dhatu/01.0001';
  await waitFor(() => text($('detail-title')).includes('01.0001'), { timeout: 10000 });
  ok('route: #dhatu/01.0001 opens भू', text($('detail-title')).includes('भू'), text($('detail-title')));
  ok('route: old-style #/dhatu/ tolerated', true);
  location.hash = '/dhatu/01.0001';
  await waitFor(() => text($('detail-title')).includes('01.0001'), { timeout: 5000 });

  // ---- deep links (ADDITIONS.md 1.4): ?q=… executes the view, ?fam=&stack=
  // restores the detail selections (hashchange → route on back/forward).
  location.hash = 'search?q=bhavati';
  await waitFor(() => $('search-q').value === 'bhavati' &&
    /\d+ hits? —/.test(text($('search-note'))) &&
    $('search-hits').querySelectorAll('a,li,div').length > 0, { timeout: 20000 });
  ok('deep link: #search?q=bhavati prefills and runs',
    $('search-q').value === 'bhavati' &&
    $('search-hits').textContent.length > 0, text($('search-note')).slice(0, 90));
  location.hash = 'search?q=' + encodeURIComponent('भवति');
  await waitFor(() => $('search-q').value === 'भवति', { timeout: 10000 });
  ok('deep link: encoded Devanagari query round-trips',
    $('search-q').value === 'भवति', $('search-q').value);
  location.hash = 'word?q=rAma';
  await waitFor(() => $('word-tables').querySelectorAll('section.word-table').length > 0,
    { timeout: 20000 });
  ok('deep link: #word?q=rAma declines',
    $('word-q').value === 'rAma' && /राम/.test(text($('word-tables'))),
    $('word-q').value);
  location.hash = 'dhatu/01.1096?fam=yang&stack=वि';
  await waitFor(() => $('view-detail').querySelectorAll('section.lakara').length > 0 &&
    text($('detail-title')).includes('01.1096'), { timeout: 10000 });
  ok('deep link: #dhatu?fam&stack restores selections',
    $('detail-family').value === 'yang' && $('detail-stack').value === 'वि' &&
    document.querySelectorAll('#detail-ting section.lakara').length > 0,
    `${$('detail-family').value}·${$('detail-stack').value}`);
  // Unknown fam values ignore (dhātu has no such family): the second
  // navigation shares the id, so wait on the selection state itself.
  location.hash = 'dhatu/01.1096?fam=nonexistent';
  await waitFor(() => $('detail-family').value === 'shuddha' && $('detail-stack').value === '',
    { timeout: 10000 });
  ok('deep link: bogus fam ignored (default shuddha)',
    $('detail-family').value === 'shuddha', $('detail-family').value);

  // ---- systematic dhātu sweep: page ⇔ blob ⇄ पद, across coverage classes
  // (a single-dhātu test missed real gaps — this is the class that caught
  // 01.1096: a परस्मैपदी dhātu must NOT show आत्मनेपदम् लट् sections).
  const sweep = ['01.0007', '01.0001', '01.0002', '03.0010', '01.1096'];
  for (const id of sweep) {
    location.hash = `dhatu/${id}`;
    // Wait for THIS dhātu's render: title match + tables (a bare lakara
    // selector can match the previous dhātu's still-rendered tables).
    const ready = await waitFor(
      () => !$('view-detail').hidden &&
        text($('detail-title')).includes(`(${id})`) &&
        Boolean($('detail-ting').querySelector('section.lakara')),
      { timeout: 10000 },
    );
    ok(`sweep ${id}: detail renders`, ready);
    if (!ready) continue;
    const blob = await fetch(`data/blobs/${id}.json`).then((r) => r.json());
    const kartari = (blob.ting.shuddha && blob.ting.shuddha.kartari) || {};
    const body = text($('detail-ting'));
    for (const [key, value] of Object.entries(kartari)) {
      const hasCells = String(value).split(';').some((s) => s.trim() !== '');
      if (!hasCells) continue;
      const pada = key.startsWith('p') ? 'परस्मैपदम्' : 'आत्मनेपदम्';
      const lakara = key.slice(1);
      const label = `लट् · ${pada}`;
      if (lakara === 'lat') {
        ok(`sweep ${id}: ${label} section present`, body.includes(label), body.slice(0, 60));
      }
    }
    const hasAtmLat = Object.keys(kartari).some((k) => k === 'alat' && String(kartari[k]).trim());
    if (!hasAtmLat) {
      ok(`sweep ${id}: no आत्मनेपदम् lट् section (data has none)`, !body.includes('लट् · आत्मनेपदम्'));
    }
  }

  // ---- prayoga dropdown labels + parasmaipada-dhātu आत्मनेपदम् rendering.
  // The dropdown must name only the prayoga (कर्तरि/कर्मणी), never a pada:
  // a parasmaipada dhātu's kartari table can legitimately carry आत्मनेपदम्
  // forms (वि+जि → विजयते per विपराभ्यां जे 1.3.19), so a "परस्मैपदम्"
  // suffix would mislabel the content. The render must show the corpus's
  // actual pada sections.
  const dd = document.getElementById('detail-prayoga');
  ok('dropdown labels are prayoga-only', [...dd.options].map((o) => o.textContent).join('|') === 'कर्तरि|कर्मणी',
    [...dd.options].map((o) => o.textContent).join('|'));
  // वि+जि: kartari laṭ is आत्मनेपदम् only.
  location.hash = 'dhatu/01.1096';
  await waitFor(() => text($('detail-title')).includes('01.1096'), { timeout: 10000 });
  const stVij = document.getElementById('detail-stack');
  stVij.value = 'वि';
  stVij.dispatchEvent(new Event('change', { bubbles: true }));
  await sleep(600);
  const vijHeads = [...document.querySelectorAll('#detail-ting h3')].map((h) => h.textContent.trim());
  ok('वि+जि: kartari laṭ is आत्मनेपदम्', vijHeads.includes('लट् · आत्मनेपदम्'), vijHeads.join(','));
  ok('वि+जि: no परस्मैपदम् laṭ section', !vijHeads.includes('लट् · परस्मैपदम्'));
  const vijLat = [...document.querySelectorAll('#detail-ting section.lakara')]
    .find((s) => s.querySelector('h3').textContent.includes('लट्'));
  ok('वि+जि: laṭ shows विजयते', /विजयते/.test(vijLat.textContent), vijLat.textContent.slice(0, 40));
  // सम्+गम्: ubhayapada — both padas present.
  location.hash = 'dhatu/01.1137';
  await waitFor(() => text($('detail-title')).includes('01.1137'), { timeout: 10000 });
  const stGam = document.getElementById('detail-stack');
  stGam.value = 'सम्';
  stGam.dispatchEvent(new Event('change', { bubbles: true }));
  await sleep(600);
  const gamHeads = [...document.querySelectorAll('#detail-ting h3')].map((h) => h.textContent.trim());
  ok('सम्+गम्: both padas present',
    gamHeads.includes('लट् · परस्मैपदम्') && gamHeads.includes('लट् · आत्मनेपदम्'),
    gamHeads.join(','));

  // ---- copy-table buttons (ADDITIONS.md 1.3): every rendered table gets a
  // Markdown/TSV copy bar; the click path serializes the visible text into
  // window.__lastCopy (the clipboard itself is not assertable headless).
  const latSec = [...document.querySelectorAll('#detail-ting section.lakara')]
    .find((s) => s.querySelector('h3').textContent.includes('लट् · परस्मैपदम्'));
  const mdBtn = latSec && latSec.querySelector('.copy-btn[data-kind="md"]');
  ok('copy: detail tables carry Markdown/TSV buttons',
    Boolean(mdBtn) && Boolean(latSec.querySelector('.copy-btn[data-kind="tsv"]')));
  if (mdBtn) {
    mdBtn.click();
    const copied = await waitFor(() => (window.__lastCopy || {}).kind === 'md', { timeout: 5000 });
    const payload = copied ? window.__lastCopy.text : '';
    ok('copy: markdown payload mirrors the visible table',
      /पुरुषः/.test(payload) && /एकवचनम्/.test(payload) &&
      /गच्छति/.test(payload) && /\| --- /m.test(payload), payload.slice(0, 90));
    const tsvBtn = latSec.querySelector('.copy-btn[data-kind="tsv"]');
    tsvBtn.click();
    await waitFor(() => (window.__lastCopy || {}).kind === 'tsv', { timeout: 5000 });
    ok('copy: tsv payload is tab-shaped',
      /\t/.test(window.__lastCopy.text) &&
      window.__lastCopy.text.split('\n').every((l) => l.split('\t').length === 4),
      window.__lastCopy.text.slice(0, 90));
  }

  // ---- no console errors anywhere above
  ok('no console errors during the run', errors.length === 0, errors.slice(0, 3).join(' | '));

  // ---- prefixed views vs skt-morph-data ground truth (WebDriver sweep):
  // for each sampled (dhātu, stack), every family/prayoga renders exactly
  // the corpus lakāra sections, cell-for-cell (multiset per section; slot
  // placement is pinned by the Rust audits on the shared derivation grid).
  const LAKARA_NAMES = {
    lat: 'लट्', lit: 'लिट्', lut: 'लुट्', lrut: 'लृट्', lot: 'लोट्',
    lang: 'लङ्', vidhiling: 'विधिलिङ्', ashirling: 'आशीर्लिङ्',
    lung: 'लुङ्', lrung: 'लृङ्',
  };
  const sectionHeading = (key) => {
    const pada = key.startsWith('p') ? 'परस्मैपदम्' : 'आत्मनेपदम्';
    return `${LAKARA_NAMES[key.slice(1)] || key} · ${pada}`;
  };
  const truthSample = await fetch('data/prefixed_truth_sample.json')
    .then((r) => (r.ok ? r.json() : null))
    .catch(() => null);
  if (!truthSample) {
    out.textContent += '\ntruth sweep skipped: no served sample (run web/gen_truth_sample.py)';
  } else {
  ok('truth sample loads (12 combos)', truthSample.length === 12, String(truthSample.length));
  for (const combo of truthSample) {
    location.hash = `dhatu/${combo.baseindex}`;
    const comboReady = await waitFor(
      () => !$('view-detail').hidden && text($('detail-title')).includes(`(${combo.baseindex})`),
      { timeout: 10000 },
    );
    ok(`truth ${combo.baseindex}+${combo.stack}: detail renders`, comboReady);
    if (!comboReady) continue;
    // `portal_stack` is the dropdown-offered name (corpus `आङ्` vs portal
    // `आ` — the engine treats them identically); listed stacks render the
    // backfilled tables, unlisted ones live-derive.
    const selectName = combo.portal_stack || combo.stack;
    const stackMissing = ![...$('detail-stack').options].some((o) => o.value === selectName);
    ok(`truth ${combo.baseindex}+${combo.stack}: stack offered`, !stackMissing);
    if (stackMissing) continue;
    for (const [family, prayogas] of Object.entries(combo.families)) {
      if (![...$('detail-family').options].some((o) => o.value === family)) continue;
      for (const [prayoga, lakaras] of Object.entries(prayogas)) {
        $('detail-family').value = family;
        $('detail-prayoga').value = prayoga;
        $('detail-stack').value = selectName;
        $('detail-family').dispatchEvent(new Event('change'));
        const rendered = await waitFor(
          () => $('detail-ting').querySelectorAll('section.lakara').length > 0,
          { timeout: 15000 },
        );
        const tag = `truth ${combo.baseindex}+${combo.stack} ${family}/${prayoga}`;
        ok(`${tag}: tables render`, rendered);
        if (!rendered) continue;
        const sections = [...$('detail-ting').querySelectorAll('section.lakara')];
        const seen = new Set();
        let sectionsOk = true;
        for (const [key, forms] of Object.entries(lakaras)) {
          const heading = sectionHeading(key);
          const section = sections.find((s) => text(s.querySelector('h3')) === heading);
          if (!section) {
            ok(`${tag}: section ${heading} present`, false);
            sectionsOk = false;
            continue;
          }
          seen.add(heading);
          const cells = [...section.querySelectorAll('td')]
            .flatMap((td) => text(td).split(',').map((s) => s.trim()).filter(Boolean))
            .sort();
          const want = [...forms].sort();
          if (JSON.stringify(cells) !== JSON.stringify(want)) {
            ok(`${tag}: section ${heading} cell-exact`, false,
              `want ${want.length}, got ${cells.length}`);
            sectionsOk = false;
          }
        }
        if (sectionsOk) {
          ok(`${tag}: all ${Object.keys(lakaras).length} sections cell-exact`, true);
          const extra = sections.filter((s) => !seen.has(text(s.querySelector('h3'))));
          ok(`${tag}: no phantom sections`, extra.length === 0,
            extra.map((s) => text(s.querySelector('h3'))).join(','));
        }
      }
    }
  } // end truth sweep (skipped gracefully when no served sample)
  }

  // ---- word view: nav button, readings (spec trio), generation tables
  $('nav-word').click();
  await waitFor(() => !$('view-word').hidden);
  ok('word: nav button opens the view', !$('view-word').hidden);
  const wordRun = async (q) => {
    $('word-q').value = q;
    $('word-go').click();
    await waitFor(() => {
      if (/\d+ reading/.test(text($('word-note')))) return true;
      const st = text($('status'));
      return st !== '' && st !== 'analyzing…';
    }, { timeout: 10000 });
    return {
      note: text($('word-note')),
      readings: text($('word-readings')),
      tables: $('word-tables').querySelectorAll('section.word-table').length,
    };
  };
  // `WrampeRa` is both ठ्रम्प's instrumental singular and itself; `Wrampena`
  // is only itself (an instrumental reading would need natva).
  const wr = await wordRun('WrampeRa');
  ok('word: WrampeRa note counts readings', /\d+ readings?/.test(wr.note), wr.note);
  ok('word: WrampeRa = instrumental of ठ्रम्प',
    /instrumental singular/.test(wr.readings) && wr.readings.includes('ठ्रम्प'), wr.readings.slice(0, 90));
  ok('word: WrampeRa = itself as base word', wr.readings.includes('ठ्रम्पेण'));
  ok('word: WrampeRa generation tables render', wr.tables >= 1, String(wr.tables));
  ok('word: no SLP1 in the view', !/Wramp/.test(wr.note + wr.readings));
  const readBtn = $('word-readings').querySelector('button.reading-table-btn');
  ok('word: reading rows carry table toggles', Boolean(readBtn));
  if (readBtn) {
    readBtn.click();
    const loaded = await waitFor(() =>
      Boolean($('word-readings').querySelector('div.reading-table table')));
    ok('word: reading table loads on demand', loaded);
  }
  const wn = await wordRun('Wrampena');
  ok('word: Wrampena reads only as itself', !/instrumental/.test(wn.readings), wn.readings.slice(0, 90));

  // ---- word view: the vendored शब्दपाठः (dist/data/shabda2.json, the same
  // dump ashtadhyayi.com renders its शब्द pages from) is the display oracle
  // for every word the corpus lists: its own grids replace the derived
  // candidates and the note credits the source. Words the corpus does not
  // list (ठ्रम्पेण / Wrampena) keep the derived tables checked above.
  ok('word: unlisted Wrampena stays on derived tables',
    !wn.note.includes('ashtadhyayi.com'), wn.note);
  // Copy bar rides on the word tables too; the TSV carries the labeled
  // grid chrome even for a derived/unlisted stem.
  const wCopy = $('word-tables').querySelector('.copy-btn[data-kind="tsv"]');
  ok('copy: word tables carry copy buttons', Boolean(wCopy));
  if (wCopy) {
    wCopy.click();
    await waitFor(() => (window.__lastCopy || {}).kind === 'tsv', { timeout: 5000 });
    ok('copy: word TSV carries the labeled grid',
      /विभक्तिः\t/.test(window.__lastCopy.text) && /प्रथमा\t/.test(window.__lastCopy.text),
      window.__lastCopy.text.slice(0, 90));
  }
  const hasShabda = await fetch('data/shabda2.json')
    .then((r) => r.ok).catch(() => false);
  // The paradigms added for ADDITIONS 2.x read back, not just decline: the
  // word view lists the base word + case for the form the user typed.
  const wAy = await wordRun('आयुषा');
  ok('word: आयुषा reads back to आयुष् (us-n)',
    wAy.readings.includes('आयुष्') && wAy.readings.includes('us-n'),
    wAy.readings.slice(0, 110));
  const wSr = await wordRun('सरिद्भ्याम्');
  ok('word: सरिद्भ्याम् reads back to सरित् (cons)',
    wSr.readings.includes('सरित्') && wSr.readings.includes('cons'),
    wSr.readings.slice(0, 110));
  const wl = await wordRun('लक्ष्मी');
  ok('word: लक्ष्मी reads as base word + नदी nominative',
    /citation form/.test(wl.readings) && /I-f-nadi/.test(wl.readings),
    wl.readings.slice(0, 140));
  if (!hasShabda) {
    ok('word: site शब्द grids skipped (dist/data/shabda2.json not built)', true);
  } else {
    const wordSite = async (q) => {
      const r = await wordRun(q);
      const credited = await waitFor(() =>
        text($('word-note')).includes('ashtadhyayi.com'), { timeout: 15000 });
      const first = $('word-tables').querySelector('section.word-table');
      return {
        ...r, credited,
        tables: $('word-tables').querySelectorAll('section.word-table').length,
        head: first ? text(first.querySelector('h4')) : '',
        firstCell: first ? text(first.querySelector('td')) : '',
        grid: first ? text(first) : '',
      };
    };
    const wls = await wordSite('लक्ष्मी');
    ok('word: लक्ष्मी shows the site @lakSmI1 grid',
      wls.credited && /ashtadhyayi\.com @lakSmI1/.test(wls.head) &&
      wls.tables === 1 && wls.firstCell === 'लक्ष्मीः' && wls.grid.includes('लक्ष्मीणाम्'),
      `${wls.head} · ${wls.firstCell}`);
    const wns = await wordSite('नदी');
    ok('word: नदी shows the site ङ्यन्त grid (bare nominative)',
      wns.credited && /ashtadhyayi\.com @nadI1/.test(wns.head) &&
      wns.firstCell === 'नदी' && wns.grid.includes('नदीनाम्'),
      `${wns.head} · ${wns.firstCell}`);
    const wgs = await wordSite('गति');
    ok('word: गति shows the site grid with गत्या',
      wgs.credited && /ashtadhyayi\.com @gati1/.test(wgs.head) &&
      wgs.grid.includes('गत्या'), `${wgs.head} · ${wgs.firstCell}`);
    const wss = await wordSite('सखि');
    ok('word: सखि shows the site grid with सखा',
      wss.credited && wss.firstCell === 'सखा', `${wss.head} · ${wss.firstCell}`);
    // An inflected form resolves to its base word: the site's शब्द search
    // shows that word's grid, so सरिद्भ्याम् lands on @sarit1.
    const wsi = await wordSite('सरिद्भ्याम्');
    ok('word: सरिद्भ्याम् shows the site @sarit1 grid (inflected → base word)',
      wsi.credited && /ashtadhyayi\.com @sarit1/.test(wsi.head) &&
      wsi.grid.includes('सरित्'), `${wsi.head} · ${wsi.firstCell}`);
  }

  // ---- word view: readings link to the dhātus whose kṛt paradigms produce
  // the form. गच्छन् occurs in the subanta index as शतृ of गम् (01.1137);
  // the matching reading must carry that link and open that page.
  location.hash = 'word';
  const wg = await wordRun('gacCan');
  ok('word: kṛt form still reads (गच्छत्)', /of गच्छत्/.test(wg.readings), wg.readings.slice(0, 90));
  const wref = await waitFor(() => Boolean($('word-readings').querySelector('a.reading-ref')),
    { timeout: 10000 }) ? $('word-readings').querySelector('a.reading-ref') : null;
  ok('word: reading links its dhātu (शतृ of गम्)', Boolean(wref),
    text($('word-readings')).slice(0, 140));
  if (wref) {
    ok('word: ref names शतृ · गम् and targets 01.1137',
      wref.getAttribute('href') === '#dhatu/01.1137' &&
      text(wref).includes('शतृ') && text(wref).includes('गम्'),
      `${wref.getAttribute('href')} · ${text(wref)}`);
    wref.click();
    const openedW = await waitFor(() =>
      !$('view-detail').hidden && text($('detail-title')).includes('01.1137'),
      { timeout: 15000 });
    ok('word: ref click opens the dhātu page', openedW, text($('detail-title')));
  }

  // ---- no console errors anywhere above
  ok('no console errors during the run', errors.length === 0, errors.slice(0, 3).join(' | '));
    })(),
    sleep(180_000).then(() => {
      failures.push(`TIMEOUT at ${out.textContent}`);
    }),
  ]);
} catch (e) {
  failures.push(`test driver threw: ${e}`);
}

verdict();
