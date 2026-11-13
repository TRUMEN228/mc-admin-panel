//! Данные игрока (world/playerdata/<uuid>.dat, сжатый NBT): просмотр и правка инвентаря, эндер-сундука,
//! брони, здоровья, голода, опыта и режима игры. Править можно только при остановленном сервере.
use crate::core::{Ctx, Res};
use crate::servers;
use fastnbt::Value as Nbt;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Формат предметов 1.20.5+ («count» Int и компоненты) против старого («Count» Byte и «tag»).
const DV_COMPONENTS: i32 = 3837;
const ARMOR_SLOTS: [(&str, i8); 5] = [("feet", 100), ("legs", 101), ("chest", 102), ("head", 103), ("offhand", -106)];

/// Файл игрока в активном мире (level-name), а не в первом попавшемся.
fn dat_path(s: &servers::Server, uuid: &str) -> Option<PathBuf> {
    let p = servers::world_dir(s).join("playerdata").join(format!("{uuid}.dat"));
    p.exists().then_some(p)
}
pub fn load(p: &Path) -> Res<Nbt> {
    let raw = std::fs::read(p).ctx("Чтение данных игрока")?;
    let bytes = if raw.starts_with(&[0x1f, 0x8b]) {
        let mut out = vec![];
        flate2::read::GzDecoder::new(&raw[..]).read_to_end(&mut out).ctx("Распаковка данных игрока")?;
        out
    } else { raw };
    fastnbt::from_bytes(&bytes).map_err(|e| format!("Не удалось разобрать данные игрока: {e}"))
}
pub fn store(p: &Path, v: &Nbt) -> Res<()> {
    let bytes = fastnbt::to_bytes(v).map_err(|e| format!("Запись NBT: {e}"))?;
    let mut gz = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
    gz.write_all(&bytes).ctx("Сжатие")?;
    let out = gz.finish().ctx("Сжатие")?;
    // копия прежнего файла рядом — на случай, если что-то пошло не так
    let _ = std::fs::copy(p, p.with_extension("dat.mcpanel-bak"));
    let tmp = p.with_extension("dat.tmp");
    std::fs::write(&tmp, out).ctx("Запись данных игрока")?;
    std::fs::rename(&tmp, p).ctx("Запись данных игрока")
}

fn to_json(v: &Nbt) -> Value {
    match v {
        Nbt::Byte(x) => json!(x), Nbt::Short(x) => json!(x), Nbt::Int(x) => json!(x), Nbt::Long(x) => json!(x),
        Nbt::Float(x) => json!(x), Nbt::Double(x) => json!(x), Nbt::String(x) => json!(x),
        Nbt::List(l) => Value::Array(l.iter().map(to_json).collect()),
        Nbt::Compound(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), to_json(v))).collect::<Map<_, _>>()),
        _ => Value::Null,
    }
}
fn num(v: Option<&Nbt>) -> f64 {
    match v { Some(Nbt::Byte(x)) => *x as f64, Some(Nbt::Short(x)) => *x as f64, Some(Nbt::Int(x)) => *x as f64, Some(Nbt::Long(x)) => *x as f64,
        Some(Nbt::Float(x)) => *x as f64, Some(Nbt::Double(x)) => *x, _ => 0.0 }
}
fn comp(v: &Nbt) -> Option<&HashMap<String, Nbt>> { if let Nbt::Compound(m) = v { Some(m) } else { None } }
fn comp_mut(v: &mut Nbt) -> Option<&mut HashMap<String, Nbt>> { if let Nbt::Compound(m) = v { Some(m) } else { None } }

