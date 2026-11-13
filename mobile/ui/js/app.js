// MC Admin Mobile — интерфейс.
const T = window.__TAURI__;
const inv = (c, a) => T.core.invoke(c, a).catch((e) => { throw new Error(typeof e === 'string' ? e : e?.message || String(e)); });
const call = (cmd, args = {}) => inv('call', { cmd, args });
const api = (cmd, args) => call(cmd, args).catch((e) => { toast(e.message, 'error'); throw e; });
const ST = { running: ['Работает', 'green'], starting: ['Запуск…', 'yellow'], stopping: ['Остановка…', 'yellow'], stopped: ['Остановлен', ''] };
const LOADER = { vanilla: 'Vanilla', paper: 'Paper', fabric: 'Fabric', forge: 'Forge', neoforge: 'NeoForge' };
const S = { app: null, servers: [], online: false, view: 'list', id: null, tab: 'console', log: [] };
const srv = () => S.servers.find((s) => s.id === S.id);

// ---------- каркас ----------
function head(title, sub = '', left = '', right = '') {
  $('#top').innerHTML = `${left}<div style="flex:1;min-width:0"><h1>${esc(title)}</h1><div class="sub">${sub}</div></div>${right}`;
}
function nav() {
  const a = (v, icon, t) => `<a href="#" class="${S.view === v || (v === 'list' && S.view === 'server') ? 'on' : ''}" onclick="go('${v}');return false">${ic(icon)}${t}</a>`;
  $('#nav').innerHTML = S.app?.panels.length ? a('list', 'server', 'Серверы') + a('settings', 'settings', 'Настройки') : '';
}
function go(v, id) {
  if (v === 'server' && S.view !== 'server') history.pushState({ v }, '');
  else if (v !== 'server' && S.view === 'server' && history.state?.v === 'server') history.back();
  S.view = v; if (id) S.id = id;
  nav();
  ({ list: vList, server: vServer, settings: vSettings, add: vAdd }[v] || vList)();
}
const connSub = () => `<span class="conn ${S.online ? 'on' : ''}"></span>${S.online ? 'подключено' : 'нет связи с панелью'}`;
const panelName = () => S.app.panels.find((p) => p.id === S.app.active)?.name || 'Панель';

// ---------- привязка ----------
function vAdd() {
  const first = !S.app.panels.length;
  head(first ? 'MC Admin Mobile' : 'Добавить панель', '', first ? '' : `<button class="ghost icon" onclick="go('settings')">${ic('chevronRight').replace('class="ic ', 'style="transform:rotate(180deg)" class="ic ')}</button>`);
  $('#body').className = 'm-body';
  $('#body').innerHTML = `${first ? `<div class="hero"><div class="logo">${ic('server')}</div><h2 style="margin:0 0 6px;font:600 20px var(--font-display)">Управляйте серверами с телефона</h2>
    <p class="muted" style="margin:0">В MC Admin Panel на компьютере откройте «Настройки → Мобильное приложение → Подключить» и отсканируйте QR-код.</p></div>` : ''}
    <button class="primary big-btn" onclick="scanQr()">${ic('eye')}Сканировать QR-код</button>
    <div class="card" style="margin-top:16px"><div class="card-head"><h3>Или вручную</h3></div><div class="stack" style="gap:10px">
      <div class="field"><label>Адрес компьютера</label><input id="a-addr" placeholder="192.168.0.10 или 100.x.x.x" inputmode="decimal"></div>
      <div class="field"><label>Порт</label><input id="a-port" value="7777" inputmode="numeric"></div>
      <div class="field"><label>Код из панели</label><input id="a-code" placeholder="123 456" inputmode="numeric"></div>
      <button class="big-btn" onclick="pairManual()">Подключить</button></div></div>
    <p class="small dim" style="margin-top:14px">Дома телефон подключается по Wi-Fi. Чтобы управлять серверами вне дома, установите Tailscale на компьютер и телефон с одним аккаунтом.</p>`;
}
async function scanQr() {
  if (!T.barcodeScanner) return toast('Сканер доступен только на телефоне', 'error');
  let r;
  try {
    const perm = await T.barcodeScanner.checkPermissions().catch(() => 'granted');
    if (perm !== 'granted' && (await T.barcodeScanner.requestPermissions()) !== 'granted') throw new Error('Нет доступа к камере');
    // камера под прозрачным интерфейсом: поверх рамка и кнопка «Отмена»
    document.documentElement.classList.add('scanning');
    history.pushState({ scan: 1 }, '');
    r = await T.barcodeScanner.scan({ windowed: true, formats: ['QR_CODE'] });
  } catch (e) {
    if (!/cancel/i.test(String(e?.message || e))) toast(e.message || String(e), 'error');
  } finally { scanDone(); }
  if (!r) return;
  try {
    let d; try { d = JSON.parse(r.content); } catch { d = {}; }
    if (!d.c || !d.a) throw new Error('Это не QR-код MC Admin Panel');
    await doPair(d.a, d.p || 7777, d.c, d.n || '');
  } catch (e) { toast(e.message || String(e), 'error'); }
}
function scanDone() {
  if (!document.documentElement.classList.contains('scanning')) return;
  document.documentElement.classList.remove('scanning');
  if (history.state?.scan) history.back();
}
function scanCancel() { T.barcodeScanner.cancel().catch(() => {}); scanDone(); }
function pairManual() {
  const a = $('#a-addr').value.trim(), c = $('#a-code').value.replace(/\s/g, '');
  if (!a || c.length !== 6) return toast('Укажите адрес и 6-значный код', 'error');
  doPair([a], +$('#a-port').value || 7777, c, '');
}
async function doPair(addresses, port, code, name) {
  toast('Подключение…', 'info', 2500);
  const model = (navigator.userAgent.match(/Android [\d.]+; ([^;)]+)/) || [])[1]?.replace(/ Build.*/, '') || 'Телефон';
  try {
    const r = await inv('pair', { addresses, port, code, device: model, name });
    toast(`Подключено к «${r.name}»`);
    await refreshApp(); startEvents(); go('list');
  } catch (e) { toast(e.message, 'error'); }
}

