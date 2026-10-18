// Pure portal rendering (no DOM, no imports): Devanagari forms in,
// English labels out. Tested under node; wired to the DOM in app.js.

export function escapeHtml(text) {
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// Slot positions 0-8: prathama/madhyama/uttama x eka/dvi/bahu.
// Only sound for 9-slot (or multiples-of-9) arrays; otherwise plain numbers.
export function slotLabel(pos, len) {
  const purusha = ['prathama', 'madhyama', 'uttama'];
  const vacana = ['ekavacana', 'dvivacana', 'bahuvacana'];
  if (len % 9 === 0 && pos < len) {
    const slot = pos % 9;
    return `${purusha[Math.floor(slot / 3)]} · ${vacana[slot % 3]}`;
  }
  return `form ${pos + 1}`;
}

// {lakara: [[variants], ...]} -> HTML tables, lakaras A-Z.
const LAKARAS = {
  lat: 'लट्', lit: 'लिट्', lut: 'लुट्', lrut: 'लृट्', lot: 'लोट्',
  lang: 'लङ्', vidhiling: 'विधिलिङ्', ashirling: 'आशीर्लिङ्', lung: 'लुङ्', lrung: 'लृङ्',
};

// Lakāra codes carry the पद as a prefix letter: `plat` = laṭ in
// परस्मैपदम्, `alat` = laṭ in आत्मनेपदम् (the `a*` tables of every
// prayoga are its ātmanepada forms — e.g. `alat` on कर्मणी = दीयते).
const lakaraLabel = (code) => {
  const pada = code.startsWith('p') ? 'परस्मैपदम्' : 'आत्मनेपदम्';
  const name = LAKARAS[code.slice(1)] || LAKARAS[code] || code;
  return `${name} · ${pada}`;
};

// परस्मैपदम् sections first, then आत्मनेपदम्, each in traditional order.
const LAKARA_ORDER = ['lat', 'lit', 'lut', 'lrut', 'lot', 'lang', 'vidhiling', 'ashirling', 'lung', 'lrung'];
const lakaraSort = (a, b) => {
  if (a[0] !== b[0]) return a[0] === 'p' ? -1 : 1;
  return LAKARA_ORDER.indexOf(a.slice(1)) - LAKARA_ORDER.indexOf(b.slice(1));
};

export function renderTingTables(tables) {
  const lakaras = Object.keys(tables).sort(lakaraSort);
  return lakaras
    .map((lakara) => {
      const slots = tables[lakara];
      const rows = slots
        .map(
          (variants, pos) =>
            `<tr><th>${escapeHtml(slotLabel(pos, slots.length))}</th>` +
            `<td>${variants.map(escapeHtml).join(' , ')}</td></tr>`,
        )
        .join('');
      return `<section class="lakara"><h3>${escapeHtml(lakaraLabel(lakara))}</h3>` +
        `<table><tbody>${rows}</tbody></table></section>`;
    })
    .join('');
}

// Kṛt link target per ashtadhyayi.com's `createSubantaForms`: m/n links
// decline the row's base, not the clicked cell; f links pass the f form
// itself only when its last letter (`WordUtils.getLettersOfWord` — matras
// count as their independent vowels, so an आ/ई ending means a trailing
// ा/ी matra) is आ or ई, otherwise the base.
export function krtLinkTarget(gender, form, base) {
  if (gender === 'f' && /[ाआीी]$/.test(form)) return form;
  return base && base !== '—' ? base : form;
}

// {krt: [{base,m,f,n}]} -> HTML; every form cell carries click metadata
// (app.js delegates clicks to krt_subanta, which resolves the site's
// reference paradigm from the dhātu row id).
export function renderKrutTables(tables, family) {
  const krts = Object.keys(tables).sort();
  return krts
    .map((krt) => {
      const rows = tables[krt]
        .map((cell, row) => {
          const base = typeof cell.base === 'string' ? cell.base : '';
          const link = (form, gender) =>
            `<a href="#" data-krt="${escapeHtml(krt)}" ` +
            `data-row="${row}" data-field="${gender}" data-gender="${gender}" ` +
            `data-value="${escapeHtml(krtLinkTarget(gender, form, base))}" ` +
            `data-family="${escapeHtml(family)}">${escapeHtml(form)}</a>`;
          const field = (value, gender) =>
            value
              .split('/')
              .filter((part) => part !== '')
              .map((part) => link(part, gender))
              .join(' ');
          return `<tr><td>${cell.base ? escapeHtml(cell.base) : '—'}</td>` +
            `<td>${cell.m ? field(cell.m, 'm') : '—'}</td>` +
            `<td>${cell.f ? field(cell.f, 'f') : '—'}</td>` +
            `<td>${cell.n ? field(cell.n, 'n') : '—'}</td></tr>`;
        })
        .join('');
      return `<section class="krt"><h3>${escapeHtml(krt)}</h3>` +
        `<table><thead><tr><th>base</th><th>m</th><th>f</th>` +
        `<th>n</th></tr></thead><tbody>${rows}</tbody></table></section>`;
    })
    .join('');
}

// WASM paradigm JSON -> HTML. The site's sidebar (`krit-subanta-table-template`)
// is a bare 8x3 grid — one row per vibhakti, one column per vacana, no
// labels — and सम्बोधन cells get the `हे ` prefix (`getFinalForm`).
export function renderParadigm(table) {
  const cells = table.cells || [];
  const rows = [];
  for (let i = 0; i < cells.length; i += 3) {
    rows.push(`<tr>${cells
      .slice(i, i + 3)
      .map((cell) => {
        const he = cell.vibhakti === 'sambodhana' ? 'हे ' : '';
        const forms = (cell.forms || []).map((form) => escapeHtml(he + form));
        return `<td>${forms.length ? forms.join(' , ') : '—'}</td>`;
      })
      .join('')}</tr>`);
  }
  return `<table><tbody>${rows.join('')}</tbody></table>`;
}

// Word-view (subanta reading) labels: engine slot codes -> English.
// सम्बोधन keeps its Sanskrit name: the vocative is the reading users ask
// about ("Wrampena as sambodhana is the base word").
const VIBHAKTIS = {
  prathamA: 'nominative', dvitIyA: 'accusative', tftIyA: 'instrumental',
  caturTI: 'dative', paJcamI: 'ablative', zazWI: 'genitive',
  saptamI: 'locative', sambodhana: 'vocative (sambodhana)',
  citation: 'citation form',
};
const VACANAS = { ekavacanam: 'singular', dvivacanam: 'dual', bahuvacanam: 'plural' };
// `A` is the site's लिङ्ग slot for the सर्वनाम entries (अस्मद्/युष्मद्).
const LINGAS = { m: 'masculine', f: 'feminine', n: 'neuter', A: 'any gender' };

// [{stem, linga, class, vibhakti, vacana}] -> reading list HTML. Each row
// states the relation ("instrumental singular of ठ्रम्प") and carries a
// toggle that loads that stem's own declension (app.js: reading-table-btn).
// `krtRefs` are corpus subanta postings ({baseindex, krt, field, vibhakti,
// vacana}) whose slot matches a reading: that reading then links to the
// dhātu whose kṛt paradigm produces this form. `meta` maps baseindex ->
// {dhatu, artha} for the link labels (app.js: dhatuCards).
export function renderWordReadings(analyses, krtRefs = [], meta = {}) {
  if (!analyses || analyses.length === 0) {
    return '<p class="muted">No readings — check the spelling. Foreign ' +
      'names work in their Sanskritized spelling, usually ending in a vowel ' +
      '(e.g. जॉन for John).</p>';
  }
  // Gender + slot must match exactly: the input form is the same string, so
  // a posting is evidence for this reading iff field/vibhakti/vacana agree
  // (corpus duplicates across families collapse to one link per dhātu).
  const refsFor = (a) => {
    const seen = new Set();
    const out = [];
    for (const p of krtRefs || []) {
      if (p.field !== a.linga || p.vibhakti !== a.vibhakti || p.vacana !== a.vacana) continue;
      const key = `${p.baseindex}|${p.krt}`;
      if (seen.has(key)) continue;
      seen.add(key);
      out.push(p);
    }
    return out;
  };
  const refLink = (p) => {
    const entry = (meta || {})[p.baseindex];
    const name = entry ? entry.dhatu : p.baseindex;
    return `<a class="reading-ref" href="#dhatu/${encodeURIComponent(p.baseindex)}" ` +
      `title="occurs in the corpus as ${escapeHtml(p.krt)} of ${escapeHtml(name)}">` +
      `${escapeHtml(p.krt)} of <span class="deva">${escapeHtml(name)}</span></a>`;
  };
  const rows = analyses.map((a) => {
    const slot = [VIBHAKTIS[a.vibhakti] || a.vibhakti, VACANAS[a.vacana] || a.vacana]
      .filter(Boolean)
      .join(' ');
    const linga = LINGAS[a.linga] || a.linga;
    const refs = refsFor(a);
    const refsHtml = refs.length
      ? ` <span class="reading-refs">— ${refs.map(refLink).join(', ')}</span>`
      : '';
    return `<li><span class="reading-slot">${escapeHtml(slot)}</span> of ` +
      `<span class="deva">${escapeHtml(a.stem)}</span> ` +
      `<span class="muted">(${escapeHtml(linga)} · ${escapeHtml(a.class)})</span> ` +
      `<button class="reading-table-btn" data-stem="${escapeHtml(a.stem)}" ` +
      `data-linga="${escapeHtml(a.linga)}">table</button>${refsHtml}</li>`;
  });
  return `<ul class="word-readings">${rows.join('')}</ul>`;
}

// [{stem, linga, lingaLabel?, class, cells}] -> headed declension sections
// (bare 8x3). `lingaLabel` overrides the single-gender label when one site
// entry stands for several लिङ्गs (e.g. ग्रामणी masc/fem).
export function renderWordTables(tables) {
  if (!tables || tables.length === 0) {
    return '<p class="muted">No declension: this shape matches no Sanskrit ' +
      'paradigm (consonant-final spellings are refused). Try the ' +
      'Sanskritized spelling ending in a vowel (e.g. जॉन for John).</p>';
  }
  return tables.map((t) =>
    `<section class="word-table"><h4><span class="deva">${escapeHtml(t.stem)}</span>` +
    ` — ${escapeHtml(t.lingaLabel || LINGAS[t.linga] || t.linga)} ` +
    `(${escapeHtml(t.class)})</h4>` +
    renderParadigm(t) + '</section>').join('');
}

// Vendored शब्दपाठः oracle (dist/data/shabda2.json, built by
// web/build-shabda.cjs) -> declension sections. `byWord` maps a Devanagari
// word to site entries `[linga, urlid, forms]`; `forms` is the site's raw
// 24-cell string — vibhakti-major (pr, dvi, tft, cat, pan, zaz, sap, samb),
// variants `-`-joined, empty cells `''`. These are the site's own grids, so
// the word view shows exactly what ashtadhyayi.com renders for a listed
// word; anything unlisted falls back to the derived engine tables.
// Entries whose grids are cell-identical collapse into one section: the
// site lists them once per लिङ्ग/urlid (ग्रामणी @grAmaNI1 masc + @grAmaNI2
// fem), and two identical grids would read as a rendering bug.
const SITE_VIB = [
  'prathamA', 'dvitIyA', 'tftIyA', 'caturTI', 'paJcamI', 'zazWI', 'saptamI',
  'sambodhana',
];
const SITE_VAC = ['ekavacanam', 'dvivacanam', 'bahuvacanam'];
// The dump's लिङ्ग codes (P/S/N/A) vs. the engine's table codes (m/f/n/A).
const SITE_LINGA_CODES = { P: 'm', S: 'f', N: 'n', A: 'A' };
export function siteTables(byWord, word) {
  const entries = byWord && (byWord.get ? byWord.get(word) : byWord[word]);
  if (!entries || entries.length === 0) return [];
  const groups = new Map(); // raw forms string -> {lingas, urlids}
  for (const [linga, urlid, forms] of entries) {
    let group = groups.get(forms);
    if (!group) groups.set(forms, (group = { lingas: [], urlids: [] }));
    if (!group.lingas.includes(linga)) group.lingas.push(linga);
    group.urlids.push(urlid);
  }
  return [...groups.entries()].map(([forms, group]) => ({
    stem: word,
    linga: SITE_LINGA_CODES[group.lingas[0]] || group.lingas[0],
    lingaLabel: group.lingas
      .map((l) => LINGAS[SITE_LINGA_CODES[l]] || l)
      .join(', '),
    class: `ashtadhyayi.com ${group.urlids.join(' ')}`,
    cells: forms.split(';').map((raw, i) => {
      const vibhakti = SITE_VIB[Math.floor(i / 3)];
      const kept = raw === '' || raw === '-' ? [] : raw.split('-');
      return {
        vibhakti,
        vacana: SITE_VAC[i % 3],
        // The site keeps `हे ` inside the cell; renderParadigm re-adds it,
        // so store the bare vocative.
        forms: kept.map((form) => {
          const trimmed = form.trim();
          return vibhakti === 'sambodhana' ? trimmed.replace(/^हे /, '') : trimmed;
        }),
      };
    }),
  }));
}

// Chandrika entries -> HTML; `<hl>` spans become highlights, quotes kept.
export function renderChandrika(entries) {
  if (!entries || entries.length === 0) {
    return '<p class="muted">No Chandrika entry for this dhātu.</p>';
  }
  const highlight = (text) => escapeHtml(text)
    .replace(/&lt;hl&gt;/g, '<mark>')
    .replace(/&lt;\/hl&gt;/g, '</mark>')
    .replace(/‘/g, '‘').replace(/’/g, '’');
  return entries.map((entry) => {
    const head = `<h4 class="deva">${escapeHtml(entry.upasarga)}</h4>` +
      `<p class="deva">${escapeHtml(entry.artha)}</p>`;
    const list = (entry.examples || []).map((ex) =>
      `<li>${highlight(ex)}</li>`).join('');
    return '<section class="chandra-entry">' + head +
      (list ? `<ol>${list}</ol>` : '') + '</section>';
  }).join('');
}

// Reverse-search hits -> HTML list linking to dhatu detail pages.
// `meta` maps baseindex -> {dhatu, artha} (Devanagari card line);
// `form` is the matched display form; `field` renders as gender words.
export function renderHits(hits, meta, form) {
  if (hits.length === 0) return '<p>No matches.</p>';
  const card = (id) => {
    const entry = (meta || {})[id];
    return entry ? ` <span class="deva">${escapeHtml(entry.dhatu)} — ${escapeHtml(entry.artha)}</span>` : '';
  };
  const match = form ? ` <span class="deva">“${escapeHtml(form)}”</span>` : '';
  const gender = { m: 'masculine', f: 'feminine', n: 'neuter' };
  const detail = (hit) => {
    if (hit.kind === 'ting') {
      return `ting · ${escapeHtml(hit.lakara)}[${hit.pos}]`;
    }
    if (hit.kind === 'krt') {
      const gend = gender[hit.field] ? ` (${hit.field}, ${gender[hit.field]})` : '';
      return `krdanta · ${escapeHtml(hit.krt)}[${hit.row}]${escapeHtml(hit.field)}${gend}`;
    }
    if (hit.kind === 'prefixed') {
      return `prefixed via <span class="deva">${escapeHtml(hit.stack)}</span> ← ${detail(hit.base)}`;
    }
    const gend = gender[hit.field] ? ` (${hit.field}, ${gender[hit.field]})` : '';
    return `subanta · ${escapeHtml(hit.krt)}${gend} · ` +
      `${escapeHtml(hit.vibhakti)}.${escapeHtml(hit.vacana)}`;
  };
  // Canonical hash: `#dhatu/NN.NNNN[/प्र]` — the stack suffix opens the
  // detail page already prefixed (app.js route/route handler).
  const hitHref = (hit) => {
    const base = `#dhatu/${encodeURIComponent(hit.baseindex)}`;
    return hit.kind === 'prefixed' && hit.stack
      ? `${base}/${encodeURIComponent(hit.stack)}`
      : base;
  };
  return '<ol>' + hits.map((hit) =>
    `<li><a href="${hitHref(hit)}">` +
    `${escapeHtml(hit.baseindex)}</a>${card(hit.baseindex)}` +
    ` <span>${detail(hit)}</span>${match}</li>`,
  ).join('') + '</ol>';
}

const GANAS = {
  1: 'bhvādi', 2: 'adādi', 3: 'juhotyādi', 4: 'divādi', 5: 'svādi',
  6: 'tudādi', 7: 'rudhādi', 8: 'tanādi', 9: 'kryādi', 10: 'curādi',
};
const PADAS = { P: 'parasmaipadī', A: 'ātmanepadī', U: 'ubhayapadī' };
const SETTVAS = { S: 'seṭ', A: 'aniṭ', V: 'veṭ' };
const KARMAS = { S: 'sakarmaka', A: 'akarmaka', D: 'dvikarmaka' };

// Blob meta -> English-labelled definition list.
export function renderMeta(meta) {
  const rows = [
    ['Dhātu', meta.dhatu || ''],
    ['Meaning (Sanskrit)', meta.artha || ''],
    ['Meaning (English)', meta.artha_english || ''],
    ['Meaning (Hindi)', meta.artha_hindi || ''],
    ['Gaṇa', GANAS[meta.gana] ? `${meta.gana} (${GANAS[meta.gana]})` : (meta.gana || '')],
    ['Pada', PADAS[meta.pada] || meta.pada || ''],
    ['Seṭtva', SETTVAS[meta.settva] || meta.settva || ''],
    ['Transitivity', KARMAS[meta.karma] || meta.karma || ''],
  ].filter(([, value]) => value !== '');
  return '<dl>' + rows.map(([label, value]) =>
    `<dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd>`).join('') + '</dl>';
}
