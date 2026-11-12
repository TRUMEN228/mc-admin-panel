// MC Mod Collector — интерфейс.
const T = window.__TAURI__;
const LOADERS = { fabric: 'Fabric', neoforge: 'NeoForge', forge: 'Forge', quilt: 'Quilt' };
const SRC = { modrinth: ['Modrinth', 'green'], curseforge: ['CurseForge', 'yellow'], url: ['Ссылка', 'blue'], file: ['Файл', ''] };
const S = { packs: [], settings: {}, info: {}, pack: null };
let cur = {};

async function call(cmd, args = {}) {
  try { return await T.core.invoke('call', { cmd, args }); } catch (e) { throw new Error(typeof e === 'string' ? e : e?.message || String(e)); }
}
async function api(cmd, args) { try { return await call(cmd, args); } catch (e) { toast(e.message, 'error'); throw e; } }
const native = {
  openUrl: (u) => u && T.opener.openUrl(u),
  openPath: (p) => api('open', { path: p }),
  async copy(t) { try { await navigator.clipboard.writeText(t); toast('Скопировано', 'ok', 1500); } catch {} },
};
const go = (h) => { if (location.hash === '#' + h) route(); else location.hash = h; };

// ---------- тема ----------
const THEMES = [['hybrid', 'Гибрид', '#0a0b0f', 'linear-gradient(90deg,#10b981,#06b6d4)', '#2b2e3a'], ['light', 'Светлая', '#eef1f7', 'linear-gradient(90deg,#6366f1,#14b8a6)', '#cbd5e1'],
  ['glow', 'Неон', '#050507', 'linear-gradient(90deg,#a855f7,#f97316)', '#2a2a33'], ['calm', 'Спокойная', '#262b35', '#88c0d0', '#3b4252']];
const themePick = (cur) => `<div class="theme-pick" id="st-theme">${THEMES.map(([k, t, bg, ac, ln]) => `<button class="${cur === k ? 'on' : ''}" data-v="${k}" onclick="setThemeId('${k}')"><div class="sw" style="background:${bg}"><i style="background:${ac}"></i><i style="background:${ln}"></i><i style="background:${ln}"></i></div><span>${t}</span></button>`).join('')}</div>`;
const themeId = () => { const t = S.settings.theme; return THEMES.some((x) => x[0] === t) ? t : t === 'light' ? 'light' : 'hybrid'; };
function applyTheme() {
  const t = themeId();
  document.documentElement.dataset.theme = t;
  T.window.getCurrentWindow().setTheme(t === 'light' ? 'light' : 'dark').catch(() => {});
}
async function setThemeId(t) { S.settings = await api('settings.save', { theme: t }); applyTheme(); $$('#st-theme button').forEach((x) => x.classList.toggle('on', x.dataset.v === t)); }

// ---------- каркас ----------
function navItem(href, icon, label, on, extra = '') {
  return `<a class="nav-item ${on ? 'on' : ''}" href="#${href}" title="${esc(label)}">${icon.startsWith('<') ? icon : ic(icon)}<span class="lbl">${esc(label)}</span>${extra}</a>`;
}
function renderNav() {
  $('#nav').innerHTML = navItem('home', 'home', 'Все сборки', cur.page === 'home') +
    `<div class="sec"><span>Сборки</span><button class="ghost icon sm" title="Новая сборка" onclick="newPack()">${ic('plus')}</button></div>` +
    (S.packs.length ? S.packs.map((p) => navItem('pack/' + p.id, p.icon ? `<img src="${esc(p.icon)}" alt="" style="width:16px;height:16px;border-radius:4px">` : 'layers', p.name, cur.page === 'pack' && cur.id === p.id, `<span class="meta">${p.count}</span>`)).join('')
      : `<div class="nav-item dim small" style="cursor:default"><span class="lbl">Пока пусто</span></div>`);
  $('#side-foot').innerHTML = navItem('settings', 'settings', 'Настройки', cur.page === 'settings');
}
function setTop({ title, crumbs, actions = '', sub = '' }) {
  $('#topbar').innerHTML = `${crumbs ? `<div class="crumbs">${crumbs.map((c) => c[1] ? `<a href="#${c[1]}">${esc(c[0])}</a>` : `<span>${esc(c[0])}</span>`).join(ic('chevronRight'))}</div>` : ''}
    <h1><span class="t">${esc(title)}</span>${sub}</h1><div class="actions" id="top-actions">${actions}</div>`;
  document.title = `${title} — MC Mod Collector`;
}
function setPage(html, fill = false) { const p = $('#page'); p.className = 'page' + (fill ? ' fill' : ''); p.innerHTML = html; }
async function loadPacks() { S.packs = await call('packs.list'); }
async function route() {
  const [page = 'home', id] = (location.hash.slice(1) || 'home').split('/');
  cur = { page, id };
  renderNav();
  try { await ({ home: pHome, pack: pPack, settings: pSettings }[page] || pHome)(id); } catch (e) { console.error(e); }
}
window.addEventListener('hashchange', route);

