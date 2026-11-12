// Сборка релиза: установщик NSIS + портативный exe в папку release/.
import { execSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const { version } = JSON.parse(readFileSync(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'));
const out = join(root, 'release');

execSync('npx tauri build', { cwd: root, stdio: 'inherit' });

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const rel = join(root, 'src-tauri', 'target', 'release');
const setup = `MC Admin Panel ${version} Setup.exe`;
const portable = `MC Admin Panel ${version} Portable.exe`;
copyFileSync(join(rel, 'bundle', 'nsis', `MC Admin Panel_${version}_x64-setup.exe`), join(out, setup));
// тот же exe: портативный режим включается по слову «Portable» в имени файла
copyFileSync(join(rel, 'mc-admin-panel.exe'), join(out, portable));
console.log(`\nГотово:\n  release/${setup}\n  release/${portable}`);
