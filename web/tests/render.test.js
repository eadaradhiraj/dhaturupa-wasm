import test from 'node:test';
import assert from 'node:assert/strict';
import {
  escapeHtml, slotLabel, renderTingTables, renderKrutTables,
  renderParadigm, renderChandrika, renderHits, renderMeta,
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
  assert.match(html, /भवति/);
  assert.match(html, /भवतः , भवथः/);
  assert.match(html, /prathama · ekavacana/);
  // Lakāra labels spell out the पद: `a*` codes are आत्मनेपदम्, and
  // परस्मैपदम् sections render first regardless of alphabetical code order.
  assert.match(html, /लट् · परस्मैपदम्/);
  assert.match(html, /लट् · आत्मनेपदम्/);
  assert.ok(html.indexOf('परस्मैपदम्') < html.indexOf('आत्मनेपदम्'), html);
});

test('renderKrutTables links full cells, splits slashes', () => {
  const html = renderKrutTables(
    { 'शतृ': [{ base: 'ददत्', m: 'ददत्/ददद्', f: 'ददती', n: '' }] }, 'shuddha');
  // Full cell is the link; `/`-cells split into per-alternant links.
  assert.match(html, /data-value="ददत्"/);
  assert.match(html, /data-value="ददद्"/);
  assert.match(html, /data-value="ददती"/);
  assert.match(html, /<td>—<\/td>/);
});

test('renderParadigm lists cells', () => {
  const html = renderParadigm({ cells: [
    { vibhakti: 'prathamA', vacana: 'ekavacanam', forms: ['रामः'] },
  ] });
  assert.match(html, /रामः/);
  assert.match(html, /prathamA · ekavacanam/);
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