// ---------- задачи ----------
const jobEls = {};
function showJob(j) {
  let el = jobEls[j.id];
  if (!el) { el = jobEls[j.id] = document.createElement('div'); $('#jobs').append(el); }
  el.className = 'job ' + j.status;
  const icon = j.status === 'running' ? ic('loader', 'spin') : j.status === 'done' ? ic('checkCircle') : ic('alert');
  el.innerHTML = `<div class="top">${icon}<b>${esc(j.title)}</b>${j.status === 'running' ? `<span class="small dim">${j.progress}%</span>` : `<button class="ghost icon sm">${ic('x')}</button>`}</div>
    <div class="bar"><i style="width:${j.progress}%"></i></div><div class="l">${esc(j.log.slice(-1)[0] || 'Подготовка…')}</div>`;
  const rm = () => { el.remove(); delete jobEls[j.id]; };
  const b = $('button', el); if (b) b.onclick = rm;
  if (j.status === 'done') setTimeout(rm, 6000);
  if (j.status === 'error') toast(`${j.title}\n${j.log.slice(-1)[0] || ''}`, 'error');
  const running = Object.values(jobEls).length && j.status === 'running';
  T.window.getCurrentWindow().setProgressBar(running ? { status: 'normal', progress: j.progress } : { status: 'none' }).catch(() => {});
}

// ---------- главная ----------
async function pHome() {
  await loadPacks(); renderNav();
  setTop({ title: 'Сборки модов', actions: `<button onclick="importPack()">${ic('import')}Импорт</button><button class="primary" onclick="newPack()">${ic('plus')}Новая сборка</button>` });
  if (!S.packs.length) return setPage(`<div class="card hero-empty empty"><div class="logo">${ic('layers')}</div><h3>Соберите свою сборку модов</h3>
    <p>Ищите моды на Modrinth и CurseForge, добавляйте .jar с любых сайтов, проверяйте совместимость.<br>Готовую сборку можно установить в свой Minecraft и отправить файлом .mrpack — админ-панель поставит её на сервер и клиент.</p>
    <div class="row" style="justify-content:center"><button class="primary" onclick="newPack()">${ic('plus')}Новая сборка</button><button onclick="importPack()">${ic('import')}Импорт папки mods или .mrpack</button></div></div>`);
  const full = await Promise.all(S.packs.map((p) => call('packs.get', { id: p.id }).catch(() => null)));
  setPage(`<div class="tab-scroll"><div class="auto-grid">${full.filter(Boolean).map((p) => `<div class="card pack-card" onclick="go('pack/${p.id}')">
      <div class="row nowrap"><div class="srv-icon">${ic('layers')}</div><div class="grow"><div class="t" style="font:600 15.5px var(--font-display)">${esc(p.name)}</div>
      <div class="small dim">${LOADERS[p.loader] || p.loader} ${esc(p.mc)} · версия ${esc(p.version)}</div></div></div>
      <div class="pack-icons">${p.mods.filter((m) => m.icon).slice(0, 12).map((m) => `<img src="${esc(m.icon)}" title="${esc(m.name)}" alt="">`).join('')}</div>
      <div class="small dim">${p.mods.length} ${plural(p.mods.length, 'мод', 'мода', 'модов')} · изменена ${fmtAgo(p.updated || p.created)}</div></div>`).join('')}</div></div>`, true);
}

async function versionFields(mc = '', loader = 'fabric') {
  const vers = await call('versions', {}).catch(() => []);
  return [{ key: 'mc', label: 'Версия Minecraft', options: vers.slice(0, 120).map((v) => [v.id, v.id]), value: mc || vers[0]?.id },
    { key: 'loader', label: 'Загрузчик', options: Object.entries(LOADERS), value: loader }];
}
async function newPack(pre = {}) {
  const r = await formDialog({ title: 'Новая сборка', icon: 'layers', ok: 'Создать',
    fields: [{ key: 'name', label: 'Название', value: pre.name || 'Моя сборка' }, ...(await versionFields(pre.mc, pre.loader))] });
  if (!r) return null;
  const p = await api('packs.create', r);
  await loadPacks(); go('pack/' + p.id);
  return p;
}
async function importPack() {
  const kind = await modal({ title: 'Импорт', icon: 'import', body: '<p>Откуда взять моды?</p>',
    buttons: [{ label: 'Отмена', value: null }, { label: 'Файл .mrpack', value: 'mrpack' }, { label: 'Папка mods', cls: 'primary', value: 'folder' }] });
  if (kind === 'mrpack') {
    const f = await T.dialog.open({ filters: [{ name: 'Сборка Modrinth', extensions: ['mrpack'] }] }); if (!f) return;
    toast('Импорт сборки…', 'info', 2500);
    const p = await api('packs.importMrpack', { path: f }); await loadPacks(); go('pack/' + p.id);
  } else if (kind === 'folder') {
    const dir = await T.dialog.open({ directory: true, title: 'Папка mods (или папка сборки)', defaultPath: S.settings.minecraftDir }); if (!dir) return;
    const d = await api('packs.detectFolder', { path: dir });
    toast(`Найдено модов: ${d.count}`, 'info', 2500);
    const p = await newPack({ name: d.name === 'mods' ? 'Моя сборка' : d.name, mc: d.mc, loader: d.loader || 'fabric' });
    if (!p) return;
    toast('Опознаю моды в каталогах…', 'info', 4000);
    const r = await api('packs.addFolder', { id: p.id, path: dir });
    toast(`Добавлено: ${r.added.length}${r.failed.length ? `\nОшибки: ${r.failed.length}` : ''}`);
    route();
  }
}

