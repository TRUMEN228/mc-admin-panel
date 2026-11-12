// MC Admin Panel — интерфейс.
const LOADERS = {
  vanilla: { name: 'Vanilla', icon: 'cube', desc: 'Официальный сервер без модов' },
  paper: { name: 'Paper', icon: 'paper', desc: 'Плагины Bukkit/Spigot, высокая производительность' },
  fabric: { name: 'Fabric', icon: 'feather', desc: 'Лёгкие современные моды' },
  forge: { name: 'Forge', icon: 'hammer', desc: 'Классические крупные моды' },
  neoforge: { name: 'NeoForge', icon: 'flame', desc: 'Современный форк Forge' },
};
const STATUS = { running: ['Работает', 'green'], starting: ['Запуск…', 'yellow'], stopping: ['Остановка…', 'yellow'], stopped: ['Остановлен', ''] };
const S = { servers: [], settings: {}, sets: [], sys: null, info: {} };
let cur = {};
const srvById = (id) => S.servers.find((s) => s.id === id);
const loaderName = (l) => LOADERS[l]?.name || l;

// ---------- тема ----------
function applyTheme() {
  const t = S.settings.theme || 'system';
  if (t === 'system') delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = t;
  if (NATIVE) T.window.getCurrentWindow().setTheme(t === 'system' ? null : t).catch(() => {});
}

// ---------- каркас ----------
function renderShell() {
  $('#search-btn').innerHTML = `${ic('search')}<span class="lbl">Поиск и команды</span><span class="kbd">Ctrl K</span>`;
  $('#search-btn').onclick = openPalette;
  renderNav(); renderFoot();
}
function navItem(href, icon, label, on, extra = '') {
  return `<a class="nav-item ${on ? 'on' : ''}" href="#${href}" title="${esc(label)}">${ic(icon)}<span class="lbl">${esc(label)}</span>${extra}</a>`;
}
function renderNav() {
  const p = cur.page;
  $('#nav').innerHTML =
    navItem('dash', 'home', 'Обзор', p === 'dash') +
    navItem('create', 'plus', 'Новый сервер', p === 'create') +
    `<div class="sec"><span>Серверы</span><button class="ghost icon sm" title="Импорт существующих" onclick="importDir('servers')">${ic('import')}</button></div>` +
    (S.servers.length ? S.servers.map((s) => navItem(`server/${s.id}/console`, 'server', s.name, p === 'server' && cur.id === s.id,
      `<span class="meta" id="nm-${s.id}">${navMeta(s)}</span><span class="dot ${s.status}" id="nd-${s.id}"></span>`)).join('')
      : `<div class="nav-item dim small" style="cursor:default"><span class="lbl">Пока пусто</span></div>`) +
    `<div class="sec"><span>Клиент</span></div>` +
    navItem('sets', 'puzzle', 'Наборы модов', p === 'sets') +
    `<div class="sec"><span>Справка</span></div>` +
    navItem('cmds', 'book', 'Команды сервера', p === 'cmds');
}
const navMeta = (s) => (s.status === 'running' && s.players.length ? `${s.players.length} ${ic('users')}` : '');
function renderFoot() {
  $('#side-foot').innerHTML = `<div class="sys-mini" id="sys-mini">${sysMini()}</div>` +
    navItem('settings', 'settings', 'Настройки', cur.page === 'settings') +
    `<a class="nav-item" onclick="toggleSide()" title="Свернуть панель (Ctrl+B)">${ic('sidebar')}<span class="lbl">Свернуть</span></a>`;
}
function sysMini() {
  const s = S.sys; if (!s) return '';
  const mp = s.memTotal ? (s.memUsed / s.memTotal) * 100 : 0;
  return `<div class="rows"><span>CPU</span><div class="meter"><i style="width:${s.cpu}%"></i></div><span class="num">${Math.round(s.cpu)}%</span>
    <span>RAM</span><div class="meter"><i style="width:${mp}%"></i></div><span class="num">${(s.memUsed / 1073741824).toFixed(1)} ГБ</span></div>`;
}
function toggleSide() {
  const c = $('#app').classList.toggle('collapsed');
  try { localStorage.setItem('mcpanel-side', c ? '1' : ''); } catch {}
}
function setTop({ title, icon, crumbs, actions = '', sub = '', status }) {
  $('#topbar').innerHTML = `${crumbs ? `<div class="crumbs">${crumbs.map((c) => c[1] ? `<a href="#${c[1]}">${esc(c[0])}</a>` : `<span>${esc(c[0])}</span>`).join(ic('chevronRight'))}</div>` : ''}
    <h1>${status ? `<span class="dot ${status}" id="top-dot"></span>` : icon ? ic(icon) : ''}<span class="t">${esc(title)}</span>${sub}</h1><div class="actions" id="top-actions">${actions}</div>`;
  document.title = `${title} — MC Admin Panel`;
}
function setPage(html, fill = false) {
  const p = $('#page');
  p.className = 'page' + (fill ? ' fill' : '');
  p.innerHTML = html;
  p.style.animation = 'none'; p.offsetHeight; p.style.animation = '';
}

// ---------- роутер ----------
async function route() {
  const [page = 'dash', id, tab] = (location.hash.slice(1) || 'dash').split('/');
  const prev = cur;
  cur = { page, id, tab };
  renderNav(); renderFoot();
  const fn = { dash: pDash, create: pCreate, server: pServer, sets: pSets, cmds: pCmds, settings: pSettings }[page] || pDash;
  if (!(page === 'server' && prev.page === 'server' && prev.id === id && prev.tab !== tab)) $('#page').scrollTop = 0;
  try { await fn(id, tab); } catch (e) { console.error(e); }
}
window.addEventListener('hashchange', route);
const go = (h) => { if (location.hash === '#' + h) route(); else location.hash = h; };
async function loadServers() { S.servers = await call('servers.list'); }

// ---------- живые обновления ----------
function live(s) {
  const nd = $('#nd-' + s.id); if (nd) nd.className = 'dot ' + s.status;
  const nm = $('#nm-' + s.id); if (nm) nm.innerHTML = navMeta(s);
  if (cur.page === 'dash') { const c = $('#card-' + s.id); if (c) c.innerHTML = srvCardInner(s); dashTotals(); }
  if (cur.page === 'server' && cur.id === s.id) {
    const d = $('#top-dot'); if (d) d.className = 'dot ' + s.status;
    const a = $('#top-actions'); if (a && !a.contains(document.activeElement)) a.innerHTML = ctrlButtons(s);
    const m = $('#srv-metrics'); if (m) m.innerHTML = srvMetrics(s);
    const pl = $('#plist'); if (pl) pl.innerHTML = s.players.map((p) => `<option value="${esc(p)}">`).join('');
    if (cur.tab === 'players') { const o = $('#online'); if (o) o.innerHTML = onlineList(s); }
  }
}
function handleEvent(m) {
  const s = m.id && srvById(m.id);
  switch (m.type) {
    case 'job': return showJob(m.job);
    case 'servers': return loadServers().then(() => {
      renderNav();
      if (cur.page === 'dash') route();
      else if (cur.page === 'server') { const x = srvById(cur.id); if (x) live(x); else go('dash'); }
    });
    case 'status':
      if (!s) return;
      if (m.status === 'starting' && s.status !== 'starting') { s.started = Date.now(); s.history = []; }
      s.status = m.status;
      if (m.status === 'stopped') { s.stats = null; s.players = []; }
      return live(s);
    case 'stats':
      if (!s) return;
      Object.assign(s, { stats: m.stats, players: m.players, status: m.status });
      if (m.stats) { (s.history ||= []).push([m.t, m.stats.cpu, Math.round(m.stats.mem / 1048576), m.players.length]); if (s.history.length > 180) s.history.shift(); }
      return live(s);
    case 'players':
      if (!s) return;
      s.players = m.players; live(s);
      if (m.joined && !(cur.page === 'server' && cur.id === s.id && cur.tab === 'console')) toast(`${m.player} зашёл на «${s.name}»`, 'info', 3000);
      return;
    case 'log': if (cur.page === 'server' && cur.id === m.id && cur.tab === 'console') appendLog(m.line); return;
    case 'backups': if (cur.page === 'server' && cur.id === m.id && cur.tab === 'backups') tBackups(srvById(m.id)); return;
    case 'sys': S.sys = m; { const e = $('#sys-mini'); if (e) e.innerHTML = sysMini(); } if (cur.page === 'dash') dashTotals(); return;
    case 'settings': return call('settings.get').then((v) => { S.settings = v; applyTheme(); });
    case 'sets': if (cur.page === 'sets') route(); return;
    case 'quitting': toast('Остановка серверов перед выходом…', 'info', 60000); return;
  }
}

// ---------- задачи ----------
const jobEls = {};
function showJob(j) {
  let el = jobEls[j.id];
  if (!el) { el = jobEls[j.id] = document.createElement('div'); $('#jobs').append(el); }
  el.className = 'job ' + j.status;
  const icon = j.status === 'running' ? ic('loader', 'spin') : j.status === 'done' ? ic('checkCircle') : ic('alert');
  el.innerHTML = `<div class="top">${icon}<b title="${esc(j.title)}">${esc(j.title)}</b>${j.status === 'running' ? `<span class="small dim">${j.progress}%</span>` : `<button class="ghost icon sm" title="Скрыть">${ic('x')}</button>`}</div>
    <div class="bar"><i style="width:${j.progress}%"></i></div><div class="l" title="${esc(j.log.slice(-1)[0] || '')}">${esc(j.log.slice(-1)[0] || 'Подготовка…')}</div>`;
  const rm = () => { el.remove(); delete jobEls[j.id]; };
  const b = $('button', el); if (b) b.onclick = rm;
  if (j.status === 'done') { setTimeout(rm, 6000); loadServers().then(() => { renderNav(); if (cur.page === 'dash') route(); else if (cur.page === 'server') { const s = srvById(cur.id); if (s) live(s); } }); }
  if (j.status === 'error') toast(`${j.title}\n${j.log.slice(-1)[0] || ''}`, 'error');
}

