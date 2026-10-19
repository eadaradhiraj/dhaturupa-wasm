import test from 'node:test';
import assert from 'node:assert/strict';
import {
  escapeHtml, slotLabel, renderTingTables, renderKrutTables, krtLinkTarget,
  renderParadigm, renderChandrika, renderHits, renderMeta,
  renderWordReadings, renderWordTables, siteTables, tablePayload,
} from '../render.js';

test('escapeHtml neutralizes markup', () => {
  assert.equal(escapeHtml('<a>&"'), '&lt;a&gt;&amp;&quot;');
});

test('slotLabel honors the 9-slot rule', () => {
  assert.equal(slotLabel(0, 9), 'prathama · ekavacana');
  assert.equal(slotLabel(8, 9), 'uttama · bahuvacana');
  assert.equal(slotLabel(10, 18), 'prathama · dvivacana');
  assert.equal(slotLabel(0, 5), 'form 1');
});

test('renderTingTables sorts lakaras and splits variants', () => {
  const slots = [['भवति'], ['भवतः', 'भवथः'], ['x'], ['x'], ['x'], ['x'], ['x'], ['x'], ['x']];
  const html = renderTingTables({ plat: slots, alat: [['भवते'], ['x'], ['x'], ['x'], ['x'], ['x'], ['x'], ['x'], ['x']] });
  // 9-slot lakāras render the traditional 3x3 matrix (पुरुषः rows,
  // वचनम् columns — ADDITIONS.md 1.1).
  assert.match(html, /भवति/);
  assert.match(html, /भवतः , भवथः/);
  assert.match(html, /<th>पुरुषः<\/th><th>एकवचनम्<\/th><th>द्विवचनम्<\/th><th>बहुवचनम्<\/th>/);
  assert.match(html, /<th>प्रथमपुरुषः<\/th><td>भवति<\/td>/);
  assert.match(html, /<th>मध्यमपुरुषः<\/th>/);
  assert.match(html, /<th>उत्तमपुरुषः<\/th>/);
  // Lakāra labels spell out the पद: `a*` codes are आत्मनेपदम्, and
  // परस्मैपदम् sections render first regardless of alphabetical code order.
  assert.match(html, /लट् · परस्मैपदम्/);
  assert.match(html, /लट् · आत्मनेपदम्/);
  assert.ok(html.indexOf('परस्मैपदम्') < html.indexOf('आत्मनेपदम्'), html);
});

test('renderTingTables keeps the numbered fallback for other lengths', () => {
  const slots = Array.from({ length: 18 }, (_, i) => [i === 0 ? 'भवति' : 'x']);
  const html = renderTingTables({ plat: slots });
  // Multiples of 9 keep the numbered rows (slotLabel), not the matrix.
  assert.match(html, /<table><tbody>/);
  assert.match(html, /prathama · ekavacana/);
  assert.match(html, /<td>भवति<\/td>/);
  assert.doesNotMatch(html, /प्रथमपुरुषः/);
});

test('renderKrutTables applies the site click convention', () => {
  const html = renderKrutTables(
    { 'शतृ': [{ base: 'ददत्', m: 'ददन्/ददद्', f: 'ददती', n: '' }] }, 'shuddha');
  // m/n links decline the row base, not the clicked cell: both m alternants
  // display themselves but pass ददत्; f passes the form itself (ी ending).
  assert.equal((html.match(/data-value="ददत्"/g) || []).length, 2);
  assert.doesNotMatch(html, /data-value="ददद्"/);
  assert.match(html, />ददद्<\/a>/);
  assert.match(html, /data-value="ददती"/);
  assert.match(html, /data-gender="m"/);
  assert.match(html, /<td>—<\/td>/);
});

test('krtLinkTarget mirrors the site f rule', () => {
  assert.equal(krtLinkTarget('m', 'ददन्', 'ददत्'), 'ददत्');
  assert.equal(krtLinkTarget('n', 'ददत्', 'ददत्'), 'ददत्');
  assert.equal(krtLinkTarget('f', 'ददती', 'ददत्'), 'ददती');
  assert.equal(krtLinkTarget('f', 'अञ्चा', 'अञ्च्'), 'अञ्चा');
  assert.equal(krtLinkTarget('f', 'भूति', 'भूति'), 'भूति'); // इ last: base
  assert.equal(krtLinkTarget('m', 'ददन्', '—'), 'ददन्'); // unusable base: cell
  assert.equal(krtLinkTarget('f', 'भूति', '—'), 'भूति');
});

