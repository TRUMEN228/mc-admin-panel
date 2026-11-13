//! Перенос питомцев на другой ник: в файлах чанков (Anvil .mca) меняем UUID хозяина у прирученных животных.
//! Поддерживаются все форматы: IntArray[4] (1.16+), строка (OwnerUUID, до 1.16) и пара OwnerUUIDMost/Least (очень старые).
use crate::core::{Ctx, Res};
use fastnbt::Value as Nbt;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// UUID «xxxxxxxx-xxxx-…» → 16 байт.
pub fn uuid_bytes(u: &str) -> Option<[u8; 16]> {
    let h = hex::decode(u.replace('-', "")).ok()?;
    h.try_into().ok()
}
fn uuid_ints(b: &[u8; 16]) -> [i32; 4] { std::array::from_fn(|i| i32::from_be_bytes(b[i * 4..i * 4 + 4].try_into().unwrap())) }
fn uuid_longs(b: &[u8; 16]) -> (i64, i64) { (i64::from_be_bytes(b[..8].try_into().unwrap()), i64::from_be_bytes(b[8..].try_into().unwrap())) }

/// Замена UUID во всём NBT: возвращает число замен.
pub fn replace_uuid(v: &mut Nbt, old: &[u8; 16], new: &[u8; 16], old_s: &str, new_s: &str) -> usize {
    let (oi, ni) = (uuid_ints(old), uuid_ints(new));
    let ((om, ol), (nm, nl)) = (uuid_longs(old), uuid_longs(new));
    fn walk(v: &mut Nbt, oi: &[i32; 4], ni: &[i32; 4], om: i64, ol: i64, nm: i64, nl: i64, os: &str, ns: &str) -> usize {
        match v {
            Nbt::IntArray(a) if a.len() == 4 && a.iter().copied().eq(oi.iter().copied()) => { *v = Nbt::IntArray(fastnbt::IntArray::new(ni.to_vec())); 1 }
            Nbt::String(s) if s.eq_ignore_ascii_case(os) => { *s = ns.to_string(); 1 }
            Nbt::List(l) => l.iter_mut().map(|x| walk(x, oi, ni, om, ol, nm, nl, os, ns)).sum(),
            Nbt::Compound(m) => {
                let mut n = 0;
                // пары …Most/…Least (OwnerUUIDMost/OwnerUUIDLeast и т. п.)
                let keys: Vec<String> = m.keys().filter(|k| k.ends_with("Most")).cloned().collect();
                for k in keys {
                    let base = &k[..k.len() - 4];
                    let lk = format!("{base}Least");
                    if let (Some(Nbt::Long(a)), Some(Nbt::Long(b))) = (m.get(&k), m.get(&lk)) {
                        if *a == om && *b == ol { m.insert(k.clone(), Nbt::Long(nm)); m.insert(lk, Nbt::Long(nl)); n += 1; }
                    }
                }
                n + m.values_mut().map(|x| walk(x, oi, ni, om, ol, nm, nl, os, ns)).sum::<usize>()
            }
            _ => 0,
        }
    }
    walk(v, &oi, &ni, om, ol, nm, nl, old_s, new_s)
}

fn find(hay: &[u8], needle: &[u8]) -> bool { !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle) }

fn decompress(kind: u8, data: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![];
    match kind {
        1 => flate2::read::GzDecoder::new(data).read_to_end(&mut out).ok()?,
        2 => flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).ok()?,
        3 => { out.extend_from_slice(data); 0 }
        _ => return None, // LZ4 и внешние .mcc не трогаем
    };
    Some(out)
}

