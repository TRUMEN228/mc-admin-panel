// Сборка релиза: node scripts/release.mjs [panel|collector]
// В папку release/<app>/ кладёт установщик, подпись, портативный exe и latest.json для автообновления.
// Имена: MCAdminPanel-2.1.1-Setup.exe, MCAdminPanel-2.1.1-Portable.exe и т.п.
import { execSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const APPS = {
  panel: { dir: 'src-tauri', exe: 'mc-admin-panel.exe', base: 'MCAdminPanel', tag: (v) => `v${v}` },
  collector: { dir: 'collector/src-tauri', exe: 'mc-mod-collector.exe', base: 'MCModCollector', tag: (v) => `collector-v${v}` },
};
const appName = process.argv[2] || 'panel';
const app = APPS[appName];
if (!app) throw new Error('Неизвестное приложение: ' + appName);

const root = join(import.meta.dirname, '..');
const conf = JSON.parse(readFileSync(join(root, app.dir, 'tauri.conf.json'), 'utf8'));
const { version, productName } = conf;
const repo = process.env.GITHUB_REPOSITORY || 'TRUMEN228/mc-admin-panel';
const out = join(root, 'release', appName);

// ключ подписи обновлений: из переменной окружения или ~/.tauri/mc-admin-panel.key
const env = { ...process.env, TAURI_APP_PATH: app.dir };
const keyFile = join(homedir(), '.tauri', 'mc-admin-panel.key');
if (!env.TAURI_SIGNING_PRIVATE_KEY && existsSync(keyFile)) env.TAURI_SIGNING_PRIVATE_KEY = readFileSync(keyFile, 'utf8').trim();
env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ??= '';
if (!env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Нет ключа подписи обновлений: ' + keyFile);
if (!process.argv.includes('--no-build')) execSync('npx tauri build', { cwd: root, stdio: 'inherit', env });

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const rel = join(root, 'target', 'release');
const nsis = join(rel, 'bundle', 'nsis', `${productName}_${version}_x64-setup.exe`);
const setup = `${app.base}-${version}-Setup.exe`;
const portable = `${app.base}-${version}-Portable.exe`;
copyFileSync(nsis, join(out, setup));
copyFileSync(nsis + '.sig', join(out, setup + '.sig'));
// тот же exe: портативный режим включается по слову «Portable» в имени файла
copyFileSync(join(rel, app.exe), join(out, portable));

const notes = process.env.RELEASE_NOTES || `${productName} ${version}`;
const latest = {
  version, notes, pub_date: new Date().toISOString(),
  platforms: { 'windows-x86_64': { signature: readFileSync(nsis + '.sig', 'utf8').trim(), url: `https://github.com/${repo}/releases/download/${app.tag(version)}/${setup}` } },
};
writeFileSync(join(out, 'latest.json'), JSON.stringify(latest, null, 2));
console.log(`\nГотово (release/${appName}):\n  ${setup}\n  ${portable}\n  latest.json`);
