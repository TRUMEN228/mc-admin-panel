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
      if (m.status === 'starting' && s.status !== 'starting') { s.started = Date.now(); s.history = []; s.crash = null; const c = $('#crash-slot'); if (c && cur.id === s.id) c.innerHTML = ''; }
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
    case 'crash':
      if (!s) return;
      s.crash = m.diag;
      toast(`«${s.name}» аварийно завершился\n${m.diag?.hints?.[0]?.title || 'Код выхода ' + m.code}`, 'error');
      if (cur.page === 'server' && cur.id === s.id && cur.tab === 'console') { const c = $('#crash-slot'); if (c) c.innerHTML = crashBanner(s); }
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
    <div class="tab-scroll"><div class="auto-grid">${S.servers.map((s) => `<div class="card srv-card" id="card-${s.id}">${srvCardInner(s)}</div>`).join('')}</div></div>`, true);
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
const C = { loader: 'paper', snap: false, dirTouched: false, mode: 'core', pack: null };
async function pCreate() {
  setTop({ title: 'Новый сервер', crumbs: [['Обзор', 'dash'], ['Новый сервер']],
    actions: `<div class="seg" id="c-mode"><button data-m="core" class="${C.mode === 'core' ? 'on' : ''}">${ic('box')}С нуля</button><button data-m="pack" class="${C.mode === 'pack' ? 'on' : ''}">${ic('layers')}Из сборки</button></div>` });
  $$('#c-mode button').forEach((b) => (b.onclick = () => { C.mode = b.dataset.m; pCreate(); }));
  const total = S.info.memTotal ? Math.floor(S.info.memTotal / 1048576) : 16384;
  const maxRam = Math.max(2048, Math.min(32768, Math.floor((total - 2048) / 512) * 512));
  const ram = Math.min(C.mode === 'pack' ? Math.max(6144, S.settings.defaultRamMb || 4096) : S.settings.defaultRamMb || 4096, maxRam);
  C.dirTouched = false;
  const coreCards = `<div class="card"><div class="card-head"><h3>${ic('box')}Ядро сервера</h3></div>
      <div class="loader-cards">${Object.entries(LOADERS).map(([k, v]) => `<div class="loader-card ${k === C.loader ? 'on' : ''}" data-l="${k}" onclick="pickLoader('${k}')" tabindex="0">${ic(v.icon)}<b>${v.name}</b><span>${v.desc}</span></div>`).join('')}</div></div>
    <div class="card"><div class="card-head"><h3>${ic('layers')}Версия</h3><label class="switch small">${sw('f-snap', C.snap, 'onchange="C.snap=this.checked;loadVers()"').replace(/^<label class="switch">|<\/label>$/g, '')}<span>Снапшоты</span></label></div>
      <div class="form-grid"><div class="field"><label>Minecraft</label><select id="f-ver" onchange="loadLoaderVers()"><option>Загрузка…</option></select></div>
      <div class="field"><label>Версия загрузчика</label><select id="f-lver" disabled><option value="">—</option></select><span class="hint" id="f-lhint"></span></div></div></div>`;
  const packCards = `<div class="card"><div class="card-head"><h3>${ic('layers')}Сборка</h3><button class="sm" onclick="pickPackFile()">${ic('upload')}Файл .mrpack…</button></div>
      <div id="pack-sel">${packSummary()}</div>
      <div class="input-icon" style="margin:12px 0 8px">${ic('search')}<input id="pq" placeholder="Поиск сборок на Modrinth: create, fabulously optimized, all the mods…" oninput="debPack()"></div>
      <div id="pres" class="scroll" style="max-height:44vh"></div></div>`;
  setPage(`<div class="stack" style="max-width:1000px">${C.mode === 'pack' ? packCards : coreCards}
    <div class="card"><div class="card-head"><h3>${ic('sliders')}Параметры</h3></div>
      <div class="form-grid">
        <div class="field"><label>Название</label><input id="f-name" value="${esc(C.mode === 'pack' && C.pack ? C.pack.name : 'Мой сервер')}" oninput="updDir()"></div>
        <div class="field"><label>Порт</label><input id="f-port" type="number" min="1" max="65535" value="${nextPort()}"></div>
        <div class="field full"><label>Память</label><div class="ram-row"><input type="range" id="f-ram" min="1024" max="${maxRam}" step="512" value="${ram}" oninput="$('#f-ramv').textContent=fmtMb(+this.value)"><span class="val" id="f-ramv">${fmtMb(ram)}</span></div>
          <span class="hint">Установлено в системе: ${fmtMb(total)}. Для ванильного сервера достаточно 2–4 ГБ, для сборок с модами — 6–10 ГБ.</span></div>
        <div class="field full"><label>Папка установки</label><div class="input-row"><input id="f-dir" oninput="C.dirTouched=true"><button onclick="browse('#f-dir')">${ic('folder')}Обзор…</button></div></div>
        ${C.mode === 'pack' ? `<div class="field full"><label class="switch">${sw('f-client', true).replace(/^<label class="switch">|<\/label>$/g, '')}<span>Создать набор модов для клиента из этой сборки</span></label></div>` : ''}
      </div></div>
    <div class="row"><button class="primary" id="f-go" onclick="doCreate()" style="height:40px;padding:0 22px">${ic('download')}Создать и установить</button>
      <span class="small dim">${C.mode === 'pack' ? 'Панель поставит нужное ядро и Java, скачает моды и настройки сборки.' : 'Панель скачает ядро, нужную версию Java, примет EULA и создаст server.properties.'}</span></div></div>`);
  updDir();
  if (C.mode === 'pack') searchPacks(); else await loadVers();
}
function packSummary() {
  const p = C.pack;
  if (!p) return `<p class="small dim" style="margin:0">Выберите сборку в каталоге ниже или откройте файл .mrpack.</p>`;
  return `<div class="list-item" style="background:var(--accent-soft)">${p.icon ? `<img src="${esc(p.icon)}" alt="">` : `<div class="ph">${ic('layers')}</div>`}
    <div class="grow"><div class="t">${esc(p.name)} <span class="dim small">${esc(p.version || '')}</span></div>
    <div class="d">${loaderName(p.loader)} ${esc(p.loaderVersion)} · Minecraft ${esc(p.mc)} · файлов для сервера: ${p.serverFiles}, для клиента: ${p.clientFiles}</div></div>
    <button class="ghost icon sm" title="Убрать" onclick="C.pack=null;$('#pack-sel').innerHTML=packSummary()">${ic('x')}</button></div>`;
}
const debPack = debounce(() => searchPacks(), 350);
async function searchPacks() {
  const res = $('#pres'); if (!res) return;
  res.innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`;
  try {
    const r = await call('catalog.search', { provider: 'modrinth', q: $('#pq').value, type: 'modpack', mc: '', loader: '' });
    res.innerHTML = r.hits.map((m) => `<div class="list-item">${m.icon ? `<img src="${esc(m.icon)}" loading="lazy" alt="">` : `<div class="ph">${ic('layers')}</div>`}
      <div class="grow"><a class="t ellipsis" href="#" style="color:inherit;display:block" onclick='showProject("modrinth",${attr(m.id)},false);return false'>${esc(m.title)}</a><div class="d">${esc(m.desc)}</div><div class="small dim">${ic('download')} ${fmtNum(m.downloads)} · ${esc(m.author || '')}</div></div>
      <button class="sm" onclick='choosePack(this,${attr({ projectId: m.id, icon: m.icon })})'>Выбрать</button></div>`).join('') || `<div class="empty">${ic('search')}<p>Ничего не найдено</p></div>`;
  } catch (e) { res.innerHTML = `<div class="empty">${ic('alert')}<p>${esc(e.message)}</p></div>`; }
}
async function choosePack(b, src) {
  if (b) { b.disabled = true; b.innerHTML = ic('loader', 'spin'); }
  try {
    const p = await api('pack.inspect', src);
    C.pack = { ...p, icon: src.icon };
    $('#pack-sel').innerHTML = packSummary();
    if (!C.dirTouched || $('#f-name').value === 'Мой сервер') { $('#f-name').value = p.name; C.dirTouched = false; updDir(); }
    $('#page').scrollTo({ top: 0, behavior: 'smooth' });
  } catch {} finally { if (b) { b.disabled = false; b.textContent = 'Выбрать'; } }
}
async function pickPackFile() {
  const f = NATIVE ? await T.dialog.open({ filters: [{ name: 'Сборка Modrinth', extensions: ['mrpack'] }], title: 'Сборка .mrpack' })
    : await promptDialog({ title: 'Сборка', label: 'Путь к файлу .mrpack на компьютере с панелью' });
  if (f) choosePack(null, { path: f });
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
  if (C.mode === 'pack') {
    if (!C.pack) return toast('Выберите сборку', 'error');
    $('#f-go').disabled = true;
    try {
      const r = await api('servers.createFromPack', { packPath: C.pack.path, name: $('#f-name').value, dir: $('#f-dir').value, port: +$('#f-port').value, ram: +$('#f-ram').value, clientSet: $('#f-client').checked });
      C.pack = null; await loadServers(); go(`server/${r.srv.id}/console`);
    } catch { $('#f-go').disabled = false; }
    return;
  }
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
const TABS = { console: ['Консоль', 'terminal'], mods: ['Моды', 'package'], backups: ['Бэкапы', 'archive'], props: ['Настройки мира', 'sliders'], players: ['Игроки', 'users'], files: ['Файлы', 'folder'], config: ['Параметры', 'wrench'] };
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
      <div class="tabs" id="srv-tabs"></div><div id="tab" style="display:flex;flex-direction:column;flex:1;min-height:0"></div></div>`, true);
  $('#page').className = 'page fill';
  $('#srv-tabs').innerHTML = Object.entries(TABS).map(([k, v]) => `<a class="tab ${k === tab ? 'on' : ''}" href="#server/${id}/${k}">${ic(v[1])}${v[0]}</a>`).join('') +
    `<span class="grow"></span><button class="ghost sm" style="align-self:center" onclick="showConnect('${id}')">${ic('wifi')}Подключение</button><button class="ghost sm" style="align-self:center" onclick="native.openPath(${attr(s.dir)})">${ic('folder')}Папка</button>`;
  $('#tab').innerHTML = '';
  await { console: tConsole, mods: tMods, backups: tBackups, props: tProps, players: tPlayers, files: tFiles, config: tConfig }[tab](s);
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
  $('#tab').innerHTML = `<div id="crash-slot">${crashBanner(s)}</div><div class="console-wrap"><div class="console">
      <div class="console-bar"><div class="input-icon grow" style="max-width:280px">${ic('filter')}<input id="log-filter" placeholder="Фильтр строк…" value="${esc(LOG.filter)}" oninput="LOG.filter=this.value;renderLog()"></div>
        <span class="grow"></span>
        <button id="follow-btn" class="${LOG.follow ? 'on' : ''}" title="Автопрокрутка" onclick="setFollow(!LOG.follow)">${ic('arrowDown')}Следить</button>
        <button title="Диагностика проблем" onclick="showDiagnose(cur.id)">${ic('activity')}Диагностика</button>
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
  $('#tab').innerHTML = `<div class="grid g2 fx-row"><div class="card fx-card drop-zone" data-drop="mods">
      <div class="card-head"><h3>${what} <span class="count">${list.length}</span></h3>
        <button class="sm" id="upd-btn" onclick="checkUpdates('mods','${s.id}')">${ic('refresh')}Обновления</button><button class="sm" onclick="pickJars('mods')">${ic('upload')}Добавить .jar</button><button class="sm ghost icon" title="Открыть папку" onclick="native.openPath(${attr(dir)})">${ic('folder')}</button></div>
      ${list.length > 6 ? `<div class="input-icon" style="margin-bottom:8px">${ic('search')}<input placeholder="Фильтр…" oninput="$$('#modlist .list-item').forEach(r=>r.hidden=!r.dataset.f.includes(this.value.toLowerCase()))"></div>` : ''}
      <div class="scroll" id="modlist">${modList(list, 'mods', s.id)}</div>
      <p class="small dim" style="margin:10px 0 0">Перетащите .jar-файлы в окно. Изменения вступают в силу после перезапуска сервера.</p></div>
    <div class="card fx-card">${searchBox('mods', s.id, s.mcVersion, s.loader, plugin ? 'plugin' : 'mod')}</div></div>`;
  doSearch();
}
function modList(list, kind, id) {
  if (!list.length) return `<div class="empty">${ic('package')}<p>Пока пусто — найдите что-нибудь в каталоге или перетащите .jar сюда</p></div>`;
  setTimeout(() => applyMeta(kind, id), 0);
  return list.map((m) => `<div class="list-item ${m.enabled ? '' : 'off'}" data-file="${esc(m.file)}" data-f="${esc(m.file.toLowerCase())}"><div class="ph">${ic('package')}</div>
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
const PROV_KEY = 'mcpanel-provider-';
function searchBox(kind, id, mc, loader, type) {
  let prov = 'modrinth';
  try { prov = localStorage.getItem(PROV_KEY + type) || 'modrinth'; } catch {}
  searchCtx = { kind, id, mc, loader, type, offset: 0, provider: prov };
  setTimeout(loadProviders, 0);
  return `<div class="card-head"><h3>${ic('globe')}Каталог</h3><div class="seg" id="prov"></div></div>
    <div class="input-row"><div class="input-icon grow">${ic('search')}<input id="q" placeholder="${type === 'plugin' ? 'worldedit, luckperms, essentials…' : 'sodium, jei, create…'}" oninput="debSearch()" onkeydown="event.key==='Enter'&&doSearch()"></div></div>
    <p class="small dim" style="margin:8px 0">Фильтр: ${esc(loaderName(loader))} ${esc(mc)} · обязательные зависимости ставятся автоматически</p>
    <div id="res" class="scroll"></div>`;
}
async function loadProviders() {
  const c = searchCtx, el = $('#prov'); if (!el) return;
  const list = await call('catalog.providers', { loader: c.loader }).catch(() => [{ id: 'modrinth', name: 'Modrinth' }]);
  if (!list.some((p) => p.id === c.provider && !p.disabled)) c.provider = 'modrinth';
  el.innerHTML = list.map((p) => `<button data-p="${p.id}" class="${p.id === c.provider ? 'on' : ''}" title="${p.needsKey ? 'Нужен бесплатный API-ключ' : ''}">${p.needsKey ? ic('key') : ''}${esc(p.name)}</button>`).join('');
  $$('button', el).forEach((b) => (b.onclick = () => {
    c.provider = b.dataset.p;
    try { localStorage.setItem(PROV_KEY + c.type, c.provider); } catch {}
    $$('button', el).forEach((x) => x.classList.toggle('on', x === b)); doSearch();
  }));
}
const debSearch = debounce(() => doSearch(), 350);
async function saveCfKey() {
  const k = $('#cf-key').value.trim(); if (!k) return;
  S.settings = await api('settings.save', { curseforgeKey: k });
  const r = await call('catalog.search', { provider: 'curseforge', q: '', mc: searchCtx.mc, loader: searchCtx.loader, type: searchCtx.type }).catch((e) => e);
  if (r instanceof Error) { toast(r.message, 'error'); S.settings = await api('settings.save', { curseforgeKey: '' }); return doSearch(); }
  toast('Ключ CurseForge сохранён'); doSearch();
}
async function doSearch(more = false) {
  const c = searchCtx, res = $('#res'); if (!res) return;
  if (!more) { c.offset = 0; res.innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`; }
  const prov = c.provider;
  if (prov === 'curseforge' && !S.settings.curseforgeKey) {
    res.innerHTML = `<div class="empty" style="padding:28px 16px">${ic('key')}<h3>Нужен API-ключ CurseForge</h3>
      <p>CurseForge выдаёт ключ бесплатно: войдите на <a href="#" onclick="native.openUrl('https://console.curseforge.com/');return false">console.curseforge.com</a>, откройте «API keys» и скопируйте ключ.</p>
      <div class="input-row" style="max-width:420px;margin:0 auto"><input id="cf-key" type="password" placeholder="Вставьте ключ" autocomplete="off"><button class="primary" onclick="saveCfKey()">Сохранить</button></div></div>`;
    return;
  }
  try {
    const r = await call('catalog.search', { provider: prov, q: $('#q').value, mc: c.mc, loader: c.loader, type: c.type, offset: c.offset });
    if (prov !== c.provider) return;
    const html = r.hits.map((m) => `<div class="list-item">${m.icon ? `<img src="${esc(m.icon)}" loading="lazy" alt="" onerror="this.replaceWith(Object.assign(document.createElement('div'),{className:'ph'}))">` : `<div class="ph">${ic('package')}</div>`}
      <div class="grow"><div class="row nowrap"><a class="t ellipsis" href="#" title="Подробнее, скриншоты" onclick='showProject(${attr(c.provider)},${attr(String(m.id))}, ${!m.external});return false' style="color:inherit">${esc(m.title)}</a>
        ${m.client === 'unsupported' ? '<span class="badge blue">сервер</span>' : m.server === 'unsupported' ? '<span class="badge yellow">клиент</span>' : ''}${m.note ? `<span class="badge">${esc(m.note)}</span>` : ''}</div>
        <div class="d">${esc(m.desc)}</div><div class="small dim">${ic('download')} ${fmtNum(m.downloads || 0)}${m.author ? ' · ' + esc(m.author) : ''}</div></div>
      ${m.external ? `<button class="sm" onclick='native.openUrl(${attr(m.url)})'>${ic('external')}Открыть</button>` : `<button class="sm" data-pid="${esc(String(m.id))}" onclick='installMod(this,${attr(String(m.id))})'>${ic('download')}Установить</button>`}</div>`).join('');
    $('#more-btn')?.remove();
    if (more) res.insertAdjacentHTML('beforeend', html); else res.innerHTML = html || `<div class="empty">${ic('search')}<p>Ничего не найдено</p></div>`;
    c.offset += r.hits.length;
    if (c.offset < r.total && r.hits.length) res.insertAdjacentHTML('beforeend', `<div id="more-btn" style="text-align:center;padding:10px"><button class="sm" onclick="doSearch(true)">Показать ещё</button></div>`);
  } catch (e) { if (prov === c.provider) res.innerHTML = `<div class="empty">${ic('alert')}<p>${esc(e.message)}</p></div>`; }
}
async function installMod(b, pid) {
  b.disabled = true; b.innerHTML = ic('loader', 'spin');
  const c = searchCtx;
  try {
    const f = await call(c.kind === 'mods' ? 'mods.install' : 'sets.install', { id: c.id, provider: c.provider, projectId: pid });
    b.innerHTML = ic('check') + 'Готово'; b.title = f.join('\n');
    toast(`Установлено: ${f[0]}${f.length > 1 ? `\n+ зависимости: ${f.slice(1).join(', ')}` : ''}`);
    const list = await call(c.kind === 'mods' ? 'mods.list' : 'sets.list', { id: c.id });
    const ml = $('#modlist');
    if (ml) ml.innerHTML = modList(c.kind === 'mods' ? list.list : list.find((x) => x.id === c.id).mods, c.kind, c.id);
  } catch (e) { b.disabled = false; b.innerHTML = ic('alert') + 'Ошибка'; toast(e.message, 'error'); }
}