/// Один файл региона: меняет UUID хозяина во всех чанках. Возвращает число замен (0 — файл не менялся).
/// Перед записью кладёт копию файла в backup (с тем же относительным путём).
pub fn process_region(path: &Path, old: &str, new: &str, backup: &Path) -> Res<usize> {
    let raw = std::fs::read(path).ctx("Чтение региона")?;
    if raw.len() < 8192 { return Ok(0); }
    let (ob, nb) = (uuid_bytes(old).ok_or("Неверный UUID")?, uuid_bytes(new).ok_or("Неверный UUID")?);
    let (old_l, new_l) = (old.to_lowercase(), new.to_lowercase());
    let mut chunks: Vec<Option<(u8, Vec<u8>)>> = vec![None; 1024];
    let mut total = 0;
    for i in 0..1024 {
        let e = &raw[i * 4..i * 4 + 4];
        let (off, cnt) = (((e[0] as usize) << 16 | (e[1] as usize) << 8 | e[2] as usize) * 4096, e[3] as usize);
        if off == 0 || cnt == 0 || off + 5 > raw.len() { continue; }
        let len = u32::from_be_bytes(raw[off..off + 4].try_into().unwrap()) as usize;
        if len == 0 || off + 4 + len > raw.len() { continue; }
        let kind = raw[off + 4];
        let data = raw[off + 5..off + 4 + len].to_vec();
        let mut keep = (kind, data);
        // быстрый фильтр по байтам UUID (первые 8 байт совпадают и для IntArray, и для пары Most/Least) и по строке
        if let Some(plain) = decompress(kind, &keep.1) {
            if find(&plain, &ob[..8]) || find(&plain, old_l.as_bytes()) {
                if let Ok(mut nbt) = fastnbt::from_bytes::<Nbt>(&plain) {
                    let n = replace_uuid(&mut nbt, &ob, &nb, &old_l, &new_l);
                    if n > 0 {
                        let bytes = fastnbt::to_bytes(&nbt).map_err(|e| format!("Запись чанка: {e}"))?;
                        let mut z = flate2::write::ZlibEncoder::new(vec![], flate2::Compression::default());
                        z.write_all(&bytes).ctx("Сжатие чанка")?;
                        keep = (2, z.finish().ctx("Сжатие чанка")?);
                        total += n;
                    }
                }
            }
        }
        chunks[i] = Some(keep);
    }
    if total == 0 { return Ok(0); }
    // резервная копия и новый файл: заголовок 8 КБ, чанки подряд, каждый выровнен по 4 КБ
    std::fs::create_dir_all(backup.parent().unwrap()).ctx("Резервная копия")?;
    std::fs::copy(path, backup).ctx("Резервная копия")?;
    let mut out = vec![0u8; 8192];
    out[4096..8192].copy_from_slice(&raw[4096..8192]); // времена изменения чанков
    for (i, c) in chunks.iter().enumerate() {
        let Some((kind, data)) = c else { continue };
        let sector = out.len() / 4096;
        let mut body = ((data.len() + 1) as u32).to_be_bytes().to_vec();
        body.push(*kind);
        body.extend_from_slice(data);
        let sectors = body.len().div_ceil(4096);
        if sectors > 255 { return Err("Слишком большой чанк — регион не изменён".into()); }
        body.resize(sectors * 4096, 0);
        out[i * 4..i * 4 + 4].copy_from_slice(&[(sector >> 16) as u8, (sector >> 8) as u8, sector as u8, sectors as u8]);
        out.extend_from_slice(&body);
    }
    let tmp = path.with_extension("mca.tmp");
    std::fs::write(&tmp, out).ctx("Запись региона")?;
    std::fs::rename(&tmp, path).ctx("Запись региона")?;
    Ok(total)
}

/// Файлы регионов мира, где живут сущности: entities/ (1.17+), иначе region/ (старые миры), во всех измерениях.
pub fn entity_regions(world: &Path) -> Vec<PathBuf> {
    let mut dims: HashMap<PathBuf, (Vec<PathBuf>, Vec<PathBuf>)> = HashMap::new();
    for e in walkdir::WalkDir::new(world).into_iter().flatten() {
        let p = e.path();
        if !p.extension().is_some_and(|x| x == "mca") { continue; }
        let Some(parent) = p.parent() else { continue };
        let Some(dim) = parent.parent() else { continue };
        let slot = dims.entry(dim.to_path_buf()).or_default();
        match parent.file_name().and_then(|n| n.to_str()) { Some("entities") => slot.0.push(p.to_path_buf()), Some("region") => slot.1.push(p.to_path_buf()), _ => {} }
    }
    dims.into_values().flat_map(|(ent, reg)| if ent.is_empty() { reg } else { ent }).collect()
}
