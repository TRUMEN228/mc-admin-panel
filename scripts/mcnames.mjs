// Русские названия предметов, блоков, зачарований и зелий из официального ru_ru.json Minecraft (ресурсы игры от Mojang).
// node scripts/mcnames.mjs [версия]  →  ui/js/mcnames.js
import fs from 'node:fs';

const ver = process.argv[2] || '1.21.11';
const get = async (u) => { const r = await fetch(u); if (!r.ok) throw new Error(`${r.status} ${u}`); return r.json(); };
const manifest = await get('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json');
const v = manifest.versions.find((x) => x.id === ver) || manifest.versions.find((x) => x.id === manifest.latest.release);
const meta = await get(v.url);
const index = await get(meta.assetIndex.url);
const obj = index.objects['minecraft/lang/ru_ru.json'];
const lang = await get(`https://resources.download.minecraft.net/${obj.hash.slice(0, 2)}/${obj.hash}`);

const out = { item: {}, ench: {}, potion: {}, tr: {} };
for (const [k, t] of Object.entries(lang)) {
  let m;
  if ((m = k.match(/^(?:item|block)\.minecraft\.([a-z0-9_]+)$/))) out.item[m[1]] ??= t;
  else if ((m = k.match(/^enchantment\.minecraft\.([a-z0-9_]+)$/))) out.ench[m[1]] = t;
  else if ((m = k.match(/^item\.minecraft\.potion\.effect\.([a-z0-9_]+)$/))) out.potion[m[1]] = t;
  // ключи, которые игра подставляет в имена предметов (карты сокровищ и т. п.)
  if (/^filled_map\./.test(k)) out.tr[k] = t;
}
fs.writeFileSync('ui/js/mcnames.js', `// Сгенерировано scripts/mcnames.mjs из ru_ru.json Minecraft ${v.id}. Не править вручную.\nwindow.MCNAMES = ${JSON.stringify(out)};\n`);
console.log(v.id, 'items', Object.keys(out.item).length, 'ench', Object.keys(out.ench).length, 'potions', Object.keys(out.potion).length);
