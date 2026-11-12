// UI-примитивы: форматирование, тосты, диалоги, графики.
const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];
const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
const attr = (v) => esc(JSON.stringify(v));
const plural = (n, a, b, c) => { const m10 = n % 10, m100 = n % 100; return m10 === 1 && m100 !== 11 ? a : m10 >= 2 && m10 <= 4 && (m100 < 10 || m100 >= 20) ? b : c; };
const fmtBytes = (b) => { if (!b) return '0 Б'; const u = ['Б', 'КБ', 'МБ', 'ГБ', 'ТБ']; const i = Math.min(4, Math.floor(Math.log(b) / Math.log(1024))); return (b / 1024 ** i).toFixed(i >= 3 ? 2 : i ? 1 : 0).replace('.', ',') + ' ' + u[i]; };
const fmtMb = (mb) => mb >= 1024 ? (mb / 1024).toFixed(mb % 1024 ? 1 : 0).replace('.', ',') + ' ГБ' : mb + ' МБ';
const fmtCpu = (v) => (Math.round((+v || 0) * 10) / 10).toString().replace('.', ',');
const fmtDate = (t) => new Date(t).toLocaleString('ru-RU', { day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
const fmtNum = (n) => n >= 1e6 ? (n / 1e6).toFixed(1).replace('.', ',') + ' млн' : n >= 1e3 ? Math.round(n / 1e3) + ' тыс.' : String(n);
function fmtAgo(t) {
  const s = (Date.now() - t) / 1000;
  if (s < 60) return 'только что';
  if (s < 3600) { const m = Math.floor(s / 60); return `${m} ${plural(m, 'минуту', 'минуты', 'минут')} назад`; }
  if (s < 86400) { const h = Math.floor(s / 3600); return `${h} ${plural(h, 'час', 'часа', 'часов')} назад`; }
  return fmtDate(t);
}
function fmtDur(ms) {
  const m = Math.floor(ms / 60e3);
  if (m < 1) return '< 1 мин';
  if (m < 60) return m + ' мин';
  const h = Math.floor(m / 60);
  return h < 24 ? `${h} ч ${m % 60} мин` : `${Math.floor(h / 24)} д ${h % 24} ч`;
}
const debounce = (fn, ms) => { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; };

// ---------- тосты ----------
function toast(msg, kind = 'ok', ms = 4500) {
  const icon = { ok: 'checkCircle', error: 'alert', info: 'info' }[kind] || 'info';
  const el = document.createElement('div');
  el.className = 'toast ' + kind;
  const [title, ...rest] = String(msg).split('\n');
  el.innerHTML = `${ic(icon)}<div class="m">${rest.length ? `<b>${esc(title)}</b>${esc(rest.join('\n'))}` : esc(title)}</div>`;
  $('#toasts').append(el);
  const close = () => { el.classList.add('out'); setTimeout(() => el.remove(), 200); };
  el.onclick = close;
  setTimeout(close, kind === 'error' ? Math.max(ms, 7000) : ms);
}

// ---------- модальные окна ----------
function modal({ title, body = '', icon, buttons, wide, onOpen }) {
  return new Promise((resolve) => {
    const ov = document.createElement('div');
    ov.className = 'overlay';
    ov.innerHTML = `<div class="modal ${wide ? 'wide' : ''}" role="dialog"><div class="body"><h3>${icon ? ic(icon) : ''}${esc(title)}</h3>${body}</div>
      <div class="foot">${buttons.map((b, i) => `<button data-i="${i}" class="${b.cls || ''}">${esc(b.label)}</button>`).join('')}</div></div>`;
    const prevFocus = document.activeElement;
    const done = (v) => { ov.remove(); document.removeEventListener('keydown', key, true); prevFocus?.focus?.(); resolve(v); };
    const pick = (i) => { const b = buttons[i]; const v = b.value !== undefined ? (typeof b.value === 'function' ? b.value(ov) : b.value) : i; if (v === false && b.keepOpen) return; done(v); };
    const key = (e) => {
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); done(null); }
      if (e.key === 'Enter' && e.target.tagName !== 'TEXTAREA' && e.target.tagName !== 'BUTTON') { e.preventDefault(); e.stopPropagation(); pick(buttons.findIndex((b) => b.default) >= 0 ? buttons.findIndex((b) => b.default) : buttons.length - 1); }
    };
    ov.addEventListener('mousedown', (e) => { if (e.target === ov) done(null); });
    $$('.foot button', ov).forEach((b) => (b.onclick = () => pick(+b.dataset.i)));
    document.addEventListener('keydown', key, true);
    document.body.append(ov);
    onOpen?.(ov);
    ($('input,select,textarea', ov) || $$('.foot button', ov).pop()).focus();
  });
}
function confirmDialog(text, { title = 'Подтверждение', ok = 'OK', danger = false, icon } = {}) {
  return modal({ title, icon: icon || (danger ? 'alert' : null), body: `<p>${esc(text)}</p>`, buttons: [{ label: 'Отмена', value: false }, { label: ok, cls: danger ? 'danger solid' : 'primary', value: true, default: true }] }).then((v) => v === true);
}
function alertDialog(text, title = 'Готово', icon = 'info') {
  return modal({ title, icon, body: `<p class="selectable">${esc(text)}</p>`, buttons: [{ label: 'OK', cls: 'primary', value: true }] });
}
function promptDialog({ title, label = '', value = '', placeholder = '', type = 'text', ok = 'OK', hint = '' }) {
  return modal({
    title, body: `<div class="field">${label ? `<label>${esc(label)}</label>` : ''}<input id="pd-in" type="${type}" value="${esc(value)}" placeholder="${esc(placeholder)}">${hint ? `<span class="hint">${esc(hint)}</span>` : ''}</div>`,
    buttons: [{ label: 'Отмена', value: null }, { label: ok, cls: 'primary', default: true, value: (ov) => $('#pd-in', ov).value }],
    onOpen: (ov) => $('#pd-in', ov).select(),
  });
}
function formDialog({ title, fields, ok = 'Сохранить', icon }) {
  const body = `<div class="stack">${fields.map((f) => `<div class="field"><label>${esc(f.label)}</label>${f.options
    ? `<select data-k="${f.key}">${f.options.map((o) => `<option value="${esc(o[0])}" ${o[0] === f.value ? 'selected' : ''}>${esc(o[1])}</option>`).join('')}</select>`
    : `<input data-k="${f.key}" value="${esc(f.value || '')}" placeholder="${esc(f.placeholder || '')}">`}${f.hint ? `<span class="hint">${esc(f.hint)}</span>` : ''}</div>`).join('')}</div>`;
  return modal({ title, icon, body, buttons: [{ label: 'Отмена', value: null }, { label: ok, cls: 'primary', default: true, value: (ov) => Object.fromEntries($$('[data-k]', ov).map((e) => [e.dataset.k, e.value])) }] });
}

