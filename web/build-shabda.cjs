#!/usr/bin/env node
// Builds the served शब्दपाठः oracle (`dist/data/shabda2.json`) from the
// site's own dump (`data/shabda_data2.json`, fetched per README).
//
// The word view prefers these grids for every word the corpus lists
// (ashtadhyayi.com renders its शब्द pages from exactly this file); words the
// corpus does not list keep the derived engine tables. Entries carry the
// site's raw 24-cell `forms` string — vibhakti-major (pr, dvi, tft, cat,
// pan, zaz, sap, samb), variants `-`-joined, empty cells `''` — so every
// displayed cell stays auditable against the source:
//
//   {"v":1, "source":"…", "entries":[["word","linga","@urlid","cell;…"], …]}
//
// usage: node web/build-shabda.cjs [raw=data/shabda_data2.json] [out=dist/data/shabda2.json]
// A missing raw file prints a notice and exits 0 (an already-built out stays
// valid; check.sh refreshes it only when the raw dump is present).
const fs = require('node:fs');
const path = require('node:path');

const rawPath = process.argv[2] || 'data/shabda_data2.json';
const outPath = process.argv[3] || 'dist/data/shabda2.json';

if (!fs.existsSync(rawPath)) {
  console.log(`build-shabda: ${rawPath} not present — keeping any existing ${outPath}`);
  process.exit(0);
}

const raw = JSON.parse(fs.readFileSync(rawPath, 'utf8'));
if (!Array.isArray(raw.data)) {
  console.error(`build-shabda: ${rawPath} has no data array — wrong file?`);
  process.exit(1);
}

// The site's own conventions; a violation means the upstream format moved.
const LINGAS = new Set(['P', 'S', 'N', 'A']);
const problems = [];
const entries = raw.data.map((e, i) => {
  const word = String(e.word || '').trim();
  const forms = String(e.forms || '');
  const cells = forms.split(';');
  if (!word) problems.push(`#${i}: empty word`);
  if (!LINGAS.has(e.linga)) problems.push(`#${i} ${word}: linga ${JSON.stringify(e.linga)}`);
  if (cells.length !== 24) problems.push(`#${i} ${word}: ${cells.length} cells, want 24`);
  for (let c = 21; c < cells.length; c += 3) {
    if (cells[c] !== '' && !cells[c].startsWith('हे ')) {
      problems.push(`#${i} ${word}: sambodhana cell ${JSON.stringify(cells[c])}`);
    }
  }
  return [word, e.linga, e.urlid || '', forms];
});

if (problems.length) {
  console.error(`build-shabda: ${problems.length} malformed entries (first 5):`);
  for (const p of problems.slice(0, 5)) console.error(`  ${p}`);
  process.exit(1);
}

const out = JSON.stringify({ v: 1, source: raw.name || 'shabdapatha_v2', entries });
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, out);
console.log(
  `build-shabda: ${entries.length} entries, ` +
    `${(out.length / 1024 / 1024).toFixed(2)} MB -> ${outPath}`);