// --- превью модов ---
const META = {};
async function applyMeta(kind, id) {
  const key = kind + ':' + id, list = $('#modlist'); if (!list) return;
  const files = $$('.list-item[data-file]', list).map((r) => r.dataset.file);
  if (!files.length) return;
  if (!META[key] || files.some((f) => !(f in META[key]))) {
    try { META[key] = await call('mods.meta', { kind, id }); } catch { return; }
  }
  const m = META[key];
  $$('.list-item[data-file]', $('#modlist') || document.createElement('div')).forEach((r) => {
    const x = m[r.dataset.file]; if (!x) return;
    if (x.icon) { const ph = $('.ph', r); if (ph) { const img = document.createElement('img'); img.src = x.icon; img.alt = ''; img.loading = 'lazy'; img.onerror = () => img.replaceWith(ph); ph.replaceWith(img); } }
    if (x.name) { const t = $('.t', r); t.textContent = x.name; t.title = r.dataset.file; }
    const d = $('.d', r);
    if (d && !d.dataset.meta) { d.dataset.meta = 1; d.innerHTML = [x.version && esc(x.version), d.innerHTML].filter(Boolean).join(' · ') + (x.desc ? `<br>${esc(x.desc)}` : ''); }
    r.style.cursor = 'pointer';
    r.onclick = (e) => { if (!e.target.closest('button,label,input')) showModInfo(r.dataset.file, x); };
  });
}
function showModInfo(file, x) {
  modal({ title: x.name || file, wide: true,
    body: `<div class="row nowrap" style="align-items:flex-start;gap:16px">${x.icon ? `<img src="${esc(x.icon)}" alt="" style="width:72px;height:72px;border-radius:14px;image-rendering:auto;flex-shrink:0">` : ''}
      <div class="grow"><p style="margin:0 0 8px">${esc(x.desc || 'Описание отсутствует')}</p><div class="small dim selectable">${x.version ? `Версия: ${esc(x.version)}<br>` : ''}Файл: ${esc(file)}</div></div></div>`,
    buttons: [...(x.url ? [{ label: 'Страница мода', value: 'url' }] : []), { label: 'Закрыть', cls: 'primary', value: null }] })
    .then((v) => v === 'url' && native.openUrl(x.url));
}