// ---------- сборка ----------
async function pPack(id) {
  let p;
  try { p = await call('packs.get', { id }); } catch { return go('home'); }
  S.pack = p;
  setTop({ title: p.name, crumbs: [['Сборки', 'home'], [p.name]],
    sub: `<span class="badge">${ic('cube')}${LOADERS[p.loader] || p.loader} ${esc(p.mc)}</span><span class="badge">v${esc(p.version)}</span>`,
    actions: `<button onclick="packMenu()">${ic('more')}</button><button onclick="installClient()">${ic('play')}В мой Minecraft</button><button class="primary" onclick="exportPack()">${ic('upload')}Экспорт .mrpack</button>` });
  setPage(`<div class="grid g2 fx-row"><div class="card fx-card drop-zone" data-drop="1">
      <div class="card-head"><h3>Моды <span class="count" id="mcount">${p.mods.length}</span></h3>
        <button class="sm" onclick="checkPack()" id="chk-btn">${ic('refresh')}Проверить</button>
        <button class="sm" onclick="addMenu()">${ic('plus')}Добавить</button></div>
      <div class="input-icon" style="margin-bottom:8px">${ic('search')}<input id="mf" placeholder="Фильтр…" oninput="filterMods()"></div>
      <div class="scroll" id="modlist">${modsHtml(p)}</div>
      <p class="small dim" style="margin:10px 0 0">Перетащите .jar в окно. «Клиент» и «Сервер» — на какую сторону мод попадёт при установке; нажмите, чтобы переключить.</p></div>
    <div class="card fx-card">${searchBox(p)}</div></div>`, true);
  doSearch();
}
function modsHtml(p) {
  if (!p.mods.length) return `<div class="empty">${ic('package')}<p>Найдите моды в каталоге справа или перетащите .jar-файлы сюда</p></div>`;
  return p.mods.map((m) => {
    const src = SRC[m.source] || [m.source, ''];
    return `<div class="list-item ${m.incompatible ? 'incompatible' : ''}" data-f="${esc((m.name + ' ' + (m.file?.name || '')).toLowerCase())}">
      ${m.icon ? `<img src="${esc(m.icon)}" alt="" loading="lazy">` : `<div class="ph">${ic('package')}</div>`}
      <div class="grow"><div class="row nowrap"><a class="t ellipsis" href="#" style="color:inherit" onclick='modInfo(${attr(m.key)});return false'>${esc(m.name)}</a>
        ${m.dependency ? '<span class="badge">зависимость</span>' : ''}${m.incompatible ? '<span class="badge red">нет для этой версии</span>' : ''}</div>
        <div class="d">${[m.version && esc(m.version), m.file?.size && fmtBytes(m.file.size), `<span class="badge ${src[1]}" style="height:18px">${src[0]}</span>`].filter(Boolean).join(' · ')}</div></div>
      <span class="badge side-chip ${m.client === 'unsupported' ? 'off' : 'blue'}" title="Ставить на клиент" onclick='toggleSide(${attr(m.key)},"client")'>Клиент</span>
      <span class="badge side-chip ${m.server === 'unsupported' ? 'off' : 'green'}" title="Ставить на сервер" onclick='toggleSide(${attr(m.key)},"server")'>Сервер</span>
      <button class="ghost icon sm" title="Убрать из сборки" onclick='removeMod(${attr(m.key)})'>${ic('trash')}</button></div>`;
  }).join('');
}
function filterMods() { const q = $('#mf').value.toLowerCase(); $$('#modlist .list-item').forEach((r) => (r.hidden = !r.dataset.f.includes(q))); }
async function refreshPack() {
  S.pack = await call('packs.get', { id: cur.id });
  const ml = $('#modlist'); if (ml) { ml.innerHTML = modsHtml(S.pack); filterMods(); }
  const c = $('#mcount'); if (c) c.textContent = S.pack.mods.length;
  markInPack(); loadPacks().then(renderNav);
}
async function toggleSide(key, side) {
  const m = S.pack.mods.find((x) => x.key === key);
  const v = { client: m.client, server: m.server };
  v[side] = v[side] === 'unsupported' ? 'required' : 'unsupported';
  if (v.client === 'unsupported' && v.server === 'unsupported') return toast('Мод должен ставиться хотя бы на одну сторону', 'error');
  await api('packs.setSide', { id: cur.id, key, ...v }); refreshPack();
}
async function removeMod(key) {
  const m = S.pack.mods.find((x) => x.key === key);
  const deps = S.pack.mods.filter((x) => x.dependency).length;
  if (!(await confirmDialog(`Убрать «${m.name}» из сборки?${deps && !m.dependency ? '\nЕго зависимости останутся — при необходимости уберите их тоже.' : ''}`, { title: 'Убрать мод', ok: 'Убрать', danger: true }))) return;
  await api('packs.remove', { id: cur.id, key }); refreshPack();
}
function modInfo(key) {
  const m = S.pack.mods.find((x) => x.key === key);
  if (m.source === 'modrinth' || m.source === 'curseforge') return showProject(m.source, m.project, false);
  modal({ title: m.name, wide: true, body: `<div class="row nowrap" style="align-items:flex-start;gap:16px">${m.icon ? `<img src="${esc(m.icon)}" style="width:64px;height:64px;border-radius:12px">` : ''}
    <div class="grow"><p>${esc(m.desc || 'Описание отсутствует')}</p><div class="small dim selectable">Файл: ${esc(m.file?.name || '')}${m.page ? `<br>Источник: ${esc(m.page)}` : ''}</div></div></div>`,
    buttons: [{ label: 'Закрыть', cls: 'primary', value: null }] });
}
async function addMenu() {
  const kind = await modal({ title: 'Добавить моды', icon: 'plus', body: `<p>Моды из каталогов удобнее искать справа. Здесь — моды с других сайтов:</p>
    <div class="set-list"><div class="set-item">${ic('upload')}<div class="txt"><b>Файлы .jar</b><span>Скачанные вручную; панель попробует найти их в каталогах</span></div></div>
    <div class="set-item">${ic('globe')}<div class="txt"><b>По ссылке</b><span>Прямая ссылка на .jar — например, с GitHub</span></div></div>
    <div class="set-item">${ic('folder')}<div class="txt"><b>Из папки</b><span>Все моды из папки mods (например, из .minecraft)</span></div></div></div>`,
    buttons: [{ label: 'Отмена', value: null }, { label: 'Из папки', value: 'folder' }, { label: 'По ссылке', value: 'url' }, { label: 'Файлы .jar', cls: 'primary', value: 'files' }] });
  if (kind === 'files') { const f = await T.dialog.open({ multiple: true, filters: [{ name: 'Моды', extensions: ['jar'] }] }); if (f?.length) addFiles(Array.isArray(f) ? f : [f]); }
  if (kind === 'folder') {
    const d = await T.dialog.open({ directory: true, defaultPath: S.settings.minecraftDir }); if (!d) return;
    toast('Опознаю моды…', 'info', 3000);
    const r = await api('packs.addFolder', { id: cur.id, path: d }); addedToast(r); refreshPack();
  }
  if (kind === 'url') {
    const u = await promptDialog({ title: 'Мод по ссылке', label: 'Прямая ссылка на .jar', placeholder: 'https://github.com/…/releases/download/…/mod.jar', ok: 'Добавить' }); if (!u) return;
    toast('Скачиваю…', 'info', 2500);
    const r = await api('packs.addUrl', { id: cur.id, url: u }); addedToast(r); refreshPack();
  }
}
async function addFiles(paths) {
  toast('Опознаю моды…', 'info', 2500);
  const r = await api('packs.addFiles', { id: cur.id, paths }).catch(() => null); if (!r) return;
  addedToast(r); refreshPack();
}
function addedToast(r) {
  toast(r.added.length ? `Добавлено: ${r.added.join(', ')}` : 'Эти моды уже в сборке', r.added.length ? 'ok' : 'info');
  if (r.failed?.length) toast('Не удалось добавить:\n' + r.failed.join('\n'), 'error');
}
async function packMenu() {
  const p = S.pack;
  const a = await modal({ title: p.name, icon: 'layers', wide: true, body: `<div class="stack">
      <div class="form-grid"><div class="field"><label>Название</label><input data-k="name" value="${esc(p.name)}"></div>
      <div class="field"><label>Версия сборки</label><input data-k="version" value="${esc(p.version)}"></div>
      <div class="field"><label>Автор</label><input data-k="author" value="${esc(p.author)}"></div>
      <div class="field"><label>Версия загрузчика</label><input data-k="loaderVersion" value="${esc(p.loaderVersion)}" placeholder="последняя"></div>
      <div class="field full"><label>Описание</label><input data-k="description" value="${esc(p.description)}" placeholder="Коротко о сборке"></div></div></div>`,
    buttons: [{ label: 'Удалить', cls: 'danger', value: 'delete' }, { label: 'Дублировать', value: 'dup' }, { label: 'Сменить версию MC', value: 'ver' },
      { label: 'Сохранить', cls: 'primary', default: true, value: (ov) => ({ save: Object.fromEntries($$('[data-k]', ov).map((e) => [e.dataset.k, e.value])) }) }] });
  if (!a) return;
  if (a.save) { await api('packs.update', { id: p.id, patch: a.save }); await loadPacks(); return route(); }
  if (a === 'dup') { const n = await api('packs.duplicate', { id: p.id }); await loadPacks(); return go('pack/' + n.id); }
  if (a === 'delete') {
    if (!(await confirmDialog(`Сборка «${p.name}» будет удалена.`, { title: 'Удалить сборку?', ok: 'Удалить', danger: true }))) return;
    await api('packs.delete', { id: p.id }); await loadPacks(); return go('home');
  }
  if (a === 'ver') changeVersion();
}
async function changeVersion() {
  const p = S.pack;
  const r = await formDialog({ title: 'Сменить версию', icon: 'layers', ok: 'Подобрать моды', fields: await versionFields(p.mc, p.loader) });
  if (!r) return;
  toast('Подбираю версии модов…', 'info', 4000);
  const res = await api('packs.changeVersion', { id: p.id, mc: r.mc, loader: r.loader, loaderVersion: '' });
  if (res.missing.length) alertDialog(`Нет версий для ${LOADERS[r.loader]} ${r.mc}:\n\n${res.missing.join('\n')}\n\nОни отмечены красным — уберите их или поищите замену.`, 'Не все моды доступны', 'alert');
  else toast(`Сборка переведена на ${LOADERS[r.loader]} ${r.mc}`);
  route();
}
async function checkPack() {
  const b = $('#chk-btn'); b.disabled = true; b.innerHTML = ic('loader', 'spin') + 'Проверка…';
  let r; try { r = await api('packs.check', { id: cur.id }); } catch { return; } finally { b.disabled = false; b.innerHTML = ic('refresh') + 'Проверить'; }
  const up = r.results.filter((x) => x.status === 'update'), miss = r.results.filter((x) => x.status === 'missing');
  if (!up.length && !miss.length) return toast(`Все моды актуальны (${r.results.length})${r.manual.length ? `\nНе проверялись (файлы и ссылки): ${r.manual.length}` : ''}`);
  const sel = await modal({ title: 'Проверка сборки', icon: 'refresh', wide: true,
    body: `${up.length ? `<p>Доступны обновления:</p><div class="scroll" style="max-height:40vh;margin:0 -8px">${up.map((u, i) => `<label class="list-item" style="cursor:pointer"><input type="checkbox" data-u="${i}" checked style="width:16px;height:16px;accent-color:var(--accent)">
        <div class="grow"><div class="t">${esc(u.name)}</div><div class="d">${esc(u.current || '')} → <b style="color:var(--accent-fg)">${esc(u.latest || '')}</b></div></div></label>`).join('')}</div>` : ''}
      ${miss.length ? `<p style="color:var(--red)">Нет версии для ${LOADERS[S.pack.loader]} ${esc(S.pack.mc)}: ${miss.map((m) => esc(m.name)).join(', ')}</p>` : ''}`,
    buttons: [{ label: 'Закрыть', value: null }, ...(up.length ? [{ label: 'Обновить выбранные', cls: 'primary', default: true, value: (ov) => $$('[data-u]', ov).filter((x) => x.checked).map((x) => up[+x.dataset.u].entry) }] : [])] });
  if (sel?.length) { const n = await api('packs.applyUpdates', { id: cur.id, entries: sel }); toast(`Обновлено: ${n.updated}`); refreshPack(); }
}
async function exportPack() {
  const p = S.pack;
  if (!p.mods.length) return toast('Сборка пустая', 'error');
  const name = `${p.name.replace(/[\\/:*?"<>|]/g, '_')} ${p.version}.mrpack`;
  const path = await T.dialog.save({ defaultPath: (S.settings.exportDir ? S.settings.exportDir.replace(/[\\/]$/, '') + '\\' : '') + name, filters: [{ name: 'Сборка Modrinth', extensions: ['mrpack'] }] });
  if (path) api('packs.export', { id: p.id, path });
}
async function installClient() {
  const p = S.pack;
  if (!(await confirmDialog(`Моды сборки «${p.name}» будут скопированы в\n${S.settings.minecraftDir}\\mods\n\nВаша текущая папка mods сохранится как mods_backup_…`, { title: 'Установить в Minecraft?', ok: 'Установить' }))) return;
  api('packs.installClient', { id: p.id });
}

