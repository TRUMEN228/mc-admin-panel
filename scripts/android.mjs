// Команды Tauri для Android-приложения с нужными путями: node scripts/android.mjs <init|dev|build> [...]
import { execSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const sdk = process.env.ANDROID_HOME || join(process.env.LOCALAPPDATA || '', 'Android', 'Sdk');
const ndkDir = join(sdk, 'ndk');
const ndk = process.env.NDK_HOME || (existsSync(ndkDir) ? join(ndkDir, readdirSync(ndkDir).sort().pop()) : '');
const java = process.env.JAVA_HOME || ['C:\\Program Files\\Java\\jdk-21.0.12', 'C:\\Program Files\\Java\\jdk-17'].find(existsSync);
const env = { ...process.env, ANDROID_HOME: sdk, NDK_HOME: ndk, JAVA_HOME: java, TAURI_APP_PATH: 'mobile/src-tauri' };
const root = join(import.meta.dirname, '..');
execSync(`npx tauri android ${process.argv.slice(2).join(' ')}`, { cwd: root, stdio: 'inherit', env });