// --- карточка проекта из каталога ---
async function showProject(provider, pid, canInstall = true) {
  const res = await modal({ title: 'Загрузка…', wide: true, icon: 'package',
    body: `<div id="pj"><div class="empty" style="padding:30px">${ic('loader', 'spin')}</div></div>`,
    buttons: [{ label: 'На сайте', value: 'url' }, ...(canInstall ? [{ label: 'Установить', cls: 'primary', value: 'install' }] : []), { label: 'Закрыть', value: null }],
    onOpen: async (o) => {
      let d;
      try { d = await call('catalog.details', { provider, projectId: pid }); } catch (e) { $('#pj', o).innerHTML = `<p>${esc(e.message)}</p>`; return; }
      o.dataset.url = d.url || '';
      $('h3', o).innerHTML = `${d.icon ? `<img src="${esc(d.icon)}" alt="" style="width:36px;height:36px;border-radius:9px">` : ic('package')}<span>${esc(d.title)}</span>`;
      $('#pj', o).innerHTML = `<p class="small dim" style="margin:-4px 0 10px">${d.author ? esc(d.author) + ' · ' : ''}${ic('download')} ${fmtNum(d.downloads || 0)}${d.versions?.length ? ' · ' + d.versions.map(esc).join(', ') : ''}</p>
        ${d.summary ? `<p style="color:var(--text)">${esc(d.summary)}</p>` : ''}
        ${d.gallery?.length ? `<div class="gallery">${d.gallery.map((g) => `<img src="${esc(g)}" loading="lazy" alt="" onclick='lightbox(${attr(g)})' onerror="this.remove()">`).join('')}</div>` : ''}
        ${d.body ? `<div class="pj-body selectable">${esc(d.body)}</div>` : ''}`;
    } });
  if (res === 'url') { const d = await call('catalog.details', { provider, projectId: pid }).catch(() => null); if (d?.url) native.openUrl(d.url); }
  if (res === 'install') { const b = $$('#res button').find((x) => x.dataset.pid === String(pid)); installMod(b || document.createElement('button'), pid); }
}
function lightbox(src) {
  const el = document.createElement('div');
  el.className = 'overlay lightbox';
  el.innerHTML = `<img src="${esc(src)}" alt="">`;
  el.onclick = () => el.remove();
  document.addEventListener('keydown', function k(e) { if (e.key === 'Escape') { el.remove(); document.removeEventListener('keydown', k, true); e.stopPropagation(); } }, true);
  document.body.append(el);
}

