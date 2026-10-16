// Speed benchmark: the portal's WASM query path, plus a payload comparison
// against ashtadhyayi.com's assets (the site's own pinned data URLs).
// run: node web/bench.cjs
const fs = require('fs');
const path = require('path');
const w = require(path.join(__dirname, '..', 'pkg-node', 'dhaturupa_wasm.js'));
const D = path.join(__dirname, '..', 'dist');
const ok = (t) => { const p = JSON.parse(t); if (!p.ok) throw new Error(p.error); return p; };

const QUERIES = [
  'भवति',      // lat table form (ting)
  'प्रयच्छति',  // prefixed table form
  'ददानम्',     // declined krdanta subanta form
  'bhavati',    // Harvard-Kyoto input
];
const SHARDS = 256;

function shardPair(text) {
  const n = ok(w.index_shard(text, SHARDS)).shard;
  return [
    ['b' + n, `data/shards/shard-${String(n).padStart(2, '0')}.index.json`],
    ['u' + n, `data/shards/sub-${String(n).padStart(2, '0')}.index.json`],
  ];
}

function runQuery(q) {
  let total = 0;
  for (const r of ok(w.query_candidates(q)).readings.slice(0, 8)) {
    for (const [key, file] of shardPair(r.text)) {
      const res = ok(w.index_query_family(key, r.text), 'query');
      total += res.total;
    }
  }
  return total;
}

const rows = [];
for (const q of QUERIES) {
  // cold: fetch + parse + load one shard pair, then query (first visit)
  let t0 = performance.now();
  const reads = ok(w.query_candidates(q)).readings;
  for (const r of reads.slice(0, 8)) {
    for (const [key, file] of shardPair(r.text)) {
      w.index_load_key(key, fs.readFileSync(path.join(D, file), 'utf8'));
    }
  }
  const cold = Math.round(performance.now() - t0);
  // warm: same query again from the WASM-side cache
  t0 = performance.now();
  const total = runQuery(q);
  const warm = Math.round(performance.now() - t0);
  rows.push({ query: q, cold_ms: cold, warm_ms: warm, hits: total });
}
console.table(rows);

// ---- payload comparison -------------------------------------------------
const ours = [
  'index.html', 'app.js', 'render.js', 'sw.js',
  'pkg/dhaturupa_wasm.js', 'pkg/dhaturupa_wasm_bg.wasm', 'data/dhatu_data.json',
];
const site = [
  ['bundle', 'https://ashtadhyayi.com/ashtadhyayi-1786933362.js'],
  ['dhatu_data.json', 'https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/data.txt'],
  ['vidyut wasm', 'https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/vidyut/vidyut_prakriya_bg_5.wasm'],
];
for (const fam of ['shuddha', 'nich', 'san', 'yang', 'yangluk']) {
  for (const pg of ['kartari', 'karmani']) {
    site.push([`ting_${fam}_${pg}`, `https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${fam}_${pg}.txt`]);
  }
  site.push([`krut_${fam}`, `https://cdn.jsdelivr.net/gh/ashtadhyayi-com/data@a9f9112/dhatu/dhatuforms_vidyut_${fam}_krut.txt`]);
}

async function payload() {
  const oursTotal = ours.reduce((sum, f) =>
    sum + fs.statSync(path.join(D, f)).size, 0);
  let siteTotal = 0;
  const detail = [];
  for (const [name, url] of site) {
    try {
      const res = await fetch(url, { method: 'HEAD' });
      const len = Number(res.headers.get('content-length') || 0);
      siteTotal += len;
      detail.push(`${name}: ${(len / 1e6).toFixed(2)}MB`);
    } catch (e) {
      detail.push(`${name}: unreachable`);
    }
  }
  console.log(`\nour first load (served):      ${(oursTotal / 1e6).toFixed(2)}MB`);
  console.log(`site bundle + data + wasm:   ${(siteTotal / 1e6).toFixed(2)}MB`);
  console.log(detail.join('\n'));
  const shard = fs.statSync(path.join(D, 'data/shards/shard-00.index.json')).size;
  console.log(`\nper query after first load: two shards (base+subanta) from disk ${(shard / 1e6).toFixed(1)}MB raw, ~0.5-1.4MB gzipped over the wire`);
}
payload();
