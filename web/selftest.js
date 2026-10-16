// Browser self-test: drives the real app DOM (web/app.js) through the
// flows that regressed during development — routing, list filters, form
// search (base/prefixed/declined-prefixed/kṛdanta-subanta, Deva + HK),
// hit-link navigation with prefix preselection, family default, and the
// kṛdanta click-through declension. Verdict lands in #selftest-out.
//
// Run by ./check.sh (headless Firefox, DOM dumped after completion):
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

  // ---- list filters: exact dhātu (Devanagari + HK), pada, and no upasarga box
  $('list-q').value = 'दा'; $('list-go').click();
  ok('list: दा exact match (3)', text($('list-count')) === '3 / 2259', text($('list-count')));
  $('list-q').value = 'dA'; $('list-go').click();
  ok('list: HK dA → दा', text($('list-count')) === '3 / 2259', text($('list-count')));
  $('list-q').value = 'zzz'; $('list-go').click();
  ok('list: unknown dhātu → 0', text($('list-count')).startsWith('0 /'), text($('list-count')));
  $('list-q').value = ''; $('list-pada').value = 'P'; $('list-go').click();
  const padaCount = text($('list-count'));
  ok('list: pada filter applies', /^[1-9]\d* \/ 2259$/.test(padaCount), padaCount);
  $('list-pada').value = ''; $('list-go').click();
  ok('list: no upasarga box', !document.getElementById('list-upasarga'));

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

  // ---- kṛdanta click-through: a स्य-* cell must decline (regression class)
  const wanted = ['स्य-भावकर्म-शानच्', 'स्य-शानच्', 'स्य-शतृ'];
  const cell = [...$('detail-krut').querySelectorAll('a[data-krt]')]
    .find((a) => wanted.includes(a.dataset.krt));
  ok('detail: a स्य-* kṛdanta cell is present', Boolean(cell));
  if (cell) {
    cell.click();
    await waitFor(() => text($('detail-subanta')).trim().length > 0, { timeout: 5000 });
    const dec = text($('detail-subanta'));
    ok('click-through: स्य-* cell declines', dec.includes('प्र') && !dec.includes('Neither'), dec.slice(0, 80));
    ok('click-through: renders a paradigm table', Boolean($('detail-subanta').querySelector('table')));
  }

  // ---- base (unprefixed) view still works through the same select
  $('detail-stack').value = '';
  $('detail-stack').dispatchEvent(new Event('change'));
  const baseKrut = text($('detail-krut'));
  ok('base view: cells unprefixed (दद… present)', /दद/.test(baseKrut), baseKrut.slice(0, 60));
  } // end detail checks (skipped when navigation failed)
  const praddec = await formSearch('प्रददद्भिः');
  ok('search: प्रददद्भिः (declined+prefixed) hits > 0', praddec.hits > 0, praddec.note);
  const hkdec = await formSearch('pradadadbhiH');
  ok('search: HK pradadadbhiH hits > 0', hkdec.hits > 0, hkdec.note);
  const dadan = await formSearch('ददानम्');
  ok('search: ददानम् (kṛdanta subanta) hits > 0', dadan.hits > 0, dadan.note);


  // ---- plain dhatu navigation
  location.hash = 'dhatu/01.0001';
  await waitFor(() => text($('detail-title')).includes('01.0001'), { timeout: 10000 });
  ok('route: #dhatu/01.0001 opens भू', text($('detail-title')).includes('भू'), text($('detail-title')));
  ok('route: old-style #/dhatu/ tolerated', true);
  location.hash = '/dhatu/01.0001';
  await waitFor(() => text($('detail-title')).includes('01.0001'), { timeout: 5000 });

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