// ---------- управление ----------
function ctrlButtons(s, small = false) {
  const on = s.status !== 'stopped', c = small ? 'sm' : '';
  if (!s.installed) return `<span class="badge yellow">${ic('loader', 'spin')}Установка…</span>`;
  return (on
    ? `<button class="${c}" ${s.status !== 'running' ? 'disabled' : ''} onclick="act(event,'${s.id}','restart')">${ic('restart')}Перезапуск</button>
       <button class="${c}" onclick="act(event,'${s.id}','stop')">${ic('stop')}Стоп</button>
       <button class="${c} danger icon ${small ? 'sm' : ''}" title="Убить процесс без сохранения" onclick="killSrv(event,'${s.id}')">${ic('skull')}</button>`
    : `<button class="primary ${c}" onclick="act(event,'${s.id}','start')">${ic('play')}Запустить</button>`);
}
async function act(e, id, a) {
  e?.stopPropagation?.(); e?.preventDefault?.();
  const b = e?.currentTarget; if (b) b.disabled = true;
  try { await api('servers.' + a, { id }); } catch {} finally { if (b) b.disabled = false; }
}
async function killSrv(e, id) {
  e.stopPropagation(); e.preventDefault();
  if (await confirmDialog('Процесс будет завершён немедленно, несохранённые изменения мира потеряются.', { title: 'Убить процесс?', ok: 'Убить', danger: true })) api('servers.kill', { id });
}

// ---------- обзор ----------
function pDash() {
  setTop({ title: 'Обзор', actions: S.servers.length ? `<button onclick="importDir('servers')">${ic('import')}Импорт</button><button class="primary" onclick="go('create')">${ic('plus')}Новый сервер</button>` : '' });
  if (!S.servers.length) return setPage(`<div class="card hero-empty empty"><div class="logo">${ic('cube')}</div><h3>Добро пожаловать в MC Admin Panel</h3>
    <p>Создайте первый сервер — панель сама скачает ядро и нужную версию Java.<br>Или подключите серверы, которые у вас уже есть.</p>
    <div class="row" style="justify-content:center"><button class="primary" onclick="go('create')">${ic('plus')}Создать сервер</button><button onclick="importDir('servers')">${ic('import')}Импортировать существующие</button></div></div>`);
  setPage(`<div class="grid g4" id="totals" style="margin-bottom:18px">${totalsHtml()}</div>
    <div class="auto-grid">${S.servers.map((s) => `<div class="card srv-card" id="card-${s.id}">${srvCardInner(s)}</div>`).join('')}</div>`);
}
function totalsHtml() {
  const run = S.servers.filter((s) => s.status === 'running'), players = S.servers.reduce((a, s) => a + s.players.length, 0);
  const sys = S.sys || { cpu: 0, memUsed: 0, memTotal: S.info.memTotal || 1 };
  const srvMem = S.servers.reduce((a, s) => a + (s.stats?.mem || 0), 0);
  const mp = (sys.memUsed / sys.memTotal) * 100;
  return `<div class="metric"><div class="k">${ic('server')}Серверы</div><div class="v">${run.length}<small>из ${S.servers.length} работают</small></div></div>
    <div class="metric"><div class="k">${ic('users')}Игроки онлайн</div><div class="v">${players}</div></div>
    <div class="metric"><div class="k">${ic('cpu')}Процессор</div><div class="v">${Math.round(sys.cpu)}<small>%</small></div><div class="bar ${sys.cpu > 85 ? 'bad' : sys.cpu > 65 ? 'warn' : ''}" style="margin-top:8px"><i style="width:${sys.cpu}%"></i></div></div>
    <div class="metric"><div class="k">${ic('memory')}Память</div><div class="v">${(sys.memUsed / 1073741824).toFixed(1).replace('.', ',')}<small>из ${(sys.memTotal / 1073741824).toFixed(0)} ГБ · серверы ${fmtBytes(srvMem)}</small></div><div class="bar ${mp > 90 ? 'bad' : mp > 75 ? 'warn' : ''}" style="margin-top:8px"><i style="width:${mp}%"></i></div></div>`;
}
const dashTotals = () => { const t = $('#totals'); if (t) t.innerHTML = totalsHtml(); };
function srvCardInner(s) {
  const st = STATUS[s.status], on = s.status !== 'stopped';
  const memMb = s.stats ? Math.round(s.stats.mem / 1048576) : 0;
  return `<div class="head"><div class="srv-icon ${s.status}">${ic(LOADERS[s.loader]?.icon || 'server')}</div>
      <div class="grow"><a class="name" href="#server/${s.id}/console">${esc(s.name)}</a><div class="small dim">${loaderName(s.loader)} ${esc(s.mcVersion)} · порт ${s.port}</div></div>
      <span class="badge ${st[1]}">${st[0]}</span></div>
    <div class="stats3"><div class="mini-stat"><div class="k">CPU</div><div class="v">${on && s.stats ? s.stats.cpu + '%' : '—'}</div></div>
      <div class="mini-stat"><div class="k">Память</div><div class="v">${on && s.stats ? fmtMb(memMb) : '—'}<span class="dim small"> / ${fmtMb(s.ram)}</span></div></div>
      <div class="mini-stat"><div class="k">Аптайм</div><div class="v">${on && s.started ? fmtDur(Date.now() - s.started) : '—'}</div></div></div>
    ${dualChart(on ? s.history : [], s.ram)}
    <div class="players-line small">${on && s.players.length ? s.players.slice(0, 8).map((p) => `<img class="head-img" title="${esc(p)}" src="https://mc-heads.net/avatar/${encodeURIComponent(p)}/22" onerror="this.style.visibility='hidden'">`).join('') + `<span class="dim">&nbsp;${s.players.length} ${plural(s.players.length, 'игрок', 'игрока', 'игроков')}</span>`
      : `<span class="dim ellipsis">${!s.installed ? 'Идёт установка ядра и Java…' : on ? 'Нет игроков онлайн' : ['Сервер остановлен', s.autoStart && 'автозапуск', s.autoBackupMin && `автобэкап каждые ${s.autoBackupMin} мин`].filter(Boolean).join(' · ')}</span>`}</div>
    <div class="row card-actions">${ctrlButtons(s, true)}<span class="grow"></span><button class="sm ghost" onclick="go('server/${s.id}/console')">${ic('terminal')}Консоль</button></div>`;
}

// ---------- создание сервера ----------
const C = { loader: 'paper', snap: false, dirTouched: false };
async function pCreate() {
  setTop({ title: 'Новый сервер', crumbs: [['Обзор', 'dash'], ['Новый сервер']] });
  const total = S.info.memTotal ? Math.floor(S.info.memTotal / 1048576) : 16384;
  const maxRam = Math.max(2048, Math.min(32768, Math.floor((total - 2048) / 512) * 512));
  const ram = Math.min(S.settings.defaultRamMb || 4096, maxRam);
  C.dirTouched = false;
  setPage(`<div class="stack" style="max-width:1000px">
    <div class="card"><div class="card-head"><h3>${ic('box')}Ядро сервера</h3></div>
      <div class="loader-cards">${Object.entries(LOADERS).map(([k, v]) => `<div class="loader-card ${k === C.loader ? 'on' : ''}" data-l="${k}" onclick="pickLoader('${k}')" tabindex="0">${ic(v.icon)}<b>${v.name}</b><span>${v.desc}</span></div>`).join('')}</div></div>
    <div class="card"><div class="card-head"><h3>${ic('layers')}Версия</h3><label class="switch small">${sw('f-snap', C.snap, 'onchange="C.snap=this.checked;loadVers()"').replace(/^<label class="switch">|<\/label>$/g, '')}<span>Снапшоты</span></label></div>
      <div class="form-grid"><div class="field"><label>Minecraft</label><select id="f-ver" onchange="loadLoaderVers()"><option>Загрузка…</option></select></div>
      <div class="field"><label>Версия загрузчика</label><select id="f-lver" disabled><option value="">—</option></select><span class="hint" id="f-lhint"></span></div></div></div>
    <div class="card"><div class="card-head"><h3>${ic('sliders')}Параметры</h3></div>
      <div class="form-grid">
        <div class="field"><label>Название</label><input id="f-name" value="Мой сервер" oninput="updDir()"></div>
        <div class="field"><label>Порт</label><input id="f-port" type="number" min="1" max="65535" value="${nextPort()}"></div>
        <div class="field full"><label>Память</label><div class="ram-row"><input type="range" id="f-ram" min="1024" max="${maxRam}" step="512" value="${ram}" oninput="$('#f-ramv').textContent=fmtMb(+this.value)"><span class="val" id="f-ramv">${fmtMb(ram)}</span></div>
          <span class="hint">Установлено в системе: ${fmtMb(total)}. Для ванильного сервера достаточно 2–4 ГБ, для сборок с модами — 6–10 ГБ.</span></div>
        <div class="field full"><label>Папка установки</label><div class="input-row"><input id="f-dir" oninput="C.dirTouched=true"><button onclick="browse('#f-dir')">${ic('folder')}Обзор…</button></div></div>
      </div></div>
    <div class="row"><button class="primary" id="f-go" onclick="doCreate()" style="height:40px;padding:0 22px">${ic('download')}Создать и установить</button>
      <span class="small dim">Панель скачает ядро, нужную версию Java, примет EULA и создаст server.properties.</span></div></div>`);
  updDir(); await loadVers();
}
const nextPort = () => { let p = 25565; while (S.servers.some((s) => s.port === p)) p++; return p; };
function pickLoader(l) { C.loader = l; $$('.loader-card').forEach((c) => c.classList.toggle('on', c.dataset.l === l)); loadLoaderVers(); }
function updDir() {
  if (C.dirTouched) return;
  const base = (S.settings.serversDir || '').replace(/[\\/]$/, ''), sep = base.includes('\\') ? '\\' : '/';
  $('#f-dir').value = base + sep + ($('#f-name').value.replace(/[^\p{L}\p{N} ._-]/gu, '_').trim() || 'server');
}
async function loadVers() {
  const sel = $('#f-ver'); if (!sel) return;
  try {
    const v = await api('versions', { snapshots: C.snap });
    sel.innerHTML = v.map((x) => `<option value="${esc(x.id)}">${esc(x.id)}${x.type !== 'release' ? ' (снапшот)' : ''}</option>`).join('');
  } catch { sel.innerHTML = '<option value="">Нет связи с Mojang</option>'; }
  loadLoaderVers();
}
async function loadLoaderVers() {
  const l = C.loader, sel = $('#f-lver'), mc = $('#f-ver')?.value, hint = $('#f-lhint');
  if (!sel) return;
  if (l === 'vanilla') { sel.innerHTML = '<option value="">Не требуется</option>'; sel.disabled = true; hint.textContent = ''; return; }
  sel.disabled = true; sel.innerHTML = '<option>Загрузка…</option>';
  try {
    const v = await call('loaderVersions', { loader: l, mc });
    if (C.loader !== l || $('#f-ver')?.value !== mc) return;
    sel.innerHTML = v.length ? v.map((x, i) => `<option value="${esc(x)}">${esc(x)}${i === 0 ? ' — последняя' : ''}</option>`).join('') : '<option value="">Нет версий для этой версии MC</option>';
    sel.disabled = !v.length;
    hint.textContent = v.length ? `Доступно версий: ${v.length}` : `${loaderName(l)} не поддерживает Minecraft ${mc}`;
  } catch (e) { sel.innerHTML = '<option value="">Ошибка загрузки</option>'; hint.textContent = e.message; }
}
async function doCreate() {
  const l = C.loader;
  if (l !== 'vanilla' && !$('#f-lver').value) return toast('Выберите версию загрузчика', 'error');
  $('#f-go').disabled = true;
  try {
    const r = await api('servers.create', { name: $('#f-name').value, mcVersion: $('#f-ver').value, loader: l, loaderVersion: $('#f-lver').value, dir: $('#f-dir').value, port: +$('#f-port').value, ram: +$('#f-ram').value });
    await loadServers(); go(`server/${r.srv.id}/console`);
  } catch { $('#f-go').disabled = false; }
}
async function browse(sel) { const p = await native.pickFolder($(sel).value); if (p) { $(sel).value = p; $(sel).dispatchEvent(new Event('input')); } }

