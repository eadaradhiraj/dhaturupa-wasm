// Full portal-flow verification against a built dist (run: node /tmp/opencode/e2e.js).
const w = require('/home/edhiraj/Documents/projs/dhaturupa-wasm/pkg-node/dhaturupa_wasm.js');
const fs = require('fs');
const path = require('path');
// Default: the repo's own dist (override: `node e2e-dist.check.cjs /path/to/dist`).
const D = (process.argv[2] || `${path.join(__dirname, '..', 'dist')}/`).replace(/\/?$/, '/');
const ok = (t, what) => {
  const p = JSON.parse(t);
  if (!p.ok) throw new Error(`${what}: ${p.error}`);
  return p;
};
let pass = 0;
const check = (name, cond) => {
  if (!cond) throw new Error(`FAIL: ${name}`);
  pass++;
  console.log(`ok ${pass}: ${name}`);
};

// 1. list source
const list = JSON.parse(fs.readFileSync(`${D}data/dhatu_data.json`, 'utf8'));
check('list has 2259 entries', list.data.length === 2259);

// 2. blob shape
const blob = JSON.parse(fs.readFileSync(`${D}data/blobs/03.0010.json`, 'utf8'));
check('blob has meta+slp1+stacks+ting+krut',
  blob.meta && blob.slp1 && blob.stacks.length > 0 && blob.ting.shuddha && blob.krut.shuddha);

// 3. ting/krut tables (family-file JSON; the portal renders blob slices
// directly in JS, these exports serve fallback/dev paths)
const famDir = '/home/edhiraj/Documents/projs/dhaturupa-wasm/data/';
const famKartari = fs.readFileSync(`${famDir}ting_shuddha_kartari.json`, 'utf8');
const famKrut = fs.readFileSync(`${famDir}krut_shuddha.json`, 'utf8');
check('ting plat present', !!ok(w.ting_table(famKartari, '03.0010'), 'ting').tables.plat);
check('krut shatr present', !!ok(w.krut_table_json(famKrut, '03.0010'), 'krut').tables['शतृ']);
const kartari = JSON.stringify(blob.ting.shuddha.kartari);
const karmani = JSON.stringify(blob.ting.shuddha.karmani || {});
const krut = JSON.stringify(blob.krut.shuddha);

// 4. krt click-through (the fixed contract; renderer passes the site
// convention value + the dhātu row id as the reference-paradigm key)
const sub = ok(w.krt_subanta('shuddha', 'शतृ', 'f', 'भवन्ती', '01.0001'), 'click');
check('click returns tables[]', Array.isArray(sub.tables) && sub.tables.length === 1);
const slash = ok(w.krt_subanta('shuddha', 'शतृ', 'n', 'ददत्/ददद्', '03.0010'), 'slash click');
check('slash click works', slash.tables.length === 1);
const mclick = ok(w.krt_subanta('shuddha', 'शतृ', 'm', 'भवत्', '01.0001'), 'base click');
check('base click declines the site reference (भवन्)',
  mclick.tables[0].cells[0].forms.includes('भवन्'));

// 5. prefixed table views (ting + krt, with dhatu ctx)
const catT = ok(w.engine_category('shuddha', 'ting'), 'cat').category;
const pt = ok(w.prefixed_table_ctx('प्र', JSON.stringify(['भवति']), 'BU', catT, ''), 'pting');
check('prefixed ting', pt.tables[0][0] === 'प्रभवति');
const catK = ok(w.engine_category('shuddha', 'krut'), 'cat').category;
const pk = ok(w.prefixed_table_ctx('प्र', JSON.stringify(['भवः']), 'BU', catK, ''), 'pkrut');
check('prefixed krt', pk.tables[0][0] === 'प्रभवः');

// 6. global search: base + subanta + unlisted-stack split
const loadShard = (kind, form) => {
  const n = JSON.parse(w.index_shard(form, 256)).shard;
  const key = kind[0] + n;
  w.index_load_key(key, fs.readFileSync(
    `${D}data/shards/${kind}-${String(n).padStart(2, '0')}.index.json`, 'utf8'));
  return key;
};
const q1 = ok(w.index_query_family(loadShard('shard', 'भवति'), 'bhavati'), 'q1');
check('bhavati hits cover 01.0001+10.0382', q1.total >= 2);
const q2 = ok(w.index_query_family(loadShard('sub', 'ददतः'), 'dadataH'), 'q2');
check('HK dadataH subanta hits', q2.total >= 1 && q2.hits[0].kind === 'subanta');
// unlisted stack via split+verify
const names = [...new Set(list.data.flatMap((e) => e.upasargas.map((u) => u.name)))];
ok(w.split_init(JSON.stringify(names)), 'split-init');
const splits = ok(w.split_query('प्रददत्', 256), 'split').splits;
check('pradadat splits to pra+dadat',
  splits.some((s) => s.stack === 'प्र' && s.base === 'ददत्'));