test('renderParadigm labels the 8x3 grid (विभक्तिः rows, वचनम् columns)', () => {
  const row = (vibhakti, forms) =>
    ['ekavacanam', 'dvivacanam', 'bahuvacanam']
      .map((vacana, i) => ({ vibhakti, vacana, forms: [forms[i]] }));
  const html = renderParadigm({ cells: [
    ...row('prathamA', ['रामः', 'रामौ', 'रामाः']),
    ...row('sambodhana', ['राम', 'रामौ', 'रामाः']),
  ] });
  // Labeled grid: वचनम् column headers, one th-labeled row per vibhakti.
  assert.equal((html.match(/<tr>/g) || []).length, 3);
  assert.match(html, /<th>विभक्तिः<\/th><th>एकवचनम्<\/th><th>द्विवचनम्<\/th><th>बहुवचनम्<\/th>/);
  assert.match(html, /<th>प्रथमा<\/th><td>रामः<\/td>/);
  assert.match(html, /<th>सम्बोधनम्<\/th><td>हे राम<\/td>/);
  assert.match(html, /<td>हे रामौ<\/td>/);
  assert.match(html, /<td>हे रामाः<\/td>/);
  // Alternants stay comma-joined inside one cell.
  const multi = renderParadigm({ cells: [
    { vibhakti: 'prathamA', vacana: 'ekavacanam', forms: ['रामः', 'रामो वा'] },
  ] });
  assert.match(multi, /रामः , रामो वा/);
});

test('tablePayload copies labeled tables as Markdown and TSV', () => {
  const rows = [
    { label: null, cells: ['विभक्तिः', 'एकवचनम्', 'द्विवचनम्', 'बहुवचनम्'] },
    { label: 'प्रथमा', cells: ['रामः', 'रामौ', 'रामाः'] },
    { label: 'सम्बोधनम्', cells: ['हे राम', '', 'हे रामाः'] },
  ];
  assert.equal(tablePayload(rows, 'md'), [
    '| विभक्तिः | एकवचनम् | द्विवचनम् | बहुवचनम् |',
    '| --- | --- | --- | --- |',
    '| प्रथमा | रामः | रामौ | रामाः |',
    '| सम्बोधनम् | हे राम |  | हे रामाः |',
  ].join('\n'));
  assert.equal(tablePayload(rows, 'tsv'), [
    'विभक्तिः\tएकवचनम्\tद्विवचनम्\tबहुवचनम्',
    'प्रथमा\tरामः\tरामौ\tरामाः',
    'सम्बोधनम्\tहे राम\t\tहे रामाः',
  ].join('\n'));
  // Alternants keep their display join; a table without a header row
  // stays data-only (no fabricated header or separator).
  assert.equal(
    tablePayload(
      [{ label: 'उत्तमपुरुषः', cells: ['भवामि', 'भवतः , भवथः'] }],
      'md',
    ),
    '| उत्तमपुरुषः | भवामि | भवतः , भवथः |',
  );
});

test('renderChandrika highlights examples', () => {
  const html = renderChandrika([
    { upasarga: 'प्र + नि', artha: 'डुदाञ् दाने', examples: ["'<hl>प्रणिदाने</hl>' इति।"] },
  ]);
  assert.match(html, /प्र \+ नि/);
  assert.match(html, /<mark>प्रणिदाने<\/mark>/);
  assert.equal(renderChandrika([]), '<p class="muted">No Chandrika entry for this dhātu.</p>');
});