// ---------- графики (SVG без библиотек) ----------
function sparkPath(vals, w, h, max) {
  if (vals.length < 2) return { line: '', area: '' };
  const m = max ?? Math.max(...vals, 1), step = w / (vals.length - 1);
  const pts = vals.map((v, i) => [i * step, h - 2 - (Math.min(v, m) / m) * (h - 6)]);
  const line = pts.map((p, i) => (i ? 'L' : 'M') + p[0].toFixed(1) + ' ' + p[1].toFixed(1)).join(' ');
  return { line, area: `${line} L${w} ${h} L0 ${h} Z` };
}
function sparkline(vals, color, max, h = 38) {
  const w = 300, { line, area } = sparkPath(vals, w, h, max);
  if (!line) return '';
  const id = 'g' + Math.random().toString(36).slice(2, 8);
  return `<svg class="spark" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none"><defs><linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${color}" stop-opacity=".22"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></linearGradient></defs>
    <path d="${area}" fill="url(#${id})"/><path d="${line}" fill="none" stroke="${color}" stroke-width="1.6" vector-effect="non-scaling-stroke"/></svg>`;
}
/// Двухсерийный график CPU/RAM для карточки сервера.
function dualChart(hist, ramMax) {
  const w = 300, h = 64;
  if (!hist || hist.length < 2) return `<svg class="chart" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none"><line x1="0" y1="${h - 1}" x2="${w}" y2="${h - 1}" stroke="var(--border)" stroke-dasharray="3 4"/></svg>`;
  const cpu = sparkPath(hist.map((x) => x[1]), w, h, 100), mem = sparkPath(hist.map((x) => x[2]), w, h, ramMax);
  return `<svg class="chart" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none">
    <path d="${mem.area}" fill="var(--chart-2)" fill-opacity=".08"/><path d="${mem.line}" fill="none" stroke="var(--chart-2)" stroke-width="1.5" vector-effect="non-scaling-stroke"/>
    <path d="${cpu.area}" fill="var(--chart-1)" fill-opacity=".1"/><path d="${cpu.line}" fill="none" stroke="var(--chart-1)" stroke-width="1.5" vector-effect="non-scaling-stroke"/></svg>`;
}
const sw = (id, checked, extra = '') => `<label class="switch"><input type="checkbox" id="${id}" ${checked ? 'checked' : ''} ${extra}><span class="track"></span></label>`;
