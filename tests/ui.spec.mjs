// Автотесты интерфейса: панель (веб-режим), мобильное приложение и коллектор с подставным API.
// Ловят «белый экран» и ошибки JS: страница должна отрисоваться без ошибок в консоли.
import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(import.meta.dirname, '..');
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.woff2': 'font/woff2', '.png': 'image/png', '.svg': 'image/svg+xml' };
const ICON = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';

const SERVERS = [
  { id: 'a1', name: 'Survival', loader: 'fabric', mcVersion: '1.21.11', port: 25565, ram: 6144, status: 'running', players: ['Steve'], stats: { cpu: 12.3, mem: 2147483648 }, tps: 19.8, started: Date.now() - 3600e3, history: [[1, 10, 2000, 1], [2, 12, 2048, 1]], installed: true, icon: ICON },
  { id: 'b2', name: 'Creative', loader: 'paper', mcVersion: '1.21.4', port: 25566, ram: 4096, status: 'stopped', players: [], stats: null, tps: null, history: [], installed: true, icon: '' },
];
const PANEL = {
  'settings.get': { theme: 'hybrid', quickCommands: [{ label: 'Привет', cmd: 'say hi' }], maxBackups: 5, serversDir: 'C:/mc' },
  'settings.save': { theme: 'glow' },
  'system.info': { memTotal: 17179869184, cores: 8, version: '0.0.0-test', dataDir: 'C:/data' },
  'servers.list': SERVERS, jobs: [], 'servers.log': ['[12:00:00] [Server thread/INFO]: Done (3.2s)!'],
  'servers.lists': { ops: [], whitelist: [], 'banned-players': [{ name: 'Griefer', reason: 'гриф' }], 'banned-ips': [] },
  'servers.playerStats': [{ name: 'Steve', first: Date.now() - 864e5, last: Date.now(), total: 7200e3, sessions: 3, online: true }],
  'players.data': { uuid: 'u-1', running: false, online: false, inventory: [{ slot: 0, id: 'minecraft:diamond_sword', count: 1, enchanted: true, name: 'Экскалибур', enchants: [{ id: 'minecraft:sharpness', lvl: 5 }, { id: 'minecraft:unbreaking', lvl: 3 }], lore: ['Меч короля'], damage: 120, maxDamage: 1561 }, { slot: 9, id: 'minecraft:stone', count: 64 }], ender: [], equipment: { head: { slot: 'head', id: 'minecraft:diamond_helmet', count: 1 } }, health: 17, food: 13, xpLevel: 27, xpP: 0.6, gameMode: 0, pos: [10, 64, -5], dimension: 'minecraft:overworld' },
  'tools.status': { bluemap: true, accepted: true, mapUrl: 'http://localhost:8100', mapPort: 8100, chunky: false },
  'mods.list': { list: [] }, 'catalog.providers': [], 'mobile.devices': [], 'backups.list': { dir: '', list: [], max: 5 }, 'server.props': {},
};
const errorsOf = (page) => { const errs = []; page.on('pageerror', (e) => errs.push(e.message + ' @ ' + (e.stack || '').split('\n').slice(1, 3).join(' '))); page.on('console', (m) => m.type() === 'error' && !/Failed to load resource|net::|mc-heads/.test(m.text()) && errs.push(m.text())); return errs; };
async function serve(page, dir, api) {
  await page.route('http://app.test/**', async (route) => {
    const u = new URL(route.request().url());
    if (u.pathname.startsWith('/api/call/')) {
      const cmd = u.pathname.slice(10);
      return route.fulfill({ json: api[cmd] ?? {} });
    }
    const f = path.join(ROOT, dir, u.pathname === '/' ? 'index.html' : decodeURIComponent(u.pathname));
    if (!fs.existsSync(f)) return route.fulfill({ status: 404, body: '' });
    return route.fulfill({ body: fs.readFileSync(f), contentType: TYPES[path.extname(f)] || 'application/octet-stream' });
  });
  await page.routeWebSocket(/\/api\/events/, () => {});
}