// --- обновления модов ---
async function checkUpdates(kind, id) {
  const b = $('#upd-btn'); if (b) { b.disabled = true; b.innerHTML = ic('loader', 'spin') + 'Проверка…'; }
  let r;
  try { r = await api('updates.check', { kind, id }); } catch { return; } finally { if (b) { b.disabled = false; b.innerHTML = ic('refresh') + 'Обновления'; } }
  if (!r.updates.length) return toast(`Всё актуально\nПроверено файлов: ${r.checked}${r.unknown.length ? `, не найдено в каталогах: ${r.unknown.length}` : ''}`, 'ok', 5000);
  const body = `<p>Найдены новые версии для ${kind === 'mods' && srvById(id) ? `«${esc(srvById(id).name)}»` : 'набора'} (${esc(searchCtx?.mc || '')}):</p>
    <div class="scroll" style="max-height:50vh;margin:0 -8px">${r.updates.map((u, i) => `<label class="list-item" style="cursor:pointer"><input type="checkbox" data-u="${i}" checked style="width:16px;height:16px;accent-color:var(--accent)">
      ${u.icon ? `<img src="${esc(u.icon)}" alt="" style="width:32px;height:32px">` : `<div class="ph" style="width:32px;height:32px">${ic('package')}</div>`}
      <div class="grow"><div class="t ellipsis">${esc(u.title || u.file)}</div><div class="d">${esc(u.current || '')} → <b style="color:var(--accent-fg)">${esc(u.latest || '')}</b> · ${esc(u.provider)}</div></div></label>`).join('')}</div>
    ${r.unknown.length ? `<p class="small dim" style="margin-top:10px">Не найдены в каталогах (проверьте вручную): ${r.unknown.map(esc).join(', ')}</p>` : ''}`;
  const sel = await modal({ title: `Доступно обновлений: ${r.updates.length}`, icon: 'refresh', wide: true, body,
    buttons: [{ label: 'Отмена', value: null }, { label: 'Обновить выбранные', cls: 'primary', default: true, value: (ov) => $$('[data-u]', ov).filter((x) => x.checked).map((x) => r.updates[+x.dataset.u]) }] });
  if (!sel?.length) return;
  toast(`Обновление: ${sel.length}…`, 'info', 2500);
  const res = await api('updates.apply', { kind, id, list: sel }).catch(() => null);
  if (!res) return;
  if (res.failed.length) toast('Не удалось обновить:\n' + res.failed.join('\n'), 'error');
  if (res.updated.length) toast(`Обновлено: ${res.updated.length}${kind === 'mods' ? '\nПерезапустите сервер, чтобы применить' : ''}`);
  refreshMods();
}