let found = 0;
for (const s of splits.slice(0, 5)) {
  loadShard('shard', s.base);
  const v = ok(w.verify_split('प्रददत्', s.stack, s.base), 'verify');
  found += v.total;
}
check('pradadat verifies end to end', found >= 1);

// 7. per-dhatu detail searches from blob slices
const stacks = JSON.stringify(blob.stacks);
const dhatu = JSON.stringify({ family: 'shuddha', baseindex: '03.0010', slp1: blob.slp1, note: '' });
const dp = ok(w.search_prefixed_dhatu(kartari, karmani, krut, dhatu, stacks, 'प्रभवति'), 'dp');
check('detail prefixed search', dp.total >= 0);
const ds = ok(w.search_subanta_dhatu(krut, 'shuddha', '03.0010', 'ददती'), 'ds');
check('detail subanta search', ds.total >= 1);

// 8. readings have no SLP1
const readings = ok(w.query_candidates('gacchantau'), 'readings').readings;
check('single HK reading', readings.length === 1 && readings[0].scheme === 'Harvard-Kyoto');

// 9. word view: readings + declension for arbitrary words, SLP1-free.
// The spec trio: `WrampeRa` is both `Wrampa`'s instrumental singular and
// itself; `Wrampena` is only itself (instrumental would need natva).
const wr = ok(w.subanta_analyze('WrampeRa'), 'word');
check('WrampeRa reads as Wrampa instrumental + itself',
  wr.analyses.some((a) => a.stem === 'ठ्रम्प' && a.vibhakti === 'tftIyA') &&
  wr.analyses.some((a) => a.stem === 'ठ्रम्पेण' && a.vibhakti === 'sambodhana') &&
  wr.tables.some((t) => t.linga === 'm' && t.class === 'a-m'));
check('word JSON never leaks SLP1', !JSON.stringify(wr).includes('Wramp'));
const wn = ok(w.subanta_analyze('Wrampena'), 'word2');
check('Wrampena reads only as itself',
  wn.analyses.length === 2 && wn.analyses.every((a) => a.stem === 'ठ्रम्पेन'));
const wd = ok(w.subanta_analyze('jona'), 'word3');
check('HK foreign name declines (John)',
  wd.analyses.some((a) => a.stem === 'जोन') && wd.tables.some((t) => t.class === 'a-m'));
const wj = ok(w.subanta_analyze('जॉन'), 'word4');
check('candra spelling जॉन folds to जोन',
  wj.word === 'जोन' && wj.tables.some((t) => t.class === 'a-m'));
// the lexical नदीसंज्ञक default for a bare `-I` stem: visarga in the
// nominative and ṇatva in the genitive plural (ashtadhyayi.com शब्द
// @lakSmI1: लक्ष्मीः … लक्ष्मीणाम्) as the first table; the कृदन्त/उगित्
// `I-f` grid (nominative = bare stem, as pinned for शतृ f भवन्ती) follows,
// and the bare stem itself reads as a citation.
const wl = ok(w.subanta_analyze('लक्ष्मी'), 'word6');
check('ī-feminine declines as नदी (लक्ष्मीः … लक्ष्मीणाम्)',
  wl.tables.some((t) => t.class === 'I-f-nadi' && t.cells[0].forms[0] === 'लक्ष्मीः' &&
    t.cells[17].forms[0] === 'लक्ष्मीणाम्') &&
  wl.tables.some((t) => t.class === 'I-f' && t.cells[0].forms[0] === 'लक्ष्मी') &&
  wl.analyses.some((a) => a.class === 'I-f-nadi' && a.vibhakti === 'citation'));
const wlh = ok(w.subanta_analyze('लक्ष्मीः'), 'word7');
check('लक्ष्मीः reads नदी nominative + accusative plural',
  wlh.analyses.some((a) => a.class === 'I-f-nadi' && a.vibhakti === 'prathamA' &&
    a.vacana === 'ekavacanam') &&
  wlh.analyses.some((a) => a.vibhakti === 'dvitIyA' && a.vacana === 'bahuvacanam'));
