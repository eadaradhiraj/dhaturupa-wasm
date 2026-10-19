// Audit (hand-run, not part of the gate): every consonant-final word the
// vendored शब्दपाठः lists must reproduce its site grid through the derived
// engine (the Cons class), so the routing does not silently mis-shape a
// word the site lists.
//
//   node web/audit-cons-stems.cjs
//
// The site grid wins in the portal (the शब्द oracle), so a divergence here
// means the derived table for that word is wrong for unlisted uses.

const fs = require('fs');
const path = require('path');
const wasm = require(path.join(__dirname, '..', 'pkg-node', 'dhaturupa_wasm.js'));

// Citations the Cons class claims: the finals त्/द्/ध्/भ्/च्.
const CONS_FINALS = ['त्', 'द्', 'ध्', 'भ्', 'च्'];
const LINGA_CODE = { P: 'm', S: 'f', N: 'n' };

function siteCells(entry) {
  return entry.forms.split(';').map((c) => c.trim());
}

function engineCells(payload, word) {
  const json = JSON.parse(payload);
  if (!json.ok) return null;
  const vib = ['prathamA', 'dvitIyA', 'tftIyA', 'caturTI',
    'paJcamI', 'zazWI', 'saptamI', 'sambodhana'];
  const vac = ['ekavacanam', 'dvivacanam', 'bahuvacanam'];
  const grids = (json.tables || []).map((table) => {
    const out = [];
    for (let i = 0; i < 24; i += 1) {
      const cell = (table.cells || []).find(
        (c) => c.vibhakti === vib[Math.floor(i / 3)] && c.vacana === vac[i % 3],
      );
      const forms = cell ? (cell.forms || []) : [];
      // The portal prefixes हे per alternant on the सम्बोधन row.
      out.push((Math.floor(i / 3) === 7 ? forms.map((f) => `हे ${f}`) : forms)
        .join(' , ').replace(/ , /g, '-'));
    }
    return out;
  });
  return grids.length ? grids : null;
}

const db = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'data', 'shabda_data2.json'), 'utf8'));
const markers = new Set(db.data.map((e) => e.linga));
console.log('db linga markers:', [...markers].join(','));
const candidates = db.data.filter((e) => CONS_FINALS.some((f) => e.word.endsWith(f)));
console.log(`consonant-final db entries: ${candidates.length}`);

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
  const grids = engineCells(wasm.subanta_table(entry.word, code), entry.word);
  if (!grids) {
    noTable += 1;
    diffs.push({ word: entry.word, id: entry.urlid, why: 'no derived table' });
    continue;
  }
  const want = siteCells(entry);
  // The site grid must be reproducible by one of the derived candidates.
  const matched = grids.some((cells) => cells.every((c, i) => c === want[i]));
  if (matched) {
    exact += 1;
    continue;
  }
  diverged += 1;
  const best = grids[0];
  const bad = [];
  for (let i = 0; i < 24; i += 1) {
    if (best[i] !== want[i]) bad.push(`${i}: got '${best[i]}' want '${want[i]}'`);
  }
  diffs.push({ word: entry.word, id: entry.urlid, why: bad.slice(0, 3).join(' | ') });
}

console.log(`exact: ${exact}  diverged: ${diverged}  no table: ${noTable}`);

// Group the misses by paradigm family so the actionable ones stand out.
function family(word) {
  if (/वत्$/.test(word)) return 'वत्-नामधातु (nominalization)';
  if (/मद्$/.test(word)) return 'सम्-अधिक-रूप';
  if (/च्$/.test(word)) return 'च्-final (no table)';
  if (/म्$/.test(word) || /ी$/.test(word)) return 'न/ई-shape (AtN)';
  return 'plain consonant-final';
}
const groups = new Map();
for (const d of diffs) {
  const k = family(d.word);
  if (!groups.has(k)) groups.set(k, []);
  groups.get(k).push(d);
}
for (const [k, list] of groups) {
  console.log(`\n${k}: ${list.length}`);
  for (const d of list.slice(0, 6)) {
    console.log(`  ${d.word} ${d.id} — ${d.why}`);
  }
  if (list.length > 6) console.log(`  … ${list.length - 6} more`);
}