// ---------- серверы ----------
async function loadServers() {
  try { S.servers = await call('servers.list'); S.online = true; } catch (e) { S.online = false; S.err = e.message; }
}
const sIco = (s) => `<div class="s-ico">${s.icon ? `<img src="${s.icon}" alt="">` : ic('server')}</div>`;
function card(s) {
  const st = ST[s.status] || ST.stopped, on = s.status !== 'stopped';
  return `<div class="card srv" onclick="go('server','${s.id}')">
    <div class="row nowrap">${sIco(s)}<div class="grow" style="min-width:0"><div class="row nowrap"><span class="dot ${s.status}"></span><span class="t ellipsis">${esc(s.name)}</span></div>
    <div class="small dim">${LOADER[s.loader] || s.loader} ${esc(s.mcVersion)} · порт ${s.port}</div></div><span class="badge ${st[1]}">${st[0]}</span></div>
    ${on ? `<div class="meters"><div>CPU<b>${s.stats ? fmtCpu(s.stats.cpu) + '%' : '—'}</b></div><div>Память<b>${s.stats ? fmtBytes(s.stats.mem) : '—'}</b></div><div>Игроки<b>${s.players.length}</b></div></div>` : ''}</div>`;
}
async function vList() {
  head(panelName(), connSub(), '', S.app.panels.length > 1 ? `<button class="ghost icon" onclick="pickPanel()">${ic('layers')}</button>` : '');
  $('#body').className = 'm-body';
  if (!S.servers.length) $('#body').innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`;
  await loadServers();
  if (S.view !== 'list') return;
  head(panelName(), connSub(), '', S.app.panels.length > 1 ? `<button class="ghost icon" onclick="pickPanel()">${ic('layers')}</button>` : '');
  $('#body').innerHTML = !S.online ? `<div class="empty">${ic('wifi')}<h3>Нет связи с панелью</h3><p>${esc(S.err || '')}</p><button onclick="vList()">${ic('refresh')}Повторить</button></div>`
    : S.servers.length ? updBanner() + S.servers.map(card).join('') : `<div class="empty">${ic('server')}<h3>Серверов нет</h3><p>Создайте сервер в панели на компьютере.</p></div>`;
}
async function pickPanel() {
  const v = await modal({ title: 'Панель', body: `<div class="stack">${S.app.panels.map((p) => `<button data-p="${p.id}" class="${p.id === S.app.active ? 'primary' : ''}" style="justify-content:flex-start">${ic('server')}${esc(p.name)}</button>`).join('')}</div>`,
    buttons: [{ label: 'Закрыть', value: null }], onOpen: (o) => $$('[data-p]', o).forEach((b) => (b.onclick = () => { o.remove(); choosePanel(b.dataset.p); })) });
  void v;
}
async function choosePanel(id) { await inv('select', { id }); await refreshApp(); S.servers = []; startEvents(); go('list'); }

// ---------- сервер ----------
/// Метрики сервера как на ПК: CPU, память из выделенной, игроки (с графиками) и аптайм.
function metrics(s) {
  const h = s.history || [], mem = s.stats ? Math.round(s.stats.mem / 1048576) : 0;
  const box = (k, v, sub, spark) => `<div class="metric"><div class="k">${k}</div><div class="v">${v}<small>${sub}</small></div>${spark}</div>`;
  return box('CPU', s.stats ? fmtCpu(s.stats.cpu) : '—', s.stats ? '%' : '', sparkline(h.map((x) => x[1]), 'var(--chart-1)', 100))
    + box('Память', s.stats ? fmtMb(mem) : '—', s.ram ? ' из ' + fmtMb(s.ram) : '', sparkline(h.map((x) => x[2]), 'var(--chart-2)', s.ram))
    + box('Игроки', s.players.length, s.maxPlayers ? ' / ' + s.maxPlayers : '', sparkline(h.map((x) => x[3]), 'var(--chart-3)'))
    + `<div class="metric"><div class="k">TPS</div><div class="v tps ${s.tps == null ? '' : s.tps >= 18 ? 'ok' : s.tps >= 15 ? 'warn' : 'bad'}">${s.status === 'running' && s.tps != null ? String(s.tps).replace('.', ',') : '—'}<small>${s.status === 'running' && s.started ? ' · ' + fmtDur(Date.now() - s.started) : ''}</small></div></div>`;
}
async function vServer() {
  const s = srv(); if (!s) return go('list');
  const st = ST[s.status] || ST.stopped, on = s.status !== 'stopped';
  head(s.name, `<span class="dot ${s.status}"></span>${st[0]} · ${LOADER[s.loader] || s.loader} ${esc(s.mcVersion)}`, `<button class="ghost icon" onclick="go('list')">${ic('chevronRight').replace('class="ic ', 'style="transform:rotate(180deg)" class="ic ')}</button>`);
  $('#body').className = 'm-body fill';
  $('#body').innerHTML = `<div class="acts" style="margin-bottom:10px">${on
      ? `<button onclick="act('restart')" ${s.status !== 'running' ? 'disabled' : ''}>${ic('restart')}Рестарт</button><button onclick="act('stop')">${ic('stop')}Стоп</button><button class="danger" onclick="killSrv()">${ic('skull')}Kill</button>`
      : `<button class="primary" style="grid-column:1/-1" onclick="act('start')" ${s.installed ? '' : 'disabled'}>${ic('play')}Запустить</button>`}</div>
    ${on ? `<div class="metrics" id="m-metrics">${metrics(s)}</div>` : ''}
    <div class="seg" id="tabs" style="margin-bottom:10px">${[['console', 'Консоль'], ['players', 'Игроки'], ['backups', 'Бэкапы']].map(([k, t]) => `<button data-t="${k}" class="${S.tab === k ? 'on' : ''}">${t}</button>`).join('')}</div>
    <div id="tab" style="flex:1;min-height:0;display:flex;flex-direction:column"></div>`;
  $$('#tabs button').forEach((b) => (b.onclick = () => { S.tab = b.dataset.t; $$('#tabs button').forEach((x) => x.classList.toggle('on', x === b)); tab(); }));
  tab();
}
function tab() { ({ console: tConsole, players: tPlayers, backups: tBackups }[S.tab])(); }
async function act(a) { try { await api('servers.' + a, { id: S.id }); toast({ start: 'Запуск…', stop: 'Остановка…', restart: 'Перезапуск…' }[a], 'info', 2000); } catch {} }
async function killSrv() { if (await confirmDialog('Процесс будет завершён без сохранения мира.', { title: 'Убить процесс?', ok: 'Убить', danger: true })) api('servers.kill', { id: S.id }); }

// подсказки команд по мере ввода (справочник как в панели)
function suggest() {
  const q = $('#cmd').value.trim().toLowerCase(), b = $('#sug'); if (!b) return;
  const list = q ? (window.COMMANDS || []).filter((c) => c[1].toLowerCase().startsWith(q) || c[1].toLowerCase().includes(' ' + q) || c[2].toLowerCase().includes(q)).slice(0, 8) : [];
  b.hidden = !list.length;
  b.innerHTML = list.map((c) => `<div onclick='pickSug(${attr(c[3] || c[1].split(' ')[0])})'><b>${esc(c[1])}</b><span>${esc(c[2])}</span></div>`).join('');
}
function pickSug(c) { const i = $('#cmd'); i.value = c.endsWith(' ') ? c : c + ' '; $('#sug').hidden = true; i.focus(); }
const QC = [['День', 'time set day'], ['Ночь', 'time set night'], ['Ясно', 'weather clear'], ['Сохранить', 'save-all'], ['Игроки', 'list']];
const cls = (l) => /WARN/.test(l) ? 'w' : /ERROR|Exception/.test(l) ? 'e' : l.startsWith('> ') ? 'c' : l.startsWith('[Панель]') ? 'p' : /joined the game|left the game/.test(l) ? 'j' : '';
async function tConsole() {
  if (!S.qc) S.qc = await call('settings.get').then((v) => (v.quickCommands || []).map((q) => (typeof q === 'string' ? [q, q] : [q.label || q.title || q.cmd, q.cmd]))).catch(() => []);
  const chips = [...S.qc, ...QC.filter(([, c]) => !S.qc.some(([, x]) => x === c))];
  $('#tab').innerHTML = `<div class="log" id="log"></div><div class="chips">${chips.map(([t, c]) => `<button class="sm" onclick='send(${attr(c)})'>${esc(t)}</button>`).join('')}</div>
    <div class="cmd"><div class="sug" id="sug" hidden></div><input id="cmd" placeholder="Команда без «/»" autocapitalize="none" autocorrect="off" spellcheck="false" autocomplete="off" oninput="suggest()" onblur="setTimeout(()=>$('#sug').hidden=true,150)" onkeydown="event.key==='Enter'&&send()"><button class="primary icon" style="width:48px" onclick="send()">${ic('send')}</button></div>`;
  S.log = await call('servers.log', { id: S.id }).catch(() => []);
  $('#log').innerHTML = S.log.slice(-300).map((l) => `<div class="${cls(l)}">${esc(l)}</div>`).join('');
  $('#log').scrollTop = 1e9;
}
function appendLog(l) {
  const el = $('#log'); if (!el) return;
  const stick = el.scrollTop + el.clientHeight > el.scrollHeight - 40;
  const d = document.createElement('div'); d.className = cls(l); d.textContent = l; el.append(d);
  if (el.childElementCount > 400) el.firstChild.remove();
  if (stick) el.scrollTop = 1e9;
}
async function send(c) {
  c = (c ?? $('#cmd').value).trim(); if (!c) return;
  if ($('#cmd')) $('#cmd').value = '';
  api('servers.command', { id: S.id, cmd: c }).catch(() => {});
}
async function tPlayers() {
  const s = srv(), on = s.status === 'running';
  $('#tab').innerHTML = `<div style="overflow:auto;flex:1">${s.players.length ? s.players.map((p) => `<div class="list-item"><img src="https://mc-heads.net/avatar/${encodeURIComponent(p)}/40" style="image-rendering:pixelated" onerror="this.style.visibility='hidden'"><div class="grow t">${esc(p)}</div>
    <button class="sm" onclick='pcmd("op ${esc(p)}")'>OP</button><button class="sm" onclick='pcmd("kick ${esc(p)}")'>Кик</button><button class="sm danger" onclick='pcmd("ban ${esc(p)}")'>Бан</button></div>`).join('')
    : `<div class="empty">${ic('users')}<p>${on ? 'Никого нет онлайн' : 'Сервер остановлен'}</p></div>`}
    ${on ? `<button style="width:100%;margin-top:10px" onclick="wl()">${ic('plus')}Добавить в белый список</button>` : ''}
    <div class="set-group-title" style="margin-top:16px">Статистика игроков</div><div id="pstats"><div class="empty">${ic('loader', 'spin')}</div></div></div>`;
  const st = await call('servers.playerStats', { id: s.id }).catch(() => []);
  const el = $('#pstats'); if (!el || S.tab !== 'players') return;
  el.innerHTML = st.length ? st.map((p) => `<div class="card pst" onclick='openPlayer(${attr(p.name)})'>
      <img src="https://mc-heads.net/avatar/${encodeURIComponent(p.name)}/40" onerror="this.style.visibility='hidden'">
      <div class="grow" style="min-width:0"><b class="ellipsis">${esc(p.name)}</b><div class="small dim">${fmtDur(p.total)} в игре · заходов ${p.sessions}</div></div>
      ${p.online ? '<span class="badge green">онлайн</span>' : `<span class="small dim">${ago(p.last)}</span>`}${ic('chevronRight')}</div>`).join('')
    : '<p class="small dim">Пока пусто — статистика появится, когда игроки зайдут на сервер</p>';
}
const ago = (t) => { const m = Math.floor((Date.now() - t) / 60000); return m < 1 ? 'только что' : m < 60 ? `${m} мин назад` : m < 1440 ? `${Math.floor(m / 60)} ч назад` : m < 43200 ? `${Math.floor(m / 1440)} д назад` : new Date(t).toLocaleDateString('ru-RU'); };

// ---------- инвентарь игрока (только просмотр) ----------
const MCA = 'https://assets.mcasset.cloud/1.21.4/assets/minecraft/textures';
const NAMES = window.MCNAMES || { item: {}, ench: {}, potion: {} };
const short = (id) => String(id || '').replace(/^minecraft:/, '');
// ключи перевода в именах (⟦filled_map.buried_treasure⟧) — по языковому файлу игры
const trText = (s) => (s ? String(s).replace(/⟦([^⟧]+)⟧/g, (_, k) => NAMES.tr?.[k] || NAMES.item[k.split('.').pop()] || k.split('.').pop().replace(/_/g, ' ')) : s);
const itemName = (id) => NAMES.item[short(id)] || short(id).replace(/_/g, ' ');
const ROMAN = ['', 'I', 'II', 'III', 'IV', 'V', 'VI', 'VII', 'VIII', 'IX', 'X'];
const enchName = (e) => `${NAMES.ench[short(e.id)] || short(e.id).replace(/_/g, ' ')} ${ROMAN[e.lvl] || e.lvl}`;
const MAXDUR = (() => {
  const m = { bow: 384, crossbow: 465, trident: 250, shield: 336, elytra: 432, fishing_rod: 64, shears: 238, flint_and_steel: 64, mace: 500, brush: 64, turtle_helmet: 275 };
  const t = { wooden: 59, stone: 131, iron: 250, golden: 32, diamond: 1561, netherite: 2031 };
  for (const [k, v] of Object.entries(t)) for (const x of ['sword', 'pickaxe', 'axe', 'shovel', 'hoe']) m[`${k}_${x}`] = v;
  const a = { leather: [55, 80, 75, 65], chainmail: [165, 240, 225, 195], iron: [165, 240, 225, 195], golden: [77, 112, 105, 91], diamond: [363, 528, 495, 429], netherite: [407, 592, 555, 481] };
  for (const [k, v] of Object.entries(a)) ['helmet', 'chestplate', 'leggings', 'boots'].forEach((x, i) => (m[`${k}_${x}`] = v[i]));
  return m;
})();
const maxDur = (it) => it.maxDamage || MAXDUR[short(it.id)] || 0;
const PV = {};
function itemIcon(id) {
  const n = short(id);
  return `<img src="https://mc.nerothe.com/img/1.21.4/minecraft_${n}.png" alt="" onerror="if(!this.dataset.f){this.dataset.f=1;this.src='${MCA}/item/${n}.png'}else if(this.dataset.f==1){this.dataset.f=2;this.src='${MCA}/block/${n}.png'}else{this.style.display='none'}">`;
}
function pslot(kind, slot, ghost) {
  const it = kind === 'equipment' ? PV.d.equipment[slot] : PV.d[kind].find((x) => x.slot === slot);
  const q = typeof slot === 'string' ? `'${slot}'` : slot;
  return `<button class="mc-slot ${it ? '' : 'empty'} ${it?.enchanted ? 'ench' : ''} ${PV.sel === kind + slot ? 'sel' : ''}" onclick="pickItem('${kind}', ${q})">
    ${it ? itemIcon(it.id) + (it.name ? `<img class="mc-tag" src="${MCA}/item/name_tag.png" alt="">` : '') + (it.count > 1 ? `<b>${it.count}</b>` : '') + (it.damage && maxDur(it) ? `<i class="dur" style="--d:${Math.max(0, 1 - it.damage / maxDur(it))}"></i>` : '') : ghost ? `<img class="mc-ghost" src="${MCA}/gui/sprites/container/slot/${ghost}.png" alt="">` : ''}</button>`;
}
// ---------- цвет имени как в игре: свой цвет имени → редкость (зачарование поднимает на ступень) ----------
const MC_COLORS = { black: '#000000', dark_blue: '#0000aa', dark_green: '#00aa00', dark_aqua: '#00aaaa', dark_red: '#aa0000', dark_purple: '#aa00aa', gold: '#ffaa00', gray: '#aaaaaa',
  dark_gray: '#555555', blue: '#5555ff', green: '#55ff55', aqua: '#55ffff', red: '#ff5555', light_purple: '#ff55ff', yellow: '#ffff55', white: '#ffffff' };
const RARITY_COLOR = { common: '#ffffff', uncommon: '#ffff55', rare: '#55ffff', epic: '#ff55ff' };
const BASE_RARITY = (() => {
  const r = {};
  'enchanted_book experience_bottle dragon_breath heart_of_the_sea totem_of_undying nether_star creeper_head zombie_head skeleton_skull wither_skeleton_skull player_head piglin_head trial_key ominous_trial_key ominous_bottle'.split(' ').forEach((k) => (r[k] = 'uncommon'));
  'beacon conduit end_crystal golden_apple trident sniffer_egg recovery_compass'.split(' ').forEach((k) => (r[k] = 'rare'));
  'enchanted_golden_apple dragon_egg dragon_head elytra mace heavy_core command_block chain_command_block repeating_command_block structure_block jigsaw barrier light debug_stick knowledge_book'.split(' ').forEach((k) => (r[k] = 'epic'));
  return r;
})();
function nameColor(it) {
  if (it.nameColor) return MC_COLORS[it.nameColor] || (/^#[0-9a-f]{6}$/i.test(it.nameColor) ? it.nameColor : '#ffffff');
  const n = short(it.id);
  let r = it.rarity || BASE_RARITY[n] || (/^music_disc_/.test(n) ? 'uncommon' : 'common');
  if (it.enchanted) r = { common: 'rare', uncommon: 'rare', rare: 'epic', epic: 'epic' }[r] || r;
  return RARITY_COLOR[r] || '#ffffff';
}
const nameHtml = (it, extra = '') => `<div class="t-name ${it.name ? 'custom' : ''}" style="color:${nameColor(it)}">${esc(trText(it.name) || itemName(it.id))}${extra}</div>`;
function itemInfo(it) {
  if (!it) return '<p class="small dim" style="margin:0">Нажмите на предмет, чтобы увидеть подробности</p>';
  const l = [nameHtml(it, it.count > 1 ? ` <span class="t-dim">×${it.count}</span>` : '')];
  [...(it.enchants || []), ...(it.stored || [])].forEach((e) => l.push(`<div class="t-ench ${/curse/.test(e.id) ? 'curse' : ''}">${esc(enchName(e))}</div>`));
  if (it.potion) l.push(`<div class="t-blue">${esc(NAMES.potion[short(it.potion).replace(/^(long|strong)_/, '')] || short(it.potion))}</div>`);
  (it.lore || []).map(trText).forEach((x) => l.push(`<div class="t-lore">${esc(x)}</div>`));
  if (it.contents?.length) l.push(...it.contents.map((c) => `<div class="t-dim">${esc(trText(c.name) || itemName(c.id))} ×${c.count}</div>`));
  if (it.unbreakable) l.push('<div class="t-blue">Неразрушимый</div>');
  if (it.damage) l.push(maxDur(it) ? `<div class="t-dim">Прочность: ${maxDur(it) - it.damage} / ${maxDur(it)}</div>` : `<div class="t-dim">Износ: ${it.damage}</div>`);
  l.push(`<div class="t-id">${esc(it.id)}</div>`);
  return l.join('');
}
function pickItem(kind, slot) {
  PV.sel = kind + slot;
  const it = kind === 'equipment' ? PV.d.equipment[slot] : PV.d[kind].find((x) => x.slot === slot);
  $$('.mc-slot.sel').forEach((x) => x.classList.toggle('sel', false));
  event?.currentTarget?.classList.add('sel');
  const box = $('#pv-info'); if (box) box.innerHTML = itemInfo(it);
}
async function openPlayer(name) {
  let d;
  try { d = await call('players.data', { id: S.id, name }); } catch (e) { return toast(e.message, 'error'); }
  Object.assign(PV, { d, name, sel: '' });
  const bar = (val, full, half, empty) => Array.from({ length: 10 }, (_, i) => { const v = val - i * 2; return `<img src="${MCA}/gui/sprites/hud/${v >= 2 ? full : v === 1 ? half : empty}.png" alt="">`; }).join('');
  const row = (k, from) => Array.from({ length: 9 }, (_, i) => pslot(k, from + i)).join('');
  const gm = ['Выживание', 'Творчество', 'Приключение', 'Наблюдатель'][d.gameMode] || '';
  await modal({ title: name, icon: 'users', body: `<div class="small dim" style="margin:-6px 0 10px">${gm}${d.running ? ' · последнее сохранённое состояние' : ''}</div>
    <div class="mc-inv m">
      <div class="mc-top"><div class="mc-eq row">${[['head', 'helmet'], ['chest', 'chestplate'], ['legs', 'leggings'], ['feet', 'boots'], ['offhand', 'shield']].map(([k, g]) => pslot('equipment', k, g)).join('')}</div></div>
      <div class="mc-grid">${row('inventory', 9)}${row('inventory', 18)}${row('inventory', 27)}</div>
      <div class="mc-hud"><div class="mc-hud-row"><div class="mc-bar">${bar(Math.round(d.health), 'heart/full', 'heart/half', 'heart/container')}</div><span class="grow"></span><div class="mc-bar rtl">${bar(Math.round(d.food), 'food_full', 'food_half', 'food_empty')}</div></div>
        <div class="mc-xp"><div class="mc-xp-bar" style="--p:${Math.max(0, Math.min(1, d.xpP || 0))}"></div><span class="mc-lvl">${d.xpLevel}</span></div></div>
      <div class="mc-grid">${row('inventory', 0)}</div>
      <div class="mc-title">Эндер-сундук</div>
      <div class="mc-grid">${row('ender', 0)}${row('ender', 9)}${row('ender', 18)}</div></div>
    <div class="mc-tip-inline" id="pv-info" style="margin-top:10px">${itemInfo(null)}</div>`,
    buttons: [{ label: 'Закрыть', cls: 'primary', value: null }] });
}
async function pcmd(c) { if (await confirmDialog(c, { title: 'Выполнить команду?', ok: 'Выполнить' })) send(c); }
async function wl() { const n = await promptDialog({ title: 'Белый список', label: 'Ник игрока', ok: 'Добавить' }); if (n) send('whitelist add ' + n.trim()); }
async function tBackups() {
  $('#tab').innerHTML = `<div class="empty">${ic('loader', 'spin')}</div>`;
  const r = await call('backups.list', { id: S.id }).catch((e) => ({ error: e.message }));
  if (S.tab !== 'backups') return;
  if (r.error) { $('#tab').innerHTML = `<p>${esc(r.error)}</p>`; return; }
  $('#tab').innerHTML = `<button class="primary" style="width:100%;margin-bottom:10px;flex-shrink:0" onclick="mkBackup()">${ic('save')}Создать бэкап</button>
    <div style="overflow:auto;flex:1">${r.list.map((b) => `<div class="list-item">${ic('archive')}<div class="grow"><div class="t ellipsis">${esc(b.file)}</div><div class="d">${fmtAgo(b.time)} · ${fmtBytes(b.size)}</div></div></div>`).join('') || `<div class="empty">${ic('archive')}<p>Бэкапов нет</p></div>`}
    <p class="small dim">Слотов: ${r.list.length} из ${r.max}. Восстановление — в панели на компьютере.</p></div>`;
}
async function mkBackup() { await api('backups.create', { id: S.id, note: 'phone' }).catch(() => {}); toast('Бэкап создаётся…', 'info'); }

// ---------- настройки ----------
const THEMES = [['hybrid', 'Гибрид', '#0a0b0f', 'linear-gradient(90deg,#10b981,#06b6d4)', '#2b2e3a'], ['light', 'Светлая', '#eef1f7', 'linear-gradient(90deg,#6366f1,#14b8a6)', '#cbd5e1'],
  ['glow', 'Неон', '#050507', 'linear-gradient(90deg,#a855f7,#f97316)', '#2a2a33'], ['calm', 'Спокойная', '#262b35', '#88c0d0', '#3b4252']];
const themePick = (cur) => `<div class="theme-pick" id="st-theme">${THEMES.map(([k, t, bg, ac, ln]) => `<button class="${cur === k ? 'on' : ''}" data-v="${k}" onclick="setThemeId('${k}')"><div class="sw" style="background:${bg}"><i style="background:${ac}"></i><i style="background:${ln}"></i><i style="background:${ln}"></i></div><span>${t}</span></button>`).join('')}</div>`;
const THEME_KEY = 'mcm-theme';
const themeId = () => { try { const t = localStorage.getItem(THEME_KEY); return THEMES.some((x) => x[0] === t) ? t : 'hybrid'; } catch { return 'hybrid'; } };
function setThemeId(t) { try { localStorage.setItem(THEME_KEY, t); } catch {} document.documentElement.dataset.theme = t; $$('#st-theme button').forEach((x) => x.classList.toggle('on', x.dataset.v === t)); }
function vSettings() {
  head('Настройки');
  $('#body').className = 'm-body';
  const a = S.app;
  const swi = (k, t, d) => `<div class="set-item" style="padding:12px 14px"><div class="txt"><b>${t}</b><span>${d}</span></div>${sw('s-' + k, a[k], `onchange="setOpt('${k}',this.checked)"`)}</div>`;
  $('#body').innerHTML = `<div class="set-group-title">Панели</div><div class="set-list">${a.panels.map((p) => `<div class="set-item" style="padding:12px 14px">${ic('server')}<div class="txt"><b>${esc(p.name)}${p.id === a.active ? ' ✓' : ''}</b><span>${p.addresses.map(esc).join(', ')} · порт ${p.port}</span></div>
      <button class="sm ghost" onclick='delPanel(${attr(p.id)},${attr(p.name)})'>${ic('trash')}</button></div>`).join('')}</div>
    <button style="width:100%;margin-top:8px" onclick="go('add')">${ic('plus')}Добавить панель</button>
    <div class="set-group-title">Оформление</div>${themePick(themeId())}
    <div class="set-group-title">Уведомления</div><div class="set-list">
      ${swi('background', 'Работать в фоне', 'Держать связь с панелью, когда приложение закрыто (значок в шторке)')}
      ${swi('notifyCrash', 'Сбои серверов', 'Сервер аварийно завершился')}
      ${swi('notifyPlayers', 'Игроки', 'Кто-то зашёл на сервер')}
      ${swi('notifyJobs', 'Ошибки задач', 'Не удался бэкап или установка')}
      ${swi('notifyAlerts', 'Лаги и ресурсы', 'TPS ниже 15, мало памяти или места на диске ПК')}</div>
    <div class="set-group-title">О приложении</div>${updBanner()}
    <button style="width:100%" onclick="checkUpd(true)">${ic('refresh')}Проверить обновления</button>
    <p class="small dim" style="margin-top:16px">MC Admin Mobile ${esc(S.upd?.current || '')} · компаньон MC Admin Panel</p>`;
}
async function setOpt(k, v) { S.app = await inv('settings', { patch: { [k]: v } }); }
async function delPanel(id, name) {
  if (!(await confirmDialog(`Телефон забудет «${name}». Чтобы подключиться снова, понадобится новый код.`, { title: 'Удалить панель?', ok: 'Удалить', danger: true }))) return;
  await inv('remove', { id }); await refreshApp(); S.servers = [];
  if (S.app.panels.length) { startEvents(); go('settings'); } else go('add');
}

// ---------- события панели ----------
function startEvents() { inv('events', { types: 'status,stats,players,crash,job,servers,log,alert' }).catch(() => {}); }
T.event.listen('conn', ({ payload }) => {
  const was = S.online; S.online = payload.online;
  if (S.view === 'list') { $('#top .sub') && ($('#top .sub').innerHTML = connSub()); if (!was && S.online) vList(); }
});
T.event.listen('msg', ({ payload: m }) => {
  const s = S.servers.find((x) => x.id === m.id);
  if (m.type === 'log') { if (S.view === 'server' && S.id === m.id && S.tab === 'console') appendLog(m.line); return; }
  if (m.type === 'servers') { loadServers().then(() => S.view === 'list' && vList()); return; }
  if (m.type === 'alert') { toast(`${m.title}\n${m.text}`, 'error', 7000); return; }
  if (m.type === 'crash' && s) toast(`«${s.name}» аварийно завершился\n${m.diag?.hints?.[0]?.title || ''}`, 'error');
  if (m.type === 'job' && m.job.status !== 'running') toast(`${m.job.title}\n${m.job.log.slice(-1)[0] || ''}`, m.job.status === 'error' ? 'error' : 'ok', 3500);
  if (!s) return;
  if (m.type === 'status') { s.status = m.status; if (m.status === 'starting' || (m.status === 'running' && !s.started)) s.started = m.started || Date.now(); if (m.status === 'stopped') { s.stats = null; s.players = []; } }
  if (m.type === 'stats') {
    Object.assign(s, { stats: m.stats, players: m.players, status: m.status, tps: m.tps });
    if (m.stats) { (s.history ||= []).push([m.t, m.stats.cpu, Math.round(m.stats.mem / 1048576), m.players.length]); if (s.history.length > 180) s.history.shift(); }
  }
  if (m.type === 'players') s.players = m.players;
  if (S.view === 'list') { const html = S.servers.map(card).join(''); if ($('#body').innerHTML !== html) $('#body').innerHTML = html; }
  if (S.view === 'server' && S.id === m.id) {
    if (m.type === 'stats') { const c = $('#m-metrics'); if (c) { c.classList.add('no-anim'); c.innerHTML = metrics(s); } else vServer(); }
    else if (m.type === 'status') vServer();
    else if (m.type === 'players' && S.tab === 'players') tPlayers();
  }
});
document.addEventListener('visibilitychange', () => { if (!document.hidden && S.app?.panels.length) { startEvents(); if (S.view === 'list') vList(); } });
// системная кнопка «Назад»
window.addEventListener('popstate', () => {
  if (document.documentElement.classList.contains('scanning')) { document.documentElement.classList.remove('scanning'); T.barcodeScanner.cancel().catch(() => {}); return; }
  if (S.view === 'server') go('list');
});

async function refreshApp() { S.app = await inv('state'); }
// ---------- обновление приложения ----------
const updBanner = () => (S.upd?.available ? `<div class="card upd" onclick="openUpd()">${ic('download')}<div class="grow"><b>Доступна версия ${esc(S.upd.version)}</b><div class="small dim">Нажмите, чтобы скачать · ${fmtBytes(S.upd.size || 0)}</div></div>${ic('chevronRight')}</div>` : '');
function openUpd() { T.opener.openUrl(S.upd.url); toast('Когда файл скачается, откройте его — Android предложит обновить приложение', 'info', 7000); }
async function checkUpd(manual) {
  try {
    const last = +(localStorage.getItem('mcm-upd-at') || 0);
    if (!manual && Date.now() - last < 12 * 3600e3) { S.upd = JSON.parse(localStorage.getItem('mcm-upd') || 'null'); return; }
    S.upd = await inv('update_check');
    localStorage.setItem('mcm-upd-at', String(Date.now())); localStorage.setItem('mcm-upd', JSON.stringify(S.upd));
    if (manual) toast(S.upd.available ? `Доступна версия ${S.upd.version}` : 'Установлена последняя версия', S.upd.available ? 'info' : 'ok');
    if (S.view === 'list') vList(); else if (S.view === 'settings') vSettings();
  } catch (e) { if (manual) toast('Не удалось проверить обновления\n' + e.message, 'error'); }
}
(async () => {
  document.documentElement.dataset.theme = themeId();
  await refreshApp();
  if (S.app.panels.length) { startEvents(); go('list'); } else go('add');
  checkUpd(false);
})();
