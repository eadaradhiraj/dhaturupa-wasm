// Audit (hand-run, not part of the gate): every word the vendored
// शब्दपाठः lists must be reproducible by the engine's own tables — the
// site grid is the oracle, so a divergence is a shape the engine does
// not derive yet.
//
//   node web/audit-site-grids.cjs            # all 9,007 entries
//   node web/audit-site-grids.cjs cons       # only the consonant-final ones
//   node web/audit-site-grids.cjs --words    # list every diverged word
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
  // The site's grid leaves cells empty where its शब्दपाठः lists no form
  // for the word (a few entries carry only the plural or only the
  // vocative-less set). An empty cell carries no information, so the
  // engine is only compared on the cells the site actually lists.
  const listed = want.map((c, i) => (c === '' ? -1 : i)).filter((i) => i >= 0);
  // The site grid must be reproducible by one of the derived candidates.
  if (grids.some((cells) => listed.every((i) => cells[i] === want[i]))) {
    exact += 1;
    continue;
  }
  diverged += 1;
  // Cluster by the engine class and the first differing cell, using the
  // best candidate (the site grid is reproducible from one of them).
  const scored = grids
    .map((cells) => ({
      cells,
      diff: listed.reduce((n, i) => n + (cells[i] === want[i] ? 0 : 1), 0),
    }))
    .sort((a, b) => a.diff - b.diff);
  const best = scored[0];
  const cls = JSON.parse(wasm.subanta_table(entry.word, code)).tables[0].class;
  const bad = [];
  for (const i of listed) {
    if (best.cells[i] !== want[i]) bad.push(`${i}: got '${best.cells[i]}' want '${want[i]}'`);
  }
  const firstDiff = listed.find((i) => best.cells[i] !== want[i]);
  diffs.push({
    word: entry.word,
    id: entry.urlid,
    why: `${cls} @${VIB[Math.floor(firstDiff / 3)]} — ${bad.slice(0, 2).join(' | ')}`,
  });
}

console.log(`exact: ${exact}  diverged: ${diverged}  no table: ${noTable}`);

// Group the misses by paradigm family so the actionable ones stand out.
function family(d) {
  // The engine class and the first differing case name the family: the
  // engine's own class label plus the cell it first gets wrong.
  const m = /^([\w-]+) @(\w+)/.exec(d.why);
  return m ? `${m[1]} @${m[2]}` : d.why;
}
const groups = new Map();
for (const d of diffs) {
  const k = family(d);
  if (!groups.has(k)) groups.set(k, []);
  groups.get(k).push(d);
}
const order = [...groups.entries()].sort((a, b) => b[1].length - a[1].length);
const showAll = process.argv.includes('--words');
for (const [k, list] of order) {
  console.log(`\n${k}: ${list.length}`);
  const shown = showAll ? list : list.slice(0, 6);
  for (const d of shown) {
    console.log(`  ${d.word} ${d.id} — ${d.why}`);
  }
  if (!showAll && list.length > 6) console.log(`  … ${list.length - 6} more`);
}
