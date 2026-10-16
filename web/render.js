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

// {krt: [{base,m,f,n}]} -> HTML; declinable cells carry click metadata
// (app.js delegates clicks to krt_subanta).
export function renderKrutTables(tables, family) {
  const krts = Object.keys(tables).sort();
  return krts
    .map((krt) => {
      const rows = tables[krt]
        .map((cell, row) => {
          const link = (value, field, gender) =>
            `<a href="#" data-krt="${escapeHtml(krt)}" ` +
            `data-row="${row}" data-field="${field}" data-gender="${gender}" ` +
            `data-value="${escapeHtml(value)}" data-family="${escapeHtml(family)}">` +
            `${escapeHtml(value)}</a>`;
          const field = (value, gender) =>
            value.includes('/')
              ? value
                  .split('/')
                  .filter((part) => part !== '')
                  .map((part) => link(part, part, gender))
                  .join(' ')
              : link(value, '', gender);
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

// WASM paradigm JSON -> HTML (subanta + click-through alike).
export function renderParadigm(table) {
  const rows = table.cells
    .map(
      (cell) =>
        `<tr><th>${escapeHtml(cell.vibhakti)} · ${escapeHtml(cell.vacana)}</th>` +
        `<td>${cell.forms.map(escapeHtml).join(' , ')}</td></tr>`,
    )
    .join('');
  return `<table><tbody>${rows}</tbody></table>`;
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