// ---------- каталог ----------
let searchCtx = {};
function searchBox(p) {
  let prov = 'modrinth'; try { prov = localStorage.getItem('mcc-provider') || 'modrinth'; } catch {}
  searchCtx = { mc: p.mc, loader: p.loader, provider: prov, offset: 0 };
  setTimeout(loadProviders, 0);
  return `<div class="card-head"><h3>${ic('globe')}Каталог</h3><div class="seg" id="prov"></div></div>
    <div class="input-icon">${ic('search')}<input id="q" placeholder="sodium, create, jei, journeymap…" oninput="debSearch()" onkeydown="event.key==='Enter'&&doSearch()"></div>
    <p class="small dim" style="margin:8px 0">Только моды для ${LOADERS[p.loader] || p.loader} ${esc(p.mc)} · зависимости добавляются сами</p>
    <div id="res" class="scroll"></div>`;
}
async function loadProviders() {
  const el = $('#prov'); if (!el) return;
  const list = await call('catalog.providers', { loader: searchCtx.loader }).catch(() => [{ id: 'modrinth', name: 'Modrinth' }]);
  el.innerHTML = list.map((p) => `<button data-p="${p.id}" class="${p.id === searchCtx.provider ? 'on' : ''}">${p.needsKey ? ic('key') : ''}${esc(p.name)}</button>`).join('');
  $$('button', el).forEach((b) => (b.onclick = () => { searchCtx.provider = b.dataset.p; try { localStorage.setItem('mcc-provider', b.dataset.p); } catch {} $$('button', el).forEach((x) => x.classList.toggle('on', x === b)); doSearch(); }));
}
const debSearch = debounce(() => doSearch(), 350);
const inPack = (prov, id) => S.pack?.mods.some((m) => m.source === prov && String(m.project) === String(id));
function markInPack() { $$('#res [data-pid]').forEach((b) => { if (inPack(searchCtx.provider, b.dataset.pid)) { b.disabled = true; b.innerHTML = ic('check') + 'В сборке'; b.classList.add('in-pack'); } }); }
async function doSearch(more = false) {
  const c = searchCtx, res = $('#res'); if (!res) return;
  if (!more) { c.offset = 0; res.innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`; }
  if (c.provider === 'curseforge' && !S.settings.curseforgeKey) {
    res.innerHTML = `<div class="empty" style="padding:28px 16px">${ic('key')}<h3>Нужен API-ключ CurseForge</h3>
      <p>Ключ бесплатный: войдите на <a href="#" onclick="native.openUrl('https://console.curseforge.com/');return false">console.curseforge.com</a> и скопируйте ключ из раздела «API keys».</p>
      <div class="input-row" style="max-width:420px;margin:0 auto"><input id="cf-key" type="password" placeholder="Вставьте ключ"><button class="primary" onclick="saveCfKey()">Сохранить</button></div></div>`;
    return;
  }
  const prov = c.provider;
  try {
    const r = await call('catalog.search', { provider: prov, q: $('#q').value, mc: c.mc, loader: c.loader, offset: c.offset });
    if (prov !== c.provider) return;
    const html = r.hits.map((m) => `<div class="list-item">${m.icon ? `<img src="${esc(m.icon)}" loading="lazy" alt="">` : `<div class="ph">${ic('package')}</div>`}
      <div class="grow"><div class="row nowrap"><a class="t ellipsis" href="#" style="color:inherit" onclick='showProject(${attr(prov)},${attr(String(m.id))},true);return false'>${esc(m.title)}</a>
        ${m.client === 'unsupported' ? '<span class="badge blue">сервер</span>' : m.server === 'unsupported' ? '<span class="badge yellow">клиент</span>' : ''}</div>
        <div class="d">${esc(m.desc)}</div><div class="small dim">${ic('download')} ${fmtNum(m.downloads || 0)}${m.author ? ' · ' + esc(m.author) : ''}</div></div>
      <button class="sm" data-pid="${esc(String(m.id))}" onclick='addMod(this,${attr(String(m.id))})'>${ic('plus')}Добавить</button></div>`).join('');
    $('#more-btn')?.remove();
    if (more) res.insertAdjacentHTML('beforeend', html); else res.innerHTML = html || `<div class="empty">${ic('search')}<p>Ничего не найдено</p></div>`;
    c.offset += r.hits.length;
    if (c.offset < r.total && r.hits.length) res.insertAdjacentHTML('beforeend', `<div id="more-btn" style="text-align:center;padding:10px"><button class="sm" onclick="doSearch(true)">Показать ещё</button></div>`);
    markInPack();
  } catch (e) { if (prov === c.provider) res.innerHTML = `<div class="empty">${ic('alert')}<p>${esc(e.message)}</p></div>`; }
}
async function saveCfKey() {
  const k = $('#cf-key').value.trim(); if (!k) return;
  S.settings = await api('settings.save', { curseforgeKey: k });
  const t = await call('catalog.search', { provider: 'curseforge', q: '', mc: searchCtx.mc, loader: searchCtx.loader }).catch((e) => e);
  if (t instanceof Error) { toast(t.message, 'error'); S.settings = await api('settings.save', { curseforgeKey: '' }); }
  else toast('Ключ CurseForge сохранён');
  doSearch();
}
async function addMod(b, pid) {
  if (b) { b.disabled = true; b.innerHTML = ic('loader', 'spin'); }
  try {
    const r = await call('packs.add', { id: cur.id, provider: searchCtx.provider, project: pid });
    toast(r.added.length ? `Добавлено: ${r.added[0]}${r.added.length > 1 ? `\n+ зависимости: ${r.added.slice(1).join(', ')}` : ''}` : 'Уже в сборке');
    await refreshPack();
  } catch (e) { toast(e.message, 'error'); if (b) { b.disabled = false; b.innerHTML = ic('plus') + 'Добавить'; } }
}
async function showProject(provider, pid, canAdd) {
  const res = await modal({ title: 'Загрузка…', wide: true, icon: 'package',
    body: `<div id="pj"><div class="empty" style="padding:30px">${ic('loader', 'spin')}</div></div>`,
    buttons: [{ label: 'На сайте', value: 'url' }, ...(canAdd && !inPack(provider, pid) ? [{ label: 'Добавить в сборку', cls: 'primary', value: 'add' }] : []), { label: 'Закрыть', value: null }],
    onOpen: async (o) => {
      let d;
      try { d = await call('catalog.details', { provider, projectId: pid }); } catch (e) { $('#pj', o).innerHTML = `<p>${esc(e.message)}</p>`; return; }
      showProject.url = d.url;
      $('h3', o).innerHTML = `${d.icon ? `<img src="${esc(d.icon)}" alt="" style="width:36px;height:36px;border-radius:9px">` : ic('package')}<span>${esc(d.title)}</span>`;
      $('#pj', o).innerHTML = `<p class="small dim" style="margin:-4px 0 10px">${d.author ? esc(d.author) + ' · ' : ''}${ic('download')} ${fmtNum(d.downloads || 0)}${d.versions?.length ? ' · ' + d.versions.map(esc).join(', ') : ''}</p>
        ${d.summary ? `<p style="color:var(--text)">${esc(d.summary)}</p>` : ''}
        ${d.gallery?.length ? `<div class="gallery">${d.gallery.map((g) => `<img src="${esc(g)}" loading="lazy" alt="" onclick='lightbox(${attr(g)})' onerror="this.remove()">`).join('')}</div>` : ''}
        ${d.body ? `<div class="pj-body selectable">${esc(d.body)}</div>` : ''}`;
    } });
  if (res === 'url' && showProject.url) native.openUrl(showProject.url);
  if (res === 'add') addMod($(`#res [data-pid="${CSS.escape(String(pid))}"]`), pid);
}
function lightbox(src) {
  const el = document.createElement('div');
  el.className = 'overlay lightbox';
  el.innerHTML = `<img src="${esc(src)}" alt="">`;
  el.onclick = () => el.remove();
  document.body.append(el);
}

// ---------- настройки ----------
async function pSettings() {
  const st = S.settings;
  setTop({ title: 'Настройки' });
  const inp = (k, type = 'text', extra = '') => `<input id="st-${k}" type="${type}" value="${esc(st[k] ?? '')}" ${extra}>`;
  const dir = (k) => `<div class="input-row" style="width:100%"><input id="st-${k}" value="${esc(st[k] ?? '')}"><button class="icon" onclick="browse('#st-${k}')">${ic('folder')}</button></div>`;
  const item = (icon, t, d, ctl, auto) => `<div class="set-item">${ic(icon)}<div class="txt"><b>${t}</b><span>${d}</span></div><div class="ctl ${auto ? 'auto' : ''}">${ctl}</div></div>`;
  setPage(`<div style="max-width:960px">
    <div class="set-group-title">Основное</div><div class="set-list">
      <div class="set-item" style="flex-wrap:wrap">${ic('sun')}<div class="txt"><b>Тема оформления</b><span>Цвета, фон и эффекты интерфейса</span></div><div style="width:100%;margin-top:10px">${themePick(themeId())}</div></div>
      ${item('users', 'Автор сборок', 'Подставляется в новые сборки', inp('author'))}
      ${item('cube', 'Папка Minecraft', 'Куда ставить моды кнопкой «В мой Minecraft» (обычно %APPDATA%\\.minecraft)', dir('minecraftDir'))}
      ${item('download', 'Папка для экспорта', 'Куда по умолчанию сохранять .mrpack', dir('exportDir'))}
    </div>
    <div class="set-group-title">Каталоги и сеть</div><div class="set-list">
      ${item('key', 'API-ключ CurseForge', 'Бесплатно на <a href="#" onclick="native.openUrl(\'https://console.curseforge.com/\');return false">console.curseforge.com</a>. Modrinth работает без ключа', inp('curseforgeKey', 'password', 'autocomplete="off" placeholder="не задан"'))}
      ${item('globe', 'Прокси', '«auto» — системный прокси Windows; пусто — напрямую; или вручную, напр. http://127.0.0.1:10809', inp('proxy'))}
    </div>
    <div class="row" style="margin:16px 0"><button class="primary" style="height:38px;padding:0 22px" onclick="saveSettings()">${ic('save')}Сохранить</button></div>
    <div class="set-group-title">О программе</div><div class="set-list">
      ${item('layers', 'MC Mod Collector ' + esc(S.info.version || ''), S.info.portable ? 'Портативная версия — данные рядом с программой' : 'Сборки хранятся в ' + esc(S.info.dataDir || ''), `<button class="sm" id="upd-check" onclick="checkAppUpdate(true)">${ic('refresh')}Проверить обновления</button>`, true)}
      ${item('download', 'Проверять обновления при запуске', 'Новые версии берутся из релизов на GitHub', sw('st-autoUpdateCheck', st.autoUpdateCheck !== false), true)}
    </div></div>`);
}
async function browse(sel) { const p = await T.dialog.open({ directory: true, defaultPath: $(sel).value || undefined }); if (p) $(sel).value = p; }
async function saveSettings() {
  const o = {};
  ['author', 'minecraftDir', 'exportDir', 'curseforgeKey', 'proxy'].forEach((k) => (o[k] = $('#st-' + k).value.trim()));
  o.autoUpdateCheck = $('#st-autoUpdateCheck').checked;
  S.settings = await api('settings.save', o); toast('Настройки сохранены');
}

// ---------- обновление приложения ----------
async function checkAppUpdate(manual = false) {
  const b = $('#upd-check'); if (b) { b.disabled = true; b.innerHTML = ic('loader', 'spin') + 'Проверка…'; }
  let upd = null;
  try { upd = await T.updater.check(); } catch (e) { if (manual) toast('Не удалось проверить обновления\n' + (e?.message || e), 'error'); }
  finally { if (b) { b.disabled = false; b.innerHTML = ic('refresh') + 'Проверить обновления'; } }
  if (!upd) { if (manual) toast(`Установлена последняя версия (${S.info.version})`); return; }
  const ok = await modal({ title: `Доступна версия ${upd.version}`, icon: 'download', wide: true,
    body: `<p>Сейчас установлена ${esc(S.info.version)}.${S.info.portable ? ' Портативную версию нужно заменить вручную.' : ''}</p>${upd.body ? `<div class="card" style="max-height:40vh;overflow:auto;white-space:pre-wrap;font-size:13px">${esc(upd.body)}</div>` : ''}`,
    buttons: [{ label: 'Позже', value: false }, { label: S.info.portable ? 'Открыть страницу релиза' : 'Обновить и перезапустить', cls: 'primary', default: true, value: true }] });
  if (!ok) return;
  if (S.info.portable) return native.openUrl('https://github.com/TRUMEN228/mc-admin-panel/releases');
  toast(`Загрузка обновления ${upd.version}…`, 'info', 60000);
  try { await upd.downloadAndInstall(); await T.process.relaunch(); } catch (e) { toast('Обновление не установлено\n' + (e?.message || e), 'error'); }
}

// ---------- события ----------
T.event.listen('msg', ({ payload: m }) => {
  if (m.type === 'job') showJob(m.job);
  if (m.type === 'exported') modal({ title: 'Сборка сохранена', icon: 'checkCircle', body: `<p class="selectable">${esc(m.path)}</p><p>Отправьте этот файл владельцу сервера — в MC Admin Panel его можно поставить и на сервер, и на клиент.</p>`,
    buttons: [{ label: 'Показать в папке', value: 'show' }, { label: 'Готово', cls: 'primary', value: null }] }).then((v) => v === 'show' && T.opener.revealItemInDir(m.path));
  if (m.type === 'installed' && m.ok) alertDialog(m.message, 'Сборка установлена', 'checkCircle');
});
T.webview.getCurrentWebview().onDragDropEvent((e) => {
  const z = $('[data-drop]'); if (!z) return;
  const p = e.payload;
  if (p.type === 'enter' || p.type === 'over') z.classList.add('over');
  else if (p.type === 'leave') z.classList.remove('over');
  else if (p.type === 'drop') {
    z.classList.remove('over');
    const jars = (p.paths || []).filter((f) => /\.jar$/i.test(f));
    const packs = (p.paths || []).filter((f) => /\.mrpack$/i.test(f));
    if (jars.length) addFiles(jars);
    else if (packs.length) api('packs.importMrpack', { path: packs[0] }).then((x) => { loadPacks(); go('pack/' + x.id); });
    else toast('Перетащите файлы .jar или .mrpack', 'error');
  }
});

(async () => {
  try {
    [S.settings, S.info] = await Promise.all([call('settings.get'), call('system.info')]);
    document.addEventListener('pointermove', (e) => { const c = e.target.closest?.('.card, .metric'); if (!c) return; const r = c.getBoundingClientRect(); c.style.setProperty('--mx', e.clientX - r.left + 'px'); c.style.setProperty('--my', e.clientY - r.top + 'px'); }, { passive: true });
    applyTheme();
    await loadPacks();
    (await call('jobs')).filter((j) => j.status === 'running').forEach(showJob);
  } catch (e) { toast('Ошибка запуска\n' + e.message, 'error'); }
  await route();
  if (S.settings.autoUpdateCheck !== false && !S.info.debug) setTimeout(() => checkAppUpdate(false), 5000);
})();