// the word view links readings to their dhātu by probing this shard itself
// (गच्छन् = शतृ of गम् 01.1137, masculine nominative/vocative)
const wk = ok(w.subanta_analyze('gacCan'), 'word5');
const wkHits = ok(w.index_query_family(loadShard('sub', wk.word), wk.word), 'word refs').hits;
check('kṛt readings link to their dhātu (शतृ of गम्)',
  wkHits.some((h) => h.kind === 'subanta' && h.krt === 'शतृ' &&
    h.baseindex === '01.1137' && h.field === 'm'));

// 10. word-view शब्द oracle: dist/data/shabda2.json ships the site's own
// 24-cell grids (built by web/build-shabda.cjs), and render.siteTables maps
// them to display sections. These pin the headline parity cases cell-exactly,
// so a data refresh that changes a grid can never slip through silently.
// (Async: dist/render.js is an ES module.)
(async () => {
  const { pathToFileURL } = require('node:url');
  const { siteTables, renderParadigm } = await import(pathToFileURL(`${D}render.js`).href);
  const sh = JSON.parse(fs.readFileSync(`${D}data/shabda2.json`, 'utf8'));
  check('shabda2.json carries every site entry (9007)', sh.entries.length === 9007);
  const byWord = new Map();
  for (const [word, linga, urlid, forms] of sh.entries) {
    if (!byWord.has(word)) byWord.set(word, []);
    byWord.get(word).push([linga, urlid, forms]);
  }
  const t = (word) => siteTables(byWord, word);
  const cell = (word, i) => t(word)[0].cells[i].forms;
  const lakzmI = t('लक्ष्मी');
  check('लक्ष्मी site grid: visarga nominative … ण gen.pl',
    lakzmI.length === 1 && cell('लक्ष्मी', 0)[0] === 'लक्ष्मीः' &&
    cell('लक्ष्मी', 17)[0] === 'लक्ष्मीणाम्');
  const nadI = t('नदी');
  check('नदी site grid: ङ्यन्त bare nominative (not नदीः) … नदीनाम्',
    nadI.length === 1 && cell('नदी', 0)[0] === 'नदी' &&
    cell('नदी', 17)[0] === 'नदीनाम्');
  check('नदी sambodhana keeps हे in the rendered grid',
    /<td>हे नदि<\/td>/.test(renderParadigm(nadI[0])));
  check('गति site grid: गत्या + गतये/गत्यै variants',
    cell('गति', 6)[0] === 'गत्या' &&
    cell('गति', 9).join('|') === 'गतये|गत्यै');
  check('सखि site grid: सखा … सख्युः',
    cell('सखि', 0)[0] === 'सखा' && cell('सखि', 12)[0] === 'सख्युः');
  const grAmaNI = t('ग्रामणी');
  check('ग्रामणी: identical masc/fem grids collapse into one section',
    grAmaNI.length === 1 && grAmaNI[0].lingaLabel === 'masculine, feminine' &&
    grAmaNI[0].class === 'ashtadhyayi.com @grAmaNI1 @grAmaNI2');
  const eka = t('एक');
  check('एक: masculine + neuter grids, empty vocative, ablative variants',
    eka.length === 2 && eka[0].cells[0].forms[0] === 'एकः' &&
    eka[1].cells[0].forms[0] === 'एकम्' &&
    eka[0].cells[21].forms.length === 0 &&
    eka[0].cells[12].forms.join('|') === 'एकस्माद्|एकस्मात्');
  check('षष् site grid: षड्/षट् nominative, षण्णाम् gen.pl',
    cell('षष्', 2).join('|') === 'षड्|षट्' && cell('षष्', 17)[0] === 'षण्णाम्');
  check('अष्टन् site grid: अष्ट/अष्टौ nominative, अष्टानाम् gen.pl',
    cell('अष्टन्', 2).join('|') === 'अष्ट|अष्टौ' &&
    cell('अष्टन्', 17)[0] === 'अष्टानाम्');
  const asmad = t('अस्मद्');
  check('अस्मद्: सर्वनाम grid with any-gender label',
    asmad.length === 1 && asmad[0].linga === 'A' &&
    asmad[0].lingaLabel === 'any gender' &&
    cell('अस्मद्', 0)[0] === 'अहम्' && cell('अस्मद्', 1)[0] === 'आवाम्');
  check('unlisted words fall through to the derived engine tables',
    t('ठ्रम्पेण').length === 0 && !byWord.has('जोन'));
})().then(() => {
  console.log(`\nALL ${pass} PORTAL-FLOW CHECKS PASSED`);
}, (e) => {
  console.error(e);
  process.exit(1);
});