// --- бэкапы ---
async function tBackups(s) {
  const { dir, list, max } = await api('backups.list', { id: s.id });
  const total = list.reduce((a, b) => a + b.size, 0);
  $('#tab').innerHTML = `<div class="fx"><div class="card fx-card" style="flex:1">
      <div class="card-head"><h3>${ic('archive')}Резервные копии <span class="count">${list.length} из ${max}</span></h3>
        <span class="small dim">${fmtBytes(total)}</span><button class="sm ghost" onclick="native.openPath(${attr(dir)})">${ic('folder')}Папка</button></div>
      <div class="row" style="margin-bottom:12px"><input id="bnote" placeholder="Заметка к бэкапу (необязательно)" style="max-width:340px"><button class="primary" onclick="mkBackup('${s.id}')">${ic('save')}Создать бэкап</button></div>
      ${list.length ? `<div class="scroll"><table class="sticky"><thead><tr><th>Файл</th><th>Создан</th><th>Размер</th><th></th></tr></thead><tbody>${list.map((b, i) => `<tr><td><span class="row nowrap">${ic('archive')}<span class="ellipsis">${esc(b.file)}</span>${i === 0 ? '<span class="badge green">новый</span>' : ''}</span></td>
        <td class="muted" title="${fmtDate(b.time)}">${fmtAgo(b.time)}</td><td class="muted">${fmtBytes(b.size)}</td>
        <td style="text-align:right;white-space:nowrap"><button class="sm" onclick='restore("${s.id}",${attr(b.file)})'>${ic('undo')}Восстановить</button>
        <button class="sm ghost icon" title="Удалить" onclick='delBackup("${s.id}",${attr(b.file)})'>${ic('trash')}</button></td></tr>`).join('')}</tbody></table></div>`
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
    <div class="tab-scroll">${Object.entries(PGROUPS).filter(([g]) => groups[g]).map(([g, t]) => `<div class="pgroup"><div class="set-group-title">${t}</div><div class="card" style="padding:4px 8px"><table><tbody>
      ${groups[g].sort().map((k) => `<tr class="prop-row" data-s="${esc((k + ' ' + (PROPS[k]?.[0] || '')).toLowerCase())}"><td><div class="k">${esc(k)}</div><div class="h">${esc(PROPS[k]?.[0] || '')}</div></td><td>${ctl(k, p[k])}</td></tr>`).join('')}
    </tbody></table></div></div>`).join('')}
    <p class="small dim">Большинство параметров применяются после перезапуска сервера. Комментарии и порядок строк в файле сохраняются.</p></div>`;
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
  $('#tab').innerHTML = `<div class="stack tab-scroll"><div class="card"><div class="card-head"><h3>${ic('wifi')}Онлайн <span class="count">${s.players.length}</span></h3></div><div id="online">${onlineList(s)}</div></div>
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


// --- файлы ---
const F = { path: '', file: null, dirty: false };
async function tFiles(s, path) {
  if (path !== undefined) F.path = path;
  if (F.sid !== s.id) { F.sid = s.id; F.path = ''; F.file = null; }
  if (F.file) return openEditor(s, F.file);
  const list = await api('files.list', { id: s.id, path: F.path }).catch(() => null);
  if (!list) { F.path = ''; return; }
  const parts = F.path ? F.path.split('/') : [];
  const crumbs = [`<a href="#" onclick="tFiles(srvById(cur.id),'');return false">${ic('home')}${esc(s.name)}</a>`]
    .concat(parts.map((p, i) => `<a href="#" onclick='tFiles(srvById(cur.id),${attr(parts.slice(0, i + 1).join('/'))});return false'>${esc(p)}</a>`)).join(ic('chevronRight'));
  const quick = ['server.properties', 'config', 'plugins', 'crash-reports', 'logs'].filter((q) => !F.path && list.some((e) => e.name === q));
  $('#tab').innerHTML = `<div class="card fx-card" style="flex:1"><div class="card-head"><h3 class="crumbs" style="width:auto;margin:0;font-size:14px;color:var(--text)">${crumbs}</h3>
      <button class="sm ghost" onclick="native.openPath(${attr((s.dir + '/' + F.path).replace(/\//g, '\\'))})">${ic('external')}В проводнике</button></div>
    ${quick.length ? `<div class="row" style="margin-bottom:10px">${quick.map((q) => `<button class="sm" onclick='fileOpen(${attr(q)},${list.find((e) => e.name === q).dir})'>${ic(list.find((e) => e.name === q).dir ? 'folder' : 'paper')}${esc(q)}</button>`).join('')}</div>` : ''}
    <div class="scroll"><table class="sticky"><thead><tr><th>Имя</th><th>Изменён</th><th style="text-align:right">Размер</th></tr></thead><tbody>
    ${F.path ? `<tr style="cursor:pointer" onclick='tFiles(srvById(cur.id),${attr(parts.slice(0, -1).join('/'))})'><td colspan="3">${ic('undo')} ..</td></tr>` : ''}
    ${list.map((e) => `<tr style="cursor:${e.dir || e.editable ? 'pointer' : 'default'}" ${e.dir || e.editable ? `onclick='fileOpen(${attr(e.name)},${e.dir})'` : ''} class="${e.dir || e.editable ? '' : 'dim'}">
      <td><span class="row nowrap">${ic(e.dir ? 'folder' : e.editable ? 'paper' : 'box')}<span class="ellipsis">${esc(e.name)}</span></span></td>
      <td class="muted small">${e.time ? fmtAgo(e.time) : ''}</td><td class="muted small" style="text-align:right">${e.dir ? '' : fmtBytes(e.size)}</td></tr>`).join('')}
    </tbody></table></div>${list.length ? '' : '<p class="dim small">Папка пуста</p>'}</div>`;
}
function fileOpen(name, dir) {
  const p = F.path ? F.path + '/' + name : name;
  if (dir) tFiles(srvById(cur.id), p); else { F.file = p; openEditor(srvById(cur.id), p); }
}
async function openEditor(s, path) {
  const r = await api('files.read', { id: s.id, path }).catch(() => null);
  if (!r) { F.file = null; return tFiles(s); }
  F.dirty = false;
  const dirOf = path.includes('/') ? path.slice(0, path.lastIndexOf('/')) : '';
  $('#tab').innerHTML = `<div class="card fx-card" style="flex:1;gap:10px">
    <div class="card-head" style="margin:0"><h3>${ic('paper')}<span class="ellipsis mono">${esc(path)}</span><span class="badge yellow" id="ed-dirty" hidden>не сохранён</span></h3>
      <button class="sm ghost" onclick="closeEditor(${attr(dirOf)})">${ic('x')}Закрыть</button>
      <button class="sm primary" id="ed-save" onclick="saveFile()">${ic('save')}Сохранить <span class="kbd" style="background:transparent;border-color:currentColor;color:inherit">Ctrl S</span></button></div>
    <textarea id="ed" spellcheck="false" class="mono" style="flex:1;min-height:160px;resize:none;white-space:pre;tab-size:4;line-height:1.55;font-size:12.5px"></textarea>
    <p class="small dim" style="margin:0">Перед сохранением панель делает копию файла с расширением .bak. Большинство конфигов применяются после перезапуска сервера.</p></div>`;
  const ed = $('#ed');
  ed.value = r.text;
  ed.oninput = () => { F.dirty = true; $('#ed-dirty').hidden = false; };
  ed.onkeydown = (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); saveFile(); }
    if (e.key === 'Tab') { e.preventDefault(); const p = ed.selectionStart; ed.setRangeText('    ', p, ed.selectionEnd, 'end'); ed.oninput(); }
  };
  ed.focus();
}
async function saveFile() {
  await api('files.write', { id: cur.id, path: F.file, text: $('#ed').value });
  F.dirty = false; $('#ed-dirty').hidden = true; toast('Файл сохранён');
}
async function closeEditor(dir) {
  if (F.dirty && !(await confirmDialog('Изменения не сохранены. Закрыть без сохранения?', { title: 'Закрыть файл?', ok: 'Закрыть', danger: true }))) return;
  F.file = null; F.dirty = false; tFiles(srvById(cur.id), dir);
}
const openFile = (id, path) => { F.sid = id; F.file = path; F.path = path.slice(0, path.lastIndexOf('/')); go(`server/${id}/files`); };

// --- подключение ---
async function showConnect(id) {
  const s = srvById(id);
  const body = `<div id="cn" class="stack" style="gap:10px"><div class="empty" style="padding:20px">${ic('loader', 'spin')}</div></div>`;
  modal({ title: 'Подключение к серверу', icon: 'wifi', wide: true, body, buttons: [{ label: 'Закрыть', value: null }],
    onOpen: async (ov) => {
      const i = await call('net.info', { id }).catch((e) => ({ error: e.message }));
      const box = $('#cn', ov); if (!box) return;
      if (i.error) { box.innerHTML = `<p>${esc(i.error)}</p>`; return; }
      const row = (icon, t, d, addr) => `<div class="set-item" style="padding:12px 14px">${ic(icon)}<div class="txt"><b>${t}</b><span>${d}</span></div>${addr ? `<div class="ctl auto"><code class="selectable">${esc(addr)}</code><button class="sm icon" title="Копировать" onclick='native.copy(${attr(addr)})'>${ic('copy')}</button></div>` : ''}</div>`;
      box.innerHTML = `${row('home', 'Этот компьютер', 'Для игры на том же ПК', `localhost:${i.port}`)}
        ${i.localIp ? row('wifi', 'Локальная сеть', 'Для друзей в той же Wi-Fi/LAN-сети', `${i.localIp}:${i.port}`) : ''}
        ${i.externalIp ? row('globe', 'Интернет', 'Работает, если на роутере проброшен порт ' + i.port + ' (TCP) на ' + (i.localIp || 'этот ПК'), `${i.externalIp}:${i.port}`) : ''}
        <div class="set-item" style="padding:12px 14px">${ic('shield')}<div class="txt"><b>Брандмауэр Windows</b><span>${i.firewall ? 'Входящие подключения на порт ' + i.port + ' разрешены' : 'Правила для порта ' + i.port + ' нет — другие устройства могут не подключиться'}</span></div>
          <div class="ctl auto">${i.firewall ? '<span class="badge green">' + ic('check') + 'Открыт</span>' : NATIVE ? `<button class="sm primary" onclick="fwAdd(this,'${id}')">${ic('shield')}Открыть порт</button>` : ''}</div></div>
        <div class="set-item" style="padding:12px 14px">${ic('zap')}<div class="txt"><b>Проверка из интернета</b><span id="cn-check">Сервис mcsrvstat.us попробует подключиться к серверу по внешнему адресу</span></div>
          <div class="ctl auto"><button class="sm" ${s.status !== 'running' ? 'disabled title="Сначала запустите сервер"' : ''} onclick="netCheck(this,'${id}')">Проверить</button></div></div>
        <p class="small dim" style="margin:4px 0 0">Если проверка не проходит: пробросьте порт на роутере, либо используйте туннель (playit.gg, Radmin VPN, ZeroTier).</p>`;
    } });
}
async function fwAdd(b, id) {
  b.disabled = true; b.innerHTML = ic('loader', 'spin') + 'Ждём подтверждения…';
  const r = await call('net.firewall', { id }).catch((e) => { toast(e.message, 'error'); return null; });
  if (r?.ok) { b.outerHTML = '<span class="badge green">' + ic('check') + 'Открыт</span>'; toast('Порт открыт в брандмауэре'); }
  else { b.disabled = false; b.innerHTML = ic('shield') + 'Открыть порт'; }
}
async function netCheck(b, id) {
  b.disabled = true; b.innerHTML = ic('loader', 'spin');
  const el = $('#cn-check');
  try {
    const r = await call('net.check', { id });
    el.innerHTML = r.online ? `<span style="color:var(--accent-fg)">Доступен из интернета: ${esc(r.address)}</span>` : `<span style="color:var(--red)">Не доступен по ${esc(r.address)} — проверьте проброс порта на роутере</span>`;
  } catch (e) { el.textContent = e.message; }
  b.disabled = false; b.textContent = 'Проверить';
}

// --- диагностика сбоев ---
function crashBanner(s) {
  const d = s.crash;
  if (!d || s.status !== 'stopped') return '';
  return `<div class="card" style="border-color:color-mix(in srgb,var(--red) 45%,transparent);background:var(--red-soft);padding:12px 16px;margin-bottom:12px">
    <div class="row nowrap" style="align-items:flex-start">${ic('alert').replace('class="ic ', 'style="color:var(--red);margin-top:2px" class="ic ')}<div class="grow">
      <b>Сервер аварийно завершился</b>${(d.hints || []).map((h) => `<div class="small" style="margin-top:4px"><b>${esc(h.title)}.</b> <span class="selectable" style="white-space:pre-wrap">${esc(h.text)}</span></div>`).join('') || '<div class="small muted">Автоматически определить причину не удалось — посмотрите журнал выше ошибки.</div>'}</div>
      <div class="row nowrap">${d.report ? `<button class="sm" onclick='openFile("${s.id}",${attr(d.report.file)})'>${ic('paper')}Отчёт о сбое</button>` : ''}<button class="sm ghost icon" title="Скрыть" onclick="srvById('${s.id}').crash=null;this.closest('.card').remove()">${ic('x')}</button></div></div></div>`;
}
async function showDiagnose(id) {
  const d = await api('servers.diagnose', { id }).catch(() => null); if (!d) return;
  const body = `${(d.hints || []).length ? d.hints.map((h) => `<div class="set-item" style="padding:12px 14px;align-items:flex-start">${ic('alert')}<div class="txt"><b>${esc(h.title)}</b><span style="white-space:pre-wrap">${esc(h.text)}</span></div></div>`).join('')
    : `<p>По последним строкам журнала проблем не найдено.</p>`}
    <div class="set-group-title" style="margin-top:16px">Отчёты о сбоях</div>${(d.reports || []).length ? d.reports.map((r) => `<div class="list-item" style="cursor:pointer" onclick='document.querySelector(".overlay")?.remove();openFile("${id}",${attr(r.file)})'>${ic('paper')}<div class="grow t mono small">${esc(r.file.replace('crash-reports/', ''))}</div><span class="small dim">${fmtAgo(r.time)}</span></div>`).join('') : '<p class="small dim">Нет</p>'}`;
  modal({ title: 'Диагностика', icon: 'activity', wide: true, body, buttons: [{ label: 'Закрыть', value: null }] });
}

// --- смена версии ---
async function changeVersion(id) {
  const s = srvById(id);
  if (s.status !== 'stopped') return toast('Сначала остановите сервер', 'error');
  const vers = await api('versions', {}).catch(() => []); if (!vers.length) return;
  const r = await modal({ title: 'Сменить версию', icon: 'layers', body: `<div class="stack">
      <p style="margin:0">Сейчас: ${loaderName(s.loader)} ${esc(s.mcVersion)} ${esc(s.loaderVersion)}. Перед сменой панель сделает бэкап мира. Моды и плагины могут оказаться несовместимы с новой версией — после смены проверьте обновления.</p>
      <div class="field"><label>Minecraft</label><select id="cv-mc">${vers.map((v) => `<option ${v.id === s.mcVersion ? 'selected' : ''}>${esc(v.id)}</option>`).join('')}</select></div>
      ${s.loader !== 'vanilla' ? `<div class="field"><label>Версия ${loaderName(s.loader)}</label><select id="cv-lv"><option>Загрузка…</option></select></div>` : ''}</div>`,
    buttons: [{ label: 'Отмена', value: null }, { label: 'Сменить версию', cls: 'primary', default: true, value: (ov) => ({ mcVersion: $('#cv-mc', ov).value, loaderVersion: $('#cv-lv', ov)?.value || '' }) }],
    onOpen: (ov) => {
      const load = async () => {
        const sel = $('#cv-lv', ov); if (!sel) return;
        sel.innerHTML = '<option>Загрузка…</option>';
        const v = await call('loaderVersions', { loader: s.loader, mc: $('#cv-mc', ov).value }).catch(() => []);
        sel.innerHTML = v.length ? v.map((x, i) => `<option value="${esc(x)}">${esc(x)}${i === 0 ? ' — последняя' : ''}</option>`).join('') : '<option value="">Нет версий</option>';
      };
      $('#cv-mc', ov).onchange = load; load();
    } });
  if (!r) return;
  if (s.loader !== 'vanilla' && !r.loaderVersion) return toast('Нет подходящей версии загрузчика', 'error');
  api('servers.changeVersion', { id, ...r }).catch(() => {});
}
const JVM_PRESETS = [['', 'Без набора', 'Только память и ваши аргументы'], ['aikar', 'Aikar (G1GC)', 'Проверенный набор для Paper и модов, меньше лагов от сборщика мусора'], ['zgc', 'ZGC', 'Минимальные паузы при большом объёме памяти, нужен Java 21+']];

// --- параметры ---
function tConfig(s) {
  const total = S.info.memTotal ? Math.floor(S.info.memTotal / 1048576) : 16384;
  const maxRam = Math.max(s.ram, Math.min(65536, Math.floor((total - 1024) / 512) * 512));
  $('#tab').innerHTML = `<div class="tab-scroll"><div style="max-width:960px"><div class="set-group-title">Запуск</div><div class="set-list">
    <div class="set-item">${ic('text')}<div class="txt"><b>Название</b><span>Отображается в панели; папка бэкапов называется так же</span></div><div class="ctl"><input id="c-name" value="${esc(s.name)}"></div></div>
    <div class="set-item">${ic('memory')}<div class="txt"><b>Память</b><span>Максимальный размер кучи Java (-Xmx)</span></div><div class="ctl"><div class="ram-row" style="width:100%"><input type="range" id="c-ram" min="1024" max="${maxRam}" step="512" value="${s.ram}" oninput="$('#c-ramv').textContent=fmtMb(+this.value)"><span class="val" id="c-ramv">${fmtMb(s.ram)}</span></div></div></div>
    <div class="set-item">${ic('wifi')}<div class="txt"><b>Порт</b><span>Также меняется в server.properties</span></div><div class="ctl"><input id="c-port" type="number" value="${s.port}" style="width:120px"></div></div>
    <div class="set-item">${ic('zap')}<div class="txt"><b>Набор флагов JVM</b><span id="c-preset-d">${JVM_PRESETS.find((p) => p[0] === (s.jvmPreset || ''))[2]}</span></div><div class="ctl"><select id="c-preset" onchange="$('#c-preset-d').textContent=JVM_PRESETS.find(p=>p[0]===this.value)[2]">${JVM_PRESETS.map((p) => `<option value="${p[0]}" ${p[0] === (s.jvmPreset || '') ? 'selected' : ''} ${p[0] === 'zgc' && s.javaMajor && s.javaMajor < 21 ? 'disabled' : ''}>${p[1]}</option>`).join('')}</select></div></div>
    <div class="set-item">${ic('terminal')}<div class="txt"><b>Дополнительные аргументы JVM</b><span>Добавляются после набора флагов</span></div><div class="ctl"><input id="c-jvm" class="mono" value="${esc(s.jvmArgs)}" placeholder="—"></div></div>
    <div class="set-item">${ic('restart')}<div class="txt"><b>Автоперезапуск при сбое</b><span>Если сервер упал после минуты работы — поднять через 5 секунд</span></div><div class="ctl auto">${sw('c-ar', s.autoRestart)}</div></div>
    <div class="set-item">${ic('power')}<div class="txt"><b>Запускать при старте панели</b><span>Вместе с автозапуском Windows сервер поднимается сам после перезагрузки</span></div><div class="ctl auto">${sw('c-as', s.autoStart)}</div></div>
  </div><div class="row" style="margin:14px 0 4px"><button class="primary" onclick="saveConfig('${s.id}')">${ic('save')}Сохранить</button></div>
  <div class="set-group-title">Сведения</div><div class="set-list">
    <div class="set-item">${ic('folder')}<div class="txt"><b>Папка сервера</b><span class="selectable">${esc(s.dir)}</span></div><div class="ctl auto"><button class="sm" onclick="native.openPath(${attr(s.dir)})">${ic('external')}Открыть</button></div></div>
    <div class="set-item">${ic('cube')}<div class="txt"><b>Ядро</b><span>${loaderName(s.loader)} ${esc(s.mcVersion)} ${esc(s.loaderVersion)} · Java ${s.javaMajor || '—'}${s.imported ? ' · импортирован' : ''}</span></div><div class="ctl auto"><button class="sm" onclick="changeVersion('${s.id}')">${ic('layers')}Сменить версию</button></div></div>
  </div>
  <div class="set-group-title" style="color:var(--red)">Опасная зона</div><div class="set-list">
    <div class="set-item">${ic('download')}<div class="txt"><b>Переустановить ядро</b><span>Заново скачать ядро и Java. Мир, моды и настройки сохранятся</span></div><div class="ctl auto"><button class="sm" onclick="reinstall('${s.id}')">Переустановить</button></div></div>
    <div class="set-item">${ic('x')}<div class="txt"><b>Убрать из панели</b><span>Файлы сервера останутся на диске</span></div><div class="ctl auto"><button class="sm danger" onclick="delSrv('${s.id}',false)">Убрать</button></div></div>
    <div class="set-item">${ic('trash')}<div class="txt"><b>Удалить сервер</b><span>Безвозвратно удалить сервер вместе со всеми файлами и миром</span></div><div class="ctl auto"><button class="sm danger solid" onclick="delSrv('${s.id}',true)">Удалить</button></div></div>
  </div></div></div>`;
}
async function saveConfig(id) {
  const port = +$('#c-port').value, oldPort = srvById(id).port;
  await saveSrv(id, { name: $('#c-name').value, ram: $('#c-ram').value, jvmArgs: $('#c-jvm').value, jvmPreset: $('#c-preset').value, autoRestart: $('#c-ar').checked, autoStart: $('#c-as').checked, port });
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
    <div class="grid fx-row" style="grid-template-columns:260px minmax(0,1fr)"><div class="card fx-card" style="padding:6px"><div class="scroll">${S.sets.map((x) => `<a class="nav-item ${x.id === set.id ? 'on' : ''}" href="#sets/${x.id}" style="padding:9px 10px">
      ${ic(x.builtin ? 'cube' : LOADERS[x.loader]?.icon || 'puzzle')}<span class="lbl"><span style="display:block" class="ellipsis">${esc(x.name)}</span><span class="small dim">${x.builtin ? 'без модов' : `${loaderName(x.loader)} ${esc(x.mcVersion)} · ${x.mods.length} ${plural(x.mods.length, 'мод', 'мода', 'модов')}`}</span></span></a>`).join('')}</div></div>
    <div class="fx">${setView(set, vers)}</div></div>`, true);
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
  <div class="grid g2 fx-row"><div class="card fx-card drop-zone" data-drop="sets"><div class="card-head"><h3>Моды <span class="count">${s.mods.length}</span></h3>
      <button class="sm" id="upd-btn" onclick="checkUpdates('sets','${s.id}')">${ic('refresh')}Обновления</button><button class="sm" onclick="pickJars('sets')">${ic('upload')}Добавить .jar</button></div><div class="scroll" id="modlist">${modList(s.mods, 'sets', s.id)}</div></div>
    <div class="card fx-card">${s.mcVersion ? searchBox('sets', s.id, s.mcVersion, s.loader, 'mod') : `<div class="empty">${ic('globe')}<p>Укажите версию Minecraft и сохраните набор, чтобы искать моды в каталоге</p></div>`}</div></div>`;
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
    <div class="card fx-card" style="padding:4px 8px;flex:1"><div class="scroll"><table class="sticky"><thead><tr><th>Синтаксис</th><th>Описание</th><th>Пример</th></tr></thead><tbody id="ct"></tbody></table></div></div>
    <p class="small dim" style="margin:10px 0 0">Нажмите на пример, чтобы скопировать. В консоли сервера команды вводятся без «/».</p>`, true);
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
    <div class="set-group-title">Каталоги модов</div><div class="set-list">
      ${item('key', 'API-ключ CurseForge', `Нужен для поиска, установки и обновления модов с CurseForge. Бесплатно: <a href="#" onclick="native.openUrl('https://console.curseforge.com/');return false">console.curseforge.com</a>. Modrinth, Hangar и SpigotMC работают без ключа`, inp('curseforgeKey', 'password', 'autocomplete="off" placeholder="не задан"'))}
    </div>
    <div class="set-group-title">Веб-доступ</div><div class="set-list">
      ${item('globe', 'Доступ из браузера', web.running ? `Работает: <a href="#" onclick="native.openUrl('http://localhost:${st.port}');return false">http://localhost:${st.port}</a>` : web.error ? `<span style="color:var(--red)">${esc(web.error)}</span>` : 'Та же панель в браузере. По умолчанию выключено', sw('st-webEnabled', st.webEnabled), true)}
      ${item('wifi', 'Порт', 'Порт веб-интерфейса', inp('port', 'number', 'style="width:140px"'), true)}
      ${item('key', 'Пароль', 'Обязателен для доступа из локальной сети', inp('webPassword', 'password', 'autocomplete="new-password" placeholder="не задан"'))}
      ${item('wifi', 'Доступ из локальной сети', 'Открыть панель для других устройств (только с паролем)', sw('st-webLan', st.webLan), true)}
    </div>
    <div class="row" style="margin:16px 0"><button class="primary" style="height:38px;padding:0 22px" onclick="saveSettings()">${ic('save')}Сохранить настройки</button></div>
    <div class="set-group-title">О программе</div><div class="set-list">
      ${item('cube', 'MC Admin Panel ' + esc(S.info.version || ''), S.info.portable ? 'Портативная версия — все данные хранятся рядом с программой' : 'Нативное приложение на Tauri и Rust',
        NATIVE ? `<button class="sm" id="upd-check" onclick="checkAppUpdate(true)">${ic('refresh')}Проверить обновления</button>` : '', true)}
      ${NATIVE ? item('download', 'Проверять обновления при запуске', 'Новые версии берутся из релизов на GitHub', sw('st-autoUpdateCheck', st.autoUpdateCheck !== false), true) : ''}
      ${NATIVE ? item('folder', 'Данные панели', esc(S.info.dataDir || ''), `<button class="sm" onclick="native.openPath(${attr(S.info.dataDir || '')})">${ic('external')}Открыть</button>`, true) : ''}
    </div></div>`);
  $$('#st-theme button').forEach((b) => (b.onclick = async () => {
    $$('#st-theme button').forEach((x) => x.classList.toggle('on', x === b));
    S.settings = await api('settings.save', { theme: b.dataset.v }); applyTheme();
  }));
}
async function saveSettings() {
  const o = {};
  ['serversDir', 'backupsDir', 'maxBackups', 'defaultRamMb', 'javaPath', 'minecraftDir', 'proxy', 'port', 'webPassword', 'curseforgeKey'].forEach((k) => { const e = $('#st-' + k); if (e) o[k] = e.value.trim(); });
  ['launchAtLogin', 'minimizeToTray', 'notifications', 'webEnabled', 'webLan', 'autoUpdateCheck'].forEach((k) => { const e = $('#st-' + k); if (e) o[k] = e.checked; });
  if (o.webLan && !o.webPassword) { toast('Для доступа из локальной сети задайте пароль', 'error'); return; }
  S.settings = await api('settings.save', o);
  toast('Настройки сохранены');
  setTimeout(() => cur.page === 'settings' && pSettings(), 400);
}

// ---------- обновление панели ----------
const RELEASES = 'https://github.com/TRUMEN228/mc-admin-panel/releases/latest';
async function checkAppUpdate(manual = false) {
  if (!NATIVE || !T.updater) return;
  const b = $('#upd-check'); if (b) { b.disabled = true; b.innerHTML = ic('loader', 'spin') + 'Проверка…'; }
  let upd = null;
  try { upd = await T.updater.check(); }
  catch (e) { if (manual) toast('Не удалось проверить обновления\n' + (e?.message || e), 'error'); }
  finally { if (b) { b.disabled = false; b.innerHTML = ic('refresh') + 'Проверить обновления'; } }
  if (!upd) { if (manual) toast(`Установлена последняя версия (${S.info.version})`); return; }
  const running = S.servers.filter((s) => s.status !== 'stopped');
  const notes = (upd.body || '').trim();
  const portable = S.info.portable;
  const ok = await modal({ title: `Доступна версия ${upd.version}`, icon: 'download', wide: true,
    body: `<p>Сейчас установлена ${esc(S.info.version)}.${portable ? ' Портативную версию нужно заменить вручную: скачайте новый Portable.exe со страницы релиза.' : ''}</p>
      ${notes ? `<div class="card" style="max-height:40vh;overflow:auto;white-space:pre-wrap;font-size:13px">${esc(notes)}</div>` : ''}
      ${!portable && running.length ? `<p style="color:var(--yellow);margin-top:12px">Запущенные серверы (${running.map((s) => esc(s.name)).join(', ')}) будут корректно остановлены перед обновлением.</p>` : ''}`,
    buttons: [{ label: 'Позже', value: false }, { label: portable ? 'Открыть страницу релиза' : 'Обновить и перезапустить', cls: 'primary', default: true, value: true }] });
  if (!ok) return;
  if (portable) return native.openUrl(RELEASES);
  if (running.length) { toast('Остановка серверов…', 'info', 30000); await call('servers.stopAll').catch(() => {}); }
  toast(`Загрузка обновления ${upd.version}…`, 'info', 60000);
  try {
    await upd.downloadAndInstall();
    await T.process.relaunch();
  } catch (e) { toast('Обновление не установлено\n' + (e?.message || e), 'error'); }
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
  if (NATIVE && S.settings.autoUpdateCheck !== false && !S.info.debug) setTimeout(() => checkAppUpdate(false), 5000);
})();