// ---------- страница сервера ----------
const TABS = { console: ['Консоль', 'terminal'], mods: ['Моды', 'package'], backups: ['Бэкапы', 'archive'], props: ['Настройки мира', 'sliders'], players: ['Игроки', 'users'], config: ['Параметры', 'wrench'] };
function srvMetrics(s) {
  const memMb = s.stats ? Math.round(s.stats.mem / 1048576) : 0, h = s.history || [];
  return `<div class="metric"><div class="k">${ic('cpu')}CPU</div><div class="v">${s.stats ? s.stats.cpu : '—'}<small>${s.stats ? '%' : ''}</small></div>${sparkline(h.map((x) => x[1]), 'var(--chart-1)', 100)}</div>
    <div class="metric"><div class="k">${ic('memory')}Память</div><div class="v">${s.stats ? fmtMb(memMb) : '—'}<small>из ${fmtMb(s.ram)}</small></div>${sparkline(h.map((x) => x[2]), 'var(--chart-2)', s.ram)}</div>
    <div class="metric"><div class="k">${ic('users')}Игроки</div><div class="v">${s.players.length}</div>${sparkline(h.map((x) => x[3]), 'var(--chart-3)')}</div>
    <div class="metric"><div class="k">${ic('clock')}Аптайм</div><div class="v" style="font-size:20px">${s.status !== 'stopped' && s.started ? fmtDur(Date.now() - s.started) : '—'}</div></div>`;
}
async function pServer(id, tab = 'console') {
  const s = srvById(id); if (!s) return go('dash');
  if (!TABS[tab]) tab = 'console';
  setTop({ title: s.name, status: s.status, crumbs: [['Обзор', 'dash'], [s.name]],
    sub: `<span class="badge">${ic(LOADERS[s.loader]?.icon || 'cube')}${loaderName(s.loader)} ${esc(s.mcVersion)}</span><span class="badge">${ic('wifi')}:${s.port}</span>`,
    actions: ctrlButtons(s) });
  // заголовок и метрики не перерисовываем при переключении вкладок
  const keep = $('#srv-shell') && $('#srv-shell').dataset.id === id;
  if (!keep) setPage(`<div id="srv-shell" data-id="${id}" style="display:flex;flex-direction:column;flex:1;min-height:0">
      <div class="grid g4" id="srv-metrics" style="margin-bottom:16px">${srvMetrics(s)}</div>
      <div class="tabs" id="srv-tabs"></div><div id="tab" style="display:flex;flex-direction:column;flex:1;min-height:0"></div></div>`, tab === 'console');
  $('#page').className = 'page' + (tab === 'console' ? ' fill' : '');
  $('#srv-tabs').innerHTML = Object.entries(TABS).map(([k, v]) => `<a class="tab ${k === tab ? 'on' : ''}" href="#server/${id}/${k}">${ic(v[1])}${v[0]}</a>`).join('') +
    `<span class="grow"></span><button class="ghost sm" style="align-self:center" onclick="native.openPath(${attr(s.dir)})">${ic('folder')}Папка сервера</button>`;
  $('#tab').innerHTML = '';
  await { console: tConsole, mods: tMods, backups: tBackups, props: tProps, players: tPlayers, config: tConfig }[tab](s);
}

