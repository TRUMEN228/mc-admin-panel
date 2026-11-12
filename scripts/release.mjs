// Сборка релиза: установщик NSIS + портативный exe в папку release/.
import { execSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const { version } = JSON.parse(readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const out = join(root, 'release');

// ключ подписи обновлений: из переменной окружения или ~/.tauri/mc-admin-panel.key
const env = { ...process.env };
const keyFile = join(homedir(), '.tauri', 'mc-admin-panel.key');
if (!env.TAURI_SIGNING_PRIVATE_KEY && existsSync(keyFile)) env.TAURI_SIGNING_PRIVATE_KEY = readFileSync(keyFile, 'utf8').trim();
env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD ??= '';
if (!env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('Нет ключа подписи обновлений: ' + keyFile);
execSync('npx tauri build', { cwd: root, stdio: 'inherit', env });

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const rel = join(root, 'src-tauri', 'target', 'release');
const setup = `MC Admin Panel ${version} Setup.exe`;
const portable = `MC Admin Panel ${version} Portable.exe`;
copyFileSync(join(rel, 'bundle', 'nsis', `MC Admin Panel_${version}_x64-setup.exe`), join(out, setup));
// тот же exe: портативный режим включается по слову «Portable» в имени файла
copyFileSync(join(rel, 'mc-admin-panel.exe'), join(out, portable));
console.log(`\nГотово:\n  release/${setup}\n  release/${portable}`);
