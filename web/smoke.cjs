// Post-build smoke: every wasm export the portal uses, driven through the
// FRESH pkg-node build + served dist artifacts. run: node web/smoke.cjs
const w = require('../pkg-node/dhaturupa_wasm.js');
const fs = require('fs');
const path = require('path');
const D = path.join(__dirname, '..', 'dist');
const ok = (t) => { const p = JSON.parse(t); if (!p.ok) throw new Error(p.error); return p; };

const dhatuList = JSON.parse(fs.readFileSync(path.join(D, 'data/dhatu_data.json'), 'utf8'));
const names = [...new Set(dhatuList.data.flatMap((e) => (e.upasargas || []).map((u) => u.name)))];

console.log('to_devanagari(dA)        ->', ok(w.to_devanagari('dA')).deva);
console.log('query_candidates(bhavati)->', ok(w.query_candidates('bhavati')).deva.join(','));
console.log('engine_category(shuddha,krut) ->', ok(w.engine_category('shuddha', 'krut')).category);
console.log('prefixed_table_ctx(प्र,ददत्) ->',
  ok(w.prefixed_table_ctx('प्र', JSON.stringify(['ददत्']), 'da\\a',
    ok(w.engine_category('shuddha', 'krut')).category, '')).tables[0].join(','));
console.log('subanta_analyze(WrampeRa) ->',
  ok(w.subanta_analyze('WrampeRa')).analyses
    .map((a) => `${a.stem} ${a.vibhakti}.${a.vacana}`).join(' | '));
console.log('subanta_table(जोन,m)      ->',
  ok(w.subanta_table('जोन', 'm')).tables.length + ' table(s)');
console.log('split_init stacks        ->', ok(w.split_init(JSON.stringify(names))).stacks);
const sq = ok(w.split_query('प्रददत्', 256));
console.log('split_query(प्रददत्)    ->', sq.splits.map((s) => s.stack + '+' + s.base).slice(0, 3).join(' | '));

// full search path: shards + split verify, with the same LRU the portal uses
const shardKeys = new Set();
function loadShard(key, file) {
  if (shardKeys.has(key)) shardKeys.delete(key);
  else ok(w.index_load_key(key, fs.readFileSync(path.join(D, 'data/shards', file), 'utf8')), 'shard');
  shardKeys.add(key);
  while (shardKeys.size > 8) {
    const oldest = shardKeys.values().next().value;
    w.index_drop_family(oldest);
    shardKeys.delete(oldest);
  }
}
for (const q of ['भवति', 'प्रददत्', 'ददानम्']) {
  const t0 = performance.now();
  const hits = [];
  const seen = new Set();
  for (const r of ok(w.query_candidates(q)).readings.slice(0, 8)) {
    const n = ok(w.index_shard(r.text, 256)).shard;
    for (const [p, f] of [['b', `shard-${String(n).padStart(2, '0')}.index.json`],
      ['u', `sub-${String(n).padStart(2, '0')}.index.json`]]) {
      loadShard(p + n, f);
      ok(w.index_query_family(p + n, r.text)).hits.forEach((h) => {
        const id = JSON.stringify(h);
        if (!seen.has(id)) { seen.add(id); hits.push(h); }
      });
    }
  }
  const tried = new Set();
  for (const s of ok(w.split_query(q, 256)).splits.slice(0, 16)) {
    const id = `${s.stack} ${s.base}`;
    if (tried.has(id)) continue;
    tried.add(id);
    loadShard('b' + s.shard, `shard-${String(s.shard).padStart(2, '0')}.index.json`);
    ok(w.verify_split(q, s.stack, s.base)).hits.forEach((h) => {
      const key = JSON.stringify(h);
      if (!seen.has(key)) { seen.add(key); hits.push(h); }
    });
  }
  console.log(`search ${q.padEnd(10)} -> ${hits.length} hits (${Math.round(performance.now() - t0)}ms cold)`);
}
console.log('\nALL SMOKE CHECKS PASSED');