// --- консоль ---
const DEFAULT_QC = [
  ['Мир', [['День', 'time set day', 'sun'], ['Ночь', 'time set night', 'moon'], ['Ясно', 'weather clear', 'cloud'], ['Дождь', 'weather rain', 'rain'], ['Сохранить', 'save-all', 'save'], ['Список игроков', 'list', 'list'], ['keepInventory', 'gamerule keepInventory true', 'shield']]],
  ['Игрок', [['Креатив', 'gamemode creative {p}', 'grid'], ['Выживание', 'gamemode survival {p}', 'pickaxe'], ['Наблюдатель', 'gamemode spectator {p}', 'eye'], ['OP', 'op {p}', 'zap'], ['DeOP', 'deop {p}', 'x'], ['Вылечить', 'effect give {p} minecraft:instant_health 1 10', 'heart'], ['В вайтлист', 'whitelist add {p}', 'check'], ['Кик', 'kick {p} {Причина}', 'logout'], ['Бан', 'ban {p} {Причина}', 'ban'], ['Разбан', 'pardon {p}', 'undo']]],
  ['Чат', [['Объявление', 'say {Сообщение}', 'megaphone'], ['Титр всем', 'title @a title {"text":"{Текст}"}', 'text']]],
];
const CMD_NAMES = [...new Set(COMMANDS.flatMap((c) => c[1].split(' / ').map((x) => x.trim().split(' ')[0])))].sort();
const LOG = { lines: [], filter: '', follow: true };
const histKey = (id) => 'mcpanel-hist-' + id;
const getHist = (id) => { try { return JSON.parse(localStorage.getItem(histKey(id)) || '[]'); } catch { return []; } };
let hi = 0;
async function tConsole(s) {
  const custom = S.settings.quickCommands || [];
  const btn = ([l, c, icon = 'terminal']) => `<button title="${esc(c)}" onclick="qc(${attr(c)})">${ic(icon)}${esc(l)}</button>`;
  $('#tab').innerHTML = `<div class="console-wrap"><div class="console">
      <div class="console-bar"><div class="input-icon grow" style="max-width:280px">${ic('filter')}<input id="log-filter" placeholder="Фильтр строк…" value="${esc(LOG.filter)}" oninput="LOG.filter=this.value;renderLog()"></div>
        <span class="grow"></span>
        <button id="follow-btn" class="${LOG.follow ? 'on' : ''}" title="Автопрокрутка" onclick="setFollow(!LOG.follow)">${ic('arrowDown')}Следить</button>
        <button title="Копировать журнал" onclick="native.copy(LOG.lines.join('\\n'))">${ic('copy')}</button>
        <button title="Очистить экран" onclick="LOG.lines=[];renderLog()">${ic('trash')}</button></div>
      <div class="log" id="log"></div>
      <div class="cmdline" style="position:relative"><span class="pr">&gt;</span><input id="cmd" autocomplete="off" spellcheck="false" placeholder="${s.status === 'stopped' ? 'Сервер остановлен' : 'Команда без «/» — Enter, ↑↓ история, Tab дополнение'}" onkeydown="cmdKey(event)" oninput="suggest()">
        <button class="primary sm" onclick="send()">${ic('send')}</button><div class="suggest" id="sug" hidden></div></div></div>
    <div class="card qc-panel"><div><div class="t">Игрок для команд</div><input id="qp" list="plist" placeholder="Ник игрока"><datalist id="plist">${s.players.map((p) => `<option value="${esc(p)}">`).join('')}</datalist></div>
      ${DEFAULT_QC.map(([t, b]) => `<div><div class="t">${t}</div><div class="g">${b.map(btn).join('')}</div></div>`).join('')}
      <div><div class="t">Свои команды</div><div class="g">${custom.map((c, i) => `<span class="btn-group">${btn([c.label, c.cmd])}<button class="icon sm" title="Удалить" onclick="delQc(${i})">${ic('x')}</button></span>`).join('')}<button class="ghost" onclick="addQc()">${ic('plus')}Добавить</button></div></div>
      <p class="small dim" style="margin:0">{p} — выбранный игрок, {Текст} — запрос значения. <a href="#cmds">Справка по командам</a></p></div></div>`;
  LOG.lines = await call('servers.log', { id: s.id }).catch(() => []);
  hi = getHist(s.id).length;
  renderLog();
  const el = $('#log');
  el.addEventListener('scroll', () => { const atEnd = el.scrollTop + el.clientHeight >= el.scrollHeight - 30; if (atEnd !== LOG.follow) setFollow(atEnd, false); });
  $('#cmd').focus();
}
function setFollow(v, scroll = true) { LOG.follow = v; $('#follow-btn')?.classList.toggle('on', v); if (v && scroll) { const el = $('#log'); if (el) el.scrollTop = el.scrollHeight; } }
function lineClass(l) { return /WARN/.test(l) ? 'w' : /ERROR|Exception|FATAL|SEVERE/.test(l) ? 'e' : l.startsWith('> ') ? 'c' : l.startsWith('[Панель]') ? 'p' : /joined the game|left the game/.test(l) ? 'j' : ''; }
function lineHtml(l) {
  let h = esc(l);
  if (LOG.filter) { const f = esc(LOG.filter).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); h = h.replace(new RegExp(f, 'gi'), (m) => `<span class="hl">${m}</span>`); }
  return h.replace(/^(\[\d{2}:\d{2}:\d{2}[^\]]*\])/, '<span class="ts">$1</span>');
}
function renderLog() {
  const el = $('#log'); if (!el) return;
  const f = LOG.filter.toLowerCase();
  const lines = f ? LOG.lines.filter((l) => l.toLowerCase().includes(f)) : LOG.lines;
  el.innerHTML = lines.slice(-2500).map((l) => `<div class="${lineClass(l)}">${lineHtml(l)}</div>`).join('');
  if (LOG.follow) el.scrollTop = el.scrollHeight;
}
function appendLog(line) {
  LOG.lines.push(line); if (LOG.lines.length > 3000) LOG.lines.shift();
  const el = $('#log'); if (!el) return;
  if (LOG.filter && !line.toLowerCase().includes(LOG.filter.toLowerCase())) return;
  const d = document.createElement('div'); d.className = lineClass(line); d.innerHTML = lineHtml(line);
  el.append(d); if (el.childElementCount > 2500) el.firstChild.remove();
  if (LOG.follow) el.scrollTop = el.scrollHeight;
}
function send(c) {
  c = (c ?? $('#cmd').value).trim(); if (!c) return;
  const h = getHist(cur.id).filter((x) => x !== c); h.push(c);
  try { localStorage.setItem(histKey(cur.id), JSON.stringify(h.slice(-100))); } catch {}
  hi = h.length; if ($('#cmd')) $('#cmd').value = ''; hideSug();
  setFollow(true);
  api('servers.command', { id: cur.id, cmd: c }).catch(() => {});
}
let sugIdx = 0;
function suggest() {
  const v = $('#cmd').value, box = $('#sug');
  if (!v || v.includes(' ')) return hideSug();
  const m = CMD_NAMES.filter((c) => c.startsWith(v.replace(/^\//, '').toLowerCase())).slice(0, 8);
  if (!m.length || (m.length === 1 && m[0] === v)) return hideSug();
  sugIdx = 0; box.hidden = false;
  box.innerHTML = m.map((c, i) => { const d = COMMANDS.find((x) => x[1].split(' ')[0] === c); return `<div class="${i === 0 ? 'on' : ''}" onmousedown="event.preventDefault();pickSug('${c}')"><code>${c}</code><span class="dim small ellipsis">${esc(d?.[2] || '')}</span></div>`; }).join('');
}
const hideSug = () => { const b = $('#sug'); if (b) b.hidden = true; };
function pickSug(c) { $('#cmd').value = c + ' '; hideSug(); $('#cmd').focus(); }
function cmdKey(e) {
  const box = $('#sug'), open = box && !box.hidden, items = open ? $$('div', box) : [];
  if (open && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
    e.preventDefault(); sugIdx = (sugIdx + (e.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
    items.forEach((x, i) => x.classList.toggle('on', i === sugIdx)); return;
  }
  if (open && e.key === 'Tab') { e.preventDefault(); pickSug(items[sugIdx].querySelector('code').textContent); return; }
  if (e.key === 'Escape') return hideSug();
  if (e.key === 'Enter') return send();
  const h = getHist(cur.id);
  if (e.key === 'ArrowUp' && hi > 0) { e.preventDefault(); $('#cmd').value = h[--hi]; }
  else if (e.key === 'ArrowDown') { e.preventDefault(); hi = Math.min(h.length, hi + 1); $('#cmd').value = h[hi] ?? ''; }
}
async function qc(c) {
  if (c.includes('{p}')) { const p = $('#qp').value.trim(); if (!p) { $('#qp').focus(); return toast('Укажите ник игрока', 'error'); } c = c.replaceAll('{p}', p); }
  for (const m of c.match(/\{[А-Яа-яЁё ]+\}/g) || []) {
    const v = await promptDialog({ title: m.slice(1, -1), label: m.slice(1, -1) });
    if (v == null) return; c = c.replace(m, v);
  }
  send(c);
}
async function addQc() {
  const r = await formDialog({ title: 'Своя кнопка', icon: 'plus', ok: 'Добавить', fields: [{ key: 'label', label: 'Название', placeholder: 'Например: Телепорт на спавн' }, { key: 'cmd', label: 'Команда', placeholder: 'tp {p} 0 64 0', hint: '{p} — выбранный игрок, {Текст} — запрос значения при нажатии' }] });
  if (!r || !r.label || !r.cmd) return;
  S.settings = await api('settings.save', { quickCommands: [...(S.settings.quickCommands || []), r] });
  tConsole(srvById(cur.id));
}
async function delQc(i) { const q = [...S.settings.quickCommands]; q.splice(i, 1); S.settings = await api('settings.save', { quickCommands: q }); tConsole(srvById(cur.id)); }

// --- моды сервера ---
async function tMods(s) {
  if (s.loader === 'vanilla') return ($('#tab').innerHTML = `<div class="card empty">${ic('package')}<h3>Моды не поддерживаются</h3><p>Ванильный сервер не умеет загружать моды.<br>Создайте сервер на Fabric, Forge, NeoForge или Paper.</p></div>`);
  const { dir, list } = await api('mods.list', { id: s.id });
  const plugin = s.loader === 'paper', what = plugin ? 'Плагины' : 'Моды';
  $('#tab').innerHTML = `<div class="grid g2" style="align-items:start"><div class="card drop-zone" data-drop="mods">
      <div class="card-head"><h3>${what} <span class="count">${list.length}</span></h3>
        <button class="sm" onclick="pickJars('mods')">${ic('upload')}Добавить .jar</button><button class="sm ghost icon" title="Открыть папку" onclick="native.openPath(${attr(dir)})">${ic('folder')}</button></div>
      ${list.length > 6 ? `<div class="input-icon" style="margin-bottom:8px">${ic('search')}<input placeholder="Фильтр…" oninput="$$('#modlist .list-item').forEach(r=>r.hidden=!r.dataset.f.includes(this.value.toLowerCase()))"></div>` : ''}
      <div class="scroll" id="modlist" style="max-height:60vh">${modList(list, 'mods', s.id)}</div>
      <p class="small dim" style="margin:10px 0 0">Перетащите .jar-файлы в окно. Изменения вступают в силу после перезапуска сервера.</p></div>
    <div class="card">${searchBox('mods', s.id, s.mcVersion, s.loader, plugin ? 'plugin' : 'mod')}</div></div>`;
  doSearch();
}
function modList(list, kind, id) {
  if (!list.length) return `<div class="empty">${ic('package')}<p>Пока пусто — найдите что-нибудь в каталоге или перетащите .jar сюда</p></div>`;
  return list.map((m) => `<div class="list-item ${m.enabled ? '' : 'off'}" data-f="${esc(m.file.toLowerCase())}"><div class="ph">${ic('package')}</div>
    <div class="grow"><div class="t ellipsis" title="${esc(m.file)}">${esc(m.file.replace(/\.disabled$/, '').replace(/\.jar$/, ''))}</div><div class="d">${fmtBytes(m.size)}${m.enabled ? '' : ' · выключен'}</div></div>
    ${sw('', m.enabled, `onchange='modToggle(${attr(kind)},${attr(id)},${attr(m.file)})' title="Включить/выключить"`)}
    <button class="ghost icon sm" title="Удалить" onclick='modDelete(${attr(kind)},${attr(id)},${attr(m.file)})'>${ic('trash')}</button></div>`).join('');
}
async function modToggle(kind, id, file) { await api(kind === 'mods' ? 'mods.toggle' : 'sets.toggle', { id, file }).catch(() => {}); refreshMods(); }
async function modDelete(kind, id, file) {
  if (!(await confirmDialog(`Удалить ${file}?`, { title: 'Удаление', ok: 'Удалить', danger: true }))) return;
  await api(kind === 'mods' ? 'mods.delete' : 'sets.deleteMod', { id, file }); refreshMods();
}
const refreshMods = () => (cur.page === 'server' ? tMods(srvById(cur.id)) : route());
async function pickJars(kind) {
  const files = await native.pickJars(); if (!files?.length) return;
  await addJars(kind, kind === 'mods' ? cur.id : cur.setId, files).catch(() => {}); refreshMods();
}
let searchCtx;
function searchBox(kind, id, mc, loader, type) {
  searchCtx = { kind, id, mc, loader, type, offset: 0 };
  return `<div class="card-head"><h3>${ic('globe')}Каталог Modrinth</h3></div>
    <div class="input-row"><div class="input-icon grow">${ic('search')}<input id="q" placeholder="${type === 'plugin' ? 'worldedit, luckperms, essentials…' : 'sodium, jei, create…'}" oninput="debSearch()" onkeydown="event.key==='Enter'&&doSearch()"></div></div>
    <p class="small dim" style="margin:8px 0">Фильтр: ${esc(loaderName(loader))} ${esc(mc)} · обязательные зависимости ставятся автоматически</p>
    <div id="res" class="scroll" style="max-height:60vh"></div>`;
}
const debSearch = debounce(() => doSearch(), 350);
async function doSearch(more = false) {
  const c = searchCtx, res = $('#res'); if (!res) return;
  if (!more) { c.offset = 0; res.innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`; }
  try {
    const r = await call('modrinth.search', { q: $('#q').value, mc: c.mc, loader: c.loader, type: c.type, offset: c.offset });
    const html = r.hits.map((m) => `<div class="list-item">${m.icon ? `<img src="${esc(m.icon)}" loading="lazy" alt="">` : `<div class="ph">${ic('package')}</div>`}
      <div class="grow"><div class="row nowrap"><span class="t ellipsis">${esc(m.title)}</span>
        ${m.client === 'unsupported' ? '<span class="badge blue">сервер</span>' : m.server === 'unsupported' ? '<span class="badge yellow">клиент</span>' : ''}</div>
        <div class="d">${esc(m.desc)}</div><div class="small dim">${ic('download')} ${fmtNum(m.downloads)} · ${esc(m.author || '')}</div></div>
      <button class="sm" onclick="installMod(this,'${esc(m.id)}')">${ic('download')}Установить</button></div>`).join('');
    $('#more-btn')?.remove();
    if (more) res.insertAdjacentHTML('beforeend', html); else res.innerHTML = html || `<div class="empty">${ic('search')}<p>Ничего не найдено</p></div>`;
    c.offset += r.hits.length;
    if (c.offset < r.total && r.hits.length) res.insertAdjacentHTML('beforeend', `<div id="more-btn" style="text-align:center;padding:10px"><button class="sm" onclick="doSearch(true)">Показать ещё</button></div>`);
  } catch (e) { res.innerHTML = `<div class="empty">${ic('alert')}<p>${esc(e.message)}</p></div>`; }
}
async function installMod(b, pid) {
  b.disabled = true; b.innerHTML = ic('loader', 'spin');
  const c = searchCtx;
  try {
    const f = await call(c.kind === 'mods' ? 'mods.install' : 'sets.install', { id: c.id, projectId: pid });
    b.innerHTML = ic('check') + 'Готово'; b.title = f.join('\n');
    toast(`Установлено: ${f[0]}${f.length > 1 ? `\n+ зависимости: ${f.slice(1).join(', ')}` : ''}`);
    const list = await call(c.kind === 'mods' ? 'mods.list' : 'sets.list', { id: c.id });
    const ml = $('#modlist');
    if (ml) ml.innerHTML = modList(c.kind === 'mods' ? list.list : list.find((x) => x.id === c.id).mods, c.kind, c.id);
  } catch (e) { b.disabled = false; b.innerHTML = ic('alert') + 'Ошибка'; toast(e.message, 'error'); }
}

// --- бэкапы ---
async function tBackups(s) {
  const { dir, list, max } = await api('backups.list', { id: s.id });
  const total = list.reduce((a, b) => a + b.size, 0);
  $('#tab').innerHTML = `<div class="stack"><div class="card">
      <div class="card-head"><h3>${ic('archive')}Резервные копии <span class="count">${list.length} из ${max}</span></h3>
        <span class="small dim">${fmtBytes(total)}</span><button class="sm ghost" onclick="native.openPath(${attr(dir)})">${ic('folder')}Папка</button></div>
      <div class="row" style="margin-bottom:12px"><input id="bnote" placeholder="Заметка к бэкапу (необязательно)" style="max-width:340px"><button class="primary" onclick="mkBackup('${s.id}')">${ic('save')}Создать бэкап</button></div>
      ${list.length ? `<table><thead><tr><th>Файл</th><th>Создан</th><th>Размер</th><th></th></tr></thead><tbody>${list.map((b, i) => `<tr><td><span class="row nowrap">${ic('archive')}<span class="ellipsis">${esc(b.file)}</span>${i === 0 ? '<span class="badge green">новый</span>' : ''}</span></td>
        <td class="muted" title="${fmtDate(b.time)}">${fmtAgo(b.time)}</td><td class="muted">${fmtBytes(b.size)}</td>
        <td style="text-align:right;white-space:nowrap"><button class="sm" onclick='restore("${s.id}",${attr(b.file)})'>${ic('undo')}Восстановить</button>
        <button class="sm ghost icon" title="Удалить" onclick='delBackup("${s.id}",${attr(b.file)})'>${ic('trash')}</button></td></tr>`).join('')}</tbody></table>`
        : `<div class="empty">${ic('archive')}<h3>Бэкапов пока нет</h3><p>Работающий сервер копируется безопасно: save-off → save-all → архив → save-on.</p></div>`}</div>
    <div class="card"><div class="card-head"><h3>${ic('clock')}Автоматизация</h3></div>
      <div class="form-grid"><div class="field"><label>Автобэкап каждые (минут)</label><input id="b-auto" type="number" min="0" value="${s.autoBackupMin}"><span class="hint">0 — выключено; срабатывает только пока сервер запущен</span></div>
        <div class="field"><label>Максимум копий</label><input id="b-max" type="number" min="0" value="${s.maxBackups}"><span class="hint">0 — как в настройках (${S.settings.maxBackups}). Старые копии удаляются автоматически</span></div>
        <div class="field full"><label>Папка бэкапов этого сервера</label><div class="input-row"><input id="b-dir" value="${esc(s.backupDir)}" placeholder="По умолчанию: ${esc(S.settings.backupsDir)}"><button onclick="browse('#b-dir')">${ic('folder')}Обзор…</button></div></div></div>
      <div class="row" style="margin-top:14px"><button class="primary" onclick="saveSrv('${s.id}',{backupDir:$('#b-dir').value,maxBackups:$('#b-max').value,autoBackupMin:$('#b-auto').value})">Сохранить</button></div></div></div>`;
}
const mkBackup = (id) => api('backups.create', { id, note: $('#bnote').value }).then(() => ($('#bnote').value = '')).catch(() => {});
async function restore(id, file) {
  const s = srvById(id);
  if (s.status !== 'stopped') return toast('Остановите сервер перед восстановлением', 'error');
  if (await confirmDialog(`Текущий мир будет заменён копией\n${file}`, { title: 'Восстановить мир?', ok: 'Восстановить', danger: true })) api('backups.restore', { id, file }).catch(() => {});
}
async function delBackup(id, file) { if (await confirmDialog(`Удалить ${file}? Это действие нельзя отменить.`, { title: 'Удалить бэкап?', ok: 'Удалить', danger: true })) api('backups.delete', { id, file }).catch(() => {}); }
async function saveSrv(id, patch) {
  const s = await api('servers.update', { id, patch });
  Object.assign(srvById(id), s); toast('Сохранено');
}

// --- server.properties ---
const PROPS = {
  'motd': ['Описание в списке серверов', 'main'], 'max-players': ['Максимум игроков', 'main'], 'difficulty': ['Сложность', 'main', ['peaceful', 'easy', 'normal', 'hard'], ['Мирная', 'Лёгкая', 'Нормальная', 'Сложная']],
  'gamemode': ['Режим игры по умолчанию', 'main', ['survival', 'creative', 'adventure', 'spectator'], ['Выживание', 'Творческий', 'Приключение', 'Наблюдатель']],
  'pvp': ['PvP между игроками', 'main'], 'hardcore': ['Хардкор', 'main'], 'online-mode': ['Проверка лицензии (выкл. — вход без аккаунта Mojang)', 'main'],
  'white-list': ['Белый список', 'access'], 'enforce-whitelist': ['Кикать игроков не из белого списка', 'access'], 'spawn-protection': ['Радиус защиты спавна', 'access'], 'allow-flight': ['Разрешить полёт (без кика за читы)', 'access'], 'enable-command-block': ['Командные блоки', 'access'],
  'level-name': ['Папка мира', 'world'], 'level-seed': ['Сид мира', 'world'], 'level-type': ['Тип мира', 'world', ['minecraft:normal', 'minecraft:flat', 'minecraft:large_biomes', 'minecraft:amplified'], ['Обычный', 'Плоский', 'Большие биомы', 'Расширенный']],
  'allow-nether': ['Незер', 'world'], 'spawn-monsters': ['Монстры', 'world'], 'generate-structures': ['Генерация построек', 'world'], 'max-world-size': ['Размер мира (радиус)', 'world'],
  'view-distance': ['Дальность прорисовки (чанки)', 'perf'], 'simulation-distance': ['Дальность симуляции (чанки)', 'perf'], 'server-port': ['Порт сервера', 'perf'], 'max-tick-time': ['Макс. время тика (мс)', 'perf'],
};
const PGROUPS = { main: 'Основное', access: 'Доступ', world: 'Мир', perf: 'Сеть и производительность', other: 'Прочее' };
async function tProps(s) {
  const p = await api('servers.props', { id: s.id });
  const keys = Object.keys(p);
  if (!keys.length) return ($('#tab').innerHTML = `<div class="card empty">${ic('sliders')}<h3>server.properties ещё не создан</h3><p>Запустите сервер один раз — файл появится автоматически.</p></div>`);
  const groups = {};
  keys.forEach((k) => (groups[PROPS[k]?.[1] || 'other'] ||= []).push(k));
  const ctl = (k, v) => {
    const d = PROPS[k];
    if (v === 'true' || v === 'false') return sw('', v === 'true', `data-k="${k}" data-orig="${v}"`);
    if (d?.[2]) { const opts = d[2].includes(v) ? d[2] : [v, ...d[2]]; return `<select data-k="${k}" data-orig="${esc(v)}">${opts.map((o) => `<option value="${esc(o)}" ${o === v ? 'selected' : ''}>${esc(d[3]?.[d[2].indexOf(o)] || o)}</option>`).join('')}</select>`; }
    return `<input data-k="${k}" data-orig="${esc(v)}" value="${esc(v)}">`;
  };
  $('#tab').innerHTML = `<div class="row" style="margin-bottom:14px"><div class="input-icon" style="width:300px">${ic('search')}<input placeholder="Поиск параметра…" oninput="filterProps(this.value)"></div>
      <span class="grow"></span><span class="small dim" id="pchg"></span><button class="primary" id="psave" disabled onclick="saveProps('${s.id}')">${ic('save')}Сохранить</button></div>
    ${Object.entries(PGROUPS).filter(([g]) => groups[g]).map(([g, t]) => `<div class="pgroup"><div class="set-group-title">${t}</div><div class="card" style="padding:4px 8px"><table><tbody>
      ${groups[g].sort().map((k) => `<tr class="prop-row" data-s="${esc((k + ' ' + (PROPS[k]?.[0] || '')).toLowerCase())}"><td><div class="k">${esc(k)}</div><div class="h">${esc(PROPS[k]?.[0] || '')}</div></td><td>${ctl(k, p[k])}</td></tr>`).join('')}
    </tbody></table></div></div>`).join('')}
    <p class="small dim">Большинство параметров применяются после перезапуска сервера. Комментарии и порядок строк в файле сохраняются.</p>`;
  $('#tab').addEventListener('input', propsChanged); $('#tab').addEventListener('change', propsChanged);
}
function propVal(e) { return e.type === 'checkbox' ? String(e.checked) : e.value; }
function propsChanged() {
  const ch = $$('[data-k]').filter((e) => propVal(e) !== e.dataset.orig);
  $$('.prop-row').forEach((r) => r.classList.toggle('changed', ch.some((e) => r.contains(e))));
  const b = $('#psave'); if (b) b.disabled = !ch.length;
  const c = $('#pchg'); if (c) c.textContent = ch.length ? `Изменено: ${ch.length}` : '';
}
function filterProps(q) {
  q = q.toLowerCase();
  $$('.prop-row').forEach((r) => (r.hidden = !r.dataset.s.includes(q)));
  $$('.pgroup').forEach((g) => (g.hidden = !$$('.prop-row', g).some((r) => !r.hidden)));
}
async function saveProps(id) {
  const o = {};
  $$('[data-k]').filter((e) => propVal(e) !== e.dataset.orig).forEach((e) => (o[e.dataset.k] = propVal(e)));
  await api('servers.saveProps', { id, props: o });
  $$('[data-k]').forEach((e) => (e.dataset.orig = propVal(e))); propsChanged();
  toast(srvById(id).status !== 'stopped' ? 'Сохранено\nПерезапустите сервер, чтобы применить изменения' : 'Сохранено');
}

// --- игроки ---
function onlineList(s) {
  const on = s.status === 'running';
  return s.players.length ? s.players.map((p) => `<div class="list-item"><img src="https://mc-heads.net/avatar/${encodeURIComponent(p)}/40" style="image-rendering:pixelated" onerror="this.style.visibility='hidden'">
    <div class="grow t">${esc(p)}</div>${[['OP', 'op', 'zap'], ['Кик', 'kick', 'logout'], ['Бан', 'ban', 'ban']].map(([t, c, i]) => `<button class="sm ${c === 'ban' ? 'danger' : ''}" ${on ? '' : 'disabled'} onclick='pc(${attr(c + ' ' + p)})'>${ic(i)}${t}</button>`).join('')}</div>`).join('')
    : `<div class="empty" style="padding:24px">${ic('users')}<p>${on ? 'Никого нет онлайн' : 'Сервер остановлен'}</p></div>`;
}
async function tPlayers(s) {
  const l = await api('servers.lists', { id: s.id });
  const names = { ops: ['Операторы', 'zap'], whitelist: ['Белый список', 'check'], 'banned-players': ['Забаненные', 'ban'], 'banned-ips': ['Забаненные IP', 'globe'] };
  const add = { ops: 'op', whitelist: 'whitelist add', 'banned-players': 'ban', 'banned-ips': 'ban-ip' };
  const rm = { ops: 'deop', whitelist: 'whitelist remove', 'banned-players': 'pardon', 'banned-ips': 'pardon-ip' };
  const on = s.status === 'running';
  $('#tab').innerHTML = `<div class="stack"><div class="card"><div class="card-head"><h3>${ic('wifi')}Онлайн <span class="count">${s.players.length}</span></h3></div><div id="online">${onlineList(s)}</div></div>
    <div class="grid g2">${Object.entries(names).map(([k, [t, i]]) => `<div class="card"><div class="card-head"><h3>${ic(i)}${t} <span class="count">${l[k].length}</span></h3>
      <button class="sm" ${on ? '' : 'disabled title="Сервер должен быть запущен"'} onclick='addToList(${attr(add[k])},${attr(t)})'>${ic('plus')}Добавить</button></div>
      ${l[k].map((e) => { const n = e.name || e.ip; return `<div class="list-item"><div class="grow"><div class="t">${esc(n)}</div>${e.reason ? `<div class="d">${esc(e.reason)}</div>` : ''}</div>${on ? `<button class="ghost icon sm" title="Убрать" onclick='pc(${attr(rm[k] + ' ' + n)})'>${ic('x')}</button>` : ''}</div>`; }).join('') || '<p class="small dim" style="margin:0">Пусто</p>'}</div>`).join('')}</div>
    ${on ? '' : `<p class="small dim">${ic('info')} Списки изменяются командами — запустите сервер, чтобы редактировать их.</p>`}</div>`;
}
async function addToList(cmd, title) {
  const n = await promptDialog({ title, label: cmd.includes('ip') ? 'IP-адрес или ник' : 'Ник игрока', ok: 'Добавить' });
  if (n) pc(cmd + ' ' + n.trim());
}
const pc = (c) => api('servers.command', { id: cur.id, cmd: c }).then(() => setTimeout(() => cur.tab === 'players' && tPlayers(srvById(cur.id)), 800)).catch(() => {});

// --- параметры ---
function tConfig(s) {
  const total = S.info.memTotal ? Math.floor(S.info.memTotal / 1048576) : 16384;
  const maxRam = Math.max(s.ram, Math.min(65536, Math.floor((total - 1024) / 512) * 512));
  $('#tab').innerHTML = `<div style="max-width:960px"><div class="set-group-title">Запуск</div><div class="set-list">
    <div class="set-item">${ic('text')}<div class="txt"><b>Название</b><span>Отображается в панели; папка бэкапов называется так же</span></div><div class="ctl"><input id="c-name" value="${esc(s.name)}"></div></div>
    <div class="set-item">${ic('memory')}<div class="txt"><b>Память</b><span>Максимальный размер кучи Java (-Xmx)</span></div><div class="ctl"><div class="ram-row" style="width:100%"><input type="range" id="c-ram" min="1024" max="${maxRam}" step="512" value="${s.ram}" oninput="$('#c-ramv').textContent=fmtMb(+this.value)"><span class="val" id="c-ramv">${fmtMb(s.ram)}</span></div></div></div>
    <div class="set-item">${ic('wifi')}<div class="txt"><b>Порт</b><span>Также меняется в server.properties</span></div><div class="ctl"><input id="c-port" type="number" value="${s.port}" style="width:120px"></div></div>
    <div class="set-item">${ic('terminal')}<div class="txt"><b>Аргументы JVM</b><span>Например: -XX:+UseG1GC -XX:MaxGCPauseMillis=200</span></div><div class="ctl"><input id="c-jvm" class="mono" value="${esc(s.jvmArgs)}" placeholder="—"></div></div>
    <div class="set-item">${ic('restart')}<div class="txt"><b>Автоперезапуск при сбое</b><span>Если сервер упал после минуты работы — поднять через 5 секунд</span></div><div class="ctl auto">${sw('c-ar', s.autoRestart)}</div></div>
    <div class="set-item">${ic('power')}<div class="txt"><b>Запускать при старте панели</b><span>Вместе с автозапуском Windows сервер поднимается сам после перезагрузки</span></div><div class="ctl auto">${sw('c-as', s.autoStart)}</div></div>
  </div><div class="row" style="margin:14px 0 4px"><button class="primary" onclick="saveConfig('${s.id}')">${ic('save')}Сохранить</button></div>
  <div class="set-group-title">Сведения</div><div class="set-list">
    <div class="set-item">${ic('folder')}<div class="txt"><b>Папка сервера</b><span class="selectable">${esc(s.dir)}</span></div><div class="ctl auto"><button class="sm" onclick="native.openPath(${attr(s.dir)})">${ic('external')}Открыть</button></div></div>
    <div class="set-item">${ic('cube')}<div class="txt"><b>Ядро</b><span>${loaderName(s.loader)} ${esc(s.mcVersion)} ${esc(s.loaderVersion)} · Java ${s.javaMajor || '—'}${s.imported ? ' · импортирован' : ''}</span></div></div>
  </div>
  <div class="set-group-title" style="color:var(--red)">Опасная зона</div><div class="set-list">
    <div class="set-item">${ic('download')}<div class="txt"><b>Переустановить ядро</b><span>Заново скачать ядро и Java. Мир, моды и настройки сохранятся</span></div><div class="ctl auto"><button class="sm" onclick="reinstall('${s.id}')">Переустановить</button></div></div>
    <div class="set-item">${ic('x')}<div class="txt"><b>Убрать из панели</b><span>Файлы сервера останутся на диске</span></div><div class="ctl auto"><button class="sm danger" onclick="delSrv('${s.id}',false)">Убрать</button></div></div>
    <div class="set-item">${ic('trash')}<div class="txt"><b>Удалить сервер</b><span>Безвозвратно удалить сервер вместе со всеми файлами и миром</span></div><div class="ctl auto"><button class="sm danger solid" onclick="delSrv('${s.id}',true)">Удалить</button></div></div>
  </div></div>`;
}
async function saveConfig(id) {
  const port = +$('#c-port').value, oldPort = srvById(id).port;
  await saveSrv(id, { name: $('#c-name').value, ram: $('#c-ram').value, jvmArgs: $('#c-jvm').value, autoRestart: $('#c-ar').checked, autoStart: $('#c-as').checked, port });
  if (port !== oldPort) await call('servers.saveProps', { id, props: { 'server-port': String(port) } }).catch(() => {});
  await loadServers(); renderNav();
  const s = srvById(id);
  setTop({ title: s.name, status: s.status, crumbs: [['Обзор', 'dash'], [s.name]], actions: ctrlButtons(s),
    sub: `<span class="badge">${ic(LOADERS[s.loader]?.icon || 'cube')}${loaderName(s.loader)} ${esc(s.mcVersion)}</span><span class="badge">${ic('wifi')}:${s.port}</span>` });
}
async function reinstall(id) { if (await confirmDialog('Ядро сервера будет скачано заново. Мир, моды и настройки сохранятся.', { title: 'Переустановить ядро?', ok: 'Переустановить' })) api('servers.reinstall', { id }).catch(() => {}); }
async function delSrv(id, files) {
  const s = srvById(id);
  if (s.status !== 'stopped') return toast('Сначала остановите сервер', 'error');
  const ok = files
    ? await confirmDialog(`Сервер «${s.name}» и ВСЕ его файлы, включая мир, будут удалены безвозвратно.\n\n${s.dir}`, { title: 'Удалить сервер?', ok: 'Удалить навсегда', danger: true })
    : await confirmDialog(`«${s.name}» исчезнет из панели, но файлы останутся на диске.`, { title: 'Убрать из панели?', ok: 'Убрать', danger: true });
  if (!ok) return;
  await api('servers.remove', { id, files }); await loadServers(); go('dash');
}

// ---------- наборы модов клиента ----------
async function pSets(id) {
  S.sets = await api('sets.list');
  const set = S.sets.find((x) => x.id === id) || S.sets[0];
  cur.setId = set?.id;
  const modServers = S.servers.filter((s) => !['vanilla', 'paper'].includes(s.loader));
  setTop({ title: 'Наборы модов клиента', actions: `<button onclick="importDir('sets')">${ic('import')}Импорт папки</button>
    ${modServers.length ? `<select style="width:auto" onchange="this.value&&fromServer(this.value);this.value=''"><option value="">Из модов сервера…</option>${modServers.map((s) => `<option value="${s.id}">${esc(s.name)}</option>`).join('')}</select>` : ''}
    <button class="primary" onclick="newSet()">${ic('plus')}Новый набор</button>` });
  const vers = await call('versions', {}).catch(() => []);
  setPage(`<p class="small dim" style="margin:0 0 14px">Применение набора заменяет папку mods в <code>${esc(S.settings.minecraftDir)}</code>. Ваша прежняя папка mods сохраняется как mods_backup_*.</p>
    <div class="grid" style="grid-template-columns:260px minmax(0,1fr);align-items:start"><div class="card" style="padding:6px">${S.sets.map((x) => `<a class="nav-item ${x.id === set.id ? 'on' : ''}" href="#sets/${x.id}" style="padding:9px 10px">
      ${ic(x.builtin ? 'cube' : LOADERS[x.loader]?.icon || 'puzzle')}<span class="lbl"><span style="display:block" class="ellipsis">${esc(x.name)}</span><span class="small dim">${x.builtin ? 'без модов' : `${loaderName(x.loader)} ${esc(x.mcVersion)} · ${x.mods.length} ${plural(x.mods.length, 'мод', 'мода', 'модов')}`}</span></span></a>`).join('')}</div>
    <div>${setView(set, vers)}</div></div>`);
  if (!set.builtin && set.mcVersion) doSearch();
}
function setView(s, vers) {
  if (s.builtin) return `<div class="card empty">${ic('cube')}<h3>${esc(s.name)}</h3><p>Отключает все моды в клиенте Minecraft.</p><button class="primary" onclick="applySet('${s.id}')">${ic('check')}Применить</button></div>`;
  const vopts = (vers.some((v) => v.id === s.mcVersion) || !s.mcVersion ? vers : [{ id: s.mcVersion }, ...vers]);
  return `<div class="card"><div class="card-head"><h3>${ic('puzzle')}${esc(s.name)}</h3><button class="sm danger" onclick="delSet('${s.id}')">${ic('trash')}Удалить</button><button class="primary sm" onclick="applySet('${s.id}')">${ic('check')}Применить к клиенту</button></div>
    <div class="form-grid" style="grid-template-columns:repeat(4,minmax(0,1fr))"><div class="field"><label>Название</label><input id="s-name" value="${esc(s.name)}"></div>
      <div class="field"><label>Minecraft</label><select id="s-ver">${!s.mcVersion ? '<option value="">— выберите —</option>' : ''}${vopts.map((v) => `<option ${v.id === s.mcVersion ? 'selected' : ''}>${esc(v.id)}</option>`).join('')}</select></div>
      <div class="field"><label>Загрузчик</label><select id="s-loader">${['fabric', 'forge', 'neoforge'].map((k) => `<option value="${k}" ${k === s.loader ? 'selected' : ''}>${loaderName(k)}</option>`).join('')}</select></div>
      <div class="field"><label>Версия загрузчика</label><div class="input-row"><input id="s-lver" value="${esc(s.loaderVersion)}" placeholder="для профиля лаунчера"><button class="icon" title="Подставить последнюю" onclick="pickLver()">${ic('wand')}</button></div></div></div>
    <div class="row" style="margin-top:12px"><button onclick="saveSet('${s.id}')">${ic('save')}Сохранить</button>${s.dir ? `<span class="small dim">Папка: ${esc(s.dir)}</span>` : ''}</div></div>
  <div class="grid g2" style="margin-top:14px;align-items:start"><div class="card drop-zone" data-drop="sets"><div class="card-head"><h3>Моды <span class="count">${s.mods.length}</span></h3>
      <button class="sm" onclick="pickJars('sets')">${ic('upload')}Добавить .jar</button></div><div class="scroll" id="modlist" style="max-height:56vh">${modList(s.mods, 'sets', s.id)}</div></div>
    <div class="card">${s.mcVersion ? searchBox('sets', s.id, s.mcVersion, s.loader, 'mod') : `<div class="empty">${ic('globe')}<p>Укажите версию Minecraft и сохраните набор, чтобы искать моды в каталоге</p></div>`}</div></div>`;
}
async function pickLver() {
  try { const v = await api('loaderVersions', { loader: $('#s-loader').value, mc: $('#s-ver').value }); $('#s-lver').value = v[0] || ''; if (!v[0]) toast('Нет версий загрузчика для этой версии Minecraft', 'error'); } catch {}
}
async function newSet() {
  const vers = await call('versions', {}).catch(() => []);
  const r = await formDialog({ title: 'Новый набор модов', icon: 'puzzle', ok: 'Создать', fields: [{ key: 'name', label: 'Название', value: 'Новый набор' }, { key: 'mcVersion', label: 'Версия Minecraft', options: vers.slice(0, 80).map((v) => [v.id, v.id]), value: vers[0]?.id }, { key: 'loader', label: 'Загрузчик', options: [['fabric', 'Fabric'], ['forge', 'Forge'], ['neoforge', 'NeoForge']], value: 'fabric' }] });
  if (!r) return;
  const s = await api('sets.create', r); go('sets/' + s.id);
}
async function saveSet(id) { await api('sets.update', { id, patch: { name: $('#s-name').value, mcVersion: $('#s-ver').value, loader: $('#s-loader').value, loaderVersion: $('#s-lver').value } }); toast('Сохранено'); route(); }
async function delSet(id) { if (await confirmDialog('Набор и его моды будут удалены из панели.', { title: 'Удалить набор?', ok: 'Удалить', danger: true })) { await api('sets.delete', { id }); go('sets'); } }
async function applySet(id) {
  if (!(await confirmDialog(`Папка mods клиента будет заменена модами набора. Старая папка сохранится как mods_backup_*.`, { title: 'Применить набор?', ok: 'Применить' }))) return;
  const r = await api('sets.apply', { id }); alertDialog(r.message, 'Набор применён', 'checkCircle');
}
async function fromServer(sid) {
  toast('Создаю набор из модов сервера…', 'info', 2500);
  const r = await api('sets.fromServer', { id: sid });
  if (r.skipped.length) alertDialog('Эти моды работают только на сервере и не добавлены в набор:\n\n' + r.skipped.join('\n'), 'Пропущены серверные моды');
  go('sets/' + r.set.id);
}

// ---------- импорт ----------
async function importDir(kind) {
  const p = await native.pickFolder(); if (!p) return;
  toast('Анализ папки…', 'info', 2000);
  const r = await api(kind === 'servers' ? 'servers.import' : 'sets.import', { path: p });
  const txt = (r.added.length ? 'Добавлено:\n' + r.added.map((x) => '• ' + x).join('\n') : 'Ничего не добавлено.') + (r.skipped.length ? '\n\nПропущено:\n' + r.skipped.map((s) => `• ${s.name} — ${s.reason}`).join('\n') : '');
  alertDialog(txt, 'Импорт завершён', r.added.length ? 'checkCircle' : 'info');
  if (kind === 'servers') await loadServers();
  route();
}

// ---------- справка ----------
function pCmds() {
  const cats = [...new Set(COMMANDS.map((c) => c[0]))];
  setTop({ title: 'Команды сервера', sub: `<span class="badge">${COMMANDS.length}</span>` });
  setPage(`<div class="row" style="margin-bottom:14px"><div class="input-icon" style="width:340px">${ic('search')}<input id="cq" placeholder="Поиск команды или описания…" oninput="fCmds()"></div>
      <div class="seg" id="cc"><button class="on" data-c="">Все</button>${cats.map((c) => `<button data-c="${esc(c)}">${esc(c)}</button>`).join('')}</div></div>
    <div class="card" style="padding:4px 8px"><table><thead><tr><th>Синтаксис</th><th>Описание</th><th>Пример</th></tr></thead><tbody id="ct"></tbody></table></div>
    <p class="small dim">Нажмите на пример, чтобы скопировать. В консоли сервера команды вводятся без «/».</p>`);
  $$('#cc button').forEach((b) => (b.onclick = () => { $$('#cc button').forEach((x) => x.classList.toggle('on', x === b)); fCmds(); }));
  fCmds(); $('#cq').focus();
}
function fCmds() {
  const q = $('#cq').value.toLowerCase(), c = $('#cc .on').dataset.c;
  const rows = COMMANDS.filter((x) => (!c || x[0] === c) && (x[1] + x[2] + x[3]).toLowerCase().includes(q));
  $('#ct').innerHTML = rows.map((x) => `<tr><td><code>${esc(x[1])}</code><div class="small dim">${esc(x[0])}</div></td><td>${esc(x[2])}</td><td><code style="cursor:pointer" title="Копировать" onclick="native.copy(this.textContent)">${esc(x[3])}</code></td></tr>`).join('') || `<tr><td colspan="3" class="dim">Ничего не найдено</td></tr>`;
}

// ---------- настройки ----------
async function pSettings() {
  const st = S.settings, web = NATIVE ? await T.core.invoke('web_status').catch(() => ({})) : { running: true };
  setTop({ title: 'Настройки' });
  const inp = (k, type = 'text', extra = '') => `<input id="st-${k}" type="${type}" value="${esc(st[k] ?? '')}" ${extra}>`;
  const dir = (k) => `<div class="input-row" style="width:100%"><input id="st-${k}" value="${esc(st[k] ?? '')}"><button class="icon" title="Обзор…" onclick="browse('#st-${k}')">${ic('folder')}</button></div>`;
  const item = (icon, t, d, ctl, auto) => `<div class="set-item">${ic(icon)}<div class="txt"><b>${t}</b><span>${d}</span></div><div class="ctl ${auto ? 'auto' : ''}">${ctl}</div></div>`;
  setPage(`<div style="max-width:960px">
    <div class="set-group-title">Оформление и поведение</div><div class="set-list">
      ${item('sun', 'Тема', 'Как в системе, светлая или тёмная', `<div class="seg" id="st-theme">${[['system', 'Системная'], ['light', 'Светлая'], ['dark', 'Тёмная']].map(([k, t]) => `<button class="${(st.theme || 'system') === k ? 'on' : ''}" data-v="${k}">${t}</button>`).join('')}</div>`, true)}
      ${NATIVE ? item('bell', 'Уведомления Windows', 'Сбои серверов, завершение установки и ошибки бэкапов, когда окно свёрнуто', sw('st-notifications', st.notifications), true) : ''}
      ${NATIVE ? item('pin', 'Сворачивать в трей при закрытии', 'Крестик окна не останавливает серверы', sw('st-minimizeToTray', st.minimizeToTray), true) : ''}
      ${NATIVE ? item('power', 'Запускать вместе с Windows', 'Панель стартует свёрнутой в трей и поднимает серверы с флагом автозапуска' + (S.info.debug ? ' (недоступно в режиме разработки)' : ''), sw('st-launchAtLogin', st.launchAtLogin), true) : ''}
    </div>
    <div class="set-group-title">Папки</div><div class="set-list">
      ${item('server', 'Серверы', 'Куда устанавливаются новые серверы', dir('serversDir'))}
      ${item('archive', 'Бэкапы', 'Общая папка резервных копий', dir('backupsDir'))}
      ${item('cube', 'Клиент Minecraft', 'Папка .minecraft для наборов модов', dir('minecraftDir'))}
    </div>
    <div class="set-group-title">Серверы по умолчанию</div><div class="set-list">
      ${item('memory', 'Память для нового сервера (МБ)', 'Можно изменить для каждого сервера отдельно', inp('defaultRamMb', 'number', 'step="512" style="width:140px"'), true)}
      ${item('archive', 'Максимум бэкапов на сервер', 'Старые копии удаляются начиная с самой ранней', inp('maxBackups', 'number', 'min="1" style="width:140px"'), true)}
      ${item('terminal', 'Java', '«auto» — панель сама скачает нужную версию (8/17/21…) под версию Minecraft', inp('javaPath'))}
    </div>
    <div class="set-group-title">Сеть</div><div class="set-list">
      ${item('globe', 'Прокси', '«auto» — системный прокси Windows или HTTPS_PROXY; пусто — напрямую; или вручную, напр. http://127.0.0.1:10809', inp('proxy'))}
    </div>
    <div class="set-group-title">Веб-доступ</div><div class="set-list">
      ${item('globe', 'Доступ из браузера', web.running ? `Работает: <a href="#" onclick="native.openUrl('http://localhost:${st.port}');return false">http://localhost:${st.port}</a>` : web.error ? `<span style="color:var(--red)">${esc(web.error)}</span>` : 'Та же панель в браузере. По умолчанию выключено', sw('st-webEnabled', st.webEnabled), true)}
      ${item('wifi', 'Порт', 'Порт веб-интерфейса', inp('port', 'number', 'style="width:140px"'), true)}
      ${item('key', 'Пароль', 'Обязателен для доступа из локальной сети', inp('webPassword', 'password', 'autocomplete="new-password" placeholder="не задан"'))}
      ${item('wifi', 'Доступ из локальной сети', 'Открыть панель для других устройств (только с паролем)', sw('st-webLan', st.webLan), true)}
    </div>
    <div class="row" style="margin:16px 0"><button class="primary" style="height:38px;padding:0 22px" onclick="saveSettings()">${ic('save')}Сохранить настройки</button></div>
    <div class="set-group-title">О программе</div><div class="set-list">
      ${item('cube', 'MC Admin Panel ' + esc(S.info.version || ''), S.info.portable ? 'Портативная версия — все данные хранятся рядом с программой' : 'Нативное приложение на Tauri и Rust', '', true)}
      ${NATIVE ? item('folder', 'Данные панели', esc(S.info.dataDir || ''), `<button class="sm" onclick="native.openPath(${attr(S.info.dataDir || '')})">${ic('external')}Открыть</button>`, true) : ''}
    </div></div>`);
  $$('#st-theme button').forEach((b) => (b.onclick = async () => {
    $$('#st-theme button').forEach((x) => x.classList.toggle('on', x === b));
    S.settings = await api('settings.save', { theme: b.dataset.v }); applyTheme();
  }));
}
async function saveSettings() {
  const o = {};
  ['serversDir', 'backupsDir', 'maxBackups', 'defaultRamMb', 'javaPath', 'minecraftDir', 'proxy', 'port', 'webPassword'].forEach((k) => { const e = $('#st-' + k); if (e) o[k] = e.value.trim(); });
  ['launchAtLogin', 'minimizeToTray', 'notifications', 'webEnabled', 'webLan'].forEach((k) => { const e = $('#st-' + k); if (e) o[k] = e.checked; });
  if (o.webLan && !o.webPassword) { toast('Для доступа из локальной сети задайте пароль', 'error'); return; }
  S.settings = await api('settings.save', o);
  toast('Настройки сохранены');
  setTimeout(() => cur.page === 'settings' && pSettings(), 400);
}

// ---------- палитра команд (Ctrl+K) ----------
function paletteItems() {
  const it = [
    ['Перейти', 'home', 'Обзор', () => go('dash')], ['Перейти', 'plus', 'Новый сервер', () => go('create')], ['Перейти', 'puzzle', 'Наборы модов', () => go('sets')],
    ['Перейти', 'book', 'Команды сервера', () => go('cmds')], ['Перейти', 'settings', 'Настройки', () => go('settings')],
    ['Действия', 'import', 'Импортировать серверы', () => importDir('servers')],
    ['Действия', 'sun', 'Переключить тему', async () => { const t = document.documentElement.dataset.theme === 'dark' || (!document.documentElement.dataset.theme && matchMedia('(prefers-color-scheme: dark)').matches) ? 'light' : 'dark'; S.settings = await api('settings.save', { theme: t }); applyTheme(); }],
  ];
  for (const s of S.servers) {
    it.push(['Серверы', 'server', s.name, () => go(`server/${s.id}/console`), STATUS[s.status][0]]);
    if (s.installed && s.status === 'stopped') it.push(['Серверы', 'play', `Запустить «${s.name}»`, () => act(null, s.id, 'start')]);
    if (s.status === 'running') it.push(['Серверы', 'restart', `Перезапустить «${s.name}»`, () => act(null, s.id, 'restart')], ['Серверы', 'stop', `Остановить «${s.name}»`, () => act(null, s.id, 'stop')]);
    if (s.installed) it.push(['Серверы', 'save', `Бэкап «${s.name}»`, () => api('backups.create', { id: s.id, note: '' })]);
    for (const [k, [t, i]] of Object.entries(TABS)) if (k !== 'console') it.push(['Разделы', i, `${s.name} → ${t}`, () => go(`server/${s.id}/${k}`)]);
  }
  return it;
}
function openPalette() {
  if ($('.palette')) return;
  const all = paletteItems();
  const el = document.createElement('div');
  el.className = 'palette';
  el.innerHTML = `<div class="box"><div class="in">${ic('search')}<input placeholder="Найти сервер, раздел или действие…"><span class="kbd">Esc</span></div><div class="res"></div></div>`;
  document.body.append(el);
  const input = $('input', el), res = $('.res', el);
  let shown = [], idx = 0;
  const draw = () => {
    const q = input.value.toLowerCase().trim();
    shown = all.filter((x) => !q || x[2].toLowerCase().includes(q) || x[0].toLowerCase().includes(q)).filter((x) => q || x[0] !== 'Разделы').slice(0, 40);
    idx = Math.min(idx, Math.max(0, shown.length - 1));
    let g = '';
    res.innerHTML = shown.map((x, i) => (x[0] !== g ? `<div class="grp">${(g = x[0])}</div>` : '') + `<div class="it ${i === idx ? 'on' : ''}" data-i="${i}">${ic(x[1])}<span>${esc(x[2])}</span>${x[4] ? `<span class="d">${esc(x[4])}</span>` : ''}</div>`).join('') || '<div class="grp">Ничего не найдено</div>';
    $$('.it', res).forEach((d) => { d.onmousemove = () => { if (idx !== +d.dataset.i) { idx = +d.dataset.i; $$('.it', res).forEach((x) => x.classList.toggle('on', x === d)); } }; d.onclick = () => run(+d.dataset.i); });
    $('.it.on', res)?.scrollIntoView({ block: 'nearest' });
  };
  const close = () => el.remove();
  const run = (i) => { const x = shown[i]; close(); x?.[3](); };
  input.oninput = () => { idx = 0; draw(); };
  input.onkeydown = (e) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); idx = (idx + (e.key === 'ArrowDown' ? 1 : -1) + shown.length) % Math.max(1, shown.length); draw(); }
    else if (e.key === 'Enter') { e.preventDefault(); run(idx); }
    else if (e.key === 'Escape') close();
  };
  el.onmousedown = (e) => { if (e.target === el) close(); };
  draw(); input.focus();
}

// ---------- клавиатура и перетаскивание ----------
document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') { e.preventDefault(); toggleSide(); }
  if (e.key === 'F5' && NATIVE) { e.preventDefault(); route(); }
});
native.onDrop((p) => {
  const zone = $('[data-drop]');
  if (!zone) return;
  if (p.type === 'enter' || p.type === 'over') zone.classList.add('over');
  else if (p.type === 'leave') zone.classList.remove('over');
  else if (p.type === 'drop') {
    zone.classList.remove('over');
    const jars = (p.paths || []).filter((f) => /\.jar$/i.test(f));
    if (!jars.length) return toast('Перетащите файлы .jar', 'error');
    const kind = zone.dataset.drop;
    addJars(kind, kind === 'mods' ? cur.id : cur.setId, jars).then(refreshMods).catch(() => {});
  }
});
if (!NATIVE) {
  document.addEventListener('dragover', (e) => { const z = $('[data-drop]'); if (z) { e.preventDefault(); z.classList.add('over'); } });
  document.addEventListener('dragleave', (e) => { if (!e.relatedTarget) $('[data-drop]')?.classList.remove('over'); });
  document.addEventListener('drop', (e) => {
    const z = $('[data-drop]'); if (!z) return; e.preventDefault(); z.classList.remove('over');
    const kind = z.dataset.drop;
    addJars(kind, kind === 'mods' ? cur.id : cur.setId, [...e.dataTransfer.files].filter((f) => /\.jar$/i.test(f.name))).then(refreshMods).catch(() => {});
  });
}
// аптайм на экране обновляется раз в 30 секунд
setInterval(() => { if (cur.page === 'dash' || cur.page === 'server') S.servers.filter((s) => s.status !== 'stopped').forEach(live); }, 30000);

// ---------- запуск ----------
(async () => {
  try { if (localStorage.getItem('mcpanel-side')) $('#app').classList.add('collapsed'); } catch {}
  try {
    [S.settings, S.info] = await Promise.all([call('settings.get'), call('system.info')]);
    if (S.info.mica) document.documentElement.classList.add('mica');
    applyTheme();
    await loadServers();
    (await call('jobs')).filter((j) => j.status === 'running').forEach(showJob);
  } catch (e) { toast('Не удалось подключиться к ядру панели\n' + e.message, 'error'); }
  onEvent(handleEvent);
  renderShell();
  await route();
  if (NATIVE) T.core.invoke('ui_ready').catch(() => {});
})();
