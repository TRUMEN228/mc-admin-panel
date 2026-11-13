// Ручная проверка окна игрока на выгрузке настоящих данных: MCPANEL_REAL_PLAYER=путь.json npx playwright test real-player
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const FILE = process.env.MCPANEL_REAL_PLAYER;
const ROOT = path.resolve(import.meta.dirname, '..');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2' };
test.skip(!FILE, 'нет выгрузки настоящих данных');

test('настоящий игрок: имена предметов', async ({ page }) => {
  const d = JSON.parse(fs.readFileSync(FILE, 'utf8'));
  const API = { 'settings.get': { theme: 'hybrid' }, 'system.info': {}, jobs: [], 'servers.log': [], 'players.data': d, 'servers.lists': { ops: [], whitelist: [], 'banned-players': [], 'banned-ips': [] },
    'servers.playerStats': [{ name: 'Intelligent_Man', total: 1, sessions: 1, last: Date.now(), first: Date.now() }],
    'servers.list': [{ id: 's', name: 'S', loader: 'fabric', mcVersion: '1.21.11', port: 1, ram: 1024, status: 'stopped', players: [], history: [], installed: true }] };
  await page.route('http://app.test/**', async (route) => {
    const u = new URL(route.request().url());
    if (u.pathname.startsWith('/api/call/')) return route.fulfill({ json: API[u.pathname.slice(10)] ?? {} });
    const f = path.join(ROOT, 'ui', u.pathname === '/' ? 'index.html' : decodeURIComponent(u.pathname));
    return fs.existsSync(f) ? route.fulfill({ body: fs.readFileSync(f), contentType: TYPES[path.extname(f)] }) : route.fulfill({ status: 404, body: '' });
  });
  await page.routeWebSocket(/\/api\/events/, () => {});
  await page.goto('http://app.test/#server/s/players');
  await page.locator('.ptable a').first().click();
  const named = page.locator('.mc-slot:has(.mc-tag)');
  console.log('переименованных в сетке:', await named.count());
  await named.first().hover();
  await expect(page.locator('#inv-tip .t-name.custom')).toBeVisible();
  console.log('подсказка:', (await page.locator('#inv-tip').innerText()).replace(/\n/g, ' | '));
  console.log('цвет имени:', await page.locator('#inv-tip .t-name').evaluate((e) => getComputedStyle(e).color + ' ' + getComputedStyle(e).fontStyle));
  await page.waitForTimeout(2500);
  await page.screenshot({ path: 'test-results/real-player.png' });
});
