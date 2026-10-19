// Audit (hand-run, not part of the gate): every word the vendored
// शब्दपाठः lists must be reproducible by the engine's own tables — the
// site grid is the oracle, so a divergence is a shape the engine does
// not derive yet.
//
//   node web/audit-site-grids.cjs            # all 9,007 entries
//   node web/audit-site-grids.cjs cons       # only the consonant-final ones
//
// The portal shows the site's grid for every listed word (the शब्द
// oracle wins), so a divergence here means the derived table is wrong
// for unlisted uses of that shape — and that the reverse analysis
// cannot read the word back.

const fs = require('fs');
const path = require('path');
const wasm = require(path.join(__dirname, '..', 'pkg-node', 'dhaturupa_wasm.js'));

// Citations the Cons class claims: the finals त्/द्/ध्/भ्/च्/थ्/श्.
const CONS_FINALS = ['त्', 'द्', 'ध्', 'भ्', 'च्', 'थ्', 'श्'];
const LINGA_CODE = { P: 'm', S: 'f', N: 'n' };
const VIB = ['prathamA', 'dvitIyA', 'tftIyA', 'caturTI', 'paJcamI', 'zazWI',
  'saptamI', 'sambodhana'];
const VAC = ['ekavacanam', 'dvivacanam', 'bahuvacanam'];

function siteCells(entry) {
  return entry.forms.split(';').map((c) => c.trim());
}

function engineGrids(payload) {
  const json = JSON.parse(payload);
  if (!json.ok) return null;
  return (json.tables || []).map((table) => {
    const out = [];
    for (let i = 0; i < 24; i += 1) {
      const cell = (table.cells || []).find(
        (c) => c.vibhakti === VIB[Math.floor(i / 3)] && c.vacana === VAC[i % 3],
      );
      const forms = cell ? (cell.forms || []) : [];
      // The portal prefixes हे per alternant on the सम्बोधन row.
      out.push((Math.floor(i / 3) === 7 ? forms.map((f) => `हे ${f}`) : forms)
        .join(' , ').replace(/ , /g, '-'));
    }
    return out;
  });
}

const db = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'data', 'shabda_data2.json'), 'utf8'));
const only = process.argv[2] || '';
const candidates = db.data.filter((e) => {
  if (only === 'cons') return CONS_FINALS.some((f) => e.word.endsWith(f));
  return true;
});
console.log(`db entries audited: ${candidates.length}`);

let exact = 0;
let diverged = 0;
let noTable = 0;
const diffs = [];
for (const entry of candidates) {
  const code = LINGA_CODE[entry.linga];
  if (!code) {
    noTable += 1;
    diffs.push({ word: entry.word, id: entry.urlid, why: `no linga code (${entry.linga})` });
    continue;
  }
  const grids = engineGrids(wasm.subanta_table(entry.word, code));
  if (!grids) {
    noTable += 1;
    diffs.push({ word: entry.word, id: entry.urlid, why: 'no derived table' });
    continue;
  }
  const want = siteCells(entry);
  // The site grid must be reproducible by one of the derived candidates.
  if (grids.some((cells) => cells.every((c, i) => c === want[i]))) {
    exact += 1;
    continue;
  }
  diverged += 1;
  const best = grids[0];
  const bad = [];
  for (let i = 0; i < 24; i += 1) {
    if (best[i] !== want[i]) bad.push(`${i}: got '${best[i]}' want '${want[i]}'`);
  }
  diffs.push({ word: entry.word, id: entry.urlid, why: bad.slice(0, 2).join(' | ') });
}

console.log(`exact: ${exact}  diverged: ${diverged}  no table: ${noTable}`);

// Group the misses by paradigm family so the actionable ones stand out.
function family(word) {
  if (/वत्$/.test(word)) return 'वत्-नामधातु';
  if (/मद्$/.test(word)) return 'सम्-अधिक-रूप';
  if (/[चञ]्च?्$/.test(word)) return 'ञ्च-स्वादि';
  if (/[तट]्$/.test(word)) return 'त्-final';
  if (/द्$/.test(word)) return 'द्-final';
  if (/ध्$/.test(word)) return 'ध्-final';
  if (/भ्$/.test(word)) return 'भ्-final';
  if (/त्$/.test(word)) return 'वत्-family';
  if (/[ािीुूृ]/.test(word)) return 'a/i/ī/u/ū/ṛ-final';
  if (/अ$|क$|घ$|च$|ज$|ट$|ड$|ण$|त$|द$|न$|प$|ब$|म$|य$|र$|ल$|व$|श$|स$|ह$/.test(word)) {
    return 'consonant-final (other)';
  }
  return 'a-stem';
}
const groups = new Map();
for (const d of diffs) {
  const k = family(d.word);
  if (!groups.has(k)) groups.set(k, []);
  groups.get(k).push(d);
}
const order = [...groups.entries()].sort((a, b) => b[1].length - a[1].length);
for (const [k, list] of order) {
  console.log(`\n${k}: ${list.length}`);
  for (const d of list.slice(0, 8)) {
    console.log(`  ${d.word} ${d.id} — ${d.why}`);
  }
  if (list.length > 8) console.log(`  … ${list.length - 8} more`);
}
