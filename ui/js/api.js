// Связь с ядром: IPC Tauri в приложении или HTTP/WebSocket в веб-режиме.
const T = window.__TAURI__;
const NATIVE = !!T;

const TOKEN_KEY = 'mcpanel-token';
const token = () => { try { return localStorage.getItem(TOKEN_KEY) || ''; } catch { return ''; } };

async function call(cmd, args = {}) {
  try {
    if (NATIVE) return await T.core.invoke('call', { cmd, args });
    const r = await fetch('/api/call/' + cmd, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Token': token() }, body: JSON.stringify(args) });
    if (r.status === 401) { await askToken(); return call(cmd, args); }
    const j = await r.json();
    if (!r.ok) throw j.error || 'Ошибка';
    return j;
  } catch (e) {
    const msg = typeof e === 'string' ? e : e?.message || String(e);
    throw new Error(msg);
  }
}
/// Вызов с показом ошибки пользователю.
async function api(cmd, args) {
  try { return await call(cmd, args); } catch (e) { toast(e.message, 'error'); throw e; }
}

async function askToken() {
  const p = await promptDialog({ title: 'Вход в панель', label: 'Пароль веб-доступа', type: 'password', ok: 'Войти' });
  if (p == null) throw new Error('Требуется пароль');
  try { localStorage.setItem(TOKEN_KEY, p); } catch {}
}

function onEvent(cb) {
  if (NATIVE) { T.event.listen('msg', (e) => cb(e.payload)); return; }
  const connect = () => {
    const ws = new WebSocket(`${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/api/events?token=${encodeURIComponent(token())}`);
    ws.onmessage = (e) => cb(JSON.parse(e.data));
    ws.onclose = () => setTimeout(connect, 2000);
  };
  connect();
}

// ---- нативные возможности (в веб-режиме — запасные варианты) ----
const native = {
  async pickFolder(def) {
    if (!NATIVE) return promptDialog({ title: 'Папка', label: 'Путь к папке на компьютере с панелью', value: def || '' });
    return T.dialog.open({ directory: true, defaultPath: def || undefined, title: 'Выбор папки' });
  },
  async pickJars() {
    if (!NATIVE) return pickFilesBrowser();
    const r = await T.dialog.open({ multiple: true, filters: [{ name: 'Моды и плагины', extensions: ['jar'] }], title: 'Добавить .jar' });
    return r ? (Array.isArray(r) ? r : [r]) : null;
  },
  openPath(p) { return NATIVE ? api('open', { path: p }) : toast('Открытие папок доступно только в приложении', 'info'); },
  openUrl(u) { if (NATIVE) T.opener.openUrl(u); else window.open(u, '_blank'); },
  async copy(text) { try { await navigator.clipboard.writeText(text); toast('Скопировано', 'ok', 1500); } catch { toast('Не удалось скопировать', 'error'); } },
  onDrop(cb) {
    if (!NATIVE) return;
    T.webview.getCurrentWebview().onDragDropEvent((e) => cb(e.payload));
  },
};
// в браузере файлы передаются содержимым (base64)
function pickFilesBrowser() {
  return new Promise((res) => {
    const i = document.createElement('input');
    i.type = 'file'; i.multiple = true; i.accept = '.jar';
    i.onchange = () => res([...i.files]);
    i.click();
  });
}
async function uploadFiles(kind, id, files) {
  const n = [];
  for (const f of files) {
    if (typeof f === 'string') continue;
    const b = new Uint8Array(await f.arrayBuffer());
    let s = ''; for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
    await api(kind + '.upload', { id, name: f.name, data: btoa(s) });
    n.push(f.name);
  }
  return n;
}
async function addJars(kind, id, files) {
  if (!files || !files.length) return;
  const added = typeof files[0] === 'string' ? await api(kind + '.addFiles', { id, paths: files }) : await uploadFiles(kind, id, files);
  toast(`Добавлено: ${added.length}`, 'ok');
}