test('renderHits links dhatu pages with cards', () => {
  const meta = { '01.0001': { dhatu: 'भू', artha: 'सत्तायाम्' } };
  const html = renderHits([
    { baseindex: '01.0001', kind: 'ting', lakara: 'plat', pos: 0 },
    { baseindex: '01.0001', kind: 'subanta', krt: 'शतृ', field: 'f', vibhakti: 'tftIyA', vacana: 'dvivacanam' },
  ], meta, 'भवति');
  assert.match(html, /#dhatu\/01\.0001/);
  assert.match(html, /भू — सत्तायाम्/);
  assert.match(html, /plat\[0\]/);
  assert.match(html, /f, feminine/);
  assert.match(html, /tftIyA\.dvivacanam/);
  assert.match(html, /“भवति”/);
  assert.equal(renderHits([], {}, ''), '<p>No matches.</p>');
  // Prefixed hits carry their stack in the hash: clicking opens the detail
  // page with that prefix already selected (`#dhatu/03.0010/प्र`).
  const prefixed = renderHits([
    { baseindex: '03.0010', kind: 'prefixed', stack: 'प्र',
      base: { kind: 'subanta', krt: 'शतृ', field: 'm', vibhakti: 'tftIyA', vacana: 'bahuvacanam' } },
  ], {}, 'प्रददद्भिः');
  assert.match(prefixed, /href="#dhatu\/03\.0010\/%E0%A4%AA%E0%A5%8D%E0%A4%B0"/);
  assert.match(prefixed, /prefixed via/);
});

test('renderMeta labels in English', () => {
  const html = renderMeta({
    dhatu: 'भू', artha: 'सत्तायाम्', artha_english: 'to be',
    artha_hindi: '', gana: '1', pada: 'P', settva: 'S', karma: 'A',
  });
  assert.match(html, /<dt>Gaṇa<\/dt><dd>1 \(bhvādi\)<\/dd>/);
  assert.match(html, /parasmaipadī/);
  assert.doesNotMatch(html, /Hindi/);
});

test('renderWordReadings states the relation and carries table toggles', () => {
  const html = renderWordReadings([
    { stem: 'ठ्रम्प', linga: 'm', class: 'a-m', vibhakti: 'tftIyA', vacana: 'ekavacanam' },
    { stem: 'ठ्रम्पेण', linga: 'n', class: 'a-n', vibhakti: 'sambodhana', vacana: 'ekavacanam' },
    { stem: '<x>', linga: 'f', class: 'I-f', vibhakti: 'citation', vacana: '' },
  ]);
  assert.match(html, /instrumental singular/);
  assert.match(html, /of <span class="deva">ठ्रम्प<\/span>/);
  assert.match(html, /\(masculine · a-m\)/);
  assert.match(html, /vocative \(sambodhana\) singular/);
  assert.match(html, /citation form/);
  assert.match(html, /\(feminine · I-f\)/);
  assert.match(html, /&lt;x&gt;/);
  assert.match(html, /data-stem="ठ्रम्प" data-linga="m"/);
  assert.match(renderWordReadings([]), /No readings/);
});

test('renderWordReadings attaches corpus kṛt links to matching readings only', () => {
  const analyses = [
    { stem: 'गच्छत्', linga: 'm', class: 'at-m', vibhakti: 'prathamA', vacana: 'ekavacanam' },
    { stem: 'गच्छन्', linga: 'm', class: 'an-m', vibhakti: 'sambodhana', vacana: 'ekavacanam' },
  ];
  const refs = [
    { baseindex: '01.1137', krt: 'शतृ', field: 'm', vibhakti: 'prathamA', vacana: 'ekavacanam' },
    // same slot, another dhātu: a second link on the same row
    { baseindex: '01.0001', krt: 'शतृ', field: 'm', vibhakti: 'prathamA', vacana: 'ekavacanam' },
    // different slot/gender: no link anywhere
    { baseindex: '03.0010', krt: 'क्त', field: 'n', vibhakti: 'tftIyA', vacana: 'ekavacanam' },
    // same dhātu+kṛt via another family: collapsed to one link
    { baseindex: '01.1137', krt: 'शतृ', field: 'm', vibhakti: 'prathamA', vacana: 'ekavacanam', family: 'nich' },
  ];
  const meta = { '01.1137': { dhatu: 'गम्' }, '01.0001': { dhatu: 'भू' } };
  const html = renderWordReadings(analyses, refs, meta);
  assert.match(html, /href="#dhatu\/01\.1137"/);
  assert.match(html, /शतृ of <span class="deva">गम्<\/span>/);
  assert.match(html, /शतृ of <span class="deva">भू<\/span>/);
  assert.doesNotMatch(html, /क्त of/);
  assert.equal((html.match(/href="#dhatu\/01\.1137"/g) || []).length, 1);
  // only the prathamA row links — the sambodhana reading has no ref
  const rows = html.split('<li>');
  assert.match(rows[1], /reading-ref/);
  assert.doesNotMatch(rows[2], /reading-ref/);
  // no meta (list not loaded): the baseindex is the fallback label
  assert.match(renderWordReadings(analyses, refs, {}), /of <span class="deva">01\.1137<\/span>/);
});

test('renderWordTables heads sections and renders bare grids', () => {
  const table = {
    stem: 'ठ्रम्प', linga: 'm', class: 'a-m',
    cells: [
      { vibhakti: 'prathamA', vacana: 'ekavacanam', forms: ['ठ्रम्पः'] },
      { vibhakti: 'dvitIyA', vacana: 'ekavacanam', forms: ['ठ्रम्पम्'] },
      { vibhakti: 'sambodhana', vacana: 'ekavacanam', forms: ['ठ्रम्प'] },
    ],
  };
  const html = renderWordTables([table]);
  assert.match(html, /masculine \(a-m\)/);
  assert.match(html, /ठ्रम्पः/);
  assert.match(html, /हे ठ्रम्प/);
  assert.match(html, /<table><thead>/);
  assert.match(renderWordTables([]), /No declension/);
});

test('siteTables maps the शब्द dump to site-exact grids', () => {
  // 24 site cells, vibhakti-major (pr, dvi, tft, cat, pan, zaz, sap, samb);
  // `''` marks a cell the site leaves empty, `-` joins variants.
  const forms = [
    'गतिः', 'गती', 'गतयः',
    'गतिम्', 'गती', 'गतीः',
    'गत्या', 'गतिभ्याम्', 'गतिभिः',
    'गतये-गत्यै', '', '',
    'गतेः-गत्याः', 'गतिभ्याम्', 'गतिभ्यः',
    'गतेः-गत्याः', 'गत्योः', 'गतीनाम्',
    'गतौ-गत्याम्', 'गत्योः', 'गतिषु',
    'हे गते', 'हे गती', 'हे गतयः',
  ].join(';');
  const byWord = new Map([['गति', [['S', '@gati1', forms]]]]);
  const tables = siteTables(byWord, 'गति');
  assert.equal(tables.length, 1);
  const t = tables[0];
  assert.equal(t.stem, 'गति');
  assert.equal(t.linga, 'f');
  assert.equal(t.lingaLabel, 'feminine');
  assert.equal(t.class, 'ashtadhyayi.com @gati1');
  assert.equal(t.cells.length, 24);
  assert.equal(t.cells[0].vibhakti, 'prathamA');
  assert.equal(t.cells[2].vacana, 'bahuvacanam');
  assert.deepEqual(t.cells[6].forms, ['गत्या']);
  assert.deepEqual(t.cells[9].forms, ['गतये', 'गत्यै']);
  assert.deepEqual(t.cells[10].forms, []);
  // The site keeps `हे ` inside the vocative cell; the grid re-adds it.
  assert.deepEqual(t.cells[21].forms, ['गते']);
  assert.match(renderParadigm(t), /<td>हे गते<\/td>/);
  assert.match(renderParadigm(t), /<td>गतये , गत्यै<\/td>/);
  assert.match(renderParadigm(t), /<td>—<\/td>/);
  assert.equal(siteTables(byWord, 'ठ्रम्पेण').length, 0);
  assert.equal(siteTables(undefined, 'गति').length, 0);
});

test('siteTables collapses cell-identical entries, labels every लिङ्ग', () => {
  const grid = new Array(24).fill('x').join(';');
  const byWord = new Map([
    ['ग्रामणी', [['P', '@grAmaNI1', grid], ['S', '@grAmaNI2', grid]]],
    ['अस्मद्', [['A', '@asmad1', new Array(24).fill('').join(';')]]],
  ]);
  const tables = siteTables(byWord, 'ग्रामणी');
  assert.equal(tables.length, 1);
  assert.equal(tables[0].lingaLabel, 'masculine, feminine');
  assert.equal(tables[0].class, 'ashtadhyayi.com @grAmaNI1 @grAmaNI2');
  assert.match(renderWordTables(tables),
    /masculine, feminine \(ashtadhyayi\.com @grAmaNI1 @grAmaNI2\)/);
  assert.equal(siteTables(byWord, 'अस्मद्')[0].lingaLabel, 'any gender');
});