/// Предмет для интерфейса: слот, id, количество, своё имя, зачарован ли.
fn item_json(it: &Nbt, slot: Value) -> Value {
    let m = comp(it).cloned().unwrap_or_default();
    let id = if let Some(Nbt::String(s)) = m.get("id") { s.clone() } else { String::new() };
    let count = num(m.get("count").or(m.get("Count"))).max(1.0) as i64;
    let comps = m.get("components").and_then(comp);
    let tag = m.get("tag").and_then(comp);
    let display = tag.and_then(|t| t.get("display")).and_then(comp);
    let c = |k: &str| comps.and_then(|c| c.get(&format!("minecraft:{k}")));
    let name_nbt = c("custom_name").or(display.and_then(|d| d.get("Name")));
    let name = name_nbt.map(text).filter(|s| !s.is_empty());
    let name_color = name_nbt.and_then(text_color);
    // редкость, заданная предмету явно (1.20.5+); иначе интерфейс берёт её по типу предмета
    let rarity = match c("rarity") { Some(Nbt::String(r)) => Some(r.clone()), _ => None };
    let lore: Vec<String> = match c("lore").or(display.and_then(|d| d.get("Lore"))) { Some(Nbt::List(l)) => l.iter().map(text).collect(), _ => vec![] };
    let enchants = c("enchantments").map(enchant_list).unwrap_or_else(|| tag.and_then(|t| t.get("Enchantments")).map(enchant_list).unwrap_or_default());
    let stored = c("stored_enchantments").map(enchant_list).unwrap_or_else(|| tag.and_then(|t| t.get("StoredEnchantments")).map(enchant_list).unwrap_or_default());
    let damage = c("damage").or(tag.and_then(|t| t.get("Damage"))).map(|v| num(Some(v)) as i64).filter(|d| *d > 0);
    let max_damage = c("max_damage").map(|v| num(Some(v)) as i64);
    let unbreakable = c("unbreakable").is_some() || tag.is_some_and(|t| num(t.get("Unbreakable")) > 0.0);
    let potion = c("potion_contents").and_then(|p| match p { Nbt::String(s) => Some(s.clone()), Nbt::Compound(m) => match m.get("potion") { Some(Nbt::String(s)) => Some(s.clone()), _ => None }, _ => None })
        .or_else(|| match tag.and_then(|t| t.get("Potion")) { Some(Nbt::String(s)) => Some(s.clone()), _ => None });
    // содержимое шалкеров и других контейнеров
    let contents: Vec<Value> = match (c("container"), c("bundle_contents")) {
        (Some(Nbt::List(l)), _) => l.iter().filter_map(|e| comp(e)).filter_map(|e| e.get("item").map(|it| item_json(it, json!(num(e.get("slot")) as i64)))).collect(),
        // мешок: просто список предметов
        (_, Some(Nbt::List(l))) => l.iter().enumerate().map(|(i, it)| item_json(it, json!(i))).collect(),
        _ => match tag.and_then(|t| t.get("BlockEntityTag")).and_then(comp).and_then(|b| b.get("Items")).or(tag.and_then(|t| t.get("Items"))) {
            Some(Nbt::List(l)) => l.iter().map(|it| item_json(it, json!(num(comp(it).and_then(|c| c.get("Slot"))) as i64))).collect(),
            _ => vec![],
        },
    };
    let ench = !enchants.is_empty() || !stored.is_empty() || matches!(c("enchantment_glint_override"), Some(Nbt::Byte(1)));
    // всё остальное — как есть, для раздела «Все данные»
    let raw = comps.map(|c| to_json(&Nbt::Compound(c.clone()))).or(tag.map(|t| to_json(&Nbt::Compound(t.clone()))));
    json!({ "slot": slot, "id": id, "count": count, "name": name, "lore": lore, "enchanted": ench, "enchants": enchants, "stored": stored,
        "nameColor": name_color, "rarity": rarity, "damage": damage, "maxDamage": max_damage, "unbreakable": unbreakable, "potion": potion, "contents": contents, "raw": raw })
}
/// Текст из текстового компонента: строка, JSON-строка ('{"text":"…","extra":[…]}') или NBT-составной (1.21.5+).
fn text(v: &Nbt) -> String {
    fn walk(j: &Value, out: &mut String) {
        match j {
            Value::String(s) => out.push_str(s),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => {
                if let Some(t) = o.get("text") { walk(t, out); }
                // ключ перевода (например, filled_map.buried_treasure) — переведёт интерфейс по языковому файлу игры
                if let Some(t) = o.get("translate").and_then(|t| t.as_str()) { if !o.contains_key("text") { out.push_str(&format!("⟦{t}⟧")); } }
                if let Some(e) = o.get("extra") { walk(e, out); }
            }
            _ => {}
        }
    }
    let j = match v { Nbt::String(s) => serde_json::from_str::<Value>(s).unwrap_or(Value::String(s.clone())), other => to_json(other) };
    let mut out = String::new();
    walk(&j, &mut out);
    out
}
/// Цвет имени из текстового компонента (как задан командой или плагином).
fn text_color(v: &Nbt) -> Option<String> {
    let j = match v { Nbt::String(s) => serde_json::from_str::<Value>(s).ok()?, other => to_json(other) };
    let pick = |o: &Value| o.get("color").and_then(|c| c.as_str()).map(String::from);
    pick(&j).or_else(|| j.get("extra").and_then(|e| e.as_array()).and_then(|a| a.iter().find_map(pick)))
        .or_else(|| j.as_array().and_then(|a| a.iter().find_map(pick)))
}
/// Зачарования: {levels:{id:lvl}} (1.20.5–1.21.4), {id:lvl} (1.21.5+) или [{id, lvl}] (до 1.20.5).
fn enchant_list(v: &Nbt) -> Vec<Value> {
    match v {
        Nbt::List(l) => l.iter().filter_map(comp).map(|e| json!({ "id": match e.get("id") { Some(Nbt::String(s)) => s.clone(), _ => String::new() }, "lvl": num(e.get("lvl")) as i64 })).collect(),
        Nbt::Compound(m) => {
            let levels = m.get("levels").and_then(comp).unwrap_or(m);
            let mut l: Vec<Value> = levels.iter().filter(|(k, _)| k.contains(':') || k.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
                .filter(|(_, v)| !matches!(v, Nbt::Compound(_) | Nbt::List(_))).map(|(k, v)| json!({ "id": k, "lvl": num(Some(v)) as i64 })).collect();
            l.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
            l
        }
        _ => vec![],
    }
}

async fn locate(id: &str, name: &str) -> Res<(PathBuf, String)> {
    let s = servers::get(id)?;
    let props = servers::read_props(id)?;
    let online = props["online-mode"].as_str() != Some("false");
    let uuid = crate::tools::uuid_of(&s, name.trim(), online).await?;
    let p = dat_path(&s, &uuid).ok_or(format!("У игрока «{}» нет сохранённых данных на этом сервере", name.trim()))?;
    Ok((p, uuid))
}

/// Всё, что показывает окно игрока.
pub async fn get(id: &str, name: &str) -> Res<Value> {
    let (p, uuid) = locate(id, name).await?;
    let v = load(&p)?;
    let m = comp(&v).ok_or("Повреждённые данные игрока")?;
    let dv = num(m.get("DataVersion")) as i32;
    let list = |k: &str| match m.get(k) { Some(Nbt::List(l)) => l.clone(), _ => vec![] };
    let mut inv = vec![];
    let mut equip = Map::new();
    let has_equipment = m.contains_key("equipment");
    for it in list("Inventory") {
        let slot = num(comp(&it).and_then(|c| c.get("Slot"))) as i8;
        if let Some((k, _)) = ARMOR_SLOTS.iter().find(|(_, s)| *s == slot) { equip.insert(k.to_string(), with_orig(item_json(&it, json!(k)), format!("equipment:{k}"))); }
        else { inv.push(with_orig(item_json(&it, json!(slot)), format!("inventory:{slot}"))); }
    }
    if let Some(e) = m.get("equipment").and_then(comp) {
        for (k, it) in e { equip.insert(k.clone(), with_orig(item_json(it, json!(k)), format!("equipment:{k}"))); }
    }
    let ender: Vec<Value> = list("EnderItems").iter().map(|it| { let sl = num(comp(it).and_then(|c| c.get("Slot"))) as i8; with_orig(item_json(it, json!(sl)), format!("ender:{sl}")) }).collect();
    let pos: Vec<f64> = list("Pos").iter().map(|x| num(Some(x))).collect();
    Ok(json!({
        "uuid": uuid, "dataVersion": dv, "components": dv >= DV_COMPONENTS, "equipmentTag": has_equipment,
        "running": servers::is_running(id), "online": servers::online_players(id).iter().any(|n| n.eq_ignore_ascii_case(name.trim())),
        "inventory": inv, "equipment": equip, "ender": ender,
        "health": num(m.get("Health")), "food": num(m.get("foodLevel")), "saturation": num(m.get("foodSaturationLevel")),
        "xpLevel": num(m.get("XpLevel")), "xpP": num(m.get("XpP")), "gameMode": num(m.get("playerGameType")),
        "pos": pos, "dimension": m.get("Dimension").map(to_json),
    }))
}

fn with_orig(mut v: Value, orig: String) -> Value { v["orig"] = json!(orig); v }
/// Все предметы игрока по месту: «inventory:5», «ender:0», «equipment:head».
fn item_pool(m: &HashMap<String, Nbt>) -> HashMap<String, Nbt> {
    let mut pool = HashMap::new();
    if let Some(Nbt::List(l)) = m.get("Inventory") {
        for it in l {
            let slot = num(comp(it).and_then(|c| c.get("Slot"))) as i8;
            let key = match ARMOR_SLOTS.iter().find(|(_, s)| *s == slot) { Some((k, _)) => format!("equipment:{k}"), None => format!("inventory:{slot}") };
            pool.insert(key, it.clone());
        }
    }
    if let Some(Nbt::List(l)) = m.get("EnderItems") { for it in l { pool.insert(format!("ender:{}", num(comp(it).and_then(|c| c.get("Slot"))) as i8), it.clone()); } }
    if let Some(Nbt::Compound(e)) = m.get("equipment") { for (k, it) in e { pool.insert(format!("equipment:{k}"), it.clone()); } }
    pool
}
/// Новый список предметов. Свойства (зачарования, имя и т. п.) берутся у исходного предмета: по orig (перетащенный)
/// или по тому же месту с тем же id; меняются только место и количество, а у правленых — и свойства.
fn rebuild(pool: &HashMap<String, Nbt>, container: &str, new: &[Value], slot_key: &str, dv: i32) -> Vec<Nbt> {
    let components = dv >= DV_COMPONENTS;
    new.iter().filter_map(|n| {
        let id = n["id"].as_str().filter(|s| !s.is_empty())?;
        let id = if id.contains(':') { id.to_string() } else { format!("minecraft:{id}") };
        let count = n["count"].as_i64().unwrap_or(1).clamp(1, 99);
        let slot = &n["slot"];
        let here = format!("{container}:{}", slot.as_i64().map(|x| x.to_string()).or(slot.as_str().map(String::from)).unwrap_or_default());
        let same_id = |o: &&Nbt| matches!(comp(o).and_then(|c| c.get("id")), Some(Nbt::String(s)) if *s == id);
        let base = n["orig"].as_str().and_then(|k| pool.get(k)).or_else(|| pool.get(&here).filter(same_id));
        let mut it = base.cloned().unwrap_or_else(|| Nbt::Compound(HashMap::new()));
        let c = comp_mut(&mut it)?;
        c.insert("id".into(), Nbt::String(id));
        if components { c.remove("Count"); c.insert("count".into(), Nbt::Int(count as i32)); } else { c.remove("count"); c.insert("Count".into(), Nbt::Byte(count as i8)); }
        if let Some(s) = slot.as_i64() { c.insert(slot_key.into(), Nbt::Byte(s as i8)); }
        // свойства меняем только у предметов, которые правили в окне (иначе форматирование имён сохраняется как есть)
        if n["dirty"] == true { apply_details(c, n, dv); }
        Some(it)
    }).collect()
}

/// Сохранить правки: inventory/ender — полный список предметов, equipment — {head, chest, legs, feet, offhand}, плюс характеристики.
pub async fn save(id: &str, name: &str, patch: &Value) -> Res<Value> {
    if servers::is_running(id) { crate::bail!("Остановите сервер: при выходе игрока или сохранении мира он перезапишет изменения"); }
    let (p, _) = locate(id, name).await?;
    let mut v = load(&p)?;
    let m = comp_mut(&mut v).ok_or("Повреждённые данные игрока")?;
    let dv = num(m.get("DataVersion")) as i32;
    let has_equipment = m.contains_key("equipment");
    let pool = item_pool(m);
    if let Some(inv) = patch["inventory"].as_array() {
        let mut items = inv.clone();
        // броня в старом формате живёт в инвентаре (слоты 100–103 и −106)
        if !has_equipment {
            if let Some(eq) = patch["equipment"].as_object() {
                for (k, s) in ARMOR_SLOTS { if let Some(it) = eq.get(k).filter(|x| x["id"].as_str().is_some_and(|s| !s.is_empty())) { let mut it = it.clone(); it["slot"] = json!(s); items.push(it); } }
            }
        }
        // броня старого формата в пуле лежит как «equipment:…», остальное — «inventory:…»
        let (armor, rest): (Vec<Value>, Vec<Value>) = items.into_iter().partition(|x| x["slot"].as_i64().is_some_and(|s| ARMOR_SLOTS.iter().any(|(_, a)| *a as i64 == s)));
        let armor: Vec<Value> = armor.into_iter().map(|mut x| { let s = x["slot"].as_i64().unwrap_or(0); if let Some((k, _)) = ARMOR_SLOTS.iter().find(|(_, a)| *a as i64 == s) { x["equip"] = json!(k); } x }).collect();
        let mut list = rebuild(&pool, "inventory", &rest, "Slot", dv);
        for x in &armor {
            let mut one = x.clone(); one["slot"] = json!(x["equip"].as_str().unwrap_or(""));
            if let Some(mut n) = rebuild(&pool, "equipment", &[one], "Slot", dv).pop() { if let Some(c) = comp_mut(&mut n) { c.insert("Slot".into(), Nbt::Byte(x["slot"].as_i64().unwrap_or(0) as i8)); } list.push(n); }
        }
        m.insert("Inventory".into(), Nbt::List(list));
    }
    if has_equipment {
        if let Some(eq) = patch["equipment"].as_object() {
            let mut out = HashMap::new();
            for (k, it) in eq {
                let mut one = it.clone(); one["slot"] = json!(k);
                if let Some(mut n) = rebuild(&pool, "equipment", &[one], "Slot", dv).pop() { if let Some(c) = comp_mut(&mut n) { c.remove("Slot"); } out.insert(k.clone(), n); }
            }
            m.insert("equipment".into(), Nbt::Compound(out));
        }
    }
    if let Some(en) = patch["ender"].as_array() {
        m.insert("EnderItems".into(), Nbt::List(rebuild(&pool, "ender", en, "Slot", dv)));
    }
    if let Some(x) = patch["health"].as_f64() { m.insert("Health".into(), Nbt::Float(x.clamp(0.0, 1024.0) as f32)); }
    if let Some(x) = patch["food"].as_i64() { m.insert("foodLevel".into(), Nbt::Int(x.clamp(0, 20) as i32)); }
    if let Some(x) = patch["xpLevel"].as_i64() { m.insert("XpLevel".into(), Nbt::Int(x.clamp(0, 100_000) as i32)); }
    if let Some(x) = patch["gameMode"].as_i64() { m.insert("playerGameType".into(), Nbt::Int(x.clamp(0, 3) as i32)); }
    store(&p, &v)?;
    get(id, name).await
}

/// Начиная с 1.21.5 тексты и зачарования хранятся без обёрток.
const DV_1215: i32 = 4325;
/// Имя, описание, зачарования, износ и «неразрушимый» — в формате версии мира.
fn apply_details(c: &mut HashMap<String, Nbt>, n: &Value, dv: i32) {
    let s = |v: &Value| v.as_str().map(str::trim).filter(|x| !x.is_empty()).map(String::from);
    let txt = |t: &str| if dv >= DV_1215 { Nbt::String(t.into()) } else { Nbt::String(json!({ "text": t, "italic": false }).to_string()) };
    let name = s(&n["name"]);
    let lore: Vec<String> = n["lore"].as_array().map(|a| a.iter().filter_map(s).collect()).unwrap_or_default();
    let ench = |key: &str| -> Vec<(String, i32)> { n[key].as_array().map(|a| a.iter().filter_map(|e| {
        let id = s(&e["id"])?; Some((if id.contains(':') { id } else { format!("minecraft:{id}") }, e["lvl"].as_i64().unwrap_or(1).clamp(1, 255) as i32))
    }).collect()).unwrap_or_default() };
    let (enchants, stored) = (ench("enchants"), ench("stored"));
    let damage = n["damage"].as_i64().unwrap_or(0).max(0) as i32;
    let unbreakable = n["unbreakable"] == true;
    if dv >= DV_COMPONENTS {
        let comps = match c.entry("components".into()).or_insert_with(|| Nbt::Compound(HashMap::new())) { Nbt::Compound(m) => m, _ => return };
        let mut set = |k: &str, v: Option<Nbt>| { let k = format!("minecraft:{k}"); match v { Some(v) => { comps.insert(k, v); } None => { comps.remove(&k); } } };
        set("custom_name", name.as_deref().map(txt));
        set("lore", (!lore.is_empty()).then(|| Nbt::List(lore.iter().map(|l| txt(l)).collect())));
        let ench_nbt = |l: &[(String, i32)]| -> Option<Nbt> {
            if l.is_empty() { return None; }
            let lv: HashMap<String, Nbt> = l.iter().map(|(k, v)| (k.clone(), Nbt::Int(*v))).collect();
            Some(if dv >= DV_1215 { Nbt::Compound(lv) } else { Nbt::Compound(HashMap::from([("levels".to_string(), Nbt::Compound(lv))])) })
        };
        set("enchantments", ench_nbt(&enchants));
        set("stored_enchantments", ench_nbt(&stored));
        set("damage", (damage > 0).then_some(Nbt::Int(damage)));
        set("unbreakable", unbreakable.then(|| Nbt::Compound(HashMap::new())));
        if comps.is_empty() { c.remove("components"); }
    } else {
        let tag = match c.entry("tag".into()).or_insert_with(|| Nbt::Compound(HashMap::new())) { Nbt::Compound(m) => m, _ => return };
        let disp = match tag.entry("display".into()).or_insert_with(|| Nbt::Compound(HashMap::new())) { Nbt::Compound(m) => m, _ => return };
        match &name { Some(t) => { disp.insert("Name".into(), txt(t)); } None => { disp.remove("Name"); } }
        if lore.is_empty() { disp.remove("Lore"); } else { disp.insert("Lore".into(), Nbt::List(lore.iter().map(|l| txt(l)).collect())); }
        if disp.is_empty() { tag.remove("display"); }
        let ench_list = |l: &[(String, i32)]| Nbt::List(l.iter().map(|(k, v)| Nbt::Compound(HashMap::from([("id".to_string(), Nbt::String(k.clone())), ("lvl".to_string(), Nbt::Short(*v as i16))]))).collect());
        if enchants.is_empty() { tag.remove("Enchantments"); } else { tag.insert("Enchantments".into(), ench_list(&enchants)); }
        if stored.is_empty() { tag.remove("StoredEnchantments"); } else { tag.insert("StoredEnchantments".into(), ench_list(&stored)); }
        if damage > 0 { tag.insert("Damage".into(), Nbt::Int(damage)); } else { tag.remove("Damage"); }
        if unbreakable { tag.insert("Unbreakable".into(), Nbt::Byte(1)); } else { tag.remove("Unbreakable"); }
        if tag.is_empty() { c.remove("tag"); }
    }
}