test.describe('Панель', () => {
  test('обзор: карточки серверов, лента событий, иконка', async ({ page }) => {
    const errs = errorsOf(page);
    await serve(page, 'ui', PANEL);
    await page.goto('http://app.test/#dash');
    await expect(page.locator('.srv-card')).toHaveCount(2);
    await expect(page.locator('.srv-card .srv-img')).toHaveCount(1);
    await expect(page.locator('#feed')).toBeVisible();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'hybrid');
    expect(errs).toEqual([]);
  });
  test('сервер: вкладки консоли, игроков, инструментов и параметров', async ({ page }) => {
    const errs = errorsOf(page);
    await serve(page, 'ui', PANEL);
    for (const tab of ['console', 'players', 'tools', 'config']) {
      await page.goto(`http://app.test/#server/a1/${tab}`);
      await expect(page.locator('#tab')).not.toBeEmpty();
    }
    await page.goto('http://app.test/#server/a1/players');
    await expect(page.locator('.ptable tbody tr')).toHaveCount(1);
    await expect(page.locator('#srv-metrics .tps')).toHaveText(/19,8/);
    await page.locator('.ptable a').first().click();
    await expect(page.locator('.mc-slot')).toHaveCount(5 + 36 + 27);
    await expect(page.locator('.mc-slot.ench')).toHaveCount(1);
    await page.locator('.mc-slot.ench').hover(); await expect(page.locator('#inv-tip')).toContainText('Острота V');
    await page.waitForTimeout(2500); await page.screenshot({ path: 'test-results/player-window.png' });
    await expect(page.locator('#pv-dirty')).toBeHidden();
    await page.locator('.mc-slot.ench').click();
    await expect(page.locator('.ench-row')).toHaveCount(2);
    await page.waitForTimeout(400); await page.screenshot({ path: 'test-results/item-edit.png' });
    await page.locator('#ie-name').fill('Новое имя');
    await page.getByRole('button', { name: 'Готово' }).click();
    await expect(page.locator('#pv-dirty')).toBeVisible();
    await expect(page.locator('.mc-slot.dirty')).toHaveCount(1);
    // подсказки: русское название и слот брони
    await page.locator('.mc-slot[data-k="inventory"][data-s="9"]').hover();
    await expect(page.locator('#inv-tip')).toContainText('Камень');
    await page.locator('.mc-slot[data-k="equipment"][data-s="head"]').hover();
    await expect(page.locator('#inv-tip')).toContainText('Алмазный шлем');
    // перетаскивание меча в эндер-сундук
    await page.locator('.mc-slot[data-k="inventory"][data-s="0"]').dragTo(page.locator('.mc-slot[data-k="ender"][data-s="4"]'));
    await expect(page.locator('.mc-slot[data-k="ender"][data-s="4"] img:not(.mc-tag)')).toHaveCount(1);
    await expect(page.locator('.mc-slot[data-k="ender"][data-s="4"] .mc-tag')).toHaveCount(1);
    await expect(page.locator('.mc-slot[data-k="inventory"][data-s="0"].empty')).toHaveCount(1);
    expect(errs).toEqual([]);
  });
  test('клик по карточке открывает сервер; настройки и темы', async ({ page }) => {
    const errs = errorsOf(page);
    await serve(page, 'ui', PANEL);
    await page.goto('http://app.test/#dash');
    await page.locator('#card-b2 .stats3').click();
    await expect(page).toHaveURL(/#server\/b2/);
    await page.goto('http://app.test/#settings');
    await expect(page.locator('#st-theme button')).toHaveCount(4);
    await page.locator('#st-theme button[data-v="glow"]').click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'glow');
    expect(errs).toEqual([]);
  });
});

test.describe('Мобильное приложение', () => {
  test.use({ viewport: { width: 400, height: 860 } });
  test('список серверов и настройки', async ({ page }) => {
    const errs = errorsOf(page);
    await page.addInitScript(({ servers }) => {
      const state = { panels: [{ id: 'p', name: 'PC', addresses: ['10.0.0.2'], port: 7777, good: '10.0.0.2' }], active: 'p', notifyCrash: true, notifyPlayers: true, notifyJobs: true, notifyAlerts: true, background: true };
      window.__TAURI__ = {
        core: { invoke: async (c, a) => ({ state, settings: state, update_check: { current: '1.1.0', available: true, version: '9.9.9', url: 'x', size: 1 }, events: null,
          call: { 'servers.list': servers, 'settings.get': { quickCommands: [] }, 'servers.log': [], 'servers.playerStats': [{ name: 'Steve', total: 7200e3, sessions: 3, last: Date.now(), online: false }],
          'players.data': { running: false, gameMode: 0, health: 15, food: 20, xpLevel: 12, xpP: 0.3, equipment: {}, ender: [], inventory: [{ slot: 0, id: 'minecraft:diamond_sword', count: 1, enchanted: true, enchants: [{ id: 'minecraft:sharpness', lvl: 5 }] }] } }[a?.cmd] ?? {} })[c] },
        event: { listen: async () => () => {} }, opener: { openUrl: async () => {} },
      };
      localStorage.removeItem('mcm-upd-at');
    }, { servers: SERVERS });
    await serve(page, 'mobile/ui', {});
    await page.goto('http://app.test/');
    await expect(page.locator('.srv')).toHaveCount(2);
    await expect(page.locator('.s-ico img')).toHaveCount(1);
    await expect(page.locator('.upd')).toBeVisible();
    // игроки: статистика и инвентарь (только просмотр)
    await page.locator('.srv').first().click();
    await page.locator('#tabs [data-t="players"]').click();
    await expect(page.locator('.pst')).toHaveCount(1);
    await page.locator('.pst').click();
    await expect(page.locator('.mc-slot')).toHaveCount(5 + 36 + 27);
    await page.locator('.mc-slot.ench').click();
    await expect(page.locator('#pv-info')).toContainText('Алмазный меч');
    await expect(page.locator('#pv-info')).toContainText('Острота V');
    await page.waitForTimeout(800); await page.screenshot({ path: 'test-results/mobile-player.png' });
    await page.getByRole('button', { name: 'Закрыть' }).click();
    await page.locator('.m-nav a').nth(1).click();
    await expect(page.locator('#st-theme button')).toHaveCount(4);
    expect(errs).toEqual([]);
  });
});

test.describe('Коллектор', () => {
  test('главная и страница сборки', async ({ page }) => {
    const errs = errorsOf(page);
    const pack = { id: 'k1', name: 'Тест', mc: '1.21.1', loader: 'fabric', version: '1.0.0', author: '', description: '', loaderVersion: '', mods: [] };
    await serve(page, 'collector/ui', { 'settings.get': { theme: 'calm' }, 'system.info': { version: 'test' }, jobs: [], 'packs.list': [{ ...pack, count: 0 }], 'packs.get': pack, 'catalog.providers': [] });
    await page.goto('http://app.test/#pack/k1');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'calm');
    await expect(page.getByRole('button', { name: /Поделиться/ })).toBeVisible();
    await expect(page.getByRole('button', { name: /Совместимость/ })).toBeVisible();
    expect(errs).toEqual([]);
  });
});
