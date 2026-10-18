// Derived-engine vs शब्दपाठः audit for नजन्त stems (P/N/S).
//
//   node web/audit-an-stems.cjs
//
// For every शब्द entry whose word ends in "न्", score the derived tables
// (pkg-node wasm, built from the current Rust) against the site's own 24
// cells: full (every variant in order), partial (some variant), none.
// Rebuilt after subanta/natva changes; the audit is a scratch gate, not
// part of ./check.sh (run it by hand when touching Class::AnM/VanM/ManN
// or site_natva).
'use strict';
const fs = require('fs');
const path = require('path');
const wasm = require('../pkg-node/dhaturupa_wasm.js');

const db = JSON.parse(
  fs.readFileSync(path.join(__dirname, '..', 'data', 'shabda_data2.json'), 'utf8')
);
const VIB = ['prathamA', 'dvitIyA', 'tftIyA', 'caturTI', 'paJcamI', 'zazWI', 'saptamI', 'sambodhana'];
const VAC = ['ekavacanam', 'dvivacanam', 'bahuvacanam'];
const LINGA = { P: 'm', S: 'f', N: 'n' };

// Same family rule as the engine's `an_weak`, in Deva.
function family(word) {
  const chars = [...word];
  if (!(chars[chars.length - 2] === 'न' && chars[chars.length - 1] === '\u094d')) return 'other';
  const p = chars.slice(0, -2);
  if (p.length === 0) return 'Keep';
  const last = p[p.length - 1];
  if (last === 'च' || last === 'ज') return 'Y';
  if (last === 'ट' || last === 'ध' || last === 'ष') return 'Drop';
  const prev = p[p.length - 2];
  if (prev === undefined) return 'Keep';
  return prev === '\u094d' ? 'Keep' : 'Drop'; // open = inherent अ
}

// Site-accepted irregulars (each is शब्द-DB-covered, so the mismatch is
// display-invisible): the numeral stems (अष्टन्-family), the देवता
// stems (अर्यमन्/पूषन् with अर्यमण्-strong), the हन्-family (घ्न-forms),
// the अहन् `day` compounds (strong -अहाः), युवन्/मघवन्/श्वन्/अर्वन्/
// प्रतिदिवन्, प्रियाष्टन् (site lists variant pairs), and हेमन्
// (site data bug: its cells have `हे` stripped from every form).
const ACCEPTED = new Set([
  'अर्वन्', 'मघवन्', 'युवन्', 'श्वन्', 'प्रतिदिवन्',
  'अष्टन्', 'दशन्', 'नवन्', 'सप्तन्', 'पञ्चन्', 'परमपञ्चन्', 'परमाष्टन्',
  'तमिस्रहन्', 'वीरहन्', 'वृत्रहन्', 'बहुवृत्रहन्',
  'दीर्घाहन्', 'अर्यमन्', 'पूषन्', 'प्रियाष्टन्', 'हेमन्',
  // पथिन्-family declines as पन्थाः (pathin irregular).
  'अतिपथिन्', 'अपथिन्', 'ऋभुक्षिन्', 'कर्मन्दिन्', 'पथिन्', 'परिपन्थिन्',
  'मथिन्', 'सुपथिन्',
]);

const seen = new Set();
const rows = [];
for (const e of db.data) {
  if (!e.word.endsWith('न्')) continue;
  const key = e.word + '/' + e.linga;
  if (seen.has(key)) continue;
  seen.add(key);
  const linga = LINGA[e.linga];
  if (!linga) continue;
  const site = e.forms.split(';').map((s) => s.split('-').map((v) => v.replace(/^हे\s*/, '')).join('-'));
  let tables = [];
  try {
    const parsed = JSON.parse(wasm.subanta_table(e.word, linga));
    tables = Array.isArray(parsed.tables) ? parsed.tables : [];
  } catch (err) { /* no table */ }
  let best = null;
  for (const t of tables) {
    let exact = 0, partial = 0, none = 0, emptySite = 0;
    for (const cell of t.cells) {
      const i = VIB.indexOf(cell.vibhakti) * 3 + VAC.indexOf(cell.vacana);
      const want = site[i] ? site[i].split('-') : null;
      const got = cell.forms;
      if (!want) { emptySite++; continue; }
      if (got.length === want.length && got.every((f, k) => f === want[k])) exact++;
      else if (got.some((f) => want.includes(f))) partial++;
      else none++;
    }
    const score = exact * 2 + partial;
    if (!best || score > best.score) best = { score, exact, partial, none, emptySite, cls: t.class };
  }
  rows.push({ word: e.word, linga: e.linga, fam: family(e.word), best });
}

for (const [kind, label] of [['P', 'masculine'], ['N', 'neuter'], ['S', 'feminine']]) {
  const list = rows.filter((r) => r.linga === kind);
  const full = list.filter((r) => r.best && r.best.exact === 24).length;
  console.log(`linga ${kind} (${label}): ${full}/${list.length} full`);
}
console.log('\nmismatching:');
for (const r of rows) {
  if (r.best && r.best.exact === 24) continue;
  const ex = r.best ? r.best.exact : 0;
  const tag = ACCEPTED.has(r.word) ? ' [accepted]' : ' [NEW — investigate]';
  console.log(`  ${r.word}/${r.linga} (${r.fam}): ${r.best ? `${r.best.cls} exact=${ex}` : 'NO TABLE'}${tag}`);
}
