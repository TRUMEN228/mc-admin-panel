//! Предупреждения: лаги (TPS), мало памяти, мало места на диске.
//! Уходят событием «alert» (лента, тост, телефон) и уведомлением Windows; одно и то же — не чаще раза в 15 минут.
use crate::core::{emit, now_ms};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::json;
use std::collections::HashMap;

static LAST: Lazy<Mutex<HashMap<String, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));
const COOLDOWN: u64 = 15 * 60 * 1000;

pub fn raise(key: &str, kind: &str, id: Option<&str>, title: &str, text: &str) {
    let now = now_ms();
    {
        let mut m = LAST.lock();
        if m.get(key).is_some_and(|t| now.saturating_sub(*t) < COOLDOWN) { return; }
        m.insert(key.to_string(), now);
    }
    emit(json!({ "type": "alert", "kind": kind, "id": id, "title": title, "text": text }));
    crate::native::notify(title, text);
}
