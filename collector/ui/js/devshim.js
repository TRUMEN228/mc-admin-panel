// Отладка в обычном браузере (dev-сборка, http://127.0.0.1:7791): заглушка API Tauri поверх HTTP.
if (!window.__TAURI__) {
  const listeners = [];
  const connect = () => {
    const ws = new WebSocket(`ws://${location.host}/api/events`);
    ws.onmessage = (e) => listeners.forEach((f) => f({ payload: JSON.parse(e.data) }));
    ws.onclose = () => setTimeout(connect, 2000);
  };
  connect();
  const noop = async () => {};
  window.__TAURI__ = {
    core: { invoke: async (_c, a) => {
      const r = await fetch('/api/call/' + a.cmd, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(a.args || {}) });
      const j = await r.json(); if (!r.ok) throw j.error; return j;
    } },
    event: { listen: async (_n, f) => listeners.push(f) },
    window: { getCurrentWindow: () => ({ setTheme: noop, setProgressBar: noop }) },
    webview: { getCurrentWebview: () => ({ onDragDropEvent: noop }) },
    dialog: { open: async (o) => prompt(o?.directory ? 'Путь к папке' : 'Путь к файлу') || null, save: async (o) => prompt('Сохранить как', o?.defaultPath || '') || null },
    opener: { openUrl: async (u) => window.open(u, '_blank'), revealItemInDir: noop },
    updater: { check: async () => null },
    process: { relaunch: noop },
  };
}
