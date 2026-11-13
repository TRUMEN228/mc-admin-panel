// Раскладка: колонки с прокруткой не должны вылезать за рамку окна (низкое окно, длинные списки).
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(import.meta.dirname, '..');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2' };
const SRV = (i) => ({ id: 's' + i, name: 'Сервер ' + i, loader: 'paper', mcVersion: '1.21.4', port: 25565 + i, ram: 4096, status: 'stopped', players: [], history: [], installed: true, icon: '' });
const API = { 'settings.get': { theme: 'hybrid' }, 'system.info': { memTotal: 17179869184 }, 'servers.list': Array.from({ length: 8 }, (_, i) => SRV(i)), jobs: [], 'servers.log': [],
  'servers.lists': { ops: [], whitelist: [], 'banned-players': [], 'banned-ips': [] }, 'servers.playerStats': [], 'tools.status': {} };

async function open(page, hash) {
  await page.route('http://app.test/**', async (route) => {
    const u = new URL(route.request().url());
    if (u.pathname.startsWith('/api/call/')) return route.fulfill({ json: API[u.pathname.slice(10)] ?? {} });
    const f = path.join(ROOT, 'ui', u.pathname === '/' ? 'index.html' : decodeURIComponent(u.pathname));
    return fs.existsSync(f) ? route.fulfill({ body: fs.readFileSync(f), contentType: TYPES[path.extname(f)] }) : route.fulfill({ status: 404, body: '' });
  });
  await page.routeWebSocket(/\/api\/events/, () => {});
  await page.addInitScript(() => localStorage.setItem('mcpanel-feed', JSON.stringify(Array.from({ length: 30 }, (_, i) => ({ k: 'ok', i: 'users', t: 'Событие номер ' + i, at: Date.now() })))));
  await page.goto('http://app.test/' + hash);
}
// нижний край прокручиваемой колонки — внутри окна контента
async function insideFrame(page, sel) {
  const [box, frame] = await Promise.all([page.locator(sel).first().boundingBox(), page.locator('.content').boundingBox()]);
  return box.y + box.height <= frame.y + frame.height + 1;
}

test.use({ viewport: { width: 1280, height: 640 } });
test('обзор: правая колонка не вылезает за рамку', async ({ page }) => {
  await open(page, '#dash');
  await expect(page.locator('#feed li')).toHaveCount(12);
  await page.waitForTimeout(800);
  await page.screenshot({ path: 'test-results/layout-dash.png' });
  await page.locator('.dash-side').evaluate((e) => (e.scrollTop = e.scrollHeight));
  await page.locator('.dash-main .tab-scroll').evaluate((e) => (e.scrollTop = e.scrollHeight));
  await page.waitForTimeout(200);
  await page.screenshot({ path: 'test-results/layout-dash-end.png' });
  expect(await insideFrame(page, '.dash-side')).toBe(true);
  expect(await insideFrame(page, '.dash-main')).toBe(true);
});
test('консоль: панель быстрых команд не вылезает за рамку', async ({ page }) => {
  API['settings.get'] = { theme: 'hybrid', quickCommands: Array.from({ length: 30 }, (_, i) => ({ label: 'Команда ' + i, cmd: 'say ' + i })) };
  await open(page, '#server/s1/console');
  await page.locator('.qc-panel').evaluate((e) => (e.scrollTop = e.scrollHeight));
  await page.waitForTimeout(300);
  const qb = await page.locator('.qc-panel').boundingBox();
  await page.mouse.move(qb.x + qb.width / 2, qb.y + qb.height - 30);
  await page.waitForTimeout(200);
  await page.screenshot({ path: 'test-results/layout-console.png', clip: { x: qb.x - 10, y: qb.y + qb.height - 260, width: qb.width + 20, height: 270 } });
  expect(await insideFrame(page, '.qc-panel')).toBe(true);
  expect(await page.locator('.qc-panel').evaluate((e) => e.scrollHeight > e.clientHeight)).toBe(true);
});
test('сервер: вкладки не вылезают за рамку', async ({ page }) => {
  for (const tab of ['players', 'tools', 'config']) {
    await open(page, `#server/s1/${tab}`);
    await page.waitForTimeout(400);
    await page.screenshot({ path: `test-results/layout-${tab}.png` });
    expect(await insideFrame(page, '#tab')).toBe(true);
  }
});
